// SPDX-License-Identifier: AGPL-3.0-or-later

//! Per-transmitter process supervision and the runtime [`Manager`].
//!
//! Each transmitter maps to one [`Instance`]. [`Instance::prepare`] writes the
//! generated `kyber_config.toml`; the supervise loop then runs the
//! `kycontroller` process, restarting it with capped exponential backoff until
//! a shutdown signal arrives.
//!
//! [`Manager`] owns the set of running instances and lets the UI start, stop
//! and inspect them at runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{error, info, warn};
use shared::{gen, paths, Transmitter};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const BACKOFF_START: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// A process that stayed up at least this long is considered healthy, so its
/// backoff is reset on the next restart.
const HEALTHY_UPTIME: Duration = Duration::from_secs(30);

/// Coarse lifecycle state of a supervised instance, surfaced to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Starting,
    Running,
    Restarting,
    Stopped,
}

impl State {
    /// Status glyph shown in the tray menu. Win32 menus draw text with GDI,
    /// which has no color-emoji support, so states are distinguished by shape
    /// rather than color: ○ starting, ● running, ◐ restarting, ✗ stopped.
    pub fn symbol(self) -> &'static str {
        match self {
            State::Starting => "○",
            State::Running => "●",
            State::Restarting => "◐",
            State::Stopped => "✗",
        }
    }

    /// Machine-readable status for the web UI / discovery endpoint.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Starting => "starting",
            State::Running => "running",
            State::Restarting => "restarting",
            State::Stopped => "stopped",
        }
    }
}

/// Shared map of instance name -> current [`State`].
pub type StatusMap = Arc<Mutex<HashMap<String, State>>>;

fn set_state(status: &StatusMap, name: &str, state: State) {
    if let Ok(mut map) = status.lock() {
        map.insert(name.to_string(), state);
    }
}

/// A prepared, supervisable transmitter instance.
pub struct Instance {
    transmitter: Transmitter,
    install_dir: PathBuf,
    config_path: PathBuf,
}

impl Instance {
    /// Generate the instance config on disk and return a supervisable handle.
    pub fn prepare(
        transmitter: Transmitter,
        defaults: &toml::Table,
        install_dir: PathBuf,
    ) -> Result<Self> {
        let dir = paths::instance_dir(&transmitter.name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating instance directory {dir:?}"))?;

        let config_path = paths::instance_config(&transmitter.name);
        let content = gen::render_config(&transmitter, defaults)
            .with_context(|| format!("rendering config for transmitter {:?}", transmitter.name))?;
        std::fs::write(&config_path, content)
            .with_context(|| format!("writing instance config {config_path:?}"))?;

        info!(
            "[{}] prepared (port {}, {}) -> {config_path:?}",
            transmitter.name,
            transmitter.port,
            transmitter.source.label()
        );

        Ok(Self {
            transmitter,
            install_dir,
            config_path,
        })
    }

    fn binary(&self) -> PathBuf {
        crate::config::kycontroller_path(&self.install_dir)
    }

    /// Run and keep restarting the kycontroller process until `shutdown` flips
    /// to `true`, reporting lifecycle transitions into `status`.
    async fn supervise(self, mut shutdown: watch::Receiver<bool>, status: StatusMap) {
        let name = self.transmitter.name.clone();
        let binary = self.binary();
        let mut backoff = BACKOFF_START;

        loop {
            if *shutdown.borrow() {
                break;
            }

            set_state(&status, &name, State::Starting);
            info!(
                "[{name}] starting {} on port {}",
                binary.display(),
                self.transmitter.port
            );

            let mut command = Command::new(&binary);
            command
                .env("KYBER_CONFIG_PATH", &self.config_path)
                .current_dir(&self.install_dir);

            // Send kycontroller's stdout/stderr to a per-instance log file rather
            // than sharing the server's console. This keeps the consoles separate
            // and gives the child clean, owned stdio handles.
            if let Some(dir) = self.config_path.parent() {
                let log_path = dir.join("kycontroller.log");
                match std::fs::File::create(&log_path) {
                    Ok(file) => match file.try_clone() {
                        Ok(err_file) => {
                            command
                                .stdout(std::process::Stdio::from(file))
                                .stderr(std::process::Stdio::from(err_file));
                        }
                        Err(err) => warn!("[{name}] could not clone log handle: {err}"),
                    },
                    Err(err) => warn!("[{name}] could not create {log_path:?}: {err}"),
                }
            }

            let started = Instant::now();
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(err) => {
                    error!("[{name}] failed to spawn {}: {err}", binary.display());
                    set_state(&status, &name, State::Restarting);
                    if wait_or_shutdown(backoff, &mut shutdown).await {
                        break;
                    }
                    backoff = (backoff * 2).min(BACKOFF_MAX);
                    continue;
                }
            };

            set_state(&status, &name, State::Running);

            tokio::select! {
                wait = child.wait() => {
                    let uptime = started.elapsed();
                    match wait {
                        Ok(code) => warn!("[{name}] exited with {code} after {uptime:.1?}"),
                        Err(err) => warn!("[{name}] wait failed: {err} after {uptime:.1?}"),
                    }
                    if uptime >= HEALTHY_UPTIME {
                        backoff = BACKOFF_START;
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("[{name}] shutdown requested, stopping process");
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        break;
                    }
                }
            }

            set_state(&status, &name, State::Restarting);
            info!("[{name}] restarting in {backoff:.1?}");
            if wait_or_shutdown(backoff, &mut shutdown).await {
                break;
            }
            backoff = (backoff * 2).min(BACKOFF_MAX);
        }

        set_state(&status, &name, State::Stopped);
        info!("[{name}] supervisor stopped");
    }
}

/// One running instance's control handle.
struct Running {
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

/// Owns the running instances and mediates start/stop requests.
pub struct Manager {
    install_dir: PathBuf,
    defaults: toml::Table,
    status: StatusMap,
    running: HashMap<String, Running>,
}

impl Manager {
    pub fn new(install_dir: PathBuf, defaults: toml::Table) -> Self {
        Self {
            install_dir,
            defaults,
            status: Arc::new(Mutex::new(HashMap::new())),
            running: HashMap::new(),
        }
    }

    /// A clonable handle to the live status map (for the UI).
    pub fn status(&self) -> StatusMap {
        self.status.clone()
    }

    /// Prepare and start supervising `transmitter`. No-op if already running.
    pub fn start(&mut self, transmitter: Transmitter) -> Result<()> {
        if self.running.contains_key(&transmitter.name) {
            warn!("[{}] already running, ignoring start", transmitter.name);
            return Ok(());
        }

        let name = transmitter.name.clone();
        let instance =
            Instance::prepare(transmitter, &self.defaults, self.install_dir.clone())
                .with_context(|| format!("preparing transmitter {name:?}"))?;

        let (shutdown, shutdown_rx) = watch::channel(false);
        set_state(&self.status, &name, State::Starting);
        let task = tokio::spawn(instance.supervise(shutdown_rx, self.status.clone()));

        self.running.insert(name, Running { shutdown, task });
        Ok(())
    }

    /// Stop and forget the named instance, waiting for the process to die.
    pub async fn stop(&mut self, name: &str) {
        if let Some(handle) = self.running.remove(name) {
            let _ = handle.shutdown.send(true);
            let _ = handle.task.await;
        }
        if let Ok(mut map) = self.status.lock() {
            map.remove(name);
        }
    }

    /// Stop every instance and wait for them all to exit.
    pub async fn shutdown_all(&mut self) {
        let handles: Vec<_> = self.running.drain().map(|(_, h)| h).collect();
        for handle in &handles {
            let _ = handle.shutdown.send(true);
        }
        for handle in handles {
            let _ = handle.task.await;
        }
    }
}

/// Sleep for `delay`, returning early if shutdown is signaled. Returns `true`
/// when shutdown was requested.
async fn wait_or_shutdown(delay: Duration, shutdown: &mut watch::Receiver<bool>) -> bool {
    if *shutdown.borrow() {
        return true;
    }
    tokio::select! {
        _ = sleep(delay) => false,
        _ = shutdown.changed() => *shutdown.borrow(),
    }
}
