// SPDX-License-Identifier: AGPL-3.0-or-later

// Release builds are a real Windows app (#21): no console window at launch.
// Debug builds keep the console (and flexi_logger's stderr sink) for the dev
// loop; the log file under %APPDATA%\kyberfrog\logs covers release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! KyberFrog — one app, installed on every machine.
//!
//! Reads `kyberfrog.toml` and runs a single supervisor that manages both roles:
//! the **transmitters** this machine publishes (one `kycontroller` each) and the
//! **viewers** it displays (one `kyclient` each). A web UI on one port (default
//! 7700), a native window over it (`shell/`, Windows) and a system tray drive
//! both halves; every change is persisted to `kyberfrog.toml`, so the machine
//! comes back on its own after a reboot.

mod app;
mod cameras;
mod discovery;
mod displays;
mod shell;
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

/// Not async: the Tauri event loop (`shell::run` on Windows) must own the main
/// thread, so the tokio runtime is built by hand and the whole async app runs
/// on it — bootstrapped via `block_on`, then driven by the shell.
fn main() -> Result<()> {
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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;

    let boot = runtime.block_on(bootstrap())?;

    // The shell owns the rest of the process lifetime: the Tauri window +
    // event loop on Windows, the plain headless command loop elsewhere.
    shell::run(runtime, boot)
}

/// Everything that used to be the async `main` up to "ready": load the config,
/// start both halves under the supervisor, the mDNS discovery, the web server
/// and the tray.
async fn bootstrap() -> Result<shell::Boot> {
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

    // Start the emitter half: the active set ("all" transmitter in send-all
    // mode, else the configured per-source list).
    let active_tx = config.emission.active_transmitters();
    for tx in &active_tx {
        if let Err(err) = manager.start_transmitter(tx) {
            error!("Failed to start transmitter {:?}: {err:#}", tx.name);
        }
    }
    info!("Started {} transmitter(s)", active_tx.len());

    // mDNS auto-discovery (#20): announce this machine's transmitters and
    // browse the LAN for the other machines'. Best-effort — the app runs fine
    // without it (the viewer form falls back to manual IP entry).
    let discovery = if config.mdns {
        match discovery::Discovery::new(app::hostname()) {
            Ok(discovery) => {
                discovery.sync(&active_tx);
                discovery.spawn_browser();
                Some(discovery)
            }
            Err(err) => {
                error!("mDNS discovery disabled: {err}");
                None
            }
        }
    } else {
        info!("mDNS discovery disabled by config (mdns = false)");
        None
    };

    // Start the receiver half (only the enabled viewers).
    let mut started = 0;
    for viewer in &config.reception.viewers {
        if viewer.enabled {
            manager.start_viewer(viewer);
            started += 1;
        }
    }
    info!("Started {started} viewer(s)");

    let tray_model = TrayModel::new(active_tx, config.reception.viewers.clone(), status.clone());

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        manager: Mutex::new(manager),
        status,
        tray_model: tray_model.clone(),
        discovery,
    });

    let web_task = web::spawn(state.clone(), web_port);

    let (tray_handle, command_rx): (
        Option<tray::TrayHandle>,
        tokio::sync::mpsc::Receiver<TrayCommand>,
    ) = match tray::spawn(tray_model) {
        Ok(pair) => (Some(pair.0), pair.1),
        Err(err) => {
            error!("Failed to start system tray: {err}. Running without it.");
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            (None, rx)
        }
    };

    Ok(shell::Boot {
        state,
        web_task,
        tray_handle,
        command_rx,
        web_port,
    })
}
