// SPDX-License-Identifier: AGPL-3.0-or-later

//! The KyberFrog Client configuration (`scene-agent.toml`).
//!
//! One scene machine displays one transmitter, so the config is flat: which
//! server/port to connect to, where `kyclient.exe` lives, and the handful of
//! knobs that make a passive display (no input forwarding, no audio, no
//! keyboard grab — just fullscreen video). Every field has a sensible default,
//! so a fresh install only needs `server` filled in.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use shared::{paths, DEFAULT_AUTH_PASSWORD, DEFAULT_AUTH_USERNAME, DEFAULT_BASE_PORT};

/// Everything the agent needs to launch and supervise one `kyclient`.
///
/// `#[serde(default)]` on the container means any field missing from the TOML
/// is filled from [`SceneConfig::default`], so partial files keep working as we
/// add knobs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneConfig {
    /// Regie host the client connects to (IP or hostname). Must be set; an
    /// empty value is rejected at load time with a helpful message.
    pub server: String,

    /// Control-plane port of the transmitter to display (matches one
    /// `[[transmitter]]` port in the server's `transmitters.toml`).
    pub port: u16,

    /// Path to `kyclient.exe` (or `kyclient` on Linux).
    pub kyclient_path: PathBuf,

    /// Basic-auth credentials. Default to the server's transparent login so a
    /// stock LAN setup needs no password management.
    pub auth_username: String,
    pub auth_password: String,

    /// Start the viewer fullscreen. On for a scene display.
    pub fullscreen: bool,

    /// Forward this machine's keyboard/mouse/gamepad to the server. Off for a
    /// passive display.
    pub forward_inputs: bool,

    /// Play the streamed audio. Off for a video-only scene wall.
    pub audio: bool,

    /// Grab the local keyboard for immersive mode. Off so the operator keeps
    /// Alt+Tab / the Windows key even in fullscreen.
    pub keyboard_grab: bool,

    /// Use Trust-On-First-Use TLS verification (auto-accepts the first cert).
    pub tls_tofu: bool,

    /// Extra raw flags appended verbatim to the `kyclient` command line, for
    /// anything not modelled above (codec, bitrate, …).
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            port: DEFAULT_BASE_PORT,
            kyclient_path: default_kyclient_path(),
            auth_username: DEFAULT_AUTH_USERNAME.to_string(),
            auth_password: DEFAULT_AUTH_PASSWORD.to_string(),
            fullscreen: true,
            forward_inputs: false,
            audio: false,
            keyboard_grab: false,
            tls_tofu: true,
            extra_args: Vec::new(),
        }
    }
}

impl SceneConfig {
    /// The full `kyclient` argument vector (server is the positional first arg).
    pub fn kyclient_args(&self) -> Vec<String> {
        let mut args = vec![self.server.clone()];

        args.push("--port".to_string());
        args.push(self.port.to_string());

        if self.tls_tofu {
            args.push("--tls-tofu".to_string());
        }

        args.push("--auth-username".to_string());
        args.push(self.auth_username.clone());
        args.push("--auth-password".to_string());
        args.push(self.auth_password.clone());

        if self.fullscreen {
            args.push("--fullscreen".to_string());
        }

        // These three take an explicit bool; kyclient defaults them all to true.
        args.push("--inputs".to_string());
        args.push(self.forward_inputs.to_string());
        args.push("--audio".to_string());
        args.push(self.audio.to_string());
        args.push("--keyboard-grab".to_string());
        args.push(self.keyboard_grab.to_string());

        args.extend(self.extra_args.iter().cloned());
        args
    }

    /// The command line as a single string, with the password masked, for logs.
    pub fn redacted_command(&self) -> String {
        let bin = self.kyclient_path.display().to_string();
        let mut parts = vec![quote(&bin)];
        let mut args = self.kyclient_args().into_iter().peekable();
        while let Some(arg) = args.next() {
            if arg == "--auth-password" {
                parts.push(arg);
                if args.next().is_some() {
                    parts.push("***".to_string());
                }
            } else {
                parts.push(quote(&arg));
            }
        }
        parts.join(" ")
    }
}

/// Load the agent config, writing a default file on first run.
pub fn load() -> Result<SceneConfig> {
    let path = paths::scene_agent_file();

    let Ok(content) = fs::read_to_string(&path) else {
        info!("No scene-agent config at {path:?}; creating a default one");
        let config = SceneConfig::default();
        save(&config)?;
        warn!("Edit {path:?} and set `server` to the regie IP, then restart the agent");
        return Ok(config);
    };

    let config: SceneConfig =
        toml::from_str(&content).with_context(|| format!("parsing scene-agent config at {path:?}"))?;
    validate(&config)?;
    Ok(config)
}

/// Persist `config` to `scene-agent.toml`, creating parent dirs as needed.
pub fn save(config: &SceneConfig) -> Result<()> {
    let path = paths::scene_agent_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {parent:?}"))?;
    }

    let body = toml::to_string_pretty(config).context("serializing scene-agent config")?;
    let header = "# KyberFrog Client — one scene PC, one fullscreen kyclient.\n\
                  # Set `server` to the regie IP and `port` to the transmitter you want\n\
                  # to display. The agent relaunches kyclient if it ever exits.\n\n";

    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing scene-agent config to {path:?}"))?;
    info!("Wrote scene-agent config to {path:?}");
    Ok(())
}

/// Reject a config that cannot run; warn about likely-wrong-but-not-fatal bits.
fn validate(config: &SceneConfig) -> Result<()> {
    anyhow::ensure!(
        !config.server.trim().is_empty(),
        "`server` is empty in {:?}; set it to the regie IP or hostname",
        paths::scene_agent_file()
    );

    if !config.kyclient_path.exists() {
        warn!(
            "kyclient not found at {:?}; the agent will keep retrying until it appears",
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

/// Wrap `s` in double quotes if it contains whitespace, for readable log lines.
fn quote(s: &str) -> String {
    if s.chars().any(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}
