# Architecture

KyberFrog is **one app, one supervisor, one web UI (`:7700`), one tray**, and one
`%APPDATA%\kyberfrog\kyberfrog.toml`. The role (emit and/or receive) is set by
the config, not by which binary you run.

## Crates

```
Cargo.toml          workspace (members: shared, kyberfrog; shared deps + version)
shared/             kyberfrog-shared — data model + config gen + paths (no Win32, tests on Linux)
  src/lib.rs          Transmitter / Source, DEFAULT_* consts, re-exports config types
  src/config.rs       Config (emission + reception), Viewer, Globals, load/save, kyclient_args(), tests
  src/gen.rs          render_config(): layer [emission.defaults] + per-transmitter values → kyber_config.toml
  src/paths.rs        every %APPDATA%\kyberfrog\ location
kyberfrog/          kyberfrog — the single binary (both roles)
  build.rs            embeds assets/kyberfrog.ico as Win resource (winresource → windres)
  src/main.rs         tokio entry, flexi_logger, builds Manager + AppState + tray + web, command loop
  src/supervisor.rs   Manager + one supervise loop for BOTH kinds (Key::Tx/Vw, StatusMap, State, Job Object)
  src/app.rs          AppState + the op_* functions both UIs call; naming/port allocation; status payload
  src/spout.rs        live Spout-sender enumeration for the "Add" picker (Win32)
  src/tray/           system tray (mod re-exports windows|stub by cfg); muda menu, both sections
  src/web.rs + web/index.html   dashboard + JSON API + GET /transmitters discovery
```

- **`shared`** is the data model and the *only* place that knows Kyber's config
  schema. It is pure (no Win32) so it type-checks and **tests on the Linux
  container target**.
- **`kyberfrog`** is the single binary: supervisor, shared state + operations,
  spout enumeration, tray, web.

## One config, two halves

`kyberfrog.toml` deserializes into
`Config { kyber_install_dir, web_port, emission, reception }`.

- `[emission]` — `base_port`, the free-form `[emission.defaults]` TOML table, and
  the `[[emission.transmitter]]`s.
- `[reception]` — the passive-display globals + transparent login and the
  `[[reception.viewer]]`s.

**Advanced settings are file-only by design** (auth, encoder, install dir, base
port, input/audio/keyboard/TLS flags). The web UI only edits transmitters and
viewers; the tray's *Ouvrir config* opens the TOML.

## Config generation is layered, not modeled

`render_config()` (`shared/src/gen.rs`) takes the operator's free-form
`[emission.defaults]` table and layers transmitter-specific values on top:

- injects `port`,
- forces `tray = false` (instances are managed from KyberFrog's tray, not their
  own),
- pins / removes `spout_sender` per `Source`,
- defaults the encoder to **x264** (AMF crashes on the project's RX 7800 XT),
- injects a **transparent basic-auth login** when the operator declared none
  (kycontroller has no anonymous mode).

**Operator-provided values always win.**

### Transparent auth

`DEFAULT_AUTH_USERNAME` / `PASSWORD` (`vj` / `kyberfrog`) in `shared` are baked
into generated configs (hashed) *and* into the kyclient args, so on a trusted
LAN the operator never types a password. Surfacing real credential management is
deferred (`IMPROVEMENTS.md` #3).

## One supervisor for both kinds

A single `Manager` (`kyberfrog/src/supervisor.rs`) supervises **both**
`kycontroller` transmitters and `kyclient` viewers. Each child runs in its own
tokio task that spawns the process, restarts it with capped exponential backoff
(reset after `HEALTHY_UPTIME`), and stops on a `watch` shutdown signal. The
per-kind differences (binary, args, `KYBER_CONFIG_PATH` env + cwd for
transmitters, log path) are resolved up front into a `Spec`; the loop is shared.

Lifecycle lands in one `StatusMap` (`Arc<Mutex<HashMap<Key, State>>>`) keyed by a
typed `Key::Tx(name)` / `Key::Vw(id)` so the two namespaces never collide.
`State` (`Starting/Running/Restarting/Stopped`) exposes `as_str()` (web) and
`symbol()` (tray — monochrome `○●◐✗` glyphs, because Win32 GDI menus can't draw
color emoji).

!!! note "Job Object — every child"
    All children (kycontroller *and* kyclient) are assigned to one Windows
    **Job Object** created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. When
    KyberFrog exits for any reason (Ctrl-C, Task Manager kill, crash), Windows
    terminates every child it spawned. The `HANDLE` is wrapped in
    `Arc<Option<JobGuard>>` so `Manager` stays `Send` (required by axum).

## Shared state + operations

`AppState { config, manager, status, tray_model }` (`kyberfrog/src/app.rs`) is
shared by every web handler and the tray-command loop. Every mutation goes
through one `op_*` function so both front-ends stay in lockstep:

> lock config → apply to the `Manager` → persist `kyberfrog.toml` → refresh the
> tray's render snapshot.

Locks are always taken **config before manager** to avoid deadlock.

Two identifiers can be chosen from the web UI (the tray always auto-picks):

- a **transmitter's port** at create time (`resolve_port`: an explicit free port
  wins, else auto-allocate from `base_port`);
- a **viewer's id/name**, at create *and* via rename on the edit form
  (`resolve_viewer_id`: a valid (`[A-Za-z0-9-]`), unique id wins, else keep the
  old / auto `viewer-N`). A rename stops the old child and starts the new id (new
  log file); the old log is left as an orphan.

## Cross-platform module pattern

Windows-specific subsystems (`tray/`, `spout`) use a `mod.rs` that re-exports
either the real impl or a no-op stub: `#[cfg(windows)] use windows as imp;` /
`#[cfg(not(windows))] use stub as imp;`. This keeps the binary compiling and
running headless off-Windows (for dev/test) while the real behavior is Win32.
The tray thread talks to the async main loop over an `mpsc` channel of
`TrayCommand`s (one unified enum carrying both `*Tx`/`*Viewer` variants).

## Run loop

`main.rs` builds the `Manager`, starts every transmitter and every *enabled*
viewer, builds the shared `AppState`, spawns the tray (falling back to headless
on failure) and the web server on one port, then `tokio::select!`s on tray
commands and Ctrl-C.

## Conventions & gotchas

- **kyclient arg ordering is strict:** `[OPTIONS] [--] [STREAMER_IP]`. The
  positional server IP must be pushed **last** in `Globals::kyclient_args()`, or
  clap desyncs and rejects the flags.
- **Binaries are found via PATH:** `kyclient_path` defaults to a bare name
  (`kyclient.exe`); `kycontroller` resolves as
  `kyber_install_dir\kycontroller.exe`. `validate()` only warns about a *missing
  absolute* path, never a bare name (resolved at spawn time).
- **MinGW `HANDLE` is `*mut c_void`** (not `isize`) — null-check raw handles with
  `is_null()` / `std::ptr::null_mut()`, never `== 0`.
- **The tray icon is embedded** at build time (`build.rs` + `winresource` →
  `windres`, resource ID 1). A `kyberfrog.ico` next to the exe overrides it.
- **Logging:** `flexi_logger`, dual sink (stderr + `logs\kyberfrog.log`),
  `detailed_format`. Per-child logs: `logs\kyclient-<id>.log` and
  `instances\<name>\kycontroller.log`.
- **Status keys are typed:** `state_of(&map, &Key::Tx(name))` / `Key::Vw(id)` —
  never a bare string.

## Relationship to the Kyber fork

KyberFrog orchestrates a private **fork of Kyber** whose repos (`txproto`,
`kymedia`, `kyber-desktop`, `kyctl`) live under the same `kyber-frog` group.
The fork carries three load-bearing changes:

- **`KYBER_CONFIG_PATH`** env override — N instances share one install;
- **`spout_sender` pinning** in kyavserver (sender id = FFmpeg `AV_CRC_32_IEEE`
  CRC-32, not plain CRC32) + the `iosys_spout` source in txproto;
- the **`--fullscreen`** flag on kyclient.

`kycontroller` enforces a **single-session-per-instance** policy (hence one
process per transmitter) and auto-allocates internal IPC ports in `9091..9100`
→ **max ~9 concurrent instances**. Building these binaries is the
**[fork build model](building.md#the-fork-build-model)**.
