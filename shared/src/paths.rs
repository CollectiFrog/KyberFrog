// SPDX-License-Identifier: AGPL-3.0-or-later

//! Well-known filesystem locations for KyberFrog.
//!
//! Everything lives under `%APPDATA%\kyberfrog` on Windows. On other platforms
//! (used only for tests / dev builds) we fall back to `$HOME` or the current
//! directory so the crate still compiles and runs.

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

/// Directory holding log files (`%APPDATA%\kyberfrog\logs`).
pub fn log_dir() -> PathBuf {
    app_data_dir().join("logs")
}

/// The KyberFrog app log file (`%APPDATA%\kyberfrog\logs\kyberfrog.log`).
pub fn app_log_file() -> PathBuf {
    log_dir().join("kyberfrog.log")
}

/// Per-viewer kyclient log file (`%APPDATA%\kyberfrog\logs\kyclient-<id>.log`).
pub fn kyclient_log_file(id: &str) -> PathBuf {
    log_dir().join(format!("kyclient-{id}.log"))
}

/// Per-transmitter kycontroller log file
/// (`%APPDATA%\kyberfrog\instances\<name>\kycontroller.log`).
pub fn kycontroller_log_file(name: &str) -> PathBuf {
    instance_dir(name).join("kycontroller.log")
}

/// Parent directory of all generated per-instance configs.
pub fn instances_dir() -> PathBuf {
    app_data_dir().join("instances")
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

fn base_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".")
}
