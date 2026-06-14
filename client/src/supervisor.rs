// SPDX-License-Identifier: AGPL-3.0-or-later

//! Per-instance `kyclient` supervision and the runtime [`Manager`].
//!
//! Each [`Instance`] runs in its own task that spawns `kyclient`, restarts it
//! with capped exponential backoff if it exits, and stops on request. The
//! child's stdout/stderr go to a per-instance log file (clean owned handles,
//! and the web UI can tail it). The [`Manager`] owns the running tasks and lets
//! the web layer start, stop, restart and inspect instances at runtime.
//!
//! All child processes are assigned to a Windows Job Object created with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When kyberfrog-client exits for any
//! reason (Ctrl-C, Task Manager kill, crash), the Job Object handle is released
//! and Windows automatically terminates every kyclient process.

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

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

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

    /// Shape-distinct monochrome glyph for Win32 tray menus (no color emoji).
    pub fn symbol(self) -> &'static str {
        match self {
            State::Starting => "○",
            State::Running => "●",
            State::Restarting => "◐",
            State::Stopped => "✗",
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

// ---------------------------------------------------------------------------
// Windows Job Object — kill-on-close guard
// ---------------------------------------------------------------------------

/// Wraps a Windows Job Object handle.  All kyclient child processes are
/// assigned to this job; when the handle is dropped (parent process exits for
/// any reason) Windows terminates them automatically via
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
#[cfg(windows)]
struct JobGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

// SAFETY: HANDLE is an opaque kernel object reference; we never alias it and
// access is serialised through the Arc.
#[cfg(windows)]
unsafe impl Send for JobGuard {}
#[cfg(windows)]
unsafe impl Sync for JobGuard {}

/// Create a Job Object configured to kill all assigned processes on close.
/// Returns `None` on failure — supervision still works, just without the
/// kill-on-close guarantee.
#[cfg(windows)]
fn create_kill_on_close_job() -> Option<JobGuard> {
    use windows_sys::Win32::System::JobObjects::{
        CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == std::ptr::null_mut() {
            warn!("CreateJobObjectW failed — orphan protection unavailable");
            return None;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            warn!("SetInformationJobObject failed — orphan protection unavailable");
            windows_sys::Win32::Foundation::CloseHandle(job);
            return None;
        }

        info!("Job Object created — kyclient processes will die with this process");
        Some(JobGuard(job))
    }
}

/// Assign a process by PID to the job object.
#[cfg(windows)]
fn assign_to_job(job: &JobGuard, pid: u32) {
    use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

    unsafe {
        let handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if handle == std::ptr::null_mut() {
            warn!("[pid {pid}] OpenProcess failed — this kyclient is not in the job");
            return;
        }
        if AssignProcessToJobObject(job.0, handle) == 0 {
            warn!("[pid {pid}] AssignProcessToJobObject failed — this kyclient is not in the job");
        }
        windows_sys::Win32::Foundation::CloseHandle(handle);
    }
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

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
    /// Shared reference to the kill-on-close job; each supervise task holds a
    /// clone so the handle stays alive as long as any child is running.
    #[cfg(windows)]
    job: Arc<Option<JobGuard>>,
}

impl Manager {
    pub fn new(globals: Globals) -> Self {
        #[cfg(windows)]
        let job = Arc::new(create_kill_on_close_job());

        Self {
            globals,
            status: Arc::new(Mutex::new(HashMap::new())),
            running: HashMap::new(),
            #[cfg(windows)]
            job,
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

        #[cfg(windows)]
        let job = self.job.clone();

        let (shutdown, shutdown_rx) = watch::channel(false);
        set_state(&self.status, &id, State::Starting);
        let task = tokio::spawn(supervise(
            id.clone(),
            binary,
            args,
            shutdown_rx,
            self.status.clone(),
            #[cfg(windows)]
            job,
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

// ---------------------------------------------------------------------------
// Supervision loop
// ---------------------------------------------------------------------------

/// Run and keep relaunching one `kyclient` until `shutdown` flips to `true`.
async fn supervise(
    id: String,
    binary: PathBuf,
    args: Vec<String>,
    mut shutdown: watch::Receiver<bool>,
    status: StatusMap,
    #[cfg(windows)] job: Arc<Option<JobGuard>>,
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

        // Assign the new child to the kill-on-close job object so it dies
        // with the parent process regardless of how we exit.
        #[cfg(windows)]
        if let Some(ref guard) = *job {
            if let Some(pid) = child.id() {
                assign_to_job(guard, pid);
            }
        }

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
