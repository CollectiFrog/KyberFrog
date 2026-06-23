// SPDX-License-Identifier: AGPL-3.0-or-later

//! The KyberFrog configuration, split across two files.
//!
//! * **`kyberfrog.toml`** — the *user config*: per-machine settings
//!   ([`UserConf`]: install dir, kyclient path, web port), the UI preferences
//!   ([`Ui`]: theme, language) and `active_setup`, a pointer to the setup
//!   document currently loaded. This file is fixed: there is exactly one per
//!   machine and it is never transported.
//! * **`setups/<name>.toml`** — a *setup document* ([`Setup`]): the emission
//!   (transmitters) and reception (viewers) halves. Self-contained and
//!   portable; this is what "save / load" and the cross-machine export/import
//!   move around. Machine paths are deliberately *not* in it.
//!
//! At runtime the two are merged into one [`Config`] aggregate, so the rest of
//! the app keeps reading `config.emission` / `config.reception` /
//! `config.kyber_install_dir` unchanged. Every mutation is persisted with
//! [`save`], which splits the aggregate back into the two files — the setup
//! half going to whichever document `active_setup` names, so edits always land
//! in the file the operator is working on.
//!
//! Advanced knobs (auth, encoder, base port, input/audio/keyboard/TLS flags)
//! stay file-only by design — the web UI only edits transmitters and viewers.

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
// Runtime aggregate
// ---------------------------------------------------------------------------

/// The whole live configuration: the machine/user settings merged with the
/// currently-active setup document. Built by [`load`]; split back into its two
/// files by [`save`].
#[derive(Clone, Debug)]
pub struct Config {
    /// Where the installed Kyber binaries live (`kycontroller.exe` + DLLs).
    /// Per-machine; never carried by a setup. Overridable; defaults to the
    /// validated deployment path.
    pub kyber_install_dir: PathBuf,

    /// Path to `kyclient.exe` (or `kyclient` on Linux). A bare name resolves via
    /// PATH at spawn time. Per-machine; never carried by a setup.
    pub kyclient_path: PathBuf,

    /// TCP port the unified web UI / `/transmitters` discovery endpoint listens
    /// on. Per-machine.
    pub web_port: u16,

    /// UI preferences served to the front-end (theme, language). Per-machine.
    pub ui: Ui,

    /// Name (bare stem) of the loaded setup document under `setups/`. Edits and
    /// `save` target this file.
    pub active_setup: String,

    /// The transmitters this machine publishes (from the active setup).
    pub emission: Emission,

    /// The viewers this machine displays (from the active setup).
    pub reception: Reception,
}

impl Default for Config {
    fn default() -> Self {
        let user = UserConf::default();
        Self {
            kyber_install_dir: user.kyber_install_dir,
            kyclient_path: user.kyclient_path,
            web_port: user.web_port,
            ui: user.ui,
            active_setup: user.active_setup,
            emission: Emission::default(),
            reception: Reception::default(),
        }
    }
}

impl Config {
    /// The non-viewer reception settings plus the machine `kyclient_path`,
    /// cloned for the runtime supervisor.
    pub fn globals(&self) -> Globals {
        self.reception.globals(self.kyclient_path.clone())
    }

    /// Split the aggregate into the two on-disk views.
    fn split(&self) -> (UserConf, Setup) {
        let user = UserConf {
            kyber_install_dir: self.kyber_install_dir.clone(),
            kyclient_path: self.kyclient_path.clone(),
            web_port: self.web_port,
            ui: self.ui.clone(),
            active_setup: self.active_setup.clone(),
        };
        let setup = Setup {
            emission: self.emission.clone(),
            reception: self.reception.clone(),
        };
        (user, setup)
    }

    /// Merge a user config and a setup document into the runtime aggregate.
    fn merge(user: UserConf, setup: Setup) -> Self {
        Self {
            kyber_install_dir: user.kyber_install_dir,
            kyclient_path: user.kyclient_path,
            web_port: user.web_port,
            ui: user.ui,
            active_setup: user.active_setup,
            emission: setup.emission,
            reception: setup.reception,
        }
    }
}

// ---------------------------------------------------------------------------
// File 1 — kyberfrog.toml (machine + UI + active-setup pointer)
// ---------------------------------------------------------------------------

/// The `kyberfrog.toml` file: everything specific to *this* machine.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConf {
    pub kyber_install_dir: PathBuf,
    pub kyclient_path: PathBuf,
    pub web_port: u16,
    pub ui: Ui,
    /// Bare stem of the loaded setup under `setups/` (no extension).
    pub active_setup: String,
}

impl Default for UserConf {
    fn default() -> Self {
        Self {
            kyber_install_dir: default_install_dir(),
            kyclient_path: default_kyclient_path(),
            web_port: DEFAULT_WEB_PORT,
            ui: Ui::default(),
            active_setup: paths::DEFAULT_SETUP_NAME.to_string(),
        }
    }
}

/// UI preferences, persisted machine-side and served to the front-end so a
/// reload (or another browser on the same machine) keeps the operator's choice.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    /// `"dark"` or `"light"`.
    pub theme: String,
    /// `"fr"` or `"en"`.
    pub lang: String,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            lang: "fr".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// File 2 — setups/<name>.toml (the portable show)
// ---------------------------------------------------------------------------

/// One setup document: the emission and reception halves, self-contained and
/// portable across machines.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    pub emission: Emission,
    pub reception: Reception,
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
///
/// Note: `kyclient_path` is *not* here — it is a per-machine path on
/// [`Config`], so a setup stays portable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Reception {
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
    /// The non-viewer settings plus the machine `kyclient_path`, cloned for the
    /// runtime supervisor.
    pub fn globals(&self, kyclient_path: PathBuf) -> Globals {
        Globals {
            kyclient_path,
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

    /// Remote-control viewer (desktop takeover): run the viewer **windowed** and
    /// forward keyboard + mouse, grabbing the keyboard, so the operator drives
    /// the remote machine — remote desktop over Kyber's QUIC transport. Forces
    /// `--inputs true --keyboard-grab true` and suppresses `--fullscreen` (so the
    /// window chrome and the Ctrl+Alt+F escape stay reachable). Mutually
    /// exclusive with `spout_out`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub remote_control: bool,

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

        // A remote-control viewer runs windowed (no --fullscreen) with inputs +
        // keyboard grab forced on. A windowless Spout relay (`spout_out`) takes
        // precedence if both are somehow set in a hand-edited config.
        let remote = viewer.remote_control && viewer.spout_out.is_none();

        if let Some(name) = &viewer.spout_out {
            // Windowless Spout relay. Conflicts with --fullscreen, so emit one
            // or the other, never both.
            args.push("--spout-out".to_string());
            args.push(name.clone());
        } else if viewer.fullscreen && !remote {
            args.push("--fullscreen".to_string());
        }

        // Remote control overrides the passive-display globals: forward inputs
        // and grab the keyboard so the operator drives the remote machine.
        let inputs = remote || self.forward_inputs;
        let keyboard_grab = remote || self.keyboard_grab;
        args.push("--inputs".to_string());
        args.push(inputs.to_string());
        args.push("--audio".to_string());
        args.push(self.audio.to_string());
        args.push("--keyboard-grab".to_string());
        args.push(keyboard_grab.to_string());

        // Positional IP last.
        args.push(viewer.server.clone());

        args
    }
}

/// Path to the `kycontroller` binary inside an install directory
/// (`kycontroller.exe` on Windows, bare `kycontroller` elsewhere).
pub fn kycontroller_path(install_dir: &Path) -> PathBuf {
    let exe = if cfg!(windows) {
        "kycontroller.exe"
    } else {
        "kycontroller"
    };
    install_dir.join(exe)
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load the configuration: read `kyberfrog.toml`, migrating a legacy
/// single-file config (transmitters/viewers inline) into a `setups/` document
/// on the way, then read the active setup. On first run, writes defaults.
pub fn load() -> Result<Config> {
    let user_path = paths::config_file();

    let Ok(content) = fs::read_to_string(&user_path) else {
        info!("No config at {user_path:?}; creating a default one");
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    };

    // A pre-split `kyberfrog.toml` carries `[emission]` / `[reception]` inline.
    // Migrate it: move those halves into the default setup document, and rewrite
    // `kyberfrog.toml` as a pure user config pointing at it.
    let raw: toml::Table =
        toml::from_str(&content).with_context(|| format!("parsing config at {user_path:?}"))?;
    if raw.contains_key("emission") || raw.contains_key("reception") {
        migrate_legacy(raw).context("migrating legacy single-file config")?;
        // Re-read the freshly written user config below.
        return load();
    }

    let user: UserConf =
        toml::from_str(&content).with_context(|| format!("parsing user config at {user_path:?}"))?;
    let setup = load_setup(&user.active_setup)?;
    let config = Config::merge(user, setup);
    validate(&config)?;
    Ok(config)
}

/// Read one setup document by name, or a default (empty) one if it is missing.
pub fn load_setup(name: &str) -> Result<Setup> {
    let path = paths::setup_file(name);
    match fs::read_to_string(&path) {
        Ok(content) => {
            toml::from_str(&content).with_context(|| format!("parsing setup at {path:?}"))
        }
        Err(_) => {
            info!("Setup {name:?} not found at {path:?}; using an empty one");
            Ok(Setup::default())
        }
    }
}

/// Persist `config`: write `kyberfrog.toml` (machine + UI + pointer) and the
/// active setup document, creating parent dirs as needed.
pub fn save(config: &Config) -> Result<()> {
    let (user, setup) = config.split();
    write_user(&user)?;
    write_setup(&config.active_setup, &setup)?;
    Ok(())
}

/// Persist only `kyberfrog.toml` (machine + UI + active-setup pointer), leaving
/// the setup document untouched — for a UI preference change or an active-setup
/// switch where the show itself didn't change.
pub fn save_user(config: &Config) -> Result<()> {
    write_user(&config.split().0)
}

/// Write the active setup under a *new* name and switch `config.active_setup`
/// to it (the "Save as" / export-to-machine operation). Returns the new active
/// name (sanitized).
pub fn save_setup_as(config: &mut Config, name: &str) -> Result<String> {
    let name = sanitize_setup_name(name);
    anyhow::ensure!(!name.is_empty(), "empty setup name");
    let (_, setup) = config.split();
    write_setup(&name, &setup)?;
    config.active_setup = name.clone();
    write_user(&config.split().0)?;
    Ok(name)
}

/// Validate and store a setup received as raw TOML text (cross-machine import /
/// upload). Parses it as a [`Setup`] first — a malformed file is rejected
/// without writing — then writes it under a sanitized name. Returns the name
/// actually written; the caller loads it to make it active.
pub fn import_setup(requested_name: &str, toml_text: &str) -> Result<String> {
    let setup: Setup = toml::from_str(toml_text).context("parsing imported setup")?;
    let name = sanitize_setup_name(requested_name);
    anyhow::ensure!(!name.is_empty(), "empty setup name");
    write_setup(&name, &setup)?;
    Ok(name)
}

/// The setup documents present under `setups/`, by bare name, sorted.
pub fn list_setups() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(paths::setups_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// `true` if `name` is safe as a setup file stem (no path components, etc.).
pub fn is_safe_setup_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Coerce an arbitrary requested name into a safe setup stem.
fn sanitize_setup_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').chars().take(64).collect()
}

fn write_user(user: &UserConf) -> Result<()> {
    let path = paths::config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {parent:?}"))?;
    }
    let body = toml::to_string_pretty(user).context("serializing user config")?;
    let header = "# KyberFrog — machine config for this PC.\n\
                  # Per-machine only: install/kyclient paths, web port, UI prefs,\n\
                  # and `active_setup` = the loaded show under setups/.\n\
                  # The transmitters/viewers themselves live in that setup file.\n\n";
    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing user config to {path:?}"))?;
    info!("Wrote user config to {path:?}");
    Ok(())
}

fn write_setup(name: &str, setup: &Setup) -> Result<()> {
    let path = paths::setup_file(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating setups directory {parent:?}"))?;
    }
    let body = toml::to_string_pretty(setup).context("serializing setup")?;
    let header = format!(
        "# KyberFrog setup — a portable show ({name}).\n\
         # [emission] = transmitters this PC publishes (kycontroller).\n\
         # [reception] = viewers this PC displays (kyclient).\n\
         # Machine paths are NOT here — load this on any PC.\n\n"
    );
    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing setup to {path:?}"))?;
    info!("Wrote setup to {path:?}");
    Ok(())
}

/// Split a legacy inline `kyberfrog.toml` table into a user config and a setup,
/// and write both (the setup as the default document).
fn migrate_legacy(mut raw: toml::Table) -> Result<()> {
    info!("Migrating legacy single-file config into a setups/ document");

    // Pull the two portable halves out of the inline table.
    let emission = raw.remove("emission");
    let reception_val = raw.remove("reception");

    // A legacy `[reception]` carried `kyclient_path`; lift it up to the machine
    // config so the new split keeps it per-machine.
    let mut kyclient_path: Option<toml::Value> = None;
    let reception = reception_val.map(|mut v| {
        if let Some(tbl) = v.as_table_mut() {
            kyclient_path = tbl.remove("kyclient_path");
        }
        v
    });

    // Build and write the default setup from the two halves.
    let mut setup_tbl = toml::Table::new();
    if let Some(emission) = emission {
        setup_tbl.insert("emission".to_string(), emission);
    }
    if let Some(reception) = reception {
        setup_tbl.insert("reception".to_string(), reception);
    }
    let setup: Setup = toml::Value::Table(setup_tbl)
        .try_into()
        .context("interpreting legacy emission/reception")?;
    write_setup(paths::DEFAULT_SETUP_NAME, &setup)?;

    // The remaining keys are the machine config; lifted kyclient_path included.
    if let Some(path) = kyclient_path {
        raw.entry("kyclient_path".to_string()).or_insert(path);
    }
    raw.insert(
        "active_setup".to_string(),
        toml::Value::String(paths::DEFAULT_SETUP_NAME.to_string()),
    );
    let user: UserConf = toml::Value::Table(raw)
        .try_into()
        .context("interpreting legacy machine settings")?;
    write_user(&user)?;
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
            "a viewer has an empty id in setup {:?}",
            config.active_setup
        );
        anyhow::ensure!(seen_ids.insert(v.id.as_str()), "duplicate viewer id {:?}", v.id);
    }

    // Binaries are resolved via PATH when given as bare names; only warn about a
    // missing *absolute* path so we don't false-positive on a valid PATH install.
    let bin = kycontroller_path(&config.kyber_install_dir);
    if !bin.exists() {
        warn!("kycontroller binary not found at {bin:?}; transmitters will fail to start");
    }
    if config.kyclient_path.is_absolute() && !config.kyclient_path.exists() {
        warn!(
            "kyclient not found at {:?} — make sure the Kyber fork is installed and on PATH",
            config.kyclient_path
        );
    }

    Ok(())
}

fn default_install_dir() -> PathBuf {
    // In a bundled install the Kyber binaries sit next to the kyberfrog binary
    // (the installer drops them in the same folder and adds it to PATH), so
    // default to the running exe's directory when kycontroller is actually
    // there. `current_exe()` resolves symlinks, so a /usr/bin/kyberfrog symlink
    // into the .deb's bin dir still lands next to the fork binaries. Fall back
    // to the platform's default install location otherwise. Overridable in
    // kyberfrog.toml.
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .filter(|dir| kycontroller_path(dir).exists())
        .unwrap_or_else(fallback_install_dir)
}

/// Platform default for the Kyber binaries directory, used when they aren't
/// found next to the running executable.
fn fallback_install_dir() -> PathBuf {
    if cfg!(windows) {
        // The NSIS installer's default directory.
        PathBuf::from(r"C:\Program Files\KyberFrog")
    } else {
        // Where the .deb stages the fork binaries.
        PathBuf::from("/usr/lib/kyberfrog/bin")
    }
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

/// Negated `bool`, by reference — for `#[serde(skip_serializing_if)]` so a
/// `false` flag is omitted from the generated TOML.
fn is_false(b: &bool) -> bool {
    !*b
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[test]
    fn setup_round_trips_both_halves() {
        let toml_src = r#"
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

        let setup: Setup = toml::from_str(toml_src).expect("parse setup");
        assert_eq!(setup.emission.transmitters.len(), 1);
        assert_eq!(
            setup.emission.transmitters[0].source,
            Source::Spout {
                sender: "Output A".to_string()
            }
        );
        assert_eq!(setup.reception.viewers.len(), 1);
        assert_eq!(setup.reception.viewers[0].server, "192.168.1.10");

        // Survives a serialize → deserialize cycle unchanged.
        let serialized = toml::to_string_pretty(&setup).expect("serialize");
        let reparsed: Setup = toml::from_str(&serialized).expect("reparse");
        assert_eq!(reparsed.emission.transmitters, setup.emission.transmitters);
        assert_eq!(reparsed.reception.viewers[0].id, "viewer-1");
    }

    #[test]
    fn user_conf_round_trips() {
        let toml_src = r#"
            kyber_install_dir = 'D:\soft\kyber'
            kyclient_path = 'kyclient.exe'
            web_port = 7700
            active_setup = "regie"
            [ui]
            theme = "light"
            lang = "en"
        "#;
        let user: UserConf = toml::from_str(toml_src).expect("parse user conf");
        assert_eq!(user.web_port, 7700);
        assert_eq!(user.active_setup, "regie");
        assert_eq!(user.ui.theme, "light");
        assert_eq!(user.ui.lang, "en");

        let serialized = toml::to_string_pretty(&user).expect("serialize");
        let reparsed: UserConf = toml::from_str(&serialized).expect("reparse");
        assert_eq!(reparsed.active_setup, "regie");
    }

    #[test]
    fn default_user_conf_and_setup_serialize() {
        let user = toml::to_string_pretty(&UserConf::default()).expect("serialize user");
        assert!(user.contains("web_port = 7700"), "got:\n{user}");
        assert!(user.contains("active_setup"), "got:\n{user}");
        let _: UserConf = toml::from_str(&user).expect("reparse user");

        let setup = toml::to_string_pretty(&Setup::default()).expect("serialize setup");
        assert!(setup.contains("base_port = 9000"), "got:\n{setup}");
        let _: Setup = toml::from_str(&setup).expect("reparse setup");
    }

    #[test]
    fn config_split_merge_is_identity() {
        let cfg = Config::default();
        let (user, setup) = cfg.split();
        let back = Config::merge(user, setup);
        assert_eq!(back.web_port, cfg.web_port);
        assert_eq!(back.active_setup, cfg.active_setup);
        assert_eq!(back.kyclient_path, cfg.kyclient_path);
    }

    #[test]
    fn legacy_table_splits_into_user_and_setup() {
        // A pre-split kyberfrog.toml: machine bits + inline emission/reception,
        // with kyclient_path under [reception] (the old home).
        let legacy = r#"
            kyber_install_dir = 'D:\soft\kyber'
            web_port = 7700
            [emission]
            base_port = 9000
            [[emission.transmitter]]
            name = "arena"
            port = 9000
            [emission.transmitter.source]
            type = "screen"
            [reception]
            kyclient_path = 'kyclient.exe'
            tls_tofu = true
            [[reception.viewer]]
            id = "v1"
            server = "10.0.0.2"
            port = 9000
        "#;
        let mut raw: toml::Table = toml::from_str(legacy).unwrap();

        // Mirror migrate_legacy's pure splitting (without touching the FS).
        let emission = raw.remove("emission").unwrap();
        let mut reception = raw.remove("reception").unwrap();
        let kyclient_path = reception
            .as_table_mut()
            .unwrap()
            .remove("kyclient_path")
            .unwrap();

        let mut setup_tbl = toml::Table::new();
        setup_tbl.insert("emission".into(), emission);
        setup_tbl.insert("reception".into(), reception);
        let setup: Setup = toml::Value::Table(setup_tbl).try_into().unwrap();
        assert_eq!(setup.emission.transmitters.len(), 1);
        assert_eq!(setup.reception.viewers[0].server, "10.0.0.2");

        raw.insert("kyclient_path".into(), kyclient_path);
        raw.insert("active_setup".into(), "setup-default".into());
        let user: UserConf = toml::Value::Table(raw).try_into().unwrap();
        assert_eq!(user.kyclient_path, PathBuf::from("kyclient.exe"));
        assert_eq!(user.active_setup, "setup-default");
    }

    #[test]
    fn sanitize_setup_name_is_filesystem_safe() {
        assert_eq!(sanitize_setup_name("  Régie Façade!! "), "R-gie-Fa-ade");
        assert_eq!(sanitize_setup_name("mur_led-2"), "mur_led-2");
        assert!(is_safe_setup_name("setup-default"));
        assert!(!is_safe_setup_name("../evil"));
        assert!(!is_safe_setup_name(""));
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
        .globals(default_kyclient_path());
        let viewer = Viewer {
            id: "v1".into(),
            server: "10.0.0.5".into(),
            port: 8081,
            fullscreen: true,
            spout_out: None,
            remote_control: false,
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
        let globals = Reception::default().globals(default_kyclient_path());
        let viewer = Viewer {
            id: "relay".into(),
            server: "10.0.0.9".into(),
            port: 8082,
            fullscreen: true, // ignored when spout_out is set
            spout_out: Some("KyberFrog".into()),
            remote_control: false,
            enabled: true,
        };
        let args = globals.kyclient_args(&viewer);
        // --spout-out wins; --fullscreen must NOT be emitted (they conflict).
        let so = args.iter().position(|a| a == "--spout-out").unwrap();
        assert_eq!(args[so + 1], "KyberFrog");
        assert!(!args.contains(&"--fullscreen".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("10.0.0.9"));
    }

    fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].as_str())
    }

    /// `--inputs true --keyboard-grab true` regardless of the passive-display
    /// globals, no `--fullscreen` (windowed so Ctrl+Alt+F stays reachable).
    #[test]
    fn remote_control_forces_inputs_and_windowed() {
        let globals = Reception::default().globals(default_kyclient_path());
        let viewer = Viewer {
            id: "takeover".into(),
            server: "10.0.0.7".into(),
            port: 8083,
            fullscreen: true, // suppressed by remote control
            spout_out: None,
            remote_control: true,
            enabled: true,
        };
        let args = globals.kyclient_args(&viewer);
        assert_eq!(arg_value(&args, "--inputs"), Some("true"));
        assert_eq!(arg_value(&args, "--keyboard-grab"), Some("true"));
        assert!(!args.contains(&"--fullscreen".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("10.0.0.7"));
    }

    #[test]
    fn spout_out_wins_over_remote_control() {
        let globals = Reception::default().globals(default_kyclient_path());
        let viewer = Viewer {
            id: "both".into(),
            server: "10.0.0.8".into(),
            port: 8084,
            fullscreen: false,
            spout_out: Some("Relay".into()),
            remote_control: true,
            enabled: true,
        };
        let args = globals.kyclient_args(&viewer);
        assert!(args.contains(&"--spout-out".to_string()));
        assert_eq!(arg_value(&args, "--inputs"), Some("false"));
        assert_eq!(arg_value(&args, "--keyboard-grab"), Some("false"));
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
                remote_control: false,
                enabled: true,
            }],
            ..Reception::default()
        };
        assert_eq!(reception.unique_id(), "viewer-2");
    }
}
