// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enumeration of Blackmagic DeckLink capture devices, and of each device's
//! own available configurations (capture modes).
//!
//! Runs the bundled `ffmpeg -f decklink` with `-list_devices` / `-list_formats`
//! and parses stderr, for the same reason [`crate::cameras`] does: the names
//! ffmpeg reports are exactly the ones the fork's lavd iosys will expose, so
//! the `[kyavserver].camera_device` pin — and the device-name CRC derived from
//! it — always match.
//!
//! **The bundled ffmpeg only carries the DeckLink demuxer when it was built
//! with `-Ddecklink=enabled`** (see the licence note in the fork's
//! `meson_options.txt`: that build is nonfree and must not be redistributed).
//! Without it, `-f decklink` fails with "Unknown input format" and every
//! function here returns empty — the UI then shows its "no device detected"
//! state, exactly as it does when no card is installed.
//!
//! Only *inputs* are listed: ffmpeg's `list_devices` for the demuxer queries
//! `IDeckLinkInput` per device, so a half-duplex card (DeckLink Studio 2) whose
//! connectors are currently configured as outputs simply does not appear.

use std::path::{Path, PathBuf};

use log::warn;
use serde::Serialize;
use tokio::process::Command;

/// Path to the bundled `ffmpeg` binary inside an install directory.
fn ffmpeg_path(install_dir: &Path) -> PathBuf {
    install_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// Build an ffmpeg [`Command`] with the environment/flags every call here
/// needs: the bundle's own shared libraries, and no console flash on Windows.
fn ffmpeg_command(install_dir: &Path) -> Command {
    let mut command = Command::new(ffmpeg_path(install_dir));
    command.arg("-hide_banner");
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
    command
}

/// One DeckLink capture mode the card itself advertises, from `ffmpeg
/// -list_formats`. `code` is the exact string to put in
/// [`shared::Source::Decklink::format_code`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecklinkFormat {
    pub code: String,
    pub description: String,
}

/// List DeckLink capture device names, in ffmpeg's order. Returns an empty list
/// when ffmpeg is missing, was built without the DeckLink demuxer, or when no
/// card answers.
pub async fn list_decklink_inputs(install_dir: &Path) -> Vec<String> {
    let mut command = ffmpeg_command(install_dir);
    command.args(["-f", "decklink", "-list_devices", "1", "-i", "dummy"]);

    let output = match command.output().await {
        Ok(output) => output,
        Err(err) => {
            warn!("decklink enumeration: failed to run ffmpeg: {err}");
            return Vec::new();
        }
    };

    // The list goes to stderr, and ffmpeg exits non-zero because the dummy
    // input cannot be opened — that is expected.
    parse_decklink_devices(&String::from_utf8_lossy(&output.stderr))
}

/// List the capture modes a specific DeckLink device advertises (`ffmpeg -f
/// decklink -list_formats 1 -i "<device>"`) — what the card itself says it can
/// do, for the format_code picker. `device` must be a name
/// [`list_decklink_inputs`] returned; an unknown name (unplugged card, typo)
/// just yields an empty list like every other failure mode here.
pub async fn list_decklink_formats(install_dir: &Path, device: &str) -> Vec<DecklinkFormat> {
    let mut command = ffmpeg_command(install_dir);
    command.args(["-f", "decklink", "-list_formats", "1", "-i", device]);

    let output = match command.output().await {
        Ok(output) => output,
        Err(err) => {
            warn!("decklink format enumeration: failed to run ffmpeg: {err}");
            return Vec::new();
        }
    };

    parse_decklink_formats(&String::from_utf8_lossy(&output.stderr))
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

/// Extract `(format_code, description)` pairs from `-list_formats` stderr.
/// ffmpeg prints one tab-indented `\t<code>\t\t<description>` data row per
/// mode, under a `\tformat_code\tdescription` header row, e.g.
///
/// ```text
/// [in#0 @ 0x…] Supported formats for 'DeckLink Mini Recorder':
/// 	format_code	description
/// 	Hi60		1920x1080 at 30000/1000 fps (interlaced, upper field first)
/// ```
///
/// The exact tab count between `code` and `description` isn't load-bearing —
/// [`str::split`] on `'\t'` and dropping empty fields handles either one or
/// two tabs the same way. The header row's own `description` cell reads back
/// as the literal string `"description"` and is excluded explicitly, since
/// nothing else distinguishes it from a real (if oddly named) mode.
fn parse_decklink_formats(stderr: &str) -> Vec<DecklinkFormat> {
    stderr
        .lines()
        .filter_map(|line| {
            if !line.starts_with('\t') {
                return None;
            }
            let mut fields = line.split('\t').filter(|f| !f.is_empty());
            let code = fields.next()?.trim();
            let description = fields.next()?.trim();
            if code.is_empty() || description.is_empty() || description == "description" {
                return None;
            }
            Some(DecklinkFormat { code: code.to_string(), description: description.to_string() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_decklink_devices, parse_decklink_formats, DecklinkFormat};

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

    #[test]
    fn parses_formats_captured_from_a_real_mini_recorder() {
        // Verbatim from `ffmpeg -f decklink -list_formats 1 -i "DeckLink Mini
        // Recorder"` against real hardware (double-tab data rows, single-tab
        // header, a 4-char code with a padding space, "Error opening input
        // file" trailer once the dummy formats probe fails to open).
        let stderr = "[in#0 @ 0x64df6f123940] Supported formats for 'DeckLink Mini Recorder':\n\
            \tformat_code\tdescription\n\
            \tntsc\t\t720x486 at 30000/1001 fps (interlaced, lower field first)\n\
            \tpal \t\t720x576 at 25000/1000 fps (interlaced, upper field first)\n\
            \tHi60\t\t1920x1080 at 30000/1000 fps (interlaced, upper field first)\n\
            Error opening input file DeckLink Mini Recorder.\n";

        let formats = parse_decklink_formats(stderr);
        assert_eq!(
            formats,
            vec![
                DecklinkFormat {
                    code: "ntsc".to_string(),
                    description: "720x486 at 30000/1001 fps (interlaced, lower field first)"
                        .to_string(),
                },
                DecklinkFormat {
                    code: "pal".to_string(),
                    description: "720x576 at 25000/1000 fps (interlaced, upper field first)"
                        .to_string(),
                },
                DecklinkFormat {
                    code: "Hi60".to_string(),
                    description: "1920x1080 at 30000/1000 fps (interlaced, upper field first)"
                        .to_string(),
                },
            ]
        );
    }

    #[test]
    fn formats_empty_when_the_demuxer_is_absent() {
        let stderr = "Unknown input format: 'decklink'\n";
        assert!(parse_decklink_formats(stderr).is_empty());
    }

    #[test]
    fn formats_empty_for_an_unknown_device() {
        let stderr = "[in#0] Blackmagic DeckLink devices:\nError opening input file dummy.\n";
        assert!(parse_decklink_formats(stderr).is_empty());
    }
}
