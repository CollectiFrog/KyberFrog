// SPDX-License-Identifier: AGPL-3.0-or-later

//! Per-instance `kyclient` supervision and the runtime [`Manager`].
//!
//! Each [`Instance`] runs in its own task that spawns `kyclient`, restarts it
//! with capped exponential backoff if it exits, and stops on request. The
//! child's stdout/stderr go to a per-instance log file (clean owned handles,
//! and the web UI can tail it). The [`Manager`] owns the running tasks and lets
//! the web layer start, stop, restart and inspect instances at runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{error, info, warn};
use shared::paths;
use tokio::process::Command;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::config::{Globals, Instance};

const BACKOFF_START: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// A viewer up at least this long is healthy, so its backoff resets.
const HEALTHY_UPTIME: Duration = Duration::from_secs(30);

/// Coarse lifecycle state of a supervised instance, surfaced to the web UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Starting,
    Running,
    Restarting,
    Stopped,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Starting => "starting",
            State::Running => "running",
            State::Restarting => "restarting",
            State::Stopped => "stopped",
        }
    }
}

/// Shared map of instance id -> current [`State`].
pub type StatusMap = Arc<Mutex<HashMap<String, State>>>;

fn set_state(status: &StatusMap, id: &str, state: State) {
    if let Ok(mut map) = status.lock() {
        map.insert(id.to_string(), state);
    }
}

/// One running instance's control handle.
struct Running {
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

/// Owns the running viewers and mediates start/stop requests.
pub struct Manager {
    globals: Globals,
    status: StatusMap,
    running: HashMap<String, Running>,
}

impl Manager {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            status: Arc::new(Mutex::new(HashMap::new())),
            running: HashMap::new(),
        }
    }

    /// A clonable handle to the live status map (for the web UI).
    pub fn status(&self) -> StatusMap {
        self.status.clone()
    }

    /// Start supervising `instance`. No-op if already running.
    pub fn start(&mut self, instance: &Instance) {
        if self.running.contains_key(&instance.id) {
            warn!("[{}] already running, ignoring start", instance.id);
            return;
        }

        let id = instance.id.clone();
        let binary = self.globals.kyclient_path.clone();
        let args = self.globals.kyclient_args(instance);

        let (shutdown, shutdown_rx) = watch::channel(false);
        set_state(&self.status, &id, State::Starting);
        let task = tokio::spawn(supervise(
            id.clone(),
            binary,
            args,
            shutdown_rx,
            self.status.clone(),
        ));

        self.running.insert(id, Running { shutdown, task });
    }

    /// Stop and forget the named instance, waiting for kyclient to die.
    pub async fn stop(&mut self, id: &str) {
        if let Some(handle) = self.running.remove(id) {
            let _ = handle.shutdown.send(true);
            let _ = handle.task.await;
        }
        if let Ok(mut map) = self.status.lock() {
            map.remove(id);
        }
    }

    /// Stop then start `instance` (apply edited fields).
    pub async fn restart(&mut self, instance: &Instance) {
        self.stop(&instance.id).await;
        self.start(instance);
    }

    /// Stop every viewer and wait for them all to exit.
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

/// Run and keep relaunching one `kyclient` until `shutdown` flips to `true`.
async fn supervise(
    id: String,
    binary: PathBuf,
    args: Vec<String>,
    mut shutdown: watch::Receiver<bool>,
    status: StatusMap,
) {
    let mut backoff = BACKOFF_START;

    loop {
        if *shutdown.borrow() {
            break;
        }

        set_state(&status, &id, State::Starting);
        info!("[{id}] launching {} {}", binary.display(), redacted(&args));

        let mut command = Command::new(&binary);
        command.args(&args);

        // Per-instance log file: clean owned stdio for the child + tailable logs.
        let log_path = paths::kyclient_log_file(&id);
        if let Some(dir) = log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::File::create(&log_path) {
            Ok(file) => match file.try_clone() {
                Ok(err_file) => {
                    command
                        .stdout(std::process::Stdio::from(file))
                        .stderr(std::process::Stdio::from(err_file));
                }
                Err(err) => warn!("[{id}] could not clone log handle: {err}"),
            },
            Err(err) => warn!("[{id}] could not create {log_path:?}: {err}"),
        }

        let started = Instant::now();
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                error!("[{id}] failed to spawn {}: {err}", binary.display());
                set_state(&status, &id, State::Restarting);
                if wait_or_shutdown(backoff, &mut shutdown).await {
                    break;
                }
                backoff = (backoff * 2).min(BACKOFF_MAX);
                continue;
            }
        };

        set_state(&status, &id, State::Running);

        tokio::select! {
            wait = child.wait() => {
                let uptime = started.elapsed();
                match wait {
                    Ok(code) => warn!("[{id}] kyclient exited with {code} after {uptime:.1?}"),
                    Err(err) => warn!("[{id}] wait failed: {err} after {uptime:.1?}"),
                }
                if uptime >= HEALTHY_UPTIME {
                    backoff = BACKOFF_START;
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("[{id}] stop requested, killing kyclient");
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    break;
                }
            }
        }

        set_state(&status, &id, State::Restarting);
        info!("[{id}] relaunching in {backoff:.1?}");
        if wait_or_shutdown(backoff, &mut shutdown).await {
            break;
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }

    set_state(&status, &id, State::Stopped);
    info!("[{id}] supervisor stopped");
}

/// Sleep for `delay`, returning `true` if shutdown is signaled first.
async fn wait_or_shutdown(delay: Duration, shutdown: &mut watch::Receiver<bool>) -> bool {
    if *shutdown.borrow() {
        return true;
    }
    tokio::select! {
        _ = sleep(delay) => false,
        _ = shutdown.changed() => *shutdown.borrow(),
    }
}

/// Command line for logs, masking the auth password.
fn redacted(args: &[String]) -> String {
    let mut out = Vec::with_capacity(args.len());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        out.push(arg.clone());
        if arg == "--auth-password" {
            if it.next().is_some() {
                out.push("***".to_string());
            }
        }
    }
    out.join(" ")
}
