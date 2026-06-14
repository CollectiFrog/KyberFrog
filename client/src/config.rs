// SPDX-License-Identifier: AGPL-3.0-or-later

//! The KyberFrog Client configuration (`scene-agent.toml`).
//!
//! A scene machine can drive several viewers at once, so the config is a set of
//! global knobs plus a list of [`Instance`]s — each one a `kyclient` connected
//! to a transmitter. The globals carry the passive-display defaults (no input
//! forwarding, no audio, keyboard free) and the transparent login; per instance
//! the operator only sets the transmitter address and whether it is fullscreen.
//!
//! Every field has a default, so a fresh install yields a valid (empty) config.
//! The web UI reads and writes this file, so it survives reboots and the agent
//! relaunches every `enabled` instance on start.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use shared::{paths, DEFAULT_AUTH_PASSWORD, DEFAULT_AUTH_USERNAME};

/// Default port for the client web UI / control endpoint.
pub const DEFAULT_WEB_PORT: u16 = 7701;

/// One viewer: a `kyclient` connected to one transmitter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    /// Stable identifier (used in URLs, commands and the log file name).
    pub id: String,

    /// Regie host the viewer connects to (IP or hostname).
    pub server: String,

    /// Control-plane port of the transmitter to display.
    pub port: u16,

    /// Start the viewer fullscreen (on the current monitor — per-monitor
    /// targeting is a planned kyclient change, see IMPROVEMENTS.md).
    #[serde(default = "default_true")]
    pub fullscreen: bool,

    /// Desired running state: `true` means "should be running", so the agent
    /// (re)launches it on start/boot. Stop clears it; start sets it.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Global settings shared by every instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientConfig {
    /// Path to `kyclient.exe` (or `kyclient` on Linux).
    pub kyclient_path: PathBuf,

    /// Port the client web UI listens on (bound on all interfaces).
    pub web_port: u16,

    /// Transparent basic-auth login used for every transmitter.
    pub auth_username: String,
    pub auth_password: String,

    /// Forward this machine's keyboard/mouse/gamepad. Off for a passive display.
    pub forward_inputs: bool,
    /// Play streamed audio. Off for a video-only scene wall.
    pub audio: bool,
    /// Grab the local keyboard for immersive mode. Off so Alt+Tab stays free.
    pub keyboard_grab: bool,
    /// Trust-On-First-Use TLS verification.
    pub tls_tofu: bool,

    /// The viewers to run.
    #[serde(default, rename = "instance")]
    pub instances: Vec<Instance>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            kyclient_path: default_kyclient_path(),
            web_port: DEFAULT_WEB_PORT,
            auth_username: DEFAULT_AUTH_USERNAME.to_string(),
            auth_password: DEFAULT_AUTH_PASSWORD.to_string(),
            forward_inputs: false,
            audio: false,
            keyboard_grab: false,
            tls_tofu: true,
            instances: Vec::new(),
        }
    }
}

impl ClientConfig {
    /// The non-instance settings, cloned for the runtime manager.
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

    /// Find an instance by id.
    pub fn get(&self, id: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }

    /// Find an instance by id (mutable).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|i| i.id == id)
    }

    /// A short unique instance id (`instance-1`, `instance-2`, …).
    pub fn unique_id(&self) -> String {
        let mut n = 1;
        loop {
            let candidate = format!("instance-{n}");
            if self.get(&candidate).is_none() {
                return candidate;
            }
            n += 1;
        }
    }
}

/// Global settings handed to the runtime [`crate::supervisor::Manager`].
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
    /// Build the `kyclient` argument vector for `instance` (server is the
    /// positional first arg).
    pub fn kyclient_args(&self, instance: &Instance) -> Vec<String> {
        let mut args = vec![instance.server.clone()];

        args.push("--port".to_string());
        args.push(instance.port.to_string());

        if self.tls_tofu {
            args.push("--tls-tofu".to_string());
        }

        args.push("--auth-username".to_string());
        args.push(self.auth_username.clone());
        args.push("--auth-password".to_string());
        args.push(self.auth_password.clone());

        if instance.fullscreen {
            args.push("--fullscreen".to_string());
        }

        args.push("--inputs".to_string());
        args.push(self.forward_inputs.to_string());
        args.push("--audio".to_string());
        args.push(self.audio.to_string());
        args.push("--keyboard-grab".to_string());
        args.push(self.keyboard_grab.to_string());

        args
    }
}

/// Load the client config, writing a default file on first run.
pub fn load() -> Result<ClientConfig> {
    let path = paths::scene_agent_file();

    let Ok(content) = fs::read_to_string(&path) else {
        info!("No client config at {path:?}; creating a default one");
        let config = ClientConfig::default();
        save(&config)?;
        return Ok(config);
    };

    let config: ClientConfig =
        toml::from_str(&content).with_context(|| format!("parsing client config at {path:?}"))?;
    validate(&config)?;
    Ok(config)
}

/// Persist `config` to `scene-agent.toml`, creating parent dirs as needed.
pub fn save(config: &ClientConfig) -> Result<()> {
    let path = paths::scene_agent_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {parent:?}"))?;
    }

    let body = toml::to_string_pretty(config).context("serializing client config")?;
    let header = "# KyberFrog Client — one scene PC, N fullscreen kyclient viewers.\n\
                  # Managed from the web UI (http://<this-pc>:<web_port>/). Each\n\
                  # [[instance]] is one viewer; `enabled` instances start on boot.\n\n";

    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing client config to {path:?}"))?;
    info!("Wrote client config to {path:?}");
    Ok(())
}

/// Reject a config that cannot run; warn about likely-wrong-but-not-fatal bits.
fn validate(config: &ClientConfig) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for inst in &config.instances {
        anyhow::ensure!(
            !inst.id.trim().is_empty(),
            "an instance has an empty id in {:?}",
            paths::scene_agent_file()
        );
        anyhow::ensure!(
            seen.insert(inst.id.as_str()),
            "duplicate instance id {:?}",
            inst.id
        );
    }

    if !config.kyclient_path.exists() {
        warn!(
            "kyclient not found at {:?}; viewers will keep retrying until it appears",
            config.kyclient_path
        );
    }

    Ok(())
}

/// Default location of the Kyber client binary (matches the validated install).
fn default_kyclient_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"D:\soft\kyber\kyclient.exe")
    } else {
        PathBuf::from("kyclient")
    }
}

fn default_true() -> bool {
    true
}
