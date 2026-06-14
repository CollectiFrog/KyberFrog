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
| `client` | `kyberfrog-client` | Scene-side: keeps one fullscreen `kyclient` alive on a scene machine, relaunching it on exit. See [`client/README.md`](client/README.md). |

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

This relies on two small upstream changes already landed on the `kyberfrog`
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
- [x] `client`: config-driven kyclient supervisor + logon-task installer
      (field test pending).
- [x] Web UI + `GET /transmitters` discovery endpoint (read-only dashboard with
      live status; browse `http://<regie>:7700/`).
- [ ] Runtime control over HTTP (add / remove / restart from the browser).

## Build

No native Rust toolchain on the regie host: cross-compile to Windows via the
mingw Docker image used for the rest of Kyber.

```sh
docker run --rm -v "$PWD":/work -w /work kyber/debian-win64:local \
    cargo build --release --target x86_64-pc-windows-gnu
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
live and open the config or log file; elsewhere it runs headless. Drop a
`kyberfrog.ico` next to the exe to brand the tray icon. Ctrl-C stops every
transmitter cleanly.

A web dashboard is served on `web_port` (default `7700`): browse
`http://<regie-ip>:7700/` to see every transmitter and its live status, with a
ready-to-copy client command. `GET /transmitters` returns the same list as JSON
for discovery by scene clients and tooling.
