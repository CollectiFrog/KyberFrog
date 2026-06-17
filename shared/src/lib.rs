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
pub mod gen;
pub mod paths;

use serde::{Deserialize, Serialize};

pub use config::{Config, Emission, Globals, Reception, Viewer};

/// The thing feeding one transmitter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// A Spout sender (Windows GPU texture share). The kyavserver instance is
    /// pinned to this sender name and ignores the display requested by clients.
    Spout { sender: String },

    /// Desktop / screen capture. With no `spout_sender` set, the kyavserver
    /// behaves as a regular screen grabber; `display` is an optional hint.
    Screen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
}

impl Source {
    /// Short human label for menus / tooltips.
    pub fn label(&self) -> String {
        match self {
            Source::Spout { sender } => format!("Spout: {sender}"),
            Source::Screen { display: Some(d) } => format!("Screen: {d}"),
            Source::Screen { display: None } => "Screen".to_string(),
        }
    }
}

/// One transmitter = one `kycontroller` instance on its own port.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transmitter {
    /// Stable identifier, used as the instance directory name. Keep it
    /// filesystem-safe (no path separators).
    pub name: String,

    /// TCP control-plane port the client connects to (8080, 8081, ...).
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

/// Default first control-plane port when none is configured.
pub const DEFAULT_BASE_PORT: u16 = 8080;

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
