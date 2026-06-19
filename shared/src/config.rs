// SPDX-License-Identifier: AGPL-3.0-or-later

//! The unified KyberFrog configuration (`kyberfrog.toml`).
//!
//! One file per machine, with two halves:
//!
//! * [`Emission`] — the transmitters this machine publishes (each → one
//!   `kycontroller`). Carries `base_port` and the free-form `[defaults]` TOML
//!   table merged into every generated `kyber_config.toml`.
//! * [`Reception`] — the viewers this machine displays (each → one `kyclient`),
//!   plus the passive-display globals and the transparent login applied to all.
//!
//! Every field has a default, so a fresh install yields a valid (empty) config:
//! no transmitters, no viewers. The advanced knobs (auth, encoder, install dir,
//! base port, input/audio/keyboard/TLS flags) are file-only by design — the web
//! UI only edits transmitters and viewers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};

use crate::{
    paths, Transmitter, DEFAULT_AUTH_PASSWORD, DEFAULT_AUTH_USERNAME, DEFAULT_BASE_PORT,
    DEFAULT_WEB_PORT,
};

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// The whole `kyberfrog.toml`: machine-level settings plus the emission and
/// reception halves.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Where the installed Kyber binaries live (`kycontroller.exe` + DLLs).
    /// Overridable; defaults to the validated deployment path.
    pub kyber_install_dir: PathBuf,

    /// TCP port the unified web UI / `/transmitters` discovery endpoint listens
    /// on (bound on all interfaces so the LAN can reach it).
    pub web_port: u16,

    /// The transmitters this machine publishes (emitter role).
    pub emission: Emission,

    /// The viewers this machine displays (receiver role).
    pub reception: Reception,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            kyber_install_dir: default_install_dir(),
            web_port: DEFAULT_WEB_PORT,
            emission: Emission::default(),
            reception: Reception::default(),
        }
    }
}

/// The emitter half: the transmitters to publish and how to generate their
/// per-instance configs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Emission {
    /// First control-plane port handed out to new transmitters. Subsequent
    /// transmitters take the next free port above it.
    pub base_port: u16,

    /// TOML merged verbatim into every generated `kyber_config.toml`. Lets the
    /// operator carry auth / TLS / encoder defaults once; per-transmitter values
    /// (port, spout sender) are layered on top.
    pub defaults: toml::Table,

    /// The transmitters to run.
    #[serde(default, rename = "transmitter")]
    pub transmitters: Vec<Transmitter>,
}

impl Default for Emission {
    fn default() -> Self {
        Self {
            base_port: DEFAULT_BASE_PORT,
            defaults: toml::Table::new(),
            transmitters: Vec::new(),
        }
    }
}

impl Emission {
    /// Find a transmitter by name.
    pub fn get(&self, name: &str) -> Option<&Transmitter> {
        self.transmitters.iter().find(|t| t.name == name)
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

/// The receiver half: the viewers to display and the globals applied to all.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Reception {
    /// Path to `kyclient.exe` (or `kyclient` on Linux). A bare name resolves via
    /// PATH at spawn time.
    pub kyclient_path: PathBuf,

    /// Transparent basic-auth login used for every transmitter we connect to.
    pub auth_username: String,
    pub auth_password: String,

    /// Forward this machine's keyboard/mouse/gamepad. Off for a passive display.
    pub forward_inputs: bool,
    /// Play streamed audio. Off for a video-only video wall.
    pub audio: bool,
    /// Grab the local keyboard for immersive mode. Off so Alt+Tab stays free.
    pub keyboard_grab: bool,
    /// Trust-On-First-Use TLS verification.
    pub tls_tofu: bool,

    /// The viewers to run.
    #[serde(default, rename = "viewer")]
    pub viewers: Vec<Viewer>,
}

impl Default for Reception {
    fn default() -> Self {
        Self {
            kyclient_path: default_kyclient_path(),
            auth_username: DEFAULT_AUTH_USERNAME.to_string(),
            auth_password: DEFAULT_AUTH_PASSWORD.to_string(),
            forward_inputs: false,
            audio: false,
            keyboard_grab: false,
            tls_tofu: true,
            viewers: Vec::new(),
        }
    }
}

impl Reception {
    /// The non-viewer settings, cloned for the runtime supervisor.
    pub fn globals(&self) -> Globals {
        Globals {
            kyclient_path: self.kyclient_path.clone(),
            auth_username: self.auth_username.clone(),
            auth_password: self.auth_password.clone(),
            forward_inputs: self.forward_inputs,
            audio: self.audio,
            keyboard_grab: self.keyboard_grab,
            tls_tofu: self.tls_tofu,
        }
    }

    /// Find a viewer by id.
    pub fn get(&self, id: &str) -> Option<&Viewer> {
        self.viewers.iter().find(|v| v.id == id)
    }

    /// Find a viewer by id (mutable).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Viewer> {
        self.viewers.iter_mut().find(|v| v.id == id)
    }

    /// A short unique viewer id (`viewer-1`, `viewer-2`, …).
    pub fn unique_id(&self) -> String {
        let mut n = 1;
        loop {
            let candidate = format!("viewer-{n}");
            if self.get(&candidate).is_none() {
                return candidate;
            }
            n += 1;
        }
    }
}

/// One viewer: a `kyclient` connected to one remote transmitter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Viewer {
    /// Stable identifier (used in URLs, commands and the log file name).
    pub id: String,

    /// Remote host the viewer connects to (IP or hostname of the emitter).
    pub server: String,

    /// Control-plane port of the remote transmitter to display.
    pub port: u16,

    /// Start the viewer fullscreen (on the current monitor — per-monitor
    /// targeting is a planned kyclient change, see IMPROVEMENTS.md).
    /// Ignored when `spout_out` is set (the kyclient flags conflict).
    #[serde(default = "default_true")]
    pub fullscreen: bool,

    /// When set, run the viewer **windowless** and re-publish the received
    /// video as a Spout sender of this name (Windows relay, e.g. for Resolume).
    /// Mutually exclusive with `fullscreen`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spout_out: Option<String>,

    /// Desired running state: `true` means "should be running", so the agent
    /// (re)launches it on start/boot. Stop clears it; start sets it.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Global viewer settings handed to the runtime supervisor.
#[derive(Clone, Debug)]
pub struct Globals {
    pub kyclient_path: PathBuf,
    pub auth_username: String,
    pub auth_password: String,
    pub forward_inputs: bool,
    pub audio: bool,
    pub keyboard_grab: bool,
    pub tls_tofu: bool,
}

impl Globals {
    /// Build the `kyclient` argument vector for `viewer` (server is the
    /// positional first arg).
    pub fn kyclient_args(&self, viewer: &Viewer) -> Vec<String> {
        // Options first, positional STREAMER_IP last (kyclient parser is strict
        // about [OPTIONS] [--] [STREAMER_IP] ordering).
        let mut args = Vec::new();

        args.push("--port".to_string());
        args.push(viewer.port.to_string());

        if self.tls_tofu {
            args.push("--tls-tofu".to_string());
        }

        args.push("--auth-username".to_string());
        args.push(self.auth_username.clone());
        args.push("--auth-password".to_string());
        args.push(self.auth_password.clone());

        if let Some(name) = &viewer.spout_out {
            // Windowless Spout relay. Conflicts with --fullscreen, so emit one
            // or the other, never both.
            args.push("--spout-out".to_string());
            args.push(name.clone());
        } else if viewer.fullscreen {
            args.push("--fullscreen".to_string());
        }

        args.push("--inputs".to_string());
        args.push(self.forward_inputs.to_string());
        args.push("--audio".to_string());
        args.push(self.audio.to_string());
        args.push("--keyboard-grab".to_string());
        args.push(self.keyboard_grab.to_string());

        // Positional IP last.
        args.push(viewer.server.clone());

        args
    }
}

/// Path to `kycontroller.exe` inside an install directory.
pub fn kycontroller_path(install_dir: &Path) -> PathBuf {
    install_dir.join("kycontroller.exe")
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load the unified config. On first run, migrates any legacy split config
/// (`transmitters.toml` + `client-agent.toml`) into `kyberfrog.toml`, otherwise
/// writes a default file.
pub fn load() -> Result<Config> {
    let path = paths::config_file();

    let Ok(content) = fs::read_to_string(&path) else {
        info!("No config at {path:?}; creating a default one");
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    };

    let config: Config =
        toml::from_str(&content).with_context(|| format!("parsing config at {path:?}"))?;
    validate(&config)?;
    Ok(config)
}

/// Persist `config` to `kyberfrog.toml`, creating parent dirs as needed.
pub fn save(config: &Config) -> Result<()> {
    let path = paths::config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {parent:?}"))?;
    }

    let body = toml::to_string_pretty(config).context("serializing config")?;
    let header = "# KyberFrog — unified config for one machine.\n\
                  # Managed from the web UI (http://<this-pc>:<web_port>/) and the tray.\n\
                  # [emission] = transmitters this PC publishes (kycontroller).\n\
                  # [reception] = viewers this PC displays (kyclient).\n\n";

    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing config to {path:?}"))?;
    info!("Wrote config to {path:?}");
    Ok(())
}

/// Reject a config that cannot run; warn about likely-wrong-but-not-fatal bits.
fn validate(config: &Config) -> Result<()> {
    // Transmitters: filesystem-safe + unique names, unique ports.
    let mut seen_names = std::collections::HashSet::new();
    let mut seen_ports = std::collections::HashMap::<u16, &str>::new();
    for tx in &config.emission.transmitters {
        anyhow::ensure!(
            tx.has_valid_name(),
            "transmitter name {:?} is empty or contains path-unsafe characters",
            tx.name
        );
        anyhow::ensure!(
            seen_names.insert(tx.name.as_str()),
            "duplicate transmitter name {:?}",
            tx.name
        );
        if let Some(other) = seen_ports.insert(tx.port, tx.name.as_str()) {
            anyhow::bail!(
                "transmitters {:?} and {:?} both use port {}",
                other,
                tx.name,
                tx.port
            );
        }
    }

    // Viewers: non-empty + unique ids.
    let mut seen_ids = std::collections::HashSet::new();
    for v in &config.reception.viewers {
        anyhow::ensure!(
            !v.id.trim().is_empty(),
            "a viewer has an empty id in {:?}",
            paths::config_file()
        );
        anyhow::ensure!(seen_ids.insert(v.id.as_str()), "duplicate viewer id {:?}", v.id);
    }

    // Binaries are resolved via PATH when given as bare names; only warn about a
    // missing *absolute* path so we don't false-positive on a valid PATH install.
    let bin = kycontroller_path(&config.kyber_install_dir);
    if !bin.exists() {
        warn!("kycontroller binary not found at {bin:?}; transmitters will fail to start");
    }
    if config.reception.kyclient_path.is_absolute() && !config.reception.kyclient_path.exists() {
        warn!(
            "kyclient not found at {:?} — make sure the Kyber fork is installed and on PATH",
            config.reception.kyclient_path
        );
    }

    Ok(())
}

fn default_install_dir() -> PathBuf {
    // In a bundled install the Kyber binaries sit next to kyberfrog.exe (the
    // installer drops them in the same folder and adds it to PATH), so default
    // to the running exe's directory when kycontroller is actually there. Fall
    // back to the installer's default location otherwise. Overridable in
    // kyberfrog.toml.
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .filter(|dir| kycontroller_path(dir).exists())
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\KyberFrog"))
}

fn default_kyclient_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("kyclient.exe")
    } else {
        PathBuf::from("kyclient")
    }
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[test]
    fn config_round_trips_both_halves() {
        let toml_src = r#"
            kyber_install_dir = 'D:\soft\kyber'
            web_port = 7700

            [emission]
            base_port = 8080
            [emission.defaults.kyavserver]
            encoder = "x264"

            [[emission.transmitter]]
            name = "stage-left"
            port = 8080
            [emission.transmitter.source]
            type = "spout"
            sender = "Output A"

            [reception]
            tls_tofu = true
            [[reception.viewer]]
            id = "viewer-1"
            server = "192.168.1.10"
            port = 8080
            fullscreen = true
            enabled = true
        "#;

        let cfg: Config = toml::from_str(toml_src).expect("parse config");
        assert_eq!(cfg.web_port, 7700);
        assert_eq!(cfg.emission.transmitters.len(), 1);
        assert_eq!(
            cfg.emission.transmitters[0].source,
            Source::Spout {
                sender: "Output A".to_string()
            }
        );
        assert_eq!(cfg.reception.viewers.len(), 1);
        assert_eq!(cfg.reception.viewers[0].server, "192.168.1.10");

        // Survives a serialize → deserialize cycle unchanged.
        let serialized = toml::to_string_pretty(&cfg).expect("serialize");
        let reparsed: Config = toml::from_str(&serialized).expect("reparse");
        assert_eq!(
            reparsed.emission.transmitters,
            cfg.emission.transmitters
        );
        assert_eq!(reparsed.reception.viewers[0].id, "viewer-1");
    }

    #[test]
    fn default_config_serializes_with_both_sections() {
        let serialized = toml::to_string_pretty(&Config::default()).expect("serialize default");
        assert!(serialized.contains("web_port = 7700"), "got:\n{serialized}");
        assert!(serialized.contains("base_port = 9000"), "got:\n{serialized}");
        // And round-trips.
        let _: Config = toml::from_str(&serialized).expect("reparse default");
    }

    #[test]
    fn kyclient_args_put_server_last_and_carry_flags() {
        let globals = Reception {
            tls_tofu: true,
            forward_inputs: false,
            audio: false,
            keyboard_grab: false,
            ..Reception::default()
        }
        .globals();
        let viewer = Viewer {
            id: "v1".into(),
            server: "10.0.0.5".into(),
            port: 8081,
            fullscreen: true,
            spout_out: None,
            enabled: true,
        };
        let args = globals.kyclient_args(&viewer);
        // Positional server IP must be the final arg.
        assert_eq!(args.last().map(String::as_str), Some("10.0.0.5"));
        assert!(args.contains(&"--fullscreen".to_string()));
        assert!(args.contains(&"--tls-tofu".to_string()));
        let port_idx = args.iter().position(|a| a == "--port").unwrap();
        assert_eq!(args[port_idx + 1], "8081");
    }

    #[test]
    fn spout_out_replaces_fullscreen_in_args() {
        let globals = Reception::default().globals();
        let viewer = Viewer {
            id: "relay".into(),
            server: "10.0.0.9".into(),
            port: 8082,
            fullscreen: true, // ignored when spout_out is set
            spout_out: Some("KyberFrog".into()),
            enabled: true,
        };
        let args = globals.kyclient_args(&viewer);
        // --spout-out wins; --fullscreen must NOT be emitted (they conflict).
        let so = args.iter().position(|a| a == "--spout-out").unwrap();
        assert_eq!(args[so + 1], "KyberFrog");
        assert!(!args.contains(&"--fullscreen".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("10.0.0.9"));
    }

    #[test]
    fn emission_port_allocation_skips_used_ports() {
        let emission = Emission {
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
            ..Emission::default()
        };
        assert_eq!(emission.next_free_port(8080), 8082);
        assert!(emission.port_in_use(8081, None));
        assert!(!emission.port_in_use(8081, Some("b")));
    }

    #[test]
    fn unique_viewer_id_avoids_collisions() {
        let reception = Reception {
            viewers: vec![Viewer {
                id: "viewer-1".into(),
                server: "x".into(),
                port: 1,
                fullscreen: true,
                spout_out: None,
                enabled: true,
            }],
            ..Reception::default()
        };
        assert_eq!(reception.unique_id(), "viewer-2");
    }
}
