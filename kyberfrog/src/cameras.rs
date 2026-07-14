// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enumeration of webcam / video capture devices (Windows DirectShow).
//!
//! Runs the bundled `ffmpeg.exe -f dshow -list_devices true` and parses its
//! stderr. Using ffmpeg (rather than COM) guarantees the reported names are
//! exactly the ones the fork's lavd iosys (libavdevice/dshow) will expose, so
//! the `[kyavserver].camera_device` pin and its CRC identifier always match.

use std::path::{Path, PathBuf};

use log::warn;
use tokio::process::Command;

/// Path to the bundled `ffmpeg` binary inside an install directory.
fn ffmpeg_path(install_dir: &Path) -> PathBuf {
    install_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// List DirectShow video capture device names, in ffmpeg's order. Returns an
/// empty list off Windows, when ffmpeg is missing, or when enumeration fails
/// (the UI then shows its "no camera detected" state).
pub async fn list_cameras(install_dir: &Path) -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }

    let ffmpeg = ffmpeg_path(install_dir);
    let mut command = Command::new(&ffmpeg);
    command.args(["-hide_banner", "-f", "dshow", "-list_devices", "true", "-i", "dummy"]);
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

    // The device list goes to stderr; ffmpeg exits non-zero because the
    // dummy input can't be opened — that is expected.
    parse_dshow_devices(&String::from_utf8_lossy(&output.stderr))
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

#[cfg(test)]
mod tests {
    use super::parse_dshow_devices;

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
}
