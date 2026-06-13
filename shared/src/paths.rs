// SPDX-License-Identifier: AGPL-3.0-or-later

//! Well-known filesystem locations for kyber-anysource.
//!
//! Everything lives under `%APPDATA%\kyber-anysource` on Windows. On other platforms
//! (used only for tests / dev builds) we fall back to `$HOME` or the current
//! directory so the crate still compiles and runs.

use std::path::PathBuf;

/// Root of the kyber-anysource data directory (`%APPDATA%\kyber-anysource`).
pub fn app_data_dir() -> PathBuf {
    base_dir().join("kyber-anysource")
}

/// The Director source-of-truth file (`%APPDATA%\kyber-anysource\transmitters.toml`).
pub fn directory_file() -> PathBuf {
    app_data_dir().join("transmitters.toml")
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
