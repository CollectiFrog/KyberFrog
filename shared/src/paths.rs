// SPDX-License-Identifier: AGPL-3.0-or-later

//! Well-known filesystem locations for KyberFrog.
//!
//! Windows keeps everything under `%APPDATA%\kyberfrog`. Elsewhere the XDG Base
//! Directory spec applies: `$XDG_CONFIG_HOME/kyberfrog` (default
//! `~/.config/kyberfrog`) for the config and the setups, and
//! `$XDG_STATE_HOME/kyberfrog` (default `~/.local/state/kyberfrog`) for the logs
//! and the generated per-instance configs — those are regenerated state, not
//! something an operator edits or backs up, and logs under `~/.config` would be
//! plainly wrong.

use std::path::PathBuf;

/// Root of the KyberFrog data directory (`%APPDATA%\kyberfrog`).
pub fn app_data_dir() -> PathBuf {
    base_dir().join("kyberfrog")
}

/// The machine/user config file (`%APPDATA%\kyberfrog\kyberfrog.toml`).
///
/// Holds only the per-machine bits (install/kyclient paths, web port), the UI
/// preferences and a pointer (`active_setup`) to the currently-loaded setup
/// document. The transmitters/viewers themselves live in a setup file under
/// [`setups_dir`].
pub fn config_file() -> PathBuf {
    app_data_dir().join("kyberfrog.toml")
}

/// Directory holding the setup documents (`%APPDATA%\kyberfrog\setups`).
///
/// Each `*.toml` here is one self-contained, portable show: the emission
/// (transmitters) and reception (viewers) halves. This is what
/// "save / load" and the cross-machine export/import operate on; the machine
/// paths in [`config_file`] are never part of it.
pub fn setups_dir() -> PathBuf {
    app_data_dir().join("setups")
}

/// The setup document for `name` (`%APPDATA%\kyberfrog\setups\<name>.toml`).
///
/// `name` is a bare stem (no extension, no separators) — see
/// [`crate::config::is_safe_setup_name`].
pub fn setup_file(name: &str) -> PathBuf {
    setups_dir().join(format!("{name}.toml"))
}

/// Name of the setup created on first run / when none is selected.
pub const DEFAULT_SETUP_NAME: &str = "setup-default";

/// Directory holding log files (`<state>/kyberfrog/logs`).
pub fn log_dir() -> PathBuf {
    app_state_dir().join("logs")
}

/// The KyberFrog app log file (`<state>/kyberfrog/logs/kyberfrog.log`).
pub fn app_log_file() -> PathBuf {
    log_dir().join("kyberfrog.log")
}

/// Per-viewer kyclient log file (`<state>/kyberfrog/logs/kyclient-<id>.log`).
pub fn kyclient_log_file(id: &str) -> PathBuf {
    log_dir().join(format!("kyclient-{id}.log"))
}

/// Per-transmitter kycontroller log file
/// (`<state>/kyberfrog/instances/<name>/kycontroller.log`).
pub fn kycontroller_log_file(name: &str) -> PathBuf {
    instance_dir(name).join("kycontroller.log")
}

/// Parent directory of all generated per-instance configs.
pub fn instances_dir() -> PathBuf {
    app_state_dir().join("instances")
}

/// The directory owned by a single transmitter instance.
pub fn instance_dir(name: &str) -> PathBuf {
    instances_dir().join(name)
}

/// The generated `kyber_config.toml` for a transmitter (target of
/// `KYBER_CONFIG_PATH`).
pub fn instance_config(name: &str) -> PathBuf {
    instance_dir(name).join("kyber_config.toml")
}

/// Root for operator-owned data: the machine config and the setup documents.
fn base_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata);
    }
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// Root for machine-generated state: logs and per-instance configs. Same place
/// as [`base_dir`] on Windows, `$XDG_STATE_HOME` elsewhere.
fn state_base_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata);
    }
    xdg_dir("XDG_STATE_HOME", ".local/state")
}

/// `$VAR` when it holds an absolute path (the spec says relative ones must be
/// ignored), else `$HOME/<fallback>`, else the current directory.
fn xdg_dir(var: &str, fallback: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(var) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return path;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(fallback);
    }
    PathBuf::from(".")
}

/// Root of the generated-state directory (`<state>/kyberfrog`).
fn app_state_dir() -> PathBuf {
    state_base_dir().join("kyberfrog")
}
