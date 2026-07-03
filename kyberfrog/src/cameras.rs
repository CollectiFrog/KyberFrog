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
    let output = Command::new(&ffmpeg)
        .args(["-hide_banner", "-f", "dshow", "-list_devices", "true", "-i", "dummy"])
        .output()
        .await;

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
/// `[dshow @ 000001] "Integrated Camera" (video)`. Audio devices and
/// "Alternative name" lines are skipped.
fn parse_dshow_devices(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.contains("[dshow") && line.trim_end().ends_with("(video)"))
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
    fn empty_on_no_devices() {
        assert!(parse_dshow_devices("dummy: Immediate exit requested\n").is_empty());
    }
}
