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
  src/main.rs         sync entry: flexi_logger + hand-built tokio runtime + bootstrap(), hands off to shell::run
  src/shell/          native desktop shell (#21): Tauri/WebView2 window over localhost:<web_port> on Windows
                      (close = hide, only the tray quits), headless command loop elsewhere
  src/supervisor.rs   Manager + one supervise loop for BOTH kinds (Key::Tx/Vw, StatusMap, State, Job Object)
  src/app.rs          AppState + the op_* functions both UIs call; naming/port allocation; status payload
  src/discovery.rs    mDNS/DNS-SD: announce one _kyber._tcp service per active transmitter + browse the LAN (GET /discovered)
  src/spout.rs        live Spout-sender enumeration for the "Add" picker (Win32)
  src/cameras.rs      capture-device enumeration for the webcam picker (DirectShow / V4L2 via the bundled ffmpeg)
  src/displays.rs     asks a remote emitter for its physical displays (viewer "source screen" picker)
  src/tray/           system tray (mod re-exports windows|stub by cfg); muda menu, both sections
  src/web.rs          JSON API + serves the React build (ui/dist) on :7700
ui/                 React + Vite dashboard (built to ui/dist, shipped next to the binary)
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

**Advanced settings are file-only by design** (auth, install dir, base port,
input/audio/keyboard/TLS flags). The web UI edits transmitters, viewers and the
machine preferences (theme, language, video encoder); the tray's *Ouvrir config*
opens the TOML.

## Config generation is layered, not modeled

`render_config()` (`shared/src/gen.rs`) takes the operator's free-form
`[emission.defaults]` table and layers transmitter-specific values on top:

- injects `port`,
- forces `tray = false` (instances are managed from KyberFrog's tray, not their
  own),
- pins / removes `spout_sender` per `Source`,
- **always writes the machine's resolved encoder** (`shared/src/encoder.rs`):
  the `encoder` machine setting, where `auto` picks AMF or NVENC from the
  vendor of DXGI adapter 0 (the adapter txproto captures and encodes on) and
  x264 otherwise; an `encoder` inherited from the setup is ignored,
- injects a **transparent basic-auth login** when the operator declared none
  (kycontroller has no anonymous mode).

**Other operator-provided values win.**

### Transparent auth

`DEFAULT_AUTH_USERNAME` / `PASSWORD` (`vj` / `kyberfrog`) in `shared` are baked
into generated configs (hashed) *and* into the kyclient args, so on a trusted
LAN the operator never types a password. Credentials in the UI are
[backlog](backlog.md) #3.

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

`AppState { config, manager, status, tray_model, discovery }`
(`kyberfrog/src/app.rs`) is shared by every web handler and the tray-command
loop. Every mutation goes through one `op_*` function so both front-ends stay
in lockstep:

> lock config → apply to the `Manager` → persist `kyberfrog.toml` → refresh the
> tray's render snapshot → re-sync mDNS announcements.

Locks are always taken **config before manager** to avoid deadlock.

## Auto-discovery (mDNS/DNS-SD)

`kyberfrog/src/discovery.rs` builds one `mdns-sd` daemon per process that plays
**both** roles:

- **Announcer** — one `_kyber._tcp.local.` service per *active* transmitter
  (instance `<tx>@<hostname>`, TXT: `version`/`tx`/`kind`). `Discovery::sync()`
  diffs the desired set against what's registered and re-announces on every
  transmitter mutation (called from `persist_and_refresh`, the same choke
  point as the tray refresh).
- **Browser** — a long-lived task (`spawn_browser`) consumes `mdns-sd`'s event
  channel and maintains a `HashMap` snapshot, served by `GET /discovered`. The
  viewer form's "Émetteurs détectés" picker polls it and fills
  name/server/port on click.

**KyberFrog is the sole announcer.** It already knows every transmitter's name
and port at runtime, so announcing needs no fork change and stays in step with
every mutation. The trade-off: a `kycontroller` started outside KyberFrog is
invisible to discovery, which does not happen in this deployment. Opt-out:
`mdns = false` in `kyberfrog.toml` (file-only, defaults to on).

Two identifiers can be chosen from the web UI (the tray always auto-picks):

- a **transmitter's port** at create time (`resolve_port`: an explicit free port
  wins, else auto-allocate from `base_port`);
- a **viewer's id/name**, at create *and* via rename on the edit form
  (`resolve_viewer_id`: a valid (`[A-Za-z0-9-]`), unique id wins, else keep the
  old / auto `viewer-N`). A rename stops the old child and starts the new id (new
  log file); the old log is left as an orphan.

## Cross-platform module pattern

Windows-specific subsystems (`tray/`, `spout`, `shell/`) use a `mod.rs` that
re-exports either the real impl or a no-op stub: `#[cfg(windows)] use windows as
imp;` / `#[cfg(not(windows))] use stub as imp;`. **Linux amd64 is a shipped
target**, and it is the stubs that make it run headless there (no tray, no Spout, no native window; the
dashboard is the browser). The UI hides what the platform cannot do by asking
`/status.platform`, so a Linux box never shows a Spout tile or a *Tout envoyer*
toggle it would silently ignore.
The tray thread talks to the async main loop over an `mpsc` channel of
`TrayCommand`s (one unified enum carrying both `*Tx`/`*Viewer` variants).

## Run loop (`main.rs` + `shell/`)

`main()` is **not** async: the Tauri event loop must own the main thread. It
builds the tokio runtime by hand, `block_on`s `bootstrap()` (Manager, every
transmitter + enabled viewer, mDNS, `AppState`, web server, tray — tray failure
falls back to a dummy channel), then hands over to `shell::run`. On Windows
that opens the **native dashboard window** — a Tauri/WebView2 chrome over the
very same `http://localhost:<web_port>/` the browser uses (same-origin, no
IPC, zero UI rewrite; `KYBERFROG_UI_URL` can point it at the vite dev server) —
and spawns the command loop as a runtime task; elsewhere the headless stub just
`block_on`s that loop. Both `tokio::select!` on tray commands and Ctrl-C and
end through one shared `shell::shutdown`. Closing the window only hides it;
**only the tray's "Quitter" (or Ctrl-C) stops the app**. The exe is a GUI app
(`windows_subsystem = "windows"`, no console in any build) and children are
spawned with `CREATE_NO_WINDOW` so no console ever pops (kycontroller, ffmpeg
enumeration — kyavserver inherits the invisible console).

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

KyberFrog orchestrates a **fork of Kyber** whose seven repos live under the same
`kyber-frog` group ([inventory](audit-fork-chain.md)). The fork carries these
load-bearing changes:

- **`KYBER_CONFIG_PATH`** env override — N instances share one install.
  Upstream 0.27 implements this natively as `KYBER_CONFIG`; migrating is
  [backlog](backlog.md) #36;
- **`spout_sender` pinning** in kyavserver (sender id = FFmpeg `AV_CRC_32_IEEE`
  CRC-32, not plain CRC32) + the `iosys_spout` source in txproto;
- **`camera_device` pinning** — the same mechanism for a capture device
  (DirectShow on Windows, V4L2 card name on Linux), plus the lavd path in txproto it took to make webcams work;
- **`all_sources`** — expose every monitor *and* every Spout sender from one
  kyavserver, backing "Tout envoyer" (`cfg(windows)` in the fork);
- **Spout output and its zero-copy path** — libVLC renders straight into the
  shared D3D11 texture, GPU→GPU, no CPU round-trip
  ([plan](plan-spout-zerocopy.md));
- **`grab_backend`** — explicit screen-capture backend selection, which is what
  makes Linux capture (`xcb` / `drm` / `wlroots` / `nvfbc`) selectable;
- the **`--fullscreen`** and `--display-idx` flags on kyclient.

`kycontroller` enforces a **single-session-per-instance** policy (hence one
process per transmitter) and auto-allocates internal IPC ports in `9091..9100`
→ **max ~9 concurrent instances**. Building these binaries is the
**[fork build model](building.md#the-fork-build-model)**.
