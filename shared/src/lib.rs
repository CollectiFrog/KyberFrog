// SPDX-License-Identifier: AGPL-3.0-or-later
//
// kyber-anysource-shared: types shared by the Director (server tray) and the
// Scene Agent (client supervisor).

//! Data model for the kyber-anysource source-transmission workflow.
//!
//! A [`Directory`] is the single source of truth (`transmitters.toml`). It
//! lists the [`Transmitter`]s the Director should run. Each transmitter maps
//! to exactly one `kycontroller` instance, isolated by TCP [`Transmitter::port`]
//! and by its own generated `kyber_config.toml` (selected at spawn time through
//! the `KYBER_CONFIG_PATH` environment variable).

pub mod gen;
pub mod paths;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

/// The Director configuration file (`transmitters.toml`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Directory {
    /// Where the installed Kyber binaries live (`kycontroller.exe` + DLLs).
    /// Overridable; defaults to the validated deployment path.
    #[serde(default = "default_install_dir")]
    pub kyber_install_dir: PathBuf,

    /// First control-plane port handed out to new transmitters. Subsequent
    /// transmitters take the next free port above it. Bump this when the
    /// default range (8080…) clashes with something else on the machine.
    /// Always written to `transmitters.toml` so it is easy to discover and edit.
    #[serde(default = "default_base_port")]
    pub base_port: u16,

    /// TCP port the Director's web UI / `/transmitters` discovery endpoint
    /// listens on (bound on all interfaces so the LAN can reach it).
    /// Always written to `transmitters.toml` so it is easy to discover and edit.
    #[serde(default = "default_web_port")]
    pub web_port: u16,

    /// TOML merged verbatim into every generated `kyber_config.toml`.
    ///
    /// This lets the operator carry auth / TLS / encoder defaults once,
    /// without the Director having to model Kyber's full config schema.
    /// Per-transmitter values (port, spout sender) are layered on top.
    #[serde(default)]
    pub defaults: toml::Table,

    /// The transmitters to run.
    #[serde(default, rename = "transmitter")]
    pub transmitters: Vec<Transmitter>,
}

impl Default for Directory {
    fn default() -> Self {
        Self {
            kyber_install_dir: default_install_dir(),
            base_port: DEFAULT_BASE_PORT,
            web_port: DEFAULT_WEB_PORT,
            defaults: toml::Table::new(),
            transmitters: Vec::new(),
        }
    }
}

/// Default first control-plane port when none is configured.
pub const DEFAULT_BASE_PORT: u16 = 8080;

/// Default port for the Director's web UI / discovery endpoint.
pub const DEFAULT_WEB_PORT: u16 = 7700;

/// Transparent default credentials.
///
/// kycontroller has no anonymous mode: a client must `POST /login` with a valid
/// basic-auth login before any stream (video included) can start. To keep the
/// operator from having to manage a password on a trusted LAN, the Director
/// bakes this fixed login into every generated config that doesn't already
/// declare its own auth, and the scene agent connects with the same pair — so
/// from the operator's point of view there is no password. This will become
/// user-configurable (tray + client app) later.
pub const DEFAULT_AUTH_USERNAME: &str = "vj";
/// Plaintext of the transparent default password (stored hashed in configs).
pub const DEFAULT_AUTH_PASSWORD: &str = "kyber-anysource";

impl Directory {
    /// Find a transmitter by name.
    pub fn get(&self, name: &str) -> Option<&Transmitter> {
        self.transmitters.iter().find(|t| t.name == name)
    }

    /// Configured first control-plane port.
    pub fn base_port(&self) -> u16 {
        self.base_port
    }

    /// Configured web UI / discovery-endpoint port.
    pub fn web_port(&self) -> u16 {
        self.web_port
    }

    /// `true` if `port` is already taken by another transmitter.
    pub fn port_in_use(&self, port: u16, except: Option<&str>) -> bool {
        self.transmitters
            .iter()
            .any(|t| t.port == port && Some(t.name.as_str()) != except)
    }

    /// Lowest free port at or above `from` (skips ports already assigned).
    pub fn next_free_port(&self, from: u16) -> u16 {
        let mut port = from;
        while self.port_in_use(port, None) {
            port = port.saturating_add(1);
        }
        port
    }
}

fn default_install_dir() -> PathBuf {
    // The Director's deployment target. Overridable in transmitters.toml.
    PathBuf::from(r"D:\soft\kyber")
}

fn default_base_port() -> u16 {
    DEFAULT_BASE_PORT
}

fn default_web_port() -> u16 {
    DEFAULT_WEB_PORT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_round_trips_both_source_kinds() {
        let toml_src = r#"
            kyber_install_dir = 'D:\soft\kyber'

            [defaults.kyavserver]
            encoder = "x264"

            [[transmitter]]
            name = "stage-left"
            port = 8080
            [transmitter.source]
            type = "spout"
            sender = "Output A"

            [[transmitter]]
            name = "preview"
            port = 8082
            [transmitter.source]
            type = "screen"
        "#;

        let dir: Directory = toml::from_str(toml_src).expect("parse directory");
        assert_eq!(dir.transmitters.len(), 2);
        assert_eq!(
            dir.transmitters[0].source,
            Source::Spout {
                sender: "Output A".to_string()
            }
        );
        assert_eq!(
            dir.transmitters[1].source,
            Source::Screen { display: None }
        );

        // And it survives a serialize -> deserialize cycle unchanged.
        let serialized = toml::to_string_pretty(&dir).expect("serialize");
        let reparsed: Directory = toml::from_str(&serialized).expect("reparse");
        assert_eq!(reparsed.transmitters, dir.transmitters);
    }

    #[test]
    fn port_allocation_skips_used_ports() {
        let dir = Directory {
            transmitters: vec![
                Transmitter {
                    name: "a".into(),
                    port: 8080,
                    source: Source::Screen { display: None },
                },
                Transmitter {
                    name: "b".into(),
                    port: 8081,
                    source: Source::Screen { display: None },
                },
            ],
            ..Directory::default()
        };
        assert_eq!(dir.next_free_port(8080), 8082);
        assert!(dir.port_in_use(8081, None));
        assert!(!dir.port_in_use(8081, Some("b")));
    }

    #[test]
    fn base_port_defaults_and_overrides() {
        assert_eq!(Directory::default().base_port(), DEFAULT_BASE_PORT);

        // The default directory always writes base_port so it is discoverable.
        let default_toml = toml::to_string_pretty(&Directory::default()).expect("serialize");
        assert!(
            default_toml.contains("base_port = 8080"),
            "default config must spell out base_port, got:\n{default_toml}"
        );

        let dir: Directory = toml::from_str("base_port = 9000").expect("parse");
        assert_eq!(dir.base_port(), 9000);

        // Round-trips through serialization.
        let serialized = toml::to_string_pretty(&dir).expect("serialize");
        let reparsed: Directory = toml::from_str(&serialized).expect("reparse");
        assert_eq!(reparsed.base_port(), 9000);
    }
}
