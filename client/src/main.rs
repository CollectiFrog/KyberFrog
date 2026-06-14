// SPDX-License-Identifier: AGPL-3.0-or-later

//! KyberFrog Client.
//!
//! Runs on a scene machine: a system-tray icon lets the operator start, stop,
//! restart and remove kyclient viewers; a web UI at
//! `http://<this-pc>:<web_port>/` offers the same controls plus live logs.
//!
//! kyclient reconnects on its own; when it gives up the client relaunches it
//! with capped backoff. Instances are persisted to `scene-agent.toml` and
//! every `enabled` one is relaunched on boot, so a scene PC comes back on its
//! own after a reboot or a regie restart.

mod config;
mod supervisor;
mod tray;
mod web;

use std::sync::Arc;

use anyhow::{Context, Result};
use flexi_logger::{Duplicate, FileSpec, Logger, WriteMode};
use log::{error, info, warn};
use shared::paths;
use supervisor::Manager;
use tokio::sync::Mutex;
use tray::{TrayCommand, TrayModel};
use web::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    let _logger = Logger::try_with_env_or_str("info")
        .context("configuring logger")?
        .log_to_file(
            FileSpec::default()
                .directory(paths::log_dir())
                .basename("kyberfrog-client")
                .suppress_timestamp(),
        )
        .append()
        .duplicate_to_stderr(Duplicate::All)
        .write_mode(WriteMode::Direct)
        .start()
        .context("starting logger")?;

    info!("KyberFrog Client starting 🐸");
    info!("Data directory: {:?}", paths::app_data_dir());
    info!("Log file: {:?}", paths::client_log_file());

    let config = config::load().context("loading client config")?;
    let web_port = config.web_port;
    let globals = config.globals();

    let mut manager = Manager::new(globals);
    let status = manager.status();

    let mut started = 0;
    for instance in &config.instances {
        if instance.enabled {
            manager.start(instance);
            started += 1;
        }
    }
    info!("Started {started} instance(s)");

    let tray_model = TrayModel::new(config.instances.clone(), status.clone(), web_port);

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        manager: Mutex::new(manager),
        status,
    });

    let web_task = web::spawn(state.clone(), web_port);

    let (mut tray_handle, mut command_rx) = match tray::spawn(tray_model.clone()) {
        Ok(pair) => (Some(pair.0), pair.1),
        Err(err) => {
            error!("Failed to start system tray: {err}. Running headless.");
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            (None, rx)
        }
    };

    info!("Client ready");

    loop {
        tokio::select! {
            maybe = command_rx.recv() => {
                let Some(command) = maybe else {
                    let _ = tokio::signal::ctrl_c().await;
                    break;
                };
                if handle_command(command, &state, &tray_model).await {
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
    if let Some(mut h) = tray_handle.take() {
        h.shutdown().await;
    }

    info!("KyberFrog Client stopped");
    Ok(())
}

/// Apply one tray command. Returns `true` when the client should quit.
async fn handle_command(
    command: TrayCommand,
    state: &Arc<AppState>,
    model: &TrayModel,
) -> bool {
    match command {
        TrayCommand::Start { id } => {
            let mut config = state.config.lock().await;
            let mut manager = state.manager.lock().await;
            if let Some(inst) = config.get_mut(&id) {
                inst.enabled = true;
                let inst = inst.clone();
                manager.start(&inst);
            } else {
                warn!("Start requested for unknown instance {id:?}");
                return false;
            }
            persist(&config, model);
        }
        TrayCommand::Stop { id } => {
            {
                let mut config = state.config.lock().await;
                if let Some(inst) = config.get_mut(&id) {
                    inst.enabled = false;
                }
                persist(&config, model);
            }
            state.manager.lock().await.stop(&id).await;
        }
        TrayCommand::Restart { id } => {
            let config = state.config.lock().await;
            let Some(inst) = config.get(&id).cloned() else {
                warn!("Restart requested for unknown instance {id:?}");
                return false;
            };
            drop(config);
            state.manager.lock().await.restart(&inst).await;
        }
        TrayCommand::Remove { id } => {
            state.manager.lock().await.stop(&id).await;
            let mut config = state.config.lock().await;
            config.instances.retain(|i| i.id != id);
            persist(&config, model);
        }
        TrayCommand::Quit => {
            info!("Quit requested from tray");
            return true;
        }
    }
    false
}

/// Persist the config and refresh the tray model.
fn persist(config: &config::ClientConfig, model: &TrayModel) {
    if let Err(err) = config::save(config) {
        error!("Failed to persist client config: {err:#}");
    }
    model.set_instances(config.instances.clone());
}
