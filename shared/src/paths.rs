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

/// The unified config file (`%APPDATA%\kyberfrog\kyberfrog.toml`).
pub fn config_file() -> PathBuf {
    app_data_dir().join("kyberfrog.toml")
}

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
