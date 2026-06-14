// SPDX-License-Identifier: AGPL-3.0-or-later

//! KyberFrog Client.
//!
//! Runs on a scene machine: serves a web UI (`http://<this-pc>:<web_port>/`) and
//! supervises N `kyclient` viewers, each connected to a transmitter. kyclient
//! reconnects on its own; when it gives up the client relaunches it with capped
//! backoff. Instances are persisted to `scene-agent.toml` and every `enabled`
//! one is relaunched on boot, so a scene PC comes back on its own.

mod config;
mod supervisor;
mod web;

use std::sync::Arc;

use anyhow::{Context, Result};
use flexi_logger::{Duplicate, FileSpec, Logger, WriteMode};
use log::info;
use shared::paths;
use supervisor::Manager;
use tokio::sync::Mutex;
use web::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to the terminal AND to %APPDATA%\kyberfrog\logs so the web UI can show
    // the client's own log. RUST_LOG still overrides the level.
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

    // Autostart: launch every instance marked as "should be running".
    let mut started = 0;
    for instance in &config.instances {
        if instance.enabled {
            manager.start(instance);
            started += 1;
        }
    }
    info!("Started {started} instance(s)");

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        manager: Mutex::new(manager),
        status,
    });

    let web_task = web::spawn(state.clone(), web_port);
    info!("Client ready");

    let _ = tokio::signal::ctrl_c().await;
    info!("Ctrl-C received, shutting down");

    state.manager.lock().await.shutdown_all().await;
    web_task.abort();

    info!("KyberFrog Client stopped");
    Ok(())
}
