# kyber-anysource

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
  Resolume ──Spout A──▶ Director ──▶ kycontroller :8080 (pinned "Output A") ──┐     │
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

| Crate        | What it is                                                            |
|--------------|----------------------------------------------------------------------|
| `shared`     | Data model (`Transmitter`, `Source`), paths, config generation.      |
| `director`   | Server-side: reads `transmitters.toml`, spawns & supervises one `kycontroller` per transmitter. |

Planned: `scene-agent` (client-side autostart + kyclient supervisor) and a web
UI for remote reconfiguration.

## How it works

The Director is the **orchestrator**. It owns a single source of truth,
`%APPDATA%\kyber-anysource\transmitters.toml` (see [`examples/transmitters.toml`](examples/transmitters.toml)),
and for each `[[transmitter]]` it:

1. Generates a self-contained `%APPDATA%\kyber-anysource\instances\<name>\kyber_config.toml`
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

This relies on two small upstream changes already landed on the `kyberFAS`
forks:

- `KYBER_CONFIG_PATH` env override (kycontroller + kyavservice) — lets N
  instances share one install.
- `spout_sender` pinning + the `iosys_spout` source in txproto.

## Status

- [x] `shared`: model + per-instance config generation (`render_config`).
- [x] `director`: supervisor (launch + auto-restart all transmitters, graceful
      shutdown) with a Windows system-tray UI (live Spout sender picker,
      add/remove/restart transmitters); falls back to headless elsewhere.
- [x] End-to-end validated: two transmitters (Spout + screen) reachable from
      `kyclient` over LAN.
- [ ] `scene-agent`: client autostart + kyclient supervision.
- [ ] Web UI + `/transmitters` discovery endpoint.

## Build

No native Rust toolchain on the regie host: cross-compile to Windows via the
mingw Docker image used for the rest of Kyber.

```sh
docker run --rm -v "$PWD":/work -w /work kyber/debian-win64:local \
    cargo build --release --target x86_64-pc-windows-gnu
```

## Run (headless, current state)

```powershell
# First run writes a default %APPDATA%\kyber-anysource\transmitters.toml
.\kyber-anysource-director.exe
```

Edit the generated file (or start from `examples/transmitters.toml`), then run
again. Ctrl-C stops every transmitter cleanly.
