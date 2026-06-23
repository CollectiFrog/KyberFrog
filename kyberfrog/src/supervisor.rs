// SPDX-License-Identifier: AGPL-3.0-or-later

//! The single runtime supervisor that manages **both** kinds of child process:
//!
//! * **transmitters** → one `kycontroller` each, fed a generated
//!   `kyber_config.toml` via `KYBER_CONFIG_PATH`;
//! * **viewers** → one `kyclient` each, connected to a remote transmitter.
//!
//! Both kinds share one supervise loop: spawn the child, restart it with capped
//! exponential backoff if it exits, stop it on a `watch` shutdown signal. Their
//! lifecycle state lands in one [`StatusMap`] keyed by a typed [`Key`] so a
//! transmitter named `x` and a viewer with id `x` never collide.
//!
//! Every child — kycontroller *and* kyclient — is assigned to a single Windows
//! **Job Object** created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When
//! KyberFrog exits for any reason (Ctrl-C, Task Manager kill, crash), Windows
//! terminates every child it spawned.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{error, info, warn};
use shared::config::{kycontroller_path, Globals};
use shared::{gen, paths, Transmitter, Viewer};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const BACKOFF_START: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// A process up at least this long is healthy, so its backoff resets.
const HEALTHY_UPTIME: Duration = Duration::from_secs(30);
/// Grace window after spawn: only transition to Running if the process is still
/// alive after this delay. A crash before this threshold stays in Starting so
/// the UI never flickers through Running on a bad-port restart loop.
const STARTUP_GRACE: Duration = Duration::from_secs(3);

// ---------------------------------------------------------------------------
// State + typed key
// ---------------------------------------------------------------------------

/// Coarse lifecycle state of a supervised child, surfaced to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Starting,
    Running,
    Restarting,
    Stopped,
}

impl State {
    /// Machine-readable status for the web UI / discovery endpoint.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Starting => "starting",
            State::Running => "running",
            State::Restarting => "restarting",
            State::Stopped => "stopped",
        }
    }

    /// Status glyph for Win32 tray menus. GDI menus have no color-emoji
    /// support, so states are distinguished by shape: ○ starting, ● running,
    /// ◐ restarting, ✗ stopped.
    pub fn symbol(self) -> &'static str {
        match self {
            State::Starting => "○",
            State::Running => "●",
            State::Restarting => "◐",
            State::Stopped => "✗",
        }
    }
}

/// Identifies one supervised child. Transmitters are keyed by name, viewers by
/// id; the variant keeps the two namespaces distinct in the shared status map.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Tx(String),
    Vw(String),
}

impl Key {
    /// Short tag used as the log-line prefix.
    fn tag(&self) -> &str {
        match self {
            Key::Tx(name) => name,
            Key::Vw(id) => id,
        }
    }
}

/// Shared map of [`Key`] -> current [`State`].
pub type StatusMap = Arc<Mutex<HashMap<Key, State>>>;

fn set_state(status: &StatusMap, key: &Key, state: State) {
    if let Ok(mut map) = status.lock() {
        map.insert(key.clone(), state);
    }
}

/// Look up a child's state in a status snapshot, defaulting to `Stopped`.
pub fn state_of(map: &HashMap<Key, State>, key: &Key) -> State {
    map.get(key).copied().unwrap_or(State::Stopped)
}

// ---------------------------------------------------------------------------
// Job Object — kill-on-close guard (applies to every child)
// ---------------------------------------------------------------------------

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
        if job.is_null() {
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

        info!("Job Object created — every child dies with this process");
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
        if handle.is_null() {
            warn!("[pid {pid}] OpenProcess failed — child is not in the job");
            return;
        }
        if AssignProcessToJobObject(job.0, handle) == 0 {
            warn!("[pid {pid}] AssignProcessToJobObject failed — child is not in the job");
        }
        windows_sys::Win32::Foundation::CloseHandle(handle);
    }
}

// ---------------------------------------------------------------------------
// Spawn spec — the per-kind differences, resolved up front
// ---------------------------------------------------------------------------

/// Everything the generic supervise loop needs to run one child.
struct Spec {
    binary: PathBuf,
    args: Vec<String>,
    env: Vec<(String, OsString)>,
    cwd: Option<PathBuf>,
    log_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

/// One running child's control handle.
struct Running {
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

/// Owns every running child and mediates start/stop requests for both roles.
pub struct Manager {
    install_dir: PathBuf,
    defaults: toml::Table,
    globals: Globals,
    status: StatusMap,
    running: HashMap<Key, Running>,
    /// Shared kill-on-close job; each supervise task holds a clone so the
    /// handle stays alive as long as any child is running.
    #[cfg(windows)]
    job: Arc<Option<JobGuard>>,
}

impl Manager {
    pub fn new(install_dir: PathBuf, defaults: toml::Table, globals: Globals) -> Self {
        #[cfg(windows)]
        let job = Arc::new(create_kill_on_close_job());

        Self {
            install_dir,
            defaults,
            globals,
            status: Arc::new(Mutex::new(HashMap::new())),
            running: HashMap::new(),
            #[cfg(windows)]
            job,
        }
    }

    /// A clonable handle to the live status map (for the UI).
    pub fn status(&self) -> StatusMap {
        self.status.clone()
    }

    /// Swap the runtime parameters used for *future* spawns — the emission
    /// `defaults` (merged into each generated `kyber_config.toml`) and the
    /// reception `globals` (kyclient path + auth + flags). Called when a new
    /// setup is loaded. Children already running are untouched; the caller
    /// stops and restarts them with the new parameters (see `op_load_setup`).
    pub fn reload_runtime(&mut self, defaults: toml::Table, globals: Globals) {
        self.defaults = defaults;
        self.globals = globals;
    }

    // -- Transmitters -------------------------------------------------------

    /// Prepare and start supervising `tx`. No-op if already running.
    pub fn start_transmitter(&mut self, tx: &Transmitter) -> Result<()> {
        let key = Key::Tx(tx.name.clone());
        if self.running.contains_key(&key) {
            warn!("[{}] transmitter already running, ignoring start", tx.name);
            return Ok(());
        }
        let spec = self
            .prepare_transmitter(tx)
            .with_context(|| format!("preparing transmitter {:?}", tx.name))?;
        self.spawn(key, spec);
        Ok(())
    }

    /// Stop and forget the named transmitter, waiting for the process to die.
    pub async fn stop_transmitter(&mut self, name: &str) {
        self.stop(&Key::Tx(name.to_string())).await;
    }

    /// Stop then start `tx` (apply edited fields / fresh config).
    pub async fn restart_transmitter(&mut self, tx: &Transmitter) -> Result<()> {
        self.stop(&Key::Tx(tx.name.clone())).await;
        self.start_transmitter(tx)
    }

    /// Generate the instance config and resolve the spawn spec for `tx`.
    fn prepare_transmitter(&self, tx: &Transmitter) -> Result<Spec> {
        let dir = paths::instance_dir(&tx.name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating instance directory {dir:?}"))?;

        let config_path = paths::instance_config(&tx.name);
        let content = gen::render_config(tx, &self.defaults)
            .with_context(|| format!("rendering config for transmitter {:?}", tx.name))?;
        std::fs::write(&config_path, content)
            .with_context(|| format!("writing instance config {config_path:?}"))?;

        info!(
            "[{}] prepared (port {}, {}) -> {config_path:?}",
            tx.name,
            tx.port,
            tx.source.label()
        );

        Ok(Spec {
            binary: kycontroller_path(&self.install_dir),
            args: Vec::new(),
            env: vec![("KYBER_CONFIG_PATH".to_string(), config_path.into_os_string())],
            cwd: Some(self.install_dir.clone()),
            log_path: paths::kycontroller_log_file(&tx.name),
        })
    }

    // -- Viewers ------------------------------------------------------------

    /// Start supervising `viewer`. No-op if already running.
    pub fn start_viewer(&mut self, viewer: &Viewer) {
        let key = Key::Vw(viewer.id.clone());
        if self.running.contains_key(&key) {
            warn!("[{}] viewer already running, ignoring start", viewer.id);
            return;
        }
        let spec = Spec {
            binary: self.globals.kyclient_path.clone(),
            args: self.globals.kyclient_args(viewer),
            env: Vec::new(),
            cwd: None,
            log_path: paths::kyclient_log_file(&viewer.id),
        };
        self.spawn(key, spec);
    }

    /// Stop and forget the named viewer, waiting for kyclient to die.
    pub async fn stop_viewer(&mut self, id: &str) {
        self.stop(&Key::Vw(id.to_string())).await;
    }

    /// Stop then start `viewer` (apply edited fields).
    pub async fn restart_viewer(&mut self, viewer: &Viewer) {
        self.stop(&Key::Vw(viewer.id.clone())).await;
        self.start_viewer(viewer);
    }

    // -- Shared plumbing ----------------------------------------------------

    /// Spawn the supervise task for `key`/`spec` and record its handle.
    fn spawn(&mut self, key: Key, spec: Spec) {
        let (shutdown, shutdown_rx) = watch::channel(false);
        set_state(&self.status, &key, State::Starting);

        #[cfg(windows)]
        let job = self.job.clone();

        let task = tokio::spawn(supervise(
            key.clone(),
            spec,
            shutdown_rx,
            self.status.clone(),
            #[cfg(windows)]
            job,
        ));

        self.running.insert(key, Running { shutdown, task });
    }

    /// Stop and forget one child, waiting for the process to exit.
    async fn stop(&mut self, key: &Key) {
        if let Some(handle) = self.running.remove(key) {
            let _ = handle.shutdown.send(true);
            let _ = handle.task.await;
        }
        if let Ok(mut map) = self.status.lock() {
            map.remove(key);
        }
    }

    /// Stop every child and wait for them all to exit.
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
// Supervision loop (shared by both kinds)
// ---------------------------------------------------------------------------

/// Run and keep relaunching one child until `shutdown` flips to `true`.
async fn supervise(
    key: Key,
    spec: Spec,
    mut shutdown: watch::Receiver<bool>,
    status: StatusMap,
    #[cfg(windows)] job: Arc<Option<JobGuard>>,
) {
    let tag = key.tag().to_string();
    let mut backoff = BACKOFF_START;

    loop {
        if *shutdown.borrow() {
            break;
        }

        set_state(&status, &key, State::Starting);
        info!("[{tag}] launching {} {}", spec.binary.display(), redacted(&spec.args));

        let mut command = Command::new(&spec.binary);
        command.args(&spec.args);
        for (name, value) in &spec.env {
            command.env(name, value);
        }
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }

        // Per-child log file: clean owned stdio for the child + tailable logs.
        if let Some(dir) = spec.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::File::create(&spec.log_path) {
            Ok(file) => match file.try_clone() {
                Ok(err_file) => {
                    command
                        .stdout(std::process::Stdio::from(file))
                        .stderr(std::process::Stdio::from(err_file));
                }
                Err(err) => warn!("[{tag}] could not clone log handle: {err}"),
            },
            Err(err) => warn!("[{tag}] could not create {:?}: {err}", spec.log_path),
        }

        let started = Instant::now();
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                error!("[{tag}] failed to spawn {}: {err}", spec.binary.display());
                set_state(&status, &key, State::Restarting);
                if wait_or_shutdown(backoff, &mut shutdown).await {
                    break;
                }
                backoff = (backoff * 2).min(BACKOFF_MAX);
                continue;
            }
        };

        // Assign the new child to the kill-on-close job so it dies with us.
        #[cfg(windows)]
        if let Some(ref guard) = *job {
            if let Some(pid) = child.id() {
                assign_to_job(guard, pid);
            }
        }

        // Only mark Running after STARTUP_GRACE. If the process exits before
        // that, it was never healthy and we go straight to Restarting without
        // ever showing Running in the UI.
        let grace = tokio::time::sleep(STARTUP_GRACE);
        tokio::pin!(grace);
        let mut grace_fired = false;

        let relaunch = 'watch: {
            loop {
                tokio::select! {
                    wait = child.wait() => {
                        let uptime = started.elapsed();
                        match wait {
                            Ok(code) => warn!("[{tag}] exited with {code} after {uptime:.1?}"),
                            Err(err) => warn!("[{tag}] wait failed: {err} after {uptime:.1?}"),
                        }
                        if uptime >= HEALTHY_UPTIME {
                            backoff = BACKOFF_START;
                        }
                        break 'watch true;
                    }
                    _ = &mut grace, if !grace_fired => {
                        grace_fired = true;
                        set_state(&status, &key, State::Running);
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            info!("[{tag}] stop requested, killing process");
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            break 'watch false;
                        }
                    }
                }
            }
        };

        if !relaunch {
            break;
        }
        set_state(&status, &key, State::Restarting);
        info!("[{tag}] relaunching in {backoff:.1?}");
        if wait_or_shutdown(backoff, &mut shutdown).await {
            break;
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }

    set_state(&status, &key, State::Stopped);
    info!("[{tag}] supervisor stopped");
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

/// Command line for logs, masking the auth password value (viewer args only).
fn redacted(args: &[String]) -> String {
    let mut out = Vec::with_capacity(args.len());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        out.push(arg.clone());
        if arg == "--auth-password" && it.next().is_some() {
            out.push("***".to_string());
        }
    }
    out.join(" ")
}
