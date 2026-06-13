// SPDX-License-Identifier: AGPL-3.0-or-later

//! kyber-anysource Director.
//!
//! Reads `transmitters.toml`, generates one `kyber_config.toml` per
//! transmitter, and supervises one `kycontroller` process per transmitter
//! (isolated by port and by `KYBER_CONFIG_PATH`). A system-tray UI lets the
//! operator add (with a live Spout-sender picker), remove and restart
//! transmitters at runtime; changes are persisted back to `transmitters.toml`.

mod config;
#[cfg_attr(not(windows), allow(dead_code))]
mod spout;
mod supervisor;
mod tray;
mod web;

use anyhow::{Context, Result};
use log::{error, info, warn};
use shared::{paths, Directory, Source, Transmitter};
use supervisor::Manager;
use tray::{TrayCommand, TrayModel};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("kyber-anysource Director starting");
    info!("Data directory: {:?}", paths::app_data_dir());

    let mut directory = config::load().context("loading transmitter directory")?;
    info!("Kyber install: {:?}", directory.kyber_install_dir);

    let mut manager = Manager::new(
        directory.kyber_install_dir.clone(),
        directory.defaults.clone(),
    );

    // Start everything already configured.
    for transmitter in &directory.transmitters {
        if let Err(err) = manager.start(transmitter.clone()) {
            error!("Failed to start transmitter {:?}: {err:#}", transmitter.name);
        }
    }
    info!("Started {} transmitter(s)", directory.transmitters.len());

    // Shared model the tray reads when rendering its menu — also the data
    // source for the web UI / discovery endpoint.
    let model = TrayModel::new(directory.transmitters.clone(), manager.status());

    // Web UI + GET /transmitters, reachable from the LAN.
    let web_task = web::spawn(model.clone(), directory.web_port());

    let (mut tray_handle, mut command_rx): (
        Option<tray::TrayHandle>,
        tokio::sync::mpsc::Receiver<TrayCommand>,
    ) = match tray::spawn(model.clone()) {
        Ok(pair) => (Some(pair.0), pair.1),
        Err(err) => {
            error!("Failed to start system tray: {err}. Running headless.");
            // Closed channel: recv() yields None and we idle until Ctrl-C.
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            (None, rx)
        }
    };

    info!("Director ready");

    loop {
        tokio::select! {
            maybe = command_rx.recv() => {
                let Some(command) = maybe else {
                    // Tray gone: idle until Ctrl-C.
                    let _ = tokio::signal::ctrl_c().await;
                    break;
                };
                if handle_command(command, &mut directory, &mut manager, &model).await {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received");
                break;
            }
        }
    }

    info!("Shutting down transmitters");
    manager.shutdown_all().await;
    web_task.abort();
    if let Some(mut handle) = tray_handle.take() {
        handle.shutdown().await;
    }

    info!("Director stopped");
    Ok(())
}

/// Apply one tray command. Returns `true` when the Director should quit.
async fn handle_command(
    command: TrayCommand,
    directory: &mut Directory,
    manager: &mut Manager,
    model: &TrayModel,
) -> bool {
    match command {
        TrayCommand::AddSpout { sender } => {
            let name = unique_name(&sender, directory);
            let port = allocate_port(directory);
            add_transmitter(
                Transmitter {
                    name,
                    port,
                    source: Source::Spout { sender },
                },
                directory,
                manager,
                model,
            );
        }
        TrayCommand::AddScreen => {
            let name = unique_name("screen", directory);
            let port = allocate_port(directory);
            add_transmitter(
                Transmitter {
                    name,
                    port,
                    source: Source::Screen { display: None },
                },
                directory,
                manager,
                model,
            );
        }
        TrayCommand::Remove { name } => {
            info!("Removing transmitter {name:?}");
            manager.stop(&name).await;
            directory.transmitters.retain(|t| t.name != name);
            persist(directory, model);
        }
        TrayCommand::Restart { name } => {
            info!("Restarting transmitter {name:?}");
            let Some(transmitter) = directory.get(&name).cloned() else {
                warn!("Restart requested for unknown transmitter {name:?}");
                return false;
            };
            manager.stop(&name).await;
            if let Err(err) = manager.start(transmitter) {
                error!("Failed to restart {name:?}: {err:#}");
            }
        }
        TrayCommand::Quit => {
            info!("Quit requested from tray");
            return true;
        }
    }
    false
}

/// Register, start and persist a new transmitter.
fn add_transmitter(
    transmitter: Transmitter,
    directory: &mut Directory,
    manager: &mut Manager,
    model: &TrayModel,
) {
    info!(
        "Adding transmitter {:?} on port {} ({})",
        transmitter.name,
        transmitter.port,
        transmitter.source.label()
    );
    if let Err(err) = manager.start(transmitter.clone()) {
        error!("Failed to start new transmitter {:?}: {err:#}", transmitter.name);
        return;
    }
    directory.transmitters.push(transmitter);
    persist(directory, model);
}

/// Save the directory and refresh the tray model.
fn persist(directory: &Directory, model: &TrayModel) {
    if let Err(err) = config::save(directory) {
        error!("Failed to persist transmitter directory: {err:#}");
    }
    model.set_transmitters(directory.transmitters.clone());
}

/// A filesystem-safe, unique transmitter name derived from `base`.
fn unique_name(base: &str, directory: &Directory) -> String {
    let mut candidate = sanitize(base);
    if candidate.is_empty() {
        candidate = "tx".to_string();
    }
    if directory.get(&candidate).is_none() {
        return candidate;
    }
    let mut i = 2;
    loop {
        let next = format!("{candidate}-{i}");
        if directory.get(&next).is_none() {
            return next;
        }
        i += 1;
    }
}

/// Lowercase, collapse runs of non-alphanumerics to single `-`, trim edges.
fn sanitize(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Pick a control-plane port for a new transmitter: the lowest port at or above
/// the configured base that is neither already assigned to another transmitter
/// nor currently bound by some other process.
fn allocate_port(directory: &Directory) -> u16 {
    let mut port = directory.base_port();
    loop {
        port = directory.next_free_port(port);
        if port_is_available(port) {
            return port;
        }
        warn!("Port {port} is already in use, trying the next one");
        match port.checked_add(1) {
            Some(next) => port = next,
            // Exhausted the range; hand back the base and let the spawn fail loudly.
            None => return directory.base_port(),
        }
    }
}

/// Best-effort check that `port` can currently be bound for TCP.
fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}
