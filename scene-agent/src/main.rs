// SPDX-License-Identifier: AGPL-3.0-or-later

//! kyber-anysource Scene Agent.
//!
//! Runs on a scene machine and keeps exactly one `kyclient` alive, fullscreen,
//! connected to the transmitter named in `scene-agent.toml`. kyclient already
//! reconnects on its own; when it gives up and exits (server gone, transmitter
//! removed, …) the agent relaunches it with capped exponential backoff. There
//! is no maintenance/quit mode: the operator drops to a window with the
//! client's own shortcut, they never voluntarily close the viewer.

mod config;

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use config::SceneConfig;
use log::{error, info, warn};
use tokio::process::Command;
use tokio::time::sleep;

const BACKOFF_START: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// A client that stayed up at least this long is considered healthy, so its
/// backoff is reset on the next relaunch.
const HEALTHY_UPTIME: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("kyber-anysource Scene Agent starting");

    let config = config::load().context("loading scene-agent config")?;
    info!(
        "Target: {}:{} via {}",
        config.server,
        config.port,
        config.kyclient_path.display()
    );

    supervise(config).await;

    info!("Scene Agent stopped");
    Ok(())
}

/// Run and relaunch `kyclient` until Ctrl-C is received.
async fn supervise(config: SceneConfig) {
    let args = config.kyclient_args();
    let mut backoff = BACKOFF_START;

    loop {
        info!("Launching: {}", config.redacted_command());

        let started = Instant::now();
        let mut child = match Command::new(&config.kyclient_path).args(&args).spawn() {
            Ok(child) => child,
            Err(err) => {
                error!(
                    "Failed to spawn {}: {err}",
                    config.kyclient_path.display()
                );
                if sleep_or_ctrl_c(backoff).await {
                    return;
                }
                backoff = (backoff * 2).min(BACKOFF_MAX);
                continue;
            }
        };

        tokio::select! {
            wait = child.wait() => {
                let uptime = started.elapsed();
                match wait {
                    Ok(status) => warn!("kyclient exited with {status} after {uptime:.1?}"),
                    Err(err) => warn!("waiting on kyclient failed: {err} after {uptime:.1?}"),
                }
                if uptime >= HEALTHY_UPTIME {
                    backoff = BACKOFF_START;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received, stopping kyclient");
                let _ = child.start_kill();
                let _ = child.wait().await;
                return;
            }
        }

        info!("Relaunching in {backoff:.1?}");
        if sleep_or_ctrl_c(backoff).await {
            return;
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}

/// Sleep for `delay`, returning `true` if Ctrl-C arrives first.
async fn sleep_or_ctrl_c(delay: Duration) -> bool {
    tokio::select! {
        _ = sleep(delay) => false,
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl-C received");
            true
        }
    }
}
