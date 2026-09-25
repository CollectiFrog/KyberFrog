// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enumeration of webcam / video capture devices, for the webcam picker.
//!
//! Runs the bundled `ffmpeg` and parses its device list: `-f dshow
//! -list_devices true` on Windows (DirectShow, on stderr), `-sources v4l2` on
//! Linux (on stdout). Using ffmpeg (rather than COM or V4L2 ioctls) guarantees
//! the reported names are exactly the ones the fork's lavd iosys exposes — same
//! FFmpeg build, same device listing — so the `[kyavserver].camera_device` pin
//! and its CRC identifier always match.
//!
//! On Linux that name is the V4L2 card name (`VIDIOC_QUERYCAP` `card`, which
//! lavd hashes into the source id), not the `/dev/videoN` node, which the fork
//! only uses to open the device.

use std::path::{Path, PathBuf};

use log::warn;
use tokio::process::Command;

/// Path to the bundled `ffmpeg` binary inside an install directory.
fn ffmpeg_path(install_dir: &Path) -> PathBuf {
    install_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// How this platform lists its capture devices through ffmpeg.
struct Probe {
    args: &'static [&'static str],
    /// The list lands on stdout (`-sources`), else on stderr (`-list_devices`).
    stdout: bool,
    parse: fn(&str) -> Vec<String>,
}

fn probe() -> Option<Probe> {
    if cfg!(windows) {
        // ffmpeg exits non-zero because the dummy input can't be opened —
        // that is expected, the list is already printed.
        Some(Probe {
            args: &["-hide_banner", "-f", "dshow", "-list_devices", "true", "-i", "dummy"],
            stdout: false,
            parse: parse_dshow_devices,
        })
    } else if cfg!(target_os = "linux") {
        // The v4l2 input has no `-list_devices` option; `-sources` is the
        // generic device listing (`avdevice_list_input_sources`).
        Some(Probe {
            args: &["-hide_banner", "-sources", "v4l2"],
            stdout: true,
            parse: parse_v4l2_sources,
        })
    } else {
        None
    }
}

/// List video capture device names, in ffmpeg's order. Returns an empty list
/// on other platforms, when ffmpeg is missing, or when enumeration fails (the
/// UI then shows its "no camera detected" state).
pub async fn list_cameras(install_dir: &Path) -> Vec<String> {
    let Some(probe) = probe() else {
        return Vec::new();
    };

    let ffmpeg = ffmpeg_path(install_dir);
    let mut command = Command::new(&ffmpeg);
    command.args(probe.args);
    // The bundled ffmpeg loads its libraries from the bundle's lib/ (Linux).
    command.envs(crate::supervisor::child_env(install_dir));
    // No console flash when the camera picker enumerates (kyberfrog itself is
    // a windowless GUI app since #21).
    #[cfg(windows)]
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    let output = command.output().await;

    let output = match output {
        Ok(output) => output,
        Err(err) => {
            warn!("camera enumeration: failed to run {ffmpeg:?}: {err}");
            return Vec::new();
        }
    };

    let list = if probe.stdout { &output.stdout } else { &output.stderr };
    (probe.parse)(&String::from_utf8_lossy(list))
}

/// Extract video device names from `-list_devices` stderr lines, e.g.
/// `[dshow @ 000001] "Integrated Camera" (video)`. The log tag varies with
/// the ffmpeg build (`[dshow @ ...]`, `[in#0 @ ...]`, ...), so match on the
/// `(video)` suffix and exclude "Alternative name" lines instead of the tag.
fn parse_dshow_devices(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| {
            line.trim_end().ends_with("(video)") && !line.contains("Alternative name")
        })
        .filter_map(|line| {
            let start = line.find('"')? + 1;
            let end = line[start..].find('"')? + start;
            Some(line[start..end].to_string())
        })
        .collect()
}

/// Extract card names from `-sources v4l2` stdout, one device per line:
/// `* /dev/video0 [HD Pro Webcam C920] (none)` — `*` marks the default device,
/// else a space. Nodes without a capture capability (a UVC camera's metadata
/// node) are already left out by ffmpeg. Two cards with the same name share
/// one source id and the fork only keeps the first, so a name is listed once.
fn parse_v4l2_sources(stdout: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in stdout.lines() {
        let entry = line.trim_start().trim_start_matches('*').trim_start();
        if !entry.starts_with('/') {
            continue; // header, or "Cannot list sources: ..."
        }
        // The card name may itself hold brackets: take everything between the
        // first " [" (after the node path) and the last "] (" (media types).
        let (Some(start), Some(end)) = (entry.find(" ["), entry.rfind("] (")) else {
            continue;
        };
        let Some(name) = entry.get(start + 2..end) else {
            continue;
        };
        if !name.is_empty() && !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::{parse_dshow_devices, parse_v4l2_sources};

    #[test]
    fn parses_video_devices_only() {
        let stderr = r#"[dshow @ 0000021a] "Integrated Camera" (video)
[dshow @ 0000021a]   Alternative name "@device_pnp_\\?\usb#vid_04f2"
[dshow @ 0000021a] "OBS Virtual Camera" (video)
[dshow @ 0000021a] "Microphone (Realtek(R) Audio)" (audio)
dummy: Immediate exit requested
"#;
        assert_eq!(
            parse_dshow_devices(stderr),
            vec!["Integrated Camera".to_string(), "OBS Virtual Camera".to_string()]
        );
    }

    #[test]
    fn parses_video_devices_with_in_tag_variant() {
        // Some ffmpeg builds log under "[in#0 @ ...]" instead of "[dshow @ ...]".
        let stderr = r#"[in#0 @ 0000024c84137780] "PC-LM1E" (video)
[in#0 @ 0000024c84137780]   Alternative name "@device_pnp_\\?\usb#vid_0c45"
[in#0 @ 0000024c84137780] "Decklink Video Capture" (none)
[in#0 @ 0000024c84137780]   Alternative name "@device_sw_{860BB310}"
[in#0 @ 0000024c84137780] "OBS Virtual Camera" (video)
[in#0 @ 0000024c84137780] "Microphone (PC-LM1E)" (audio)
Error opening input file dummy.
"#;
        assert_eq!(
            parse_dshow_devices(stderr),
            vec!["PC-LM1E".to_string(), "OBS Virtual Camera".to_string()]
        );
    }

    #[test]
    fn empty_on_no_devices() {
        assert!(parse_dshow_devices("dummy: Immediate exit requested\n").is_empty());
    }

    #[test]
    fn v4l2_parses_card_names_not_nodes() {
        let stdout = "Auto-detected sources for v4l2:
* /dev/video0 [HD Pro Webcam C920] (none)
  /dev/video4 [KF Test Cam] (none)
";
        assert_eq!(
            parse_v4l2_sources(stdout),
            vec!["HD Pro Webcam C920".to_string(), "KF Test Cam".to_string()]
        );
    }

    #[test]
    fn v4l2_lists_a_shared_card_name_once() {
        // Two identical cameras hash to the same source id in the fork.
        let stdout = "Auto-detected sources for v4l2:
* /dev/video0 [USB Video] (none)
  /dev/video2 [USB Video] (none)
  /dev/video4 [C790] (none)
";
        assert_eq!(
            parse_v4l2_sources(stdout),
            vec!["USB Video".to_string(), "C790".to_string()]
        );
    }

    #[test]
    fn v4l2_keeps_brackets_inside_a_card_name() {
        let stdout = "  /dev/video0 [Cam [HDMI] (1080p)] (video)\n";
        assert_eq!(parse_v4l2_sources(stdout), vec!["Cam [HDMI] (1080p)".to_string()]);
    }

    #[test]
    fn v4l2_empty_when_listing_fails() {
        let stdout = "Auto-detected sources for v4l2:
Cannot list sources: No such file or directory
";
        assert!(parse_v4l2_sources(stdout).is_empty());
        assert!(parse_v4l2_sources("").is_empty());
    }
}
