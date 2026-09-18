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
//! No child may outlive KyberFrog. On **Windows** every child — kycontroller
//! *and* kyclient — is assigned to a single **Job Object** created with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so when KyberFrog exits for any reason
//! (Ctrl-C, Task Manager kill, crash) Windows terminates the whole set.
//!
//! **Linux** has no equivalent primitive, so the guarantee is rebuilt from two
//! halves: the packaged systemd *user* service runs under a cgroup with
//! `KillMode=control-group`, which reaps the tree when the service stops, and
//! each child additionally gets `PR_SET_PDEATHSIG` (see [`supervise`]) so a dev
//! run — or a SIGKILLed KyberFrog, where no cleanup code gets to run — still
//! takes its children down.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{error, info, warn};
use shared::config::{kycontroller_path, Globals};
use shared::{encoder, gen, paths, EncoderChoice, GpuAdapter, ScreenBackend, Transmitter, Viewer};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{watch, Notify};
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

/// Names of the transmitters whose hardware encoder failed and that now run on
/// x264. Kept until the encoder setting changes or KyberFrog restarts, so a
/// restart of the transmitter does not replay the failure.
pub type FallbackSet = Arc<Mutex<HashSet<String>>>;

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
    /// Set for a transmitter on a hardware encoder: what to do if that encoder
    /// fails (see [`encoder::is_hardware_encoder_failure`]).
    encoder_fallback: Option<EncoderFallback>,
}

/// The x264 config to swap in when a transmitter's hardware encoder fails.
struct EncoderFallback {
    name: String,
    encoder: &'static str,
    config_path: PathBuf,
    x264_config: String,
    fallbacks: FallbackSet,
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
    /// Machine capture backend, written into every generated config on Linux.
    screen_backend: Option<ScreenBackend>,
    /// Machine encoder setting and the primary GPU it resolves against.
    encoder: EncoderChoice,
    gpu: Option<GpuAdapter>,
    globals: Globals,
    status: StatusMap,
    fallbacks: FallbackSet,
    running: HashMap<Key, Running>,
    /// Shared kill-on-close job; each supervise task holds a clone so the
    /// handle stays alive as long as any child is running.
    #[cfg(windows)]
    job: Arc<Option<JobGuard>>,
}

impl Manager {
    pub fn new(
        install_dir: PathBuf,
        defaults: toml::Table,
        screen_backend: Option<ScreenBackend>,
        encoder: EncoderChoice,
        gpu: Option<GpuAdapter>,
        globals: Globals,
    ) -> Self {
        #[cfg(windows)]
        let job = Arc::new(create_kill_on_close_job());

        Self {
            install_dir,
            defaults,
            screen_backend,
            encoder,
            gpu,
            globals,
            status: Arc::new(Mutex::new(HashMap::new())),
            fallbacks: Arc::new(Mutex::new(HashSet::new())),
            running: HashMap::new(),
            #[cfg(windows)]
            job,
        }
    }

    /// A clonable handle to the live status map (for the UI).
    pub fn status(&self) -> StatusMap {
        self.status.clone()
    }

    /// A clonable handle to the transmitters running on the x264 fallback.
    pub fn encoder_fallbacks(&self) -> FallbackSet {
        self.fallbacks.clone()
    }

    /// Swap the runtime parameters used for *future* spawns — the emission
    /// `defaults` (merged into each generated `kyber_config.toml`) and the
    /// reception `globals` (kyclient path + auth + flags). Called when a new
    /// setup is loaded. Children already running are untouched; the caller
    /// stops and restarts them with the new parameters (see `op_load_setup`).
    pub fn reload_runtime(
        &mut self,
        defaults: toml::Table,
        screen_backend: Option<ScreenBackend>,
        globals: Globals,
    ) {
        self.defaults = defaults;
        self.screen_backend = screen_backend;
        self.globals = globals;
    }

    /// Change the machine encoder setting for *future* transmitter spawns;
    /// running transmitters keep theirs until restarted. Past fallbacks are
    /// forgotten: the new setting gets its own chance.
    pub fn set_encoder(&mut self, encoder: EncoderChoice) {
        self.encoder = encoder;
        if let Ok(mut fallbacks) = self.fallbacks.lock() {
            fallbacks.clear();
        }
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
        preflight_ipc_dir()?;

        let dir = paths::instance_dir(&tx.name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating instance directory {dir:?}"))?;

        let config_path = paths::instance_config(&tx.name);
        let fell_back = self.fallbacks.lock().is_ok_and(|f| f.contains(&tx.name));
        let video_encoder = if fell_back {
            "x264"
        } else {
            encoder::resolve(self.encoder, self.gpu.as_ref())
        };
        let render = |encoder| {
            gen::render_config(tx, &self.defaults, self.screen_backend, encoder)
                .with_context(|| format!("rendering config for transmitter {:?}", tx.name))
        };
        std::fs::write(&config_path, render(video_encoder)?)
            .with_context(|| format!("writing instance config {config_path:?}"))?;

        let encoder_fallback = if video_encoder == "x264" {
            None
        } else {
            Some(EncoderFallback {
                name: tx.name.clone(),
                encoder: video_encoder,
                config_path: config_path.clone(),
                x264_config: render("x264")?,
                fallbacks: self.fallbacks.clone(),
            })
        };

        info!(
            "[{}] prepared (port {}, {}, encoder {video_encoder}) -> {config_path:?}",
            tx.name,
            tx.port,
            tx.source.label()
        );

        let mut env = vec![("KYBER_CONFIG_PATH".to_string(), config_path.into_os_string())];
        env.extend(child_env(&self.install_dir));

        // kycontroller keeps its *own* log4rs file appender, and off Windows its
        // path is the **relative** `log/kycontroller.log` — i.e. relative to the
        // child's working directory, which is the read-only install dir. It
        // panics on `Permission denied` before doing anything useful (seen as
        // root working, as a normal user not). `KYBER_LOG_DIR` is the fork's own
        // override, so point it inside the instance directory.
        //
        // A `log/` sub-directory rather than the instance directory itself: our
        // stdout/stderr capture already writes `<instance>/kycontroller.log`, and
        // the appender would target that very file — two writers, one truncating
        // the other. Windows is left alone; there the path is absolute and works.
        #[cfg(unix)]
        {
            let log_dir = dir.join("log");
            std::fs::create_dir_all(&log_dir)
                .with_context(|| format!("creating kycontroller log directory {log_dir:?}"))?;
            env.push(("KYBER_LOG_DIR".to_string(), log_dir.into_os_string()));
        }

        Ok(Spec {
            binary: kycontroller_path(&self.install_dir),
            args: Vec::new(),
            env,
            cwd: Some(self.install_dir.clone()),
            log_path: paths::kycontroller_log_file(&tx.name),
            encoder_fallback,
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
            env: child_env(&self.install_dir),
            cwd: None,
            log_path: paths::kyclient_log_file(&viewer.id),
            encoder_fallback: None,
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

/// Extra environment handed to every spawned child.
///
/// On Unix the fork binaries load their bundled `.so` from a sibling `lib/`
/// directory — the `.deb` layout is `<prefix>/bin` + `<prefix>/lib`. KyberFrog
/// spawns the binaries directly instead of going through the fork's
/// `run_*.sh` wrappers, so it has to replicate the `LD_LIBRARY_PATH` those
/// scripts set. An inherited `LD_LIBRARY_PATH` is preserved, appended after
/// ours so the bundle wins.
/// Fail early, and legibly, when the fork's IPC directory is not ours to use.
///
/// `libkypc` hardcodes `/tmp/kyber` as the base folder for its Unix sockets
/// (`kyutil/libkypc/src/transport/ipc/unix.rs`). It is a single shared path with
/// no per-user component, so whoever creates it first owns it: run KyberFrog
/// once as root and every later run as a normal user dies with
/// `IPC couldn't bind address /tmp/kyber/0: Permission denied` — a message that
/// says nothing about the cause or the cure. Two users on one machine collide
/// the same way.
///
/// We cannot fix the path from here (it is upstream's, and `kyutil` is not even
/// one of our forks), so we do the next best thing: detect it and say exactly
/// what to run.
#[cfg(unix)]
fn preflight_ipc_dir() -> Result<()> {
    use std::io::ErrorKind;

    let dir = Path::new("/tmp/kyber");
    if !dir.exists() {
        // kycontroller creates it on first use, owned by us. Nothing to check.
        return Ok(());
    }

    // Probe rather than inspect ownership: what matters is whether *we* can
    // create a socket in there, which sticky bits and ACLs also decide.
    let probe = dir.join(format!(".kyberfrog-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::PermissionDenied => Err(anyhow::anyhow!(
            "{} exists but belongs to another user, so kycontroller cannot create \
             its IPC socket there. It was most likely left behind by a run as root. \
             Remove it and start the transmitter again:\n    sudo rm -rf {}",
            dir.display(),
            dir.display()
        )),
        // Anything else (full disk, read-only /tmp…): let kycontroller report it
        // in its own words rather than guessing here.
        Err(_) => Ok(()),
    }
}

#[cfg(not(unix))]
fn preflight_ipc_dir() -> Result<()> {
    Ok(())
}

/// Mirrors the fork's own `run_kyclient.sh` / `run_kycontroller.sh`, which
/// export **two** variables — both are required:
///
/// * `PATH=$BASE_DIR/bin:$PATH` — kycontroller spawns `kyavserver` (and
///   `kynputserver`) by bare name. Windows also searches the child's working
///   directory, which is the install dir, so this went unnoticed there; Linux
///   does not, and the transmitter dies on
///   `Process kyavserver spawn failed: NotFound`.
/// * `LD_LIBRARY_PATH=$BASE_DIR/lib:$BASE_DIR/lib/<triplet>:$BASE_DIR/lib64`
///   (plus `lib/vlc` for kyclient). **The multiarch sub-directory is not
///   optional**: the bundle keeps `libtxproto`, `libkyclient`, `libkynput` and
///   the FFmpeg libraries under `lib/x86_64-linux-gnu/`.
///
/// Inherited values are preserved, appended after ours so the bundle wins.
#[cfg(unix)]
fn child_env(install_dir: &Path) -> Vec<(String, OsString)> {
    let mut env = Vec::new();

    let mut path_dirs = vec![install_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        path_dirs.extend(std::env::split_paths(&existing));
    }
    if let Ok(joined) = std::env::join_paths(path_dirs) {
        env.push(("PATH".to_string(), joined));
    }

    let mut lib_dirs = vec![install_dir.to_path_buf()];
    if let Some(prefix) = install_dir.parent() {
        // Debian multiarch triplet, e.g. `x86_64-linux-gnu` / `aarch64-linux-gnu`.
        let triplet = format!("{}-linux-gnu", std::env::consts::ARCH);
        let lib = prefix.join("lib");
        lib_dirs.push(lib.join(&triplet));
        lib_dirs.push(lib.join("vlc"));
        lib_dirs.push(prefix.join("lib64"));
        lib_dirs.push(lib);
    }
    if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
        lib_dirs.extend(std::env::split_paths(&existing));
    }
    if let Ok(joined) = std::env::join_paths(lib_dirs) {
        env.push(("LD_LIBRARY_PATH".to_string(), joined));
    }

    env
}

/// Nothing to add off Unix: on Windows the DLLs and the sibling binaries sit
/// next to each other and are found through the child's working directory and
/// the PATH entry the installer adds.
#[cfg(not(unix))]
fn child_env(_install_dir: &Path) -> Vec<(String, OsString)> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// Supervision loop (shared by both kinds)
// ---------------------------------------------------------------------------

/// Run and keep relaunching one child until `shutdown` flips to `true`.
async fn supervise(
    key: Key,
    mut spec: Spec,
    mut shutdown: watch::Receiver<bool>,
    status: StatusMap,
    #[cfg(windows)] job: Arc<Option<JobGuard>>,
) {
    let tag = key.tag().to_string();
    let mut backoff = BACKOFF_START;
    // Set after an encoder fallback: the next run appends to the log instead of
    // truncating it, so the error that caused the fallback stays readable.
    let mut keep_log = false;

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

        // kyberfrog is a windowless GUI app (#21): without this flag every
        // console child (kycontroller) would pop its own empty console window.
        // The invisible console it gets instead is inherited by the child's
        // own children (kyavserver); kyclient's video window is unaffected.
        #[cfg(windows)]
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);

        // Per-child log file, written by us from the child's piped stdout/stderr
        // (see `pipe_to_log`) so each line can be inspected on the way.
        if let Some(dir) = spec.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(keep_log)
            .truncate(!keep_log)
            .open(&spec.log_path);
        keep_log = false;
        let log_files = match log_file {
            Ok(file) => match file.try_clone() {
                Ok(err_file) => {
                    command
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());
                    Some((file, err_file))
                }
                Err(err) => {
                    warn!("[{tag}] could not clone log handle: {err}");
                    None
                }
            },
            Err(err) => {
                warn!("[{tag}] could not create {:?}: {err}", spec.log_path);
                None
            }
        };

        // Linux counterpart of the Windows Job Object's kill-on-close: ask the
        // kernel to SIGTERM this child when its parent dies. The packaged
        // service leans primarily on the systemd cgroup
        // (`KillMode=control-group`); this is the safety net for dev runs and
        // for a SIGKILLed KyberFrog, where no cleanup code of ours ever runs —
        // all of its threads die, so the parent-death signal fires.
        // PR_SET_PDEATHSIG is Linux-specific (not POSIX, absent on macOS).
        #[cfg(target_os = "linux")]
        // SAFETY: pre_exec runs in the forked child, between fork and exec. We
        // only call prctl(2), which is async-signal-safe, and allocate nothing.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
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

        // Copy the child's output into its log file, watching for a failing
        // hardware encoder when there is an x264 config to fall back to.
        let encoder_failed = Arc::new(Notify::new());
        let watch_encoder = spec.encoder_fallback.is_some().then(|| encoder_failed.clone());
        let mut log_tasks = Vec::new();
        if let Some((out_file, err_file)) = log_files {
            if let Some(stdout) = child.stdout.take() {
                log_tasks.push(pipe_to_log(stdout, out_file, watch_encoder.clone()));
            }
            if let Some(stderr) = child.stderr.take() {
                log_tasks.push(pipe_to_log(stderr, err_file, watch_encoder));
            }
        }

        // Only mark Running after STARTUP_GRACE. If the process exits before
        // that, it was never healthy and we go straight to Restarting without
        // ever showing Running in the UI.
        let grace = tokio::time::sleep(STARTUP_GRACE);
        tokio::pin!(grace);
        let mut grace_fired = false;

        let exit = 'watch: {
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
                        break 'watch Exit::Relaunch;
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
                            break 'watch Exit::Stop;
                        }
                    }
                    _ = encoder_failed.notified(), if spec.encoder_fallback.is_some() => {
                        // Taken: one fallback per launch spec, x264 cannot fail over.
                        if let Some(fallback) = spec.encoder_fallback.take() {
                            fallback.apply(&tag);
                        }
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        backoff = BACKOFF_START;
                        break 'watch Exit::RelaunchNow;
                    }
                }
            }
        };

        // The pipes close with the child (and its kyavserver, which dies in
        // kycontroller's own job); don't hang on a straggler holding them.
        for task in log_tasks {
            let abort = task.abort_handle();
            if tokio::time::timeout(Duration::from_secs(1), task).await.is_err() {
                abort.abort();
            }
        }

        match exit {
            Exit::Stop => break,
            Exit::RelaunchNow => {
                keep_log = true;
                continue;
            }
            Exit::Relaunch => {}
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

/// How one run of a supervised child ended.
enum Exit {
    /// Stop requested: leave the loop.
    Stop,
    /// The child died on its own: relaunch after the backoff.
    Relaunch,
    /// Killed by us to apply the encoder fallback: relaunch at once.
    RelaunchNow,
}

impl EncoderFallback {
    /// Swap the x264 config in and remember the fallback for later starts.
    fn apply(self, tag: &str) {
        warn!(
            "[{tag}] hardware encoder {} failed (see the transmitter log), \
             falling back to x264 and restarting",
            self.encoder
        );
        if let Err(err) = std::fs::write(&self.config_path, &self.x264_config) {
            error!("[{tag}] could not write the x264 config {:?}: {err}", self.config_path);
            return;
        }
        if let Ok(mut fallbacks) = self.fallbacks.lock() {
            fallbacks.insert(self.name);
        }
    }
}

/// Copy a child's output stream line by line into its log file. When `encoder`
/// is set, a line reporting a failing hardware encoder notifies it.
///
/// Our own copy rather than handing the file to the child: that is what lets
/// the supervisor see the fork's encoder errors, which never make the child
/// exit. A few lines per second — no measurable cost.
fn pipe_to_log(
    stream: impl AsyncRead + Unpin + Send + 'static,
    mut file: std::fs::File,
    encoder: Option<Arc<Notify>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stream);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let _ = file.write_all(&line);
            if let Some(encoder) = &encoder {
                if encoder::is_hardware_encoder_failure(&String::from_utf8_lossy(&line)) {
                    encoder.notify_one();
                }
            }
        }
    })
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
