// SPDX-License-Identifier: AGPL-3.0-or-later

//! KyberFrog — one app, installed on every machine.
//!
//! Reads `kyberfrog.toml` and runs a single supervisor that manages both roles:
//! the **transmitters** this machine publishes (one `kycontroller` each) and the
//! **viewers** it displays (one `kyclient` each). A web UI on one port (default
//! 7700) and a system tray drive both halves; every change is persisted to
//! `kyberfrog.toml`, so the machine comes back on its own after a reboot.

mod app;
#[cfg_attr(not(windows), allow(dead_code))]
mod spout;
mod supervisor;
mod tray;
mod web;

use std::sync::Arc;

use anyhow::{Context, Result};
use flexi_logger::{Duplicate, FileSpec, Logger, WriteMode};
use log::{error, info};
use shared::paths;
use supervisor::Manager;
use tokio::sync::Mutex;
use tray::{TrayCommand, TrayModel};

use app::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to the terminal AND to a file under %APPDATA%\kyberfrog\logs, so the
    // tray's "Ouvrir logs" item has something to show. RUST_LOG still overrides.
    let _logger = Logger::try_with_env_or_str("info")
        .context("configuring logger")?
        .log_to_file(
            FileSpec::default()
                .directory(paths::log_dir())
                .basename("kyberfrog")
                .suppress_timestamp(),
        )
        .format(flexi_logger::detailed_format)
        .append()
        .duplicate_to_stderr(Duplicate::All)
        .write_mode(WriteMode::Direct)
        .start()
        .context("starting logger")?;

    info!("KyberFrog starting 🐸");
    info!("Data directory: {:?}", paths::app_data_dir());
    info!("Log file: {:?}", paths::app_log_file());

    let config = shared::config::load().context("loading config")?;
    info!("Kyber install: {:?}", config.kyber_install_dir);
    let web_port = config.web_port;

    let mut manager = Manager::new(
        config.kyber_install_dir.clone(),
        config.emission.defaults.clone(),
        config.globals(),
    );
    let status = manager.status();

    // Start the emitter half.
    for tx in &config.emission.transmitters {
        if let Err(err) = manager.start_transmitter(tx) {
            error!("Failed to start transmitter {:?}: {err:#}", tx.name);
        }
    }
    info!("Started {} transmitter(s)", config.emission.transmitters.len());

    // Start the receiver half (only the enabled viewers).
    let mut started = 0;
    for viewer in &config.reception.viewers {
        if viewer.enabled {
            manager.start_viewer(viewer);
            started += 1;
        }
    }
    info!("Started {started} viewer(s)");

    let tray_model = TrayModel::new(
        config.emission.transmitters.clone(),
        config.reception.viewers.clone(),
        status.clone(),
        web_port,
    );

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        manager: Mutex::new(manager),
        status,
        tray_model: tray_model.clone(),
    });

    let web_task = web::spawn(state.clone(), web_port);

    let (mut tray_handle, mut command_rx): (
        Option<tray::TrayHandle>,
        tokio::sync::mpsc::Receiver<TrayCommand>,
    ) = match tray::spawn(tray_model.clone()) {
        Ok(pair) => (Some(pair.0), pair.1),
        Err(err) => {
            error!("Failed to start system tray: {err}. Running headless.");
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            (None, rx)
        }
    };

    info!("KyberFrog ready");

    loop {
        tokio::select! {
            maybe = command_rx.recv() => {
                let Some(command) = maybe else {
                    // Tray gone: idle until Ctrl-C.
                    let _ = tokio::signal::ctrl_c().await;
                    break;
                };
                if handle_command(command, &state).await {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received");
                break;
            }
        }
    }

    info!("Shutting down");
    state.manager.lock().await.shutdown_all().await;
    web_task.abort();
    if let Some(mut handle) = tray_handle.take() {
        handle.shutdown().await;
    }

    info!("KyberFrog stopped");
    Ok(())
}

/// Apply one tray command. Returns `true` when the app should quit.
async fn handle_command(command: TrayCommand, state: &Arc<AppState>) -> bool {
    match command {
        TrayCommand::AddSpout { sender } => app::op_add_spout(state, sender, None).await,
        TrayCommand::AddScreen => app::op_add_screen(state, None).await,
        TrayCommand::RestartTx { name } => app::op_restart_transmitter(state, &name).await,
        TrayCommand::RemoveTx { name } => app::op_remove_transmitter(state, &name).await,
        TrayCommand::StartViewer { id } => app::op_start_viewer(state, &id).await,
        TrayCommand::StopViewer { id } => app::op_stop_viewer(state, &id).await,
        TrayCommand::RestartViewer { id } => app::op_restart_viewer(state, &id).await,
        TrayCommand::RemoveViewer { id } => app::op_remove_viewer(state, &id).await,
        TrayCommand::Quit => {
            info!("Quit requested from tray");
            return true;
        }
    }
    false
}
