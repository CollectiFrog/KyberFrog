# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

KyberFrog is an orchestration layer on top of the **kyber-frog fork of Kyber**
(QUIC video transport, a drop-in NDI replacement for LAN). It turns one Kyber
install into **N independent transmitters** and supervises the viewers:

- **Server** (regie/host machine): reads `transmitters.toml`, generates one
  `kyber_config.toml` per transmitter, and supervises one `kycontroller` process
  per transmitter (isolated by port + `KYBER_CONFIG_PATH`).
- **Client** (display machine): reads `client-agent.toml`, supervises N
  fullscreen `kyclient` viewers.

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
# Build both exes → target/x86_64-pc-windows-gnu/release/{kyberfrog-server,kyberfrog-client}.exe
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu

# Run the whole test suite (unit tests live in shared/: lib.rs, gen.rs)
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test

# Run a single test by name
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo test -p kyberfrog-shared directory_round_trips_both_source_kinds
```

The `x86_64-pc-windows-gnu` target matters: the Win32 code (tray, Job Object,
icon loading) only compiles for Windows, and MinGW defines `HANDLE` as
`*mut c_void` (not `isize`) — null-check raw handles with `std::ptr::null_mut()`,
never `== 0`.

## Project layout

```
Cargo.toml                       workspace (members: shared, server, client; shared deps + version)
README.md                        user-facing: prerequisites (install kyber fork + PATH), install, build, run
IMPROVEMENTS.md                  deferred work / tech debt, numbered (#1 monitor targeting … #7 GitLab CI)
examples/transmitters.toml       reference server config (the auth schema here is the *correct* one)

shared/                          kyberfrog-shared — model + config gen + paths (no Windows code, testable on Linux)
  src/lib.rs                       Directory / Transmitter / Source, DEFAULT_* consts, port allocation, unit tests
  src/gen.rs                       render_config(): layer [defaults] + per-transmitter values → kyber_config.toml
  src/paths.rs                     every %APPDATA%\kyberfrog\ location (configs, logs, per-instance dirs)

server/                          kyberfrog-server — regie orchestrator (one kycontroller per transmitter)
  build.rs                         embeds assets/kyberfrog.ico as Win resource (winresource → windres)
  assets/kyberfrog.ico             Collecti'Frog logo, embedded + override-next-to-exe
  src/main.rs                      tokio entry, flexi_logger, Manager + tray + web, command loop
  src/config.rs                    load/save transmitters.toml + validation, kycontroller_path()
  src/supervisor.rs                Manager + per-transmitter supervise loop (backoff, StatusMap, State)
  src/spout.rs                     live Spout-sender enumeration for the tray "Add" picker (Win32)
  src/tray/{mod,windows,stub}.rs   system tray (mod re-exports windows|stub by cfg); muda menu
  src/web.rs + web/index.html      read-only dashboard + GET /transmitters discovery JSON (axum)

client/                          kyberfrog-client — display agent (N fullscreen kyclient viewers)
  build.rs                         embeds the same icon (path = ../server/assets/kyberfrog.ico)
  install/install-client-agent.ps1 registers an AtLogOn scheduled task for hands-off autostart
  src/main.rs                      tokio entry, flexi_logger, Manager + tray + web, autostarts enabled instances
  src/config.rs                    ClientConfig (globals + Vec<Instance>), kyclient_args(), legacy migration
  src/supervisor.rs                Manager (start/stop/restart per id) + Job Object (kill-on-close)
  src/tray/{mod,windows,stub}.rs   same tray pattern as server
  src/web.rs + web/index.html      editable dashboard; proxies regie /transmitters via GET /available (CORS)
  README.md                        client-specific setup + autostart/autologon notes
```

## Architecture

### Crates
- **`shared` (`kyberfrog-shared`)** — the data model and the *only* place that
  knows Kyber's config schema. `lib.rs` holds `Directory` / `Transmitter` /
  `Source`; `gen.rs` renders per-instance `kyber_config.toml`; `paths.rs` is the
  single source of truth for every on-disk location (all under
  `%APPDATA%\kyberfrog\`).
- **`server` (`kyberfrog-server`)** — orchestrator: `config.rs` (load/save
  `transmitters.toml`), `supervisor.rs` (`Manager` + per-transmitter supervise
  loop), `spout.rs` (live Spout-sender enumeration for the tray picker), `tray/`,
  `web/`.
- **`client` (`kyberfrog-client`)** — viewer supervisor: same shape
  (`config.rs` for `client-agent.toml`, `supervisor.rs`, `tray/`, `web/`).

### Config generation is layered, not modeled (`shared/src/gen.rs`)
The server never models Kyber's full config. `render_config()` takes the
operator's free-form `[defaults]` TOML table and layers transmitter-specific
values on top: injects `port`, forces `tray = false` (instances are managed from
the KyberFrog tray, not their own), pins/removes `spout_sender` per `Source`,
defaults the encoder to **x264** (AMF crashes on the project's RX 7800 XT), and
injects a **transparent basic-auth login** when the operator declared none
(kycontroller has no anonymous mode). Operator-provided values always win.

### Transparent auth
`DEFAULT_AUTH_USERNAME`/`PASSWORD` (`vj`/`kyberfrog`) in `shared` are baked into
generated configs (hashed) *and* into the client's kyclient args, so on a trusted
LAN the operator never types a password. Surfacing real credential management is
deferred (see `IMPROVEMENTS.md`).

### Supervision (both `supervisor.rs` files)
Each instance runs in its own tokio task that spawns the child, restarts it with
capped exponential backoff (reset after `HEALTHY_UPTIME`), and stops on a
`watch` shutdown signal. A `StatusMap` (`Arc<Mutex<HashMap<name, State>>>`) is
shared between the supervisor and the UI layers; the four-state `State`
(`Starting/Running/Restarting/Stopped`) exposes both `as_str()` (web) and
`symbol()` (tray — monochrome `○●◐✗` glyphs because Win32 GDI menus can't draw
color emoji). Child stdout/stderr go to per-instance log files.

**Client-only:** all kyclient children are assigned to a Windows **Job Object**
created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When kyberfrog-client exits
for any reason (Ctrl-C, Task Manager kill, crash), Windows terminates every
kyclient. The `HANDLE` is wrapped in `Arc<Option<JobGuard>>` so `Manager` stays
`Send` (required by axum).

### Cross-platform module pattern (`tray/`, server `spout`)
Windows-specific subsystems use a `mod.rs` that re-exports either the real impl
or a no-op stub: `#[cfg(windows)] use windows as imp;` /
`#[cfg(not(windows))] use stub as imp;`. This keeps the binary compiling and
running headless off-Windows (for dev/test) while the real behavior is Win32.
The tray thread talks to the async main loop over an `mpsc` channel of
`TrayCommand`s.

### Run loop
`main.rs` builds the `Manager`, starts everything in the config, spawns the tray
(falling back to headless on failure) and the web server, then `tokio::select!`s
on tray commands and Ctrl-C. Tray/web mutations go through a shared `AppState`
and are persisted back to the TOML source of truth immediately.

## Conventions & gotchas
- **kyclient arg ordering is strict:** `[OPTIONS] [--] [STREAMER_IP]`. The
  positional server IP must be pushed **last** in `Globals::kyclient_args()`, or
  clap desyncs and rejects the flags.
- **Binaries are found via PATH:** `kyclient_path`/`kycontroller_path` default to
  bare names (`kyclient.exe`). `validate()` only warns about a *missing absolute*
  path, never a bare name (that resolves at spawn time).
- **The tray icon is embedded** in each exe at build time via `build.rs` +
  `winresource` (calls `windres` to embed `server/assets/kyberfrog.ico` as
  Windows resource ID 1). A `kyberfrog.ico` next to the exe overrides it.
- **Logging:** `flexi_logger`, dual sink (stderr + `%APPDATA%\kyberfrog\logs\`),
  `detailed_format` for timestamps. `suppress_timestamp()` only affects the log
  *filename*, not the line content.
- **Ports:** transmitters start at `base_port` (8080) and take the next free one;
  server web UI 7700, client web UI 7701. All configurable in the TOML.
- **Legacy migration:** the client config was renamed `scene-agent.toml` →
  `client-agent.toml`; `migrate_legacy_config()` renames it transparently on
  first run. Keep `legacy_scene_agent_file()` until installs have migrated.

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
(this is also the historical `default_install_dir()` in `shared/src/lib.rs`,
overridable via `kyber_install_dir` in `transmitters.toml`).

**Dev loop.** No native Rust on the host — build/test through
`kyber/debian-win64:local` (a locally-built image; the GitLab registry copy
can't be pulled). When mounting the volume, **use PowerShell, not git-bash**:
git-bash rewrites `-w /work` into a Windows path and breaks the container. The
`shared` crate is pure (no Win32) so it type-checks and tests on the Linux
container target; the Win32 code only compiles for `x86_64-pc-windows-gnu`.

**UX decision — exiting fullscreen.** A passive display has no quit shortcut by
design; the operator escape hatch is **Ctrl+Alt+F** (drops to windowed and
releases the keyboard grab, giving back Windows access). The client agent has no
"maintenance mode" because you never voluntarily quit a viewer — you just go
windowed.

**Known cleanup.** A temporary test login `kybertest` / `kyspout-poc-2026` may
still sit in `D:\soft\kyber\kyber_config.toml` — to be removed.
