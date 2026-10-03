// SPDX-License-Identifier: AGPL-3.0-or-later

//! What a transmitter's source is doing, for the dashboard.
//!
//! A capture that fails never makes kycontroller exit: the transmitter stays
//! "Running" while viewers get nothing. The fork logs each capture problem
//! once, with a stable wording, and once more when it clears; this module
//! turns those lines into a [`SourceIssue`] the UI can show next to the
//! transmitter. The wordings live in `txproto` (`src/iosys_spout.c`,
//! `src/iosys_lavd.c`) and FFmpeg's DeckLink demuxer.

use serde::Serialize;

/// Something keeping a source from delivering pictures. `code` is stable (the
/// UI translates it); `detail` is the fork's own words, for the tooltip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceIssue {
    pub code: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

impl SourceIssue {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        SourceIssue { code, detail: detail.into() }
    }
}

/// What one transmitter log line says about its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogSignal {
    Issue(SourceIssue),
    /// Pictures flow again.
    Clear,
}

/// Codes, shared with the UI (`ui/src/hooks/useLang.ts`, `issue_*`).
pub mod code {
    /// The pinned Spout sender is not in the Spout registry.
    pub const SPOUT_MISSING: &str = "spout_missing";
    /// The Spout sender is listed but sends no frame.
    pub const SPOUT_STALLED: &str = "spout_stalled";
    /// The sender's pixel format cannot be read.
    pub const SPOUT_FORMAT: &str = "spout_format";
    /// The sender's texture cannot be opened (another GPU).
    pub const SPOUT_GPU: &str = "spout_gpu";
    /// A camera / capture card cannot be opened.
    pub const CAPTURE_OPEN: &str = "capture_open";
    /// A camera / capture card stopped delivering, being reopened.
    pub const CAPTURE_LOST: &str = "capture_lost";
    /// A DeckLink input sees no signal it recognises.
    pub const NO_SIGNAL: &str = "no_signal";
}

/// Classify one line of a transmitter's log. `None` for everything that says
/// nothing about the source.
pub fn classify(line: &str) -> Option<LogSignal> {
    let issue = |code, detail: &str| Some(LogSignal::Issue(SourceIssue::new(code, detail.trim())));
    let message = line.split_once(" - ").map_or(line, |(_, m)| m).trim();

    if message.contains("First frame acquired")
        || message.contains("Capture back:")
        || (message.starts_with("Spout: sender \"") && message.ends_with("\" is back"))
    {
        return Some(LogSignal::Clear);
    }
    if message.starts_with("Spout: sender \"") {
        if message.ends_with("not found, waiting for it") {
            return issue(code::SPOUT_MISSING, message);
        }
        if message.ends_with("sends no frame, waiting for it") {
            return issue(code::SPOUT_STALLED, message);
        }
    }
    if message.starts_with("Unsupported sender texture format:") {
        return issue(code::SPOUT_FORMAT, message);
    }
    if message.starts_with("Spout: cannot open the texture of sender") {
        return issue(code::SPOUT_GPU, message);
    }
    if message.starts_with("Capture lost:") {
        return issue(code::CAPTURE_LOST, message);
    }
    if message.starts_with("Unable to open context for source") {
        return issue(code::CAPTURE_OPEN, message);
    }
    // FFmpeg's DeckLink demuxer (libavdevice/decklink_dec.cpp), relayed.
    if message.contains("Cannot Autodetect input stream or No signal")
        || message.contains("No input signal detected")
    {
        return issue(code::NO_SIGNAL, message);
    }
    None
}

/// A Spout sender as the registry describes it: size and DXGI pixel format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpoutInfo {
    pub width: u32,
    pub height: u32,
    /// DXGI_FORMAT number, as the fork logs it.
    pub format: u32,
    /// Human name of the format, `None` when the fork cannot read it.
    pub format_name: Option<&'static str>,
}

impl SpoutInfo {
    pub fn new(width: u32, height: u32, format: u32) -> Self {
        SpoutInfo { width, height, format, format_name: spout_format_name(format) }
    }
}

/// The DXGI formats the fork's Spout capture reads, by name — the same list
/// as `spout_format_lookup()` in `txproto` `src/iosys_spout.c`. `None` = the
/// capture refuses it (integer, depth, video formats…).
pub fn spout_format_name(format: u32) -> Option<&'static str> {
    Some(match format {
        0 | 87 | 90 => "BGRA 8 bits",
        91 => "BGRA 8 bits sRGB",
        88 | 92 => "BGRX 8 bits",
        93 => "BGRX 8 bits sRGB",
        28 | 27 => "RGBA 8 bits",
        29 => "RGBA 8 bits sRGB",
        24 | 23 => "RGB 10 bits",
        10 | 9 => "RGBA 16 bits float",
        11 => "RGBA 16 bits",
        2 | 1 => "RGBA 32 bits float",
        26 => "RGB 11/11/10 float",
        34 => "RG 16 bits float",
        35 => "RG 16 bits",
        16 => "RG 32 bits float",
        49 => "RG 8 bits",
        61 | 60 => "mono 8 bits",
        56 | 53 => "mono 16 bits",
        54 => "mono 16 bits float",
        41 | 39 => "mono 32 bits float",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(line: &str) -> Option<&'static str> {
        match classify(line) {
            Some(LogSignal::Issue(issue)) => Some(issue.code),
            _ => None,
        }
    }

    #[test]
    fn spout_lost_and_back() {
        let lost = "2026-09-29T00:40:01.487[21652] WARN txproto::kybench-src - Spout: sender \"kybench-src\" not found, waiting for it";
        assert_eq!(code_of(lost), Some(code::SPOUT_MISSING));
        match classify(lost) {
            Some(LogSignal::Issue(issue)) => {
                assert_eq!(issue.detail, "Spout: sender \"kybench-src\" not found, waiting for it")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            code_of("… WARN txproto::x - Spout: sender \"td\" sends no frame, waiting for it"),
            Some(code::SPOUT_STALLED)
        );
        assert_eq!(
            classify("… INFO txproto::kybench-src - Spout: sender \"kybench-src\" is back"),
            Some(LogSignal::Clear)
        );
        assert_eq!(classify("… INFO txproto::td - First frame acquired"), Some(LogSignal::Clear));
    }

    #[test]
    fn spout_refusals() {
        assert_eq!(
            code_of("… ERROR txproto::spout - Unsupported sender texture format: 45 (multisampled)"),
            Some(code::SPOUT_FORMAT)
        );
        assert_eq!(
            code_of("… ERROR txproto::spout - Spout: cannot open the texture of sender \"a\" (OpenSharedResource 0x80070057), is it rendered on another GPU?"),
            Some(code::SPOUT_GPU)
        );
    }

    #[test]
    fn capture_devices() {
        assert_eq!(
            code_of("… ERROR txproto::dshow - Capture lost: \"video=Cam\" (dshow): I/O error, reopening"),
            Some(code::CAPTURE_LOST)
        );
        assert_eq!(
            classify("… INFO txproto::dshow - Capture back: \"video=Cam\" (dshow) reopened"),
            Some(LogSignal::Clear)
        );
        assert_eq!(
            code_of("… ERROR txproto::lavd - Unable to open context for source \"dshow\" (video=Cam): I/O error"),
            Some(code::CAPTURE_OPEN)
        );
        assert_eq!(
            code_of("… ERROR txproto::decklink - Cannot Autodetect input stream or No signal"),
            Some(code::NO_SIGNAL)
        );
    }

    #[test]
    fn ignores_everything_else() {
        for line in [
            "… INFO txproto::spout - Spout: New sender registered: \"a\" (1280x720)",
            "… WARN txproto::spout - Spout: sender \"bench-out\" has no info map, skipping",
            "… ERROR txproto::lavc:h264_amf - Init() failed with error 5",
            "… INFO txproto::spout - Spout: sender \"a\" removed",
        ] {
            assert_eq!(classify(line), None, "{line}");
        }
    }

    #[test]
    fn spout_formats_match_the_fork() {
        assert_eq!(spout_format_name(87), Some("BGRA 8 bits"));
        assert_eq!(spout_format_name(10), Some("RGBA 16 bits float"));
        assert_eq!(spout_format_name(24), Some("RGB 10 bits"));
        assert_eq!(spout_format_name(61), Some("mono 8 bits"));
        // R8G8B8A8_UINT, D32_FLOAT, NV12: refused by the capture
        assert_eq!(spout_format_name(30), None);
        assert_eq!(spout_format_name(40), None);
        assert_eq!(spout_format_name(103), None);
    }
}
