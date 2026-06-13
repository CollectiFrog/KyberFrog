// SPDX-License-Identifier: AGPL-3.0-or-later

//! Loading and saving the Director source-of-truth (`transmitters.toml`).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::info;
use shared::{paths, Directory};

/// Load the Director configuration, creating an empty default file on first run.
pub fn load() -> Result<Directory> {
    let path = paths::directory_file();

    let Ok(content) = fs::read_to_string(&path) else {
        info!("No directory file at {path:?}; creating a default one");
        let directory = Directory::default();
        save(&directory)?;
        return Ok(directory);
    };

    let directory: Directory = toml::from_str(&content)
        .with_context(|| format!("parsing transmitter directory at {path:?}"))?;

    validate(&directory)?;
    Ok(directory)
}

/// Persist `directory` to `transmitters.toml`, creating parent dirs as needed.
pub fn save(directory: &Directory) -> Result<()> {
    let path = paths::directory_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {parent:?}"))?;
    }

    let body = toml::to_string_pretty(directory).context("serializing transmitter directory")?;
    let header = "# kyber-anysource Director — transmitter directory.\n\
                  # Each [[transmitter]] becomes one kycontroller instance.\n\n";

    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("writing transmitter directory to {path:?}"))?;
    info!("Wrote transmitter directory to {path:?}");
    Ok(())
}

/// Reject configurations that would collide at runtime (duplicate names/ports,
/// unsafe instance names, missing install dir).
fn validate(directory: &Directory) -> Result<()> {
    let mut seen_names = std::collections::HashSet::new();
    let mut seen_ports = std::collections::HashMap::<u16, &str>::new();

    for tx in &directory.transmitters {
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

    let bin = kycontroller_path(&directory.kyber_install_dir);
    if !bin.exists() {
        // Not fatal — the operator may be editing the directory before the
        // install is in place — but worth a loud warning.
        log::warn!("kycontroller binary not found at {bin:?}; transmitters will fail to start");
    }

    Ok(())
}

/// Path to `kycontroller.exe` inside an install directory.
pub fn kycontroller_path(install_dir: &std::path::Path) -> PathBuf {
    install_dir.join("kycontroller.exe")
}
