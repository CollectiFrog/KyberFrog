# KyberFrog 🐸

A polyvalent orchestration layer on top of [Kyber](https://kyber.stream):
publish **any source** as one of **N independent transmitters** and supervise
the clients, for low-latency, source-agnostic streaming over LAN (a drop-in
replacement for NDI).

Today the supported sources are **Spout** (Windows GPU texture share) and
**screen capture**; the model is designed to grow more input types (video
files, NDI, …) without touching the orchestration.

The motivating setup (VJing): **Resolume Arena** on a regie machine publishes
several **Spout** outputs; each output is streamed over LAN to one or more
scene machines via Kyber's QUIC transport.

```
                 ┌──────────────────────────── PCRegie ───────────────────────────┐
                 │                                                                  │
  Resolume ──Spout A──▶  Server  ──▶ kycontroller :8080 (pinned "Output A") ──┐     │
  Resolume ──Spout B──▶          ──▶ kycontroller :8081 (pinned "Output B") ──┤     │
                 │                                                            │     │
                 └────────────────────────────────────────────────────────── │ ────┘
                                                                              │ LAN (QUIC)
                                ┌──────────────┬──────────────────────────────┘
                                ▼              ▼
                          PCSceneJar     PCSceneCour
                        (kyclient FS)   (kyclient FS)
```

## Workspace

| Crate    | Package            | What it is                                                            |
|----------|--------------------|----------------------------------------------------------------------|
| `shared` | `kyberfrog-shared` | Data model (`Transmitter`, `Source`), paths, config generation.      |
| `server` | `kyberfrog-server` | Regie-side: reads `transmitters.toml`, spawns & supervises one `kycontroller` per transmitter, with a system-tray UI and a web dashboard. |
| `client` | `kyberfrog-client` | Scene-side: a web UI + supervisor managing N fullscreen `kyclient` viewers on a scene machine, relaunching them on exit. See [`client/README.md`](client/README.md). |

## How it works

The **Server** is the orchestrator. It owns a single source of truth,
`%APPDATA%\kyberfrog\transmitters.toml` (see [`examples/transmitters.toml`](examples/transmitters.toml)),
and for each `[[transmitter]]` it:

1. Generates a self-contained `%APPDATA%\kyberfrog\instances\<name>\kyber_config.toml`
   from your `[defaults]` plus the transmitter's `port` and `source`.
2. Spawns `kycontroller.exe` with `KYBER_CONFIG_PATH` pointing at that file and
   the working directory set to the Kyber install, so all instances share one
   set of binaries.
3. Supervises the process, restarting it with capped backoff if it exits.

A **Spout** source pins the kyavserver to a sender name (the client's requested
display is ignored). A **Screen** source is a plain desktop grabber.

New transmitters get the lowest free port at or above `base_port` (default
`8080`); set `base_port` in `transmitters.toml` to move the whole range when
`8080` clashes with something else. Ports already bound by another process are
skipped automatically.

This relies on two small upstream changes already landed on the `kyber-frog`
forks:

- `KYBER_CONFIG_PATH` env override (kycontroller + kyavservice) — lets N
  instances share one install.
- `spout_sender` pinning + the `iosys_spout` source in txproto.

## Status

- [x] `shared`: model + per-instance config generation (`render_config`).
- [x] `server`: supervisor (launch + auto-restart all transmitters, graceful
      shutdown) with a Windows system-tray UI (custom icon, emoji status, live
      Spout sender picker, add/remove/restart, open config/logs); falls back to
      headless elsewhere. Logs to the terminal and to `%APPDATA%\kyberfrog\logs`.
- [x] End-to-end validated: two transmitters (Spout + screen) reachable from
      `kyclient` over LAN.
- [x] `client`: web UI managing N kyclient viewers (add/start/stop/restart,
      machine info, live logs), persisted + autostarted on boot
      (`http://<scene-pc>:7701/`); logon-task installer. Field test pending.
- [x] Server web UI + `GET /transmitters` discovery endpoint (read-only
      dashboard with live status; browse `http://<regie>:7700/`).
- [ ] Server-side runtime control over HTTP (add / remove / restart). On hold —
      see [`IMPROVEMENTS.md`](IMPROVEMENTS.md).

## Prerequisites

KyberFrog drives the **kyber-frog** fork of Kyber: it spawns and supervises the
Kyber binaries for you, so those binaries must be present and reachable on
**PATH** on every machine. This is the *only* prerequisite — do it once per
machine and you never touch it again.

Which binaries each role needs:

| Machine | Role | Needs |
|---|---|---|
| regie | runs `kyberfrog-server` | `kycontroller.exe`, `kyavserver.exe` |
| scene | runs `kyberfrog-client` | `kyclient.exe` |

### Step by step (regie *and* scene)

1. **Download** the latest Windows x64 build of the kyber-frog fork from its
   releases page:
   👉 https://gitlab.com/kyber-frog/kyber/-/releases
   (grab the `kyber-frog-win64.zip` asset of the newest release).

2. **Extract** it to a permanent folder, e.g. `C:\Program Files\kyber\` or
   `D:\soft\kyber\`. All the `.exe` files (and their DLLs) must stay together in
   that folder.

3. **Add that folder to the system PATH** so it survives reboots. Open
   **PowerShell as Administrator** and run (adjust the path to where you
   extracted):
   ```powershell
   [Environment]::SetEnvironmentVariable(
       "PATH",
       [Environment]::GetEnvironmentVariable("PATH", "Machine") + ";C:\Program Files\kyber",
       "Machine"
   )
   ```

4. **Verify** in a brand-new terminal (PATH changes only apply to terminals
   opened *after* step 3):
   ```powershell
   kycontroller --version   # on a regie machine
   kyclient --version       # on a scene machine
   ```
   If you see a version number, you're done. If you get
   *"is not recognized…"*, the folder isn't on PATH yet — recheck step 3 and
   open a fresh terminal.

## Installation

No build required — grab the prebuilt KyberFrog executables from the package
registry. Each release is published automatically by GitLab CI.

1. **Download** the executable for the machine's role from the KyberFrog
   releases page:
   👉 https://gitlab.com/kyber-frog/kyberfrog/-/releases

   | Machine | Download |
   |---|---|
   | regie | `kyberfrog-server.exe` |
   | scene | `kyberfrog-client.exe` |

2. Put it wherever you like (e.g. `C:\Program Files\KyberFrog\`). The app icon
   is baked into the exe — nothing else to copy.

3. **Run it** — double-click, or from a terminal:
   ```powershell
   .\kyberfrog-server.exe   # regie
   .\kyberfrog-client.exe   # scene
   ```
   On first launch the server writes a default config under `%APPDATA%\kyberfrog\`
   and shows a system-tray icon (see [Run](#run) below).

> Make sure the [Prerequisites](#prerequisites) are done first, otherwise the
> app launches but can't start any Kyber process.

> **Future:** a single-click Windows installer that bundles KyberFrog *and* the
> Kyber fork binaries (so even the PATH step disappears) is planned — see
> [`IMPROVEMENTS.md`](IMPROVEMENTS.md) item 6.

## Build (from source)

For development only — end users should use [Installation](#installation) above.
There's no native Rust toolchain on the dev env, so cross-compile to Windows via
the same mingw Docker image used for the rest of Kyber:

```sh
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo build --release --target x86_64-pc-windows-gnu
```

This produces `kyberfrog-server.exe` (regie) and `kyberfrog-client.exe` (scene)
under `target/x86_64-pc-windows-gnu/release/`.

## Run

```powershell
# First run writes a default %APPDATA%\kyberfrog\transmitters.toml
.\kyberfrog-server.exe
```

Edit the generated file (or start from `examples/transmitters.toml`), then run
again. On Windows a system-tray icon lets you add/remove/restart transmitters
live and open the config or log file; elsewhere it runs headless. The tray icon
([`server/assets/kyberfrog.ico`](server/assets/kyberfrog.ico), the Collecti'Frog
logo) is embedded in the exe at build time; dropping a `kyberfrog.ico` next to
`kyberfrog-server.exe` overrides it. Ctrl-C stops every transmitter cleanly.

A web dashboard is served on `web_port` (default `7700`): browse
`http://<regie-ip>:7700/` to see every transmitter and its live status, with a
ready-to-copy client command. `GET /transmitters` returns the same list as JSON
for discovery by scene clients and tooling.
