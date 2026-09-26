# AGENTS.md

This file provides guidance to coding agents (Claude Code and others) when
working with code in this repository.

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

## Workflow

- **No direct pushes to `dev` or `main`.** Every change goes through a
  `feat/<slug>` or `fix/<slug>` branch and a merge request. `dev` is the
  integration branch (protected, no force-push); `main` tracks releases.
- **Update `CHANGELOG.md` in the same branch/MR as the change**, not after
  the fact. Add the line under `## [Non publié]` (create the section — with
  the `### Ajouté` / `### Modifié` / `### Corrigé` / `### Limitations connues`
  / `### CI / build` subsections it needs — if it isn't there yet) as part of
  the commit that introduces the change, not as a separate follow-up.
  **One change = one physical line, no wrap**: details live in the commit
  body or in `docs/`, never spilled onto a second line of the bullet itself.
  Scripts, CI and fork tooling (`rebase-fork.sh`, `fork-lint.sh`,
  `build-*.sh`) go under `### CI / build`, not `Modifié`/`Corrigé`. When a
  line under `[Non publié]` quotes a SHA or a pin and a later commit moves
  it, **update that line** — never leave a stale SHA.
- **Keep the board true in the same MR.** A merge into `dev` that touches a
  numbered item also updates, in that MR: its card *and* its detail row in
  `docs/dev/backlog.md` (never name a branch that the merge deletes — say
  "merged into `dev`, `<sha>`"; if only a run is left, the card moves to
  🧪 *To run* and gets a validation-queue row), `docs/dev/todo-linux.md` for
  Linux items, and the status sentences of its `plan-*.md`. Then recount:
  the stats bar, every column header and the area bar at the top of
  `backlog.md` must match the cards. A validation-queue result is recorded
  the same way, everywhere the "to verify" wording appears (`grep` the item
  number). Any long-lived branch — whoever wrote it — gets a card with its
  branch name.
- **A release is its tag.** Write `## [x.y.z] — <date>` in `CHANGELOG.md`
  only in the release MR (dev → main) that is tagged `vx.y.z` right after;
  a section without a tag is a lie. In that same MR: every shipped item
  leaves `backlog.md` for `backlog-archive.md`, every `unreleased` in the
  archive's *Version* column becomes the version, and the compare link
  `[x.y.z]: …/compare/v<prev>...vx.y.z` is added at the bottom of the
  CHANGELOG. Tagging and pushing are the operator's call — prepare, don't
  tag on your own.
- **Rebasing the Kyber fork chain** (`kyber-desktop`/`kysdk`/`kyctl`/`kymedia`/
  `kynput`/`txproto`/`vlc-rs` onto a new upstream `kyber.stream` version) is
  its own procedure — use the `.claude/skills/rebase-fork` skill rather than
  improvising `packaging/rebase-fork.sh` calls from scratch.
- **When a structural change is proposed or applied** (a new workflow rule, a
  process change, a convention, an architecture decision — not a regular
  feature/fix), proactively propose updating this file (`AGENTS.md`) to
  record it, in the same turn, rather than letting the doc drift out of sync
  with how the project actually works.

## Build & test

There is **no native Rust toolchain on the dev host** — every build runs in
Docker. `./dev.sh` (PowerShell/cmd: `.\dev`) owns the `docker run` lines and
pulls a missing build image on first use; run it from the repo root:

```sh
./dev.sh setup     # once per machine: build image, pinned fork bundle, ui/dist
./dev.sh exe       # → target/x86_64-pc-windows-gnu/release/kyberfrog.exe
./dev.sh test      # whole suite (unit tests live in shared/: config.rs, gen.rs, lib.rs)
./dev.sh test -p kyberfrog-shared config_round_trips_both_halves   # one test
./dev.sh check     # cargo check for the Windows target, Win32 code included
```

**UI dev loop (CSS / React changes, no Rust rebuild needed).**
`npm` n'est pas disponible nativement sur l'hôte — `./dev.sh ui` le lance dans
l'image `node:22-alpine`. Depuis la racine du repo :

```powershell
# 1. Build l'UI (TypeScript + Vite)
.\dev ui

# 2. Copier le dist dans le dossier du binaire debug
Copy-Item -Recurse -Force ui\dist\* `
  target\x86_64-pc-windows-gnu\debug\ui\dist\
```

Ensuite **F5 dans le navigateur** suffit — pas besoin de relancer `kyberfrog.exe`.

Un `cargo build` fait l'étape 2 tout seul (`kyberfrog/build.rs::stage_ui_dist`
recopie `ui/dist` à côté de l'exe du profil) — mais ne rebuild **pas** l'UI :
refaire l'étape 1 après toute modif de `ui/src`, sinon warning cargo.

Pour lancer l'app (si elle n'est pas déjà en cours) :

```powershell
# Depuis la racine du repo
Start-Process -FilePath .\target\x86_64-pc-windows-gnu\debug\kyberfrog.exe `
  -WorkingDirectory .\target\x86_64-pc-windows-gnu\debug
Start-Process "http://localhost:7700"
```

**Release build (single-file installer).** `./dev.sh installer` runs
`packaging/build-installer.sh`: it builds `kyberfrog.exe`, stages it with the web
UI and the fork binaries bundle, and runs `makensis` →
`dist/KyberFrog-Setup-<ver>.exe`. Without `-f`, the bundle is the one CI built
for the pinned `kyber-desktop` SHA, fetched by `packaging/fork-bundle.sh`;
`-f <dir|zip>` passes a local fork build instead. The pin is the gitlink of the
submodule `vendor/kyber-desktop` — empty unless `./dev.sh setup --fork` — and
`packaging/versions.sh` reads it.

CI (`.gitlab-ci.yml`) runs the same script on a `v*` tag and publishes a Release.
See `docs/dev/backlog-archive.md` (#9) and `packaging/windows/INSTALL.md`.

**Linux amd64 (portage livré).** Le même binaire tourne sous Linux et s'y
installe par un `.deb`. Le bundle du fork se construit **en local**, pas en CI
(~20 min contre ~1 h 30) : `packaging/linux/build-fork-local.sh` dans l'image
`kyber/debian-linux:local`, puis `./dev.sh deb` (`packaging/linux/build-deb.sh`)
pour le paquet.
Deux pièges Git Bash pour tout `docker run` : `cygpath -m` sur les chemins
*hôte*, et `MSYS_NO_PATHCONV=1` pour que les chemins *conteneur* (`/src`,
`-w /build/...`) ne soient pas réécrits en chemins Windows. Les chemins de
données suivent la plateforme (`shared/src/paths.rs`) : `%APPDATA%\kyberfrog`
sous Windows, `~/.config/kyberfrog` (config, setups) et
`~/.local/state/kyberfrog` (logs, instances) sous Linux. Détails :
`docs/dev/building.md` (§ Building for Linux) et `docs/dev/todo-linux.md`.

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
docs/dev/backlog.md              open work, numbered, with state + access labels
docs/dev/backlog-archive.md      shipped items, numbers preserved for old references
examples/kyberfrog.toml          reference unified config (the auth schema here is the *correct* one)

shared/                          kyberfrog-shared — model + config gen + paths (no Windows code, testable on Linux)
  src/lib.rs                       Transmitter / Source, DEFAULT_* consts, re-exports config types
  src/config.rs                    Config (emission + reception), Viewer, Globals, load/save,
                                   kyclient_args(), kycontroller_path(), unit tests
  src/gen.rs                       render_config(): layer [emission.defaults] + per-transmitter values → kyber_config.toml
  src/encoder.rs                   machine `encoder` setting, `auto` → AMF / NVENC / x264 (#28-1)
  src/paths.rs                     every data location (%APPDATA%\kyberfrog on Windows, XDG dirs on Linux)

kyberfrog/                       kyberfrog — the single binary (both roles)
  build.rs                         embeds assets/kyberfrog.ico as Win resource (winresource → windres)
  assets/kyberfrog.ico             Collecti'Frog logo, embedded + override-next-to-exe
  install/install-kyberfrog.ps1    registers an AtLogOn scheduled task for hands-off autostart
  src/main.rs                      sync entry: flexi_logger + hand-built tokio runtime + bootstrap() (Manager,
                                   AppState, tray, web), then hands the main thread to shell::run
  src/shell/{mod,windows,stub}.rs  native desktop shell (#21): Tauri/WebView2 window over http://localhost:<web_port>
                                   on Windows (close = hide, tray quits), headless command loop elsewhere;
                                   mod.rs owns Boot/dispatch/shutdown shared by both
  src/supervisor.rs                Manager + one supervise loop for BOTH kinds (Key::Tx/Vw, StatusMap, State, Job Object)
  src/app.rs                       AppState + the op_* functions both UIs call; naming/port allocation; status payload
  src/discovery.rs                 mDNS/DNS-SD (#20): announce one _kyber._tcp service per active transmitter + browse the LAN (GET /discovered)
  src/spout.rs                     live Spout-sender enumeration for the "Add" picker (tray + web) (Win32)
  src/cameras.rs                   webcam enumeration via the bundled ffmpeg (dshow on Windows, v4l2 on Linux)
  src/displays.rs                  asks a remote emitter for its screens (viewer "source screen" picker, #18-B)
  src/gpu.rs                       DXGI adapter 0 vendor → `auto` encoder (#28-1)
  src/session.rs                   graphical-session env for children + Linux capture backend pick
  src/tray/{mod,windows,stub}.rs   system tray (mod re-exports windows|stub by cfg); muda menu, both sections
  src/web.rs                       axum: JSON API + serves ui/dist (the React dashboard)

ui/                              React + Vite dashboard (`./dev.sh ui` → ui/dist, staged next to the exe by build.rs)
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
install dir, base port, input/audio/keyboard/TLS flags) — the web UI edits
transmitters, viewers and the machine preferences (theme, language, **video
encoder**); the tray's "Ouvrir config" opens the TOML.

### Config generation is layered, not modeled (`shared/src/gen.rs`)
`render_config()` takes the operator's free-form `[emission.defaults]` table and
layers transmitter-specific values on top: injects `port`, forces `tray = false`
(instances are managed from the KyberFrog tray, not their own), pins/removes
`spout_sender` per `Source`, **always writes the machine's resolved encoder**
(`shared/src/encoder.rs`: the `encoder` machine setting, `auto` = AMF/NVENC by
the vendor of DXGI adapter 0 — the adapter txproto captures and encodes on —
else x264; an `encoder` inherited from the setup is ignored), and injects a
**transparent basic-auth login** when the operator declared none (kycontroller
has no anonymous mode). Other operator-provided values win.

### Transparent auth
`DEFAULT_AUTH_USERNAME`/`PASSWORD` (`vj`/`kyberfrog`) in `shared` are baked into
generated configs (hashed) *and* into the kyclient args, so on a trusted LAN the
operator never types a password. Surfacing real credential management is deferred
(see `docs/dev/backlog.md`, #3).

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

### Run loop (`main.rs` + `shell/`)
`main()` is **not** async: the Tauri event loop must own the main thread, so it
builds the tokio runtime by hand, `block_on`s `bootstrap()` (Manager, every
transmitter + enabled viewer, mDNS, `AppState`, web server, tray — tray failure
falls back to a dummy channel), then hands `shell::Boot` to `shell::run`. On
Windows that runs the Tauri window (a pure chrome over
`http://localhost:<web_port>/` — same-origin with the API, no IPC, zero UI
rewrite; `KYBERFROG_UI_URL` can point it at the vite dev server) and spawns the
old command loop as a runtime task; elsewhere the stub just `block_on`s that
loop. Both `tokio::select!` on tray commands and Ctrl-C and end through one
shared `shell::shutdown`. Closing the window only hides it; **only the tray's
"Quitter" (or Ctrl-C) stops the app** — and `#![windows_subsystem = "windows"]`
hides the console in every build (debug included: read the log file / UI log
drawer, stderr goes nowhere on Windows).

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
- **Ports:** transmitters start at `base_port` (9000) and take the next free one;
  the single web UI is `web_port` (7700). All in the TOML.
- **Status keys are typed:** look a child's state up with
  `state_of(&map, &Key::Tx(name))` / `Key::Vw(id)` — never a bare string.

## Project context (so a fresh session doesn't re-derive it)

**GitLab.** Origin is `git@gitlab.com:kyber-frog/kyberfrog.git`, branch `main`,
public, AGPL-3.0. Note the spelling split: the GitLab **group path is
`kyber-frog`** (hyphen, because the bare `kyberfrog` namespace was globally
taken) while the **internal code name is `kyberfrog`** (no hyphen — used for
crate/package names, `%APPDATA%\kyberfrog`, the icon). This is *not* a Kyber
fork, so its remote is `origin` (the actual Kyber forks use `fork`). `glab` is
installed on the host but needs `glab auth login` once; without a token, fall
back to `git` + the GitLab web UI. Author: Tristan Perrault
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
running kyclient fullscreen. The regie GPU is an AMD RX 7800 XT. Its **AMF
encoder used to crash in a silent loop**, which is why x264 was the default
until 0.6.0; with the current bundle (FFmpeg 8.1, driver 32.0.31041.1004) AMF
held 10 min at ~4 ms Spout → Spout on the latency bench, so `auto` now picks
it (x264 was ~26 ms). `default_install_dir()` (`shared/src/config.rs`) resolves to the
running exe's own directory when `kycontroller.exe` sits next to it (the bundled
installer case), else falls back to `C:\Program Files\KyberFrog` (the installer's
default dir); overridable via `kyber_install_dir` in `kyberfrog.toml`.

**Dev loop.** No native Rust on the host — build/test through `./dev.sh`, in
`kyber/debian-win64:local` (pulled from this project's registry on first use).
Typed by hand from git-bash, `docker run` breaks: git-bash rewrites `-w /work`
into a Windows path — `dev.sh` applies `MSYS_NO_PATHCONV` and `cygpath -m`. The
`shared` crate is pure (no Win32) so it type-checks and tests on the Linux
container target; the Win32 code only compiles for `x86_64-pc-windows-gnu`.

**UX decision — exiting fullscreen.** A passive display has no quit shortcut by
design; the operator escape hatch is **Ctrl+Alt+F** (drops to windowed and
releases the keyboard grab, giving back Windows access). There is no "maintenance
mode" because you never voluntarily quit a viewer — you just go windowed.

## In-flight restructuring (improvements brief)

This unified-app shape is **Amélioration 1** of a 3-step plan agreed with the
operator:
1. ✅ **Unified app + web UI** (this) — one binary, one supervisor, one web UI on
   7700, one `kyberfrog.toml`, tray keeps quick actions for both roles, advanced
   settings stay file-only.
2. ✅ **Spout output from kyclient** — shipped as #8 (validated E2E against
   Resolume Arena, see `docs/dev/backlog-archive.md`).
3. ✅ **Tauri desktop app** (#21) — shipped & operator-validated 2026-07-15.
   Architecture and gotchas in `docs/dev/plan-tauri-shell.md`
   (window = chrome over the axum URL, NSIS kept, tray kept, close = hide,
   `WebView2Loader.dll` must ship next to the exe on windows-gnu).
