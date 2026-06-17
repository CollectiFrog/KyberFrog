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
