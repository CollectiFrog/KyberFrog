# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

KyberFrog is an orchestration layer on top of the **kyber-frog fork of Kyber**
(QUIC video transport, a drop-in NDI replacement for LAN). It is **one app,
installed on every machine**; the role (emit and/or receive) is set entirely by
the config and the UI, not by which binary you run:

- **Emission** (regie/host): for each `[[emission.transmitter]]` it generates a
  `kyber_config.toml` and supervises one `kycontroller` process (isolated by
  port + `KYBER_CONFIG_PATH`).
- **Reception** (display): for each `[[reception.viewer]]` it supervises one
  fullscreen `kyclient`.

A pure receiver simply has no transmitters; a pure emitter no viewers. Both
halves are driven by **one supervisor**, **one web UI** (default port 7700) and
**one tray**, and persisted to a single `%APPDATA%\kyberfrog\kyberfrog.toml`.

KyberFrog does **not** reimplement Kyber — it spawns the fork's binaries
(`kycontroller`, `kyavserver`, `kyclient`), which must be on `PATH`. Two upstream
changes the fork already carries are load-bearing here: the `KYBER_CONFIG_PATH`
env override (lets N instances share one install) and `spout_sender` pinning in
kyavserver.

## Build & test

There is **no native Rust toolchain on the dev host** — everything cross-compiles
to Windows through the same MinGW Docker image used by the rest of Kyber. Run all
cargo commands inside it, from the workspace root:

```sh
# Build the single exe → target/x86_64-pc-windows-gnu/release/kyberfrog.exe
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu

# Run the whole test suite (unit tests live in shared/: config.rs, gen.rs, lib.rs)
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test

# Run a single test by name
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo test -p kyberfrog-shared config_round_trips_both_halves
```

The `x86_64-pc-windows-gnu` target matters: the Win32 code (tray, Job Object,
spout enumeration, icon loading) only compiles for Windows, and MinGW defines
`HANDLE` as `*mut c_void` (not `isize`) — null-check raw handles with
`is_null()` / `std::ptr::null_mut()`, never `== 0`. A plain `cargo check` (Linux
host target) is the fast inner loop: it compiles everything except the Win32
modules (which fall back to no-op stubs), so it catches all the non-Win32 logic.

## Project layout

```
Cargo.toml                       workspace (members: shared, kyberfrog; shared deps + version)
README.md                        user-facing: prerequisites (install kyber fork + PATH), install, build, run
IMPROVEMENTS.md                  deferred work / tech debt, numbered
examples/kyberfrog.toml          reference unified config (the auth schema here is the *correct* one)

shared/                          kyberfrog-shared — model + config gen + paths (no Windows code, testable on Linux)
  src/lib.rs                       Transmitter / Source, DEFAULT_* consts, re-exports config types
  src/config.rs                    Config (emission + reception), Viewer, Globals, load/save,
                                   kyclient_args(), kycontroller_path(), unit tests
  src/gen.rs                       render_config(): layer [emission.defaults] + per-transmitter values → kyber_config.toml
  src/paths.rs                     every %APPDATA%\kyberfrog\ location (config, logs, per-instance dirs)

kyberfrog/                       kyberfrog — the single binary (both roles)
  build.rs                         embeds assets/kyberfrog.ico as Win resource (winresource → windres)
  assets/kyberfrog.ico             Collecti'Frog logo, embedded + override-next-to-exe
  install/install-kyberfrog.ps1    registers an AtLogOn scheduled task for hands-off autostart
  src/main.rs                      tokio entry, flexi_logger, builds Manager + AppState + tray + web, command loop
  src/supervisor.rs                Manager + one supervise loop for BOTH kinds (Key::Tx/Vw, StatusMap, State, Job Object)
  src/app.rs                       AppState + the op_* functions both UIs call; naming/port allocation; status payload
  src/spout.rs                     live Spout-sender enumeration for the "Add" picker (tray + web) (Win32)
  src/tray/{mod,windows,stub}.rs   system tray (mod re-exports windows|stub by cfg); muda menu, both sections
  src/web.rs + web/index.html      dashboard (Émission + Réception + logs) + JSON API + GET /transmitters discovery
```

## Architecture

### Crates
- **`shared` (`kyberfrog-shared`)** — the data model and the *only* place that
  knows Kyber's config schema. `config.rs` holds the unified `Config`
  (`Emission` + `Reception`), `Viewer`, `Globals`, load/save, and
  `kyclient_args()`; `lib.rs` holds `Transmitter`/`Source` and the `DEFAULT_*`
  consts; `gen.rs` renders per-instance `kyber_config.toml`; `paths.rs` is the
  single source of truth for every on-disk location.
- **`kyberfrog`** — the single binary. `supervisor.rs` (one `Manager`),
  `app.rs` (shared state + operations), `spout.rs`, `tray/`, `web.rs`.

### One config, two halves (`shared/src/config.rs`)
`kyberfrog.toml` deserializes into `Config { kyber_install_dir, web_port,
emission, reception }`. `[emission]` carries `base_port`, the free-form
`[emission.defaults]` TOML table, and `[[emission.transmitter]]`s.
`[reception]` carries the passive-display globals + transparent login and
`[[reception.viewer]]`s. **Advanced settings are file-only by design** (auth,
encoder, install dir, base port, input/audio/keyboard/TLS flags) — the web UI
only edits transmitters and viewers; the tray's "Ouvrir config" opens the TOML.

### Config generation is layered, not modeled (`shared/src/gen.rs`)
`render_config()` takes the operator's free-form `[emission.defaults]` table and
layers transmitter-specific values on top: injects `port`, forces `tray = false`
(instances are managed from the KyberFrog tray, not their own), pins/removes
`spout_sender` per `Source`, defaults the encoder to **x264** (AMF crashes on
the project's RX 7800 XT), and injects a **transparent basic-auth login** when
the operator declared none (kycontroller has no anonymous mode). Operator-
provided values always win.

### Transparent auth
`DEFAULT_AUTH_USERNAME`/`PASSWORD` (`vj`/`kyberfrog`) in `shared` are baked into
generated configs (hashed) *and* into the kyclient args, so on a trusted LAN the
operator never types a password. Surfacing real credential management is deferred
(see `IMPROVEMENTS.md`).

### One supervisor for both kinds (`kyberfrog/src/supervisor.rs`)
A single `Manager` supervises **both** `kycontroller` transmitters and
`kyclient` viewers. Each child runs in its own tokio task that spawns the
process, restarts it with capped exponential backoff (reset after
`HEALTHY_UPTIME`), and stops on a `watch` shutdown signal. The per-kind
differences (binary, args, `KYBER_CONFIG_PATH` env + cwd for transmitters, log
path) are resolved up front into a `Spec`; the loop is shared. Lifecycle lands
in one `StatusMap` (`Arc<Mutex<HashMap<Key, State>>>`) keyed by a typed
`Key::Tx(name)` / `Key::Vw(id)` so the two namespaces never collide. `State`
(`Starting/Running/Restarting/Stopped`) exposes `as_str()` (web) and `symbol()`
(tray — monochrome `○●◐✗` glyphs because Win32 GDI menus can't draw color
emoji). Child stdout/stderr go to per-child log files.

**Job Object (every child).** All children — kycontroller *and* kyclient — are
assigned to one Windows **Job Object** created with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When KyberFrog exits for any reason
(Ctrl-C, Task Manager kill, crash), Windows terminates every child it spawned.
The `HANDLE` is wrapped in `Arc<Option<JobGuard>>` so `Manager` stays `Send`
(required by axum).

### Shared state + operations (`kyberfrog/src/app.rs`)
`AppState { config, manager, status, tray_model }` is shared by every web
handler and the tray-command loop. Every mutation goes through one `op_*`
function so both front-ends stay in lockstep: lock config → apply to the
`Manager` → persist `kyberfrog.toml` → refresh the tray's render snapshot. Locks
are always taken **config before manager** to avoid deadlock.

Two identifiers can be chosen from the web UI (the tray always auto-picks):
- a **transmitter's port** at create time (`resolve_port`: an explicit free port
  wins, else auto-allocate from `base_port`);
- a **viewer's id/name**, at create *and* via rename on the edit form
  (`resolve_viewer_id`: a valid (`[A-Za-z0-9-]`), unique id wins, else keep the
  old / auto `viewer-N`). A rename stops the old child and starts the new id (new
  log file); the old log is left as an orphan.

### Cross-platform module pattern (`tray/`, `spout`)
Windows-specific subsystems use a `mod.rs` that re-exports either the real impl
or a no-op stub: `#[cfg(windows)] use windows as imp;` /
`#[cfg(not(windows))] use stub as imp;`. This keeps the binary compiling and
running headless off-Windows (for dev/test) while the real behavior is Win32.
The tray thread talks to the async main loop over an `mpsc` channel of
`TrayCommand`s; the unified `TrayCommand` carries both `*Tx`/`*Viewer` variants.

### Run loop
`main.rs` builds the `Manager`, starts every transmitter and every *enabled*
viewer, builds the shared `AppState`, spawns the tray (falling back to headless
on failure) and the web server on one port, then `tokio::select!`s on tray
commands and Ctrl-C.

## Conventions & gotchas
- **kyclient arg ordering is strict:** `[OPTIONS] [--] [STREAMER_IP]`. The
  positional server IP must be pushed **last** in `Globals::kyclient_args()`, or
  clap desyncs and rejects the flags.
- **Binaries are found via PATH:** `kyclient_path` defaults to a bare name
  (`kyclient.exe`); `kycontroller` is resolved as `kyber_install_dir\kycontroller.exe`.
  `validate()` only warns about a *missing absolute* path, never a bare name
  (that resolves at spawn time).
- **The tray icon is embedded** in the exe at build time via `build.rs` +
  `winresource` (calls `windres` to embed `kyberfrog/assets/kyberfrog.ico` as
  Windows resource ID 1). A `kyberfrog.ico` next to the exe overrides it.
- **Logging:** `flexi_logger`, dual sink (stderr + `%APPDATA%\kyberfrog\logs\kyberfrog.log`),
  `detailed_format` for timestamps. `suppress_timestamp()` only affects the log
  *filename*, not the line content. Per-child logs: `logs\kyclient-<id>.log` and
  `instances\<name>\kycontroller.log` (the web UI tails these).
- **Ports:** transmitters start at `base_port` (8080) and take the next free one;
  the single web UI is `web_port` (7700). All in the TOML.
- **Status keys are typed:** look a child's state up with
  `state_of(&map, &Key::Tx(name))` / `Key::Vw(id)` — never a bare string.

## Project context (so a fresh session doesn't re-derive it)

**GitLab.** Origin is `git@gitlab.com:kyber-frog/kyberfrog.git`, branch `main`,
public, AGPL-3.0. Note the spelling split: the GitLab **group path is
`kyber-frog`** (hyphen, because the bare `kyberfrog` namespace was globally
taken) while the **internal code name is `kyberfrog`** (no hyphen — used for
crate/package names, `%APPDATA%\kyberfrog`, the icon). This is *not* a Kyber
fork, so its remote is `origin` (the actual Kyber forks use `fork`). No `glab`/
`gh` CLI on the host; use `git` + the GitLab web UI. Author: Tristan Perrault
<tritriper35@gmail.com>.

**Relationship to Kyber.** KyberFrog orchestrates a private **fork of Kyber**
(kyber.stream) whose repos — `txproto`, `kymedia`, `kyber-desktop`, `kyctl` —
also live under the `kyber-frog` group. The fork carries the three changes this
project depends on: `KYBER_CONFIG_PATH` env override (share one install across N
instances), `spout_sender` pinning in kyavserver (Spout sender id =
**FFmpeg `AV_CRC_32_IEEE`** CRC-32, not plain CRC32), and the `--fullscreen`
flag on kyclient. kycontroller enforces a **single-session-per-instance** policy
— hence one kycontroller process per transmitter. Its internal IPC ports
auto-allocate in 9091..9100, so **max ~9 concurrent instances**.

**Deployment (the motivating VJ setup).** Resolume Arena on the regie PC
publishes Spout outputs; KyberFrog streams each over LAN (QUIC) to display PCs
running kyclient fullscreen. The regie GPU is an AMD RX 7800 XT whose **AMF
encoder crashes in a silent loop**, which is why the generated config defaults
to **x264**. On the dev/regie machine Kyber is installed at `D:\soft\kyber`
(this is also the historical `default_install_dir()` in `shared/src/config.rs`,
overridable via `kyber_install_dir` in `kyberfrog.toml`).

**Dev loop.** No native Rust on the host — build/test through
`kyber/debian-win64:local` (a locally-built image; the GitLab registry copy
can't be pulled). When mounting the volume, **use PowerShell, not git-bash**:
git-bash rewrites `-w /work` into a Windows path and breaks the container. The
`shared` crate is pure (no Win32) so it type-checks and tests on the Linux
container target; the Win32 code only compiles for `x86_64-pc-windows-gnu`.

**UX decision — exiting fullscreen.** A passive display has no quit shortcut by
design; the operator escape hatch is **Ctrl+Alt+F** (drops to windowed and
releases the keyboard grab, giving back Windows access). There is no "maintenance
mode" because you never voluntarily quit a viewer — you just go windowed.

**Known cleanup.** A temporary test login `kybertest` / `kyspout-poc-2026` may
still sit in `D:\soft\kyber\kyber_config.toml` — to be removed.

## In-flight restructuring (improvements brief)

This unified-app shape is **Amélioration 1** of a 3-step plan agreed with the
operator:
1. ✅ **Unified app + web UI** (this) — one binary, one supervisor, one web UI on
   7700, one `kyberfrog.toml`, tray keeps quick actions for both roles, advanced
   settings stay file-only.
2. **Spout output from kyclient** (next, in 2 sub-steps) — let a viewer
   re-publish the received video as a Spout sender for other local apps
   (Resolume, MadMapper). Partly depends on a fork-side change.
3. **Tauri desktop app** (later) — wrap the existing web UI as a real Windows
   app. **Do not start before 1 and 2 are done.**
