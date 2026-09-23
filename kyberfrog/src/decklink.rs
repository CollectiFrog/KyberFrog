// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enumeration of Blackmagic DeckLink capture devices.
//!
//! Runs the bundled `ffmpeg -f decklink -list_devices 1` and parses its stderr,
//! for the same reason [`crate::cameras`] does: the names ffmpeg reports are
//! exactly the ones the fork's lavd iosys will expose, so the
//! `[kyavserver].camera_device` pin — and the device-name CRC derived from it —
//! always match.
//!
//! **The bundled ffmpeg only carries the DeckLink demuxer when it was built
//! with `-Ddecklink=enabled`** (see the licence note in the fork's
//! `meson_options.txt`: that build is nonfree and must not be redistributed).
//! Without it, `-f decklink` fails with "Unknown input format" and this returns
//! an empty list — the UI then shows its "no device detected" state, exactly as
//! it does when no card is installed.
//!
//! Only *inputs* are listed: ffmpeg's `list_devices` for the demuxer queries
//! `IDeckLinkInput` per device, so a half-duplex card (DeckLink Studio 2) whose
//! connectors are currently configured as outputs simply does not appear.

use std::path::{Path, PathBuf};

use log::warn;
use tokio::process::Command;

/// Path to the bundled `ffmpeg` binary inside an install directory.
fn ffmpeg_path(install_dir: &Path) -> PathBuf {
    install_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// List DeckLink capture device names, in ffmpeg's order. Returns an empty list
/// when ffmpeg is missing, was built without the DeckLink demuxer, or when no
/// card answers.
pub async fn list_decklink_inputs(install_dir: &Path) -> Vec<String> {
    let ffmpeg = ffmpeg_path(install_dir);
    let mut command = Command::new(&ffmpeg);
    command.args(["-hide_banner", "-f", "decklink", "-list_devices", "1", "-i", "dummy"]);
    // ffmpeg is dynamically linked against the bundle's own libavdevice/libavcodec/…
    // (lib/<triplet>/ on Linux), which is not on the default loader path — it
    // fails to even start with "error while loading shared libraries" otherwise.
    // The supervisor already builds exactly this PATH/LD_LIBRARY_PATH for
    // kycontroller/kyclient; reuse it here instead of duplicating it.
    #[cfg(unix)]
    command.envs(crate::supervisor::child_env(install_dir));
    // No console flash when the picker enumerates (kyberfrog is a windowless
    // GUI app on Windows since #21).
    #[cfg(windows)]
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);

    let output = match command.output().await {
        Ok(output) => output,
        Err(err) => {
            warn!("decklink enumeration: failed to run {ffmpeg:?}: {err}");
            return Vec::new();
        }
    };

    // The list goes to stderr, and ffmpeg exits non-zero because the dummy
    // input cannot be opened — that is expected.
    parse_decklink_devices(&String::from_utf8_lossy(&output.stderr))
}

/// Extract device names from `-list_devices` stderr. ffmpeg prints one
/// tab-indented `\t'Name'` line per device under a header, e.g.
///
/// ```text
/// [decklink @ 0x55f0] Blackmagic DeckLink devices:
/// [decklink @ 0x55f0]     'DeckLink Mini Recorder'
/// ```
///
/// The log tag varies between builds, so the tag itself is skipped rather than
/// matched, but the **quoted name must be indented** behind it — that is what
/// separates a device line from an error that merely happens to quote
/// something, such as the `Unknown input format: 'decklink'` a redistributable
/// build prints. Without that check the error itself would surface as a phantom
/// device in the picker.
fn parse_decklink_devices(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter_map(|line| {
            let rest = line.trim_end();
            // Drop a leading "[tag @ 0x…]" when there is one; what follows must
            // be whitespace-indented for the line to be a device entry.
            let payload = match rest.find(']') {
                Some(idx) => &rest[idx + 1..],
                None => rest,
            };
            if !payload.starts_with(|c: char| c.is_whitespace()) {
                return None;
            }
            let payload = payload.trim();
            let name = payload.strip_prefix('\'')?.strip_suffix('\'')?;
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_decklink_devices;

    #[test]
    fn parses_quoted_device_names() {
        let stderr = r#"[decklink @ 0x55f0a1] Blackmagic DeckLink devices:
[decklink @ 0x55f0a1]     'DeckLink Mini Recorder'
[decklink @ 0x55f0a1]     'DeckLink Studio 2'
dummy: Immediate exit requested
"#;
        assert_eq!(
            parse_decklink_devices(stderr),
            vec![
                "DeckLink Mini Recorder".to_string(),
                "DeckLink Studio 2".to_string()
            ]
        );
    }

    #[test]
    fn empty_when_the_demuxer_is_absent() {
        // What a redistributable build (no --enable-decklink) prints.
        let stderr = "Unknown input format: 'decklink'\n";
        assert!(parse_decklink_devices(stderr).is_empty());
    }

    #[test]
    fn empty_when_no_card_is_installed() {
        let stderr = "[decklink @ 0x55f0a1] Blackmagic DeckLink devices:\n";
        assert!(parse_decklink_devices(stderr).is_empty());
    }
}
