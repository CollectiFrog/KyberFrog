// SPDX-License-Identifier: AGPL-3.0-or-later
//
// kyberfrog-shared: types shared across the unified KyberFrog app (one binary
// that both emits — supervises `kycontroller` transmitters — and receives —
// supervises `kyclient` viewers).

//! Data model for KyberFrog.
//!
//! A single [`Config`] (`kyberfrog.toml`) is the source of truth for one
//! machine. It has two halves:
//!
//! * [`Emission`] — the [`Transmitter`]s to publish. Each maps to exactly one
//!   `kycontroller` instance, isolated by TCP [`Transmitter::port`] and by its
//!   own generated `kyber_config.toml` (selected at spawn time through the
//!   `KYBER_CONFIG_PATH` environment variable).
//! * [`Reception`] — the [`Viewer`]s to display. Each maps to one `kyclient`
//!   process connected to a remote transmitter.
//!
//! A given machine can do either or both: a pure receiver simply has no
//! transmitters, a pure emitter no viewers.

pub mod config;
pub mod encoder;
pub mod gen;
pub mod paths;

use serde::{Deserialize, Serialize};

pub use config::{Config, Emission, Globals, Reception, Setup, Ui, UserConf, Viewer};
pub use encoder::{EncoderChoice, EncoderInfo, GpuAdapter};

/// Which capture API the kyavserver uses for screen grabs **on Linux**.
///
/// Serialises to the exact lowercase string the fork expects for
/// `[kyavserver].grab_backend`. No effect on Windows, where screen capture goes
/// through DXGI/Spout and the fork does not even compile the key in.
///
/// This is a **machine** setting ([`UserConf::screen_backend`]), never part of a
/// setup: the right backend depends on the session the app is running in, so a
/// show saved on a Wayland box must not carry `wlroots` onto an X11 box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenBackend {
    /// NVIDIA Framebuffer Capture. Fastest where it exists, NVIDIA-only.
    NvFbc,
    /// DRM/KMS scanout capture — works with no display server at all, which is
    /// what a headless display box wants.
    Drm,
    /// X11 capture through XCB/SHM.
    Xcb,
    /// Wayland capture through the **wlroots** screencopy protocol. Note that
    /// GNOME and KDE do *not* implement it (they expose xdg-desktop-portal /
    /// PipeWire instead), so this only works on wlroots compositors — sway,
    /// Hyprland, river…
    Wlroots,
}

impl ScreenBackend {
    /// The `grab_backend` string written into the generated kyavserver config.
    /// Kept in lockstep with the serde representation and the fork's own enum.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenBackend::NvFbc => "nvfbc",
            ScreenBackend::Drm => "drm",
            ScreenBackend::Xcb => "xcb",
            ScreenBackend::Wlroots => "wlroots",
        }
    }

    /// Guess the backend from a session description.
    ///
    /// Pure so it can be tested; [`ScreenBackend::detect`] supplies the real
    /// environment. `NvFbc` is never guessed — it is strictly faster but only on
    /// NVIDIA hardware, and a wrong guess produces a transmitter that starts and
    /// then captures nothing. It stays an explicit operator choice.
    pub fn detect_from(
        session_type: Option<&str>,
        wayland_display: Option<&str>,
        x_display: Option<&str>,
    ) -> ScreenBackend {
        // The session type is authoritative when the session manager set it.
        match session_type.map(str::trim) {
            Some("wayland") => return ScreenBackend::Wlroots,
            Some("x11") => return ScreenBackend::Xcb,
            // "tty" (or anything else) means no display server: fall through to
            // the socket check, then to DRM.
            _ => {}
        }
        if wayland_display.is_some_and(|v| !v.is_empty()) {
            ScreenBackend::Wlroots
        } else if x_display.is_some_and(|v| !v.is_empty()) {
            ScreenBackend::Xcb
        } else {
            // No display server in sight — headless box driving a screen through
            // KMS directly.
            ScreenBackend::Drm
        }
    }

    /// The backend to use on this machine, or `None` off Linux.
    ///
    /// KyberFrog **always** writes an explicit backend on Linux: the fork
    /// defaults `grab_backend` to `NvFbc`, so leaving the key out means a
    /// transmitter that silently captures nothing on every non-NVIDIA machine.
    pub fn detect() -> Option<ScreenBackend> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        let session = std::env::var("XDG_SESSION_TYPE").ok();
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        let x11 = std::env::var("DISPLAY").ok();
        Some(ScreenBackend::detect_from(
            session.as_deref(),
            wayland.as_deref(),
            x11.as_deref(),
        ))
    }
}

/// The thing feeding one transmitter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// A Spout sender (Windows GPU texture share). The kyavserver instance is
    /// pinned to this sender name and ignores the display requested by clients.
    Spout { sender: String },

    /// Desktop / screen capture. With no `spout_sender` set, the kyavserver
    /// behaves as a regular screen grabber. Which physical display a viewer
    /// receives is chosen **client-side** at stream start (kyclient's
    /// `--display-idx`, surfaced as [`Viewer::display_idx`]); the emitter serves
    /// whatever display each client requests. Scoped to physical monitors only
    /// (the fork's default `[kyavserver]` capture excludes Spout senders).
    Screen {},

    /// A webcam / capture device (Windows DirectShow through the fork's lavd
    /// iosys). The kyavserver instance is pinned to this device name
    /// (`[kyavserver].camera_device`) and ignores the display requested by
    /// clients — same pinning mechanism as [`Source::Spout`].
    Camera { device: String },

    /// Expose **every** source of the machine at once — all physical monitors
    /// *and* all Spout senders. Backs the "Tout envoyer" mode: a single
    /// transmitter a viewer can pick any source from. Generated config sets
    /// `[kyavserver].all_sources = true`; not offered as a per-source tile.
    All {},
}

impl Source {
    /// Short human label for menus / tooltips.
    pub fn label(&self) -> String {
        match self {
            Source::Spout { sender } => format!("Spout: {sender}"),
            Source::Screen {} => "Screen".to_string(),
            Source::Camera { device } => format!("Webcam: {device}"),
            Source::All {} => "Toutes les sources".to_string(),
        }
    }
}

/// One transmitter = one `kycontroller` instance on its own port.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transmitter {
    /// Stable identifier, used as the instance directory name. Keep it
    /// filesystem-safe (no path separators).
    pub name: String,

    /// TCP control-plane port the client connects to (9000, 9001, ...).
    pub port: u16,

    /// What this transmitter streams.
    pub source: Source,
}

impl Transmitter {
    /// `true` if `name` is safe to use as a directory component.
    pub fn has_valid_name(&self) -> bool {
        !self.name.is_empty()
            && !self.name.contains(|c: char| {
                matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            })
    }
}

/// Instance name of the synthetic transmitter used by the "Tout envoyer" mode
/// (`Emission::send_all`). Filesystem-safe (it names an instance dir + logs).
pub const ALL_TX_NAME: &str = "tout-envoyer";

/// Default first control-plane port when none is configured. 9000 avoids the
/// very common 8080 (dev servers, proxies) and stays clear of kycontroller's
/// internal IPC range 9091..9100 (max ~9 instances → 9000..9008).
pub const DEFAULT_BASE_PORT: u16 = 9000;

/// Default port for the unified web UI / discovery endpoint.
pub const DEFAULT_WEB_PORT: u16 = 7700;

/// Transparent default credentials.
///
/// kycontroller has no anonymous mode: a client must `POST /login` with a valid
/// basic-auth login before any stream (video included) can start. To keep the
/// operator from having to manage a password on a trusted LAN, the emitter bakes
/// this fixed login into every generated config that doesn't already declare its
/// own auth, and the receiver connects with the same pair — so from the
/// operator's point of view there is no password.
pub const DEFAULT_AUTH_USERNAME: &str = "vj";
/// Plaintext of the transparent default password (stored hashed in configs).
pub const DEFAULT_AUTH_PASSWORD: &str = "kyberfrog";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_type_wins_over_stray_sockets() {
        // A Wayland session usually also exposes DISPLAY through XWayland;
        // trusting DISPLAY there would pick the wrong backend.
        assert_eq!(
            ScreenBackend::detect_from(Some("wayland"), Some("wayland-0"), Some(":0")),
            ScreenBackend::Wlroots
        );
        assert_eq!(
            ScreenBackend::detect_from(Some("x11"), None, Some(":0")),
            ScreenBackend::Xcb
        );
    }

    #[test]
    fn falls_back_to_sockets_when_session_type_is_unset() {
        assert_eq!(
            ScreenBackend::detect_from(None, Some("wayland-0"), None),
            ScreenBackend::Wlroots
        );
        assert_eq!(
            ScreenBackend::detect_from(None, None, Some(":0")),
            ScreenBackend::Xcb
        );
    }

    #[test]
    fn headless_falls_back_to_drm() {
        // The display-box case: autologin on a tty, no display server.
        assert_eq!(
            ScreenBackend::detect_from(Some("tty"), None, None),
            ScreenBackend::Drm
        );
        assert_eq!(ScreenBackend::detect_from(None, None, None), ScreenBackend::Drm);
    }

    #[test]
    fn empty_vars_are_not_a_display_server() {
        // `DISPLAY=` exported empty is a real thing (the fork's own README
        // suggests `export WAYLAND_DISPLAY=""` to force X11 compatibility).
        assert_eq!(
            ScreenBackend::detect_from(None, Some(""), Some(":0")),
            ScreenBackend::Xcb
        );
        assert_eq!(ScreenBackend::detect_from(None, Some(""), Some("")), ScreenBackend::Drm);
    }

    #[test]
    fn nvfbc_is_never_guessed() {
        // It only works on NVIDIA; a wrong guess yields a transmitter that
        // starts and captures nothing. Explicit operator choice only.
        for case in [
            ScreenBackend::detect_from(Some("wayland"), None, None),
            ScreenBackend::detect_from(Some("x11"), None, None),
            ScreenBackend::detect_from(None, None, None),
        ] {
            assert_ne!(case, ScreenBackend::NvFbc);
        }
    }

    #[test]
    fn backend_strings_match_the_fork_enum() {
        // These must stay identical to kyavservice's GrabBackend serde names.
        assert_eq!(ScreenBackend::NvFbc.as_str(), "nvfbc");
        assert_eq!(ScreenBackend::Drm.as_str(), "drm");
        assert_eq!(ScreenBackend::Xcb.as_str(), "xcb");
        assert_eq!(ScreenBackend::Wlroots.as_str(), "wlroots");
        // …and serde must agree with as_str(), in both directions: gen.rs
        // writes as_str() while the machine config round-trips through serde.
        for b in [
            ScreenBackend::NvFbc,
            ScreenBackend::Drm,
            ScreenBackend::Xcb,
            ScreenBackend::Wlroots,
        ] {
            let serialized = toml::Value::try_from(b).unwrap();
            assert_eq!(serialized.as_str(), Some(b.as_str()));
            let back: ScreenBackend = serialized.try_into().unwrap();
            assert_eq!(back, b);
        }
    }
}
