# KyberFrog 🐸

[![Licence](https://img.shields.io/badge/licence-AGPL--3.0-blue.svg)](LICENSE)
[![Latest release](https://gitlab.com/kyber-frog/kyberfrog/-/badges/release.svg)](https://gitlab.com/kyber-frog/kyberfrog/-/releases)
[![Pipeline](https://gitlab.com/kyber-frog/kyberfrog/badges/main/pipeline.svg)](https://gitlab.com/kyber-frog/kyberfrog/-/pipelines)
[![Docs](https://img.shields.io/badge/docs-online-brightgreen.svg)](https://kyber-frog.gitlab.io/kyberfrog/)
[![Platform](https://img.shields.io/badge/platform-Windows-0078D6.svg)](#)

> **KyberFrog.exe lets you create transmitters and clients — from its web UI on
> `:7700` — to send Spout sources between Windows PCs with very low latency.**

A self-hosted, drop-in alternative to **NDI** for the LAN, built on
[Kyber](https://kyber.stream)'s QUIC video transport. **One app on every
machine**: whether a box *emits*, *receives*, or *both* is set by the config and
the web UI — there is no separate server and client build.

📖 **[Documentation](https://kyber-frog.gitlab.io/kyberfrog/)** · 📦 **[Download](https://gitlab.com/kyber-frog/kyberfrog/-/releases)** · 🐸 Made for VJs (Resolume → Spout → LAN → displays)

<!-- TODO: drop a screenshot of the web dashboard here once captured:
     ![KyberFrog dashboard](docs/assets/dashboard.png) -->

## Features

- 🎥 **Any source → N transmitters** — Spout (Windows GPU texture share) and
  screen capture today; the model grows more input types without touching the
  orchestration.
- 🔌 **One binary, any role** — emit, receive, or both, decided by the config and
  the web UI. No "server vs client" builds.
- 🌐 **Web UI + system tray on `:7700`** — add and manage transmitters and
  viewers, watch live status, tail every child's logs.
- 🛰️ **Low-latency QUIC transport** — Kyber over the LAN, a drop-in NDI replacement.
- 🖥️ **Flexible viewers** — fullscreen displays, a windowless **Spout-out relay**
  (re-publish to Resolume/MadMapper), and a **remote-control** viewer (keyboard +
  mouse takeover over QUIC).
- 📦 **Single-click installer** — bundles the Kyber fork binaries; no separate
  Kyber install, no manual PATH.
- 🛟 **Supervised, no orphans** — one Job Object terminates every child if
  KyberFrog exits; children auto-restart with capped backoff.
- 🆓 **AGPL-3.0**, self-hosted, no cloud.

## Quickstart

1. Download `KyberFrog-Setup.exe` from the **[Releases page](https://gitlab.com/kyber-frog/kyberfrog/-/releases)**.
2. **Double-click** it (needs admin: Program Files + PATH) and finish the wizard.
   It launches to a **system-tray icon**.
3. Open **<http://localhost:7700/>** → add a transmitter (regie PC) and/or a
   viewer (display PC).

> **Exit a fullscreen viewer:** there is no quit shortcut by design — press
> **Ctrl+Alt+F** to drop to a window and release the keyboard.

Full guide: **[Installation](https://kyber-frog.gitlab.io/kyberfrog/user/installation/)** ·
**[Getting started](https://kyber-frog.gitlab.io/kyberfrog/user/getting-started/)**.

## How it works

```
            ┌──────────── Regie PC (KyberFrog) ───────────┐
  Resolume ─Spout A─▶  emission ─▶ kycontroller :9000 ─┐   │
  Resolume ─Spout B─▶           ─▶ kycontroller :9001 ─┤   │
            └──────────────────────────────────────────│───┘
                                                        │ LAN (QUIC)
                          ┌─────────────────────────────┘
                          ▼                     ▼
              Display A (KyberFrog)   Display B (KyberFrog)
                reception → kyclient    reception → kyclient
                 fullscreen viewers      fullscreen viewers
```

KyberFrog owns one `kyberfrog.toml` per machine. For each **transmitter** it
generates a config and supervises a `kycontroller` serving one source over QUIC;
for each **viewer** it supervises a `kyclient` connected to a remote transmitter.
It orchestrates the Kyber fork binaries — it does not reimplement Kyber.

## Build & contribute

No native Rust toolchain on the dev host — everything cross-compiles to Windows
through the MinGW Docker image (use **PowerShell**, not git-bash, to mount):

```sh
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu
```

See the developer docs for the rest:
**[Architecture](https://kyber-frog.gitlab.io/kyberfrog/dev/architecture/)** ·
**[Building from source](https://kyber-frog.gitlab.io/kyberfrog/dev/building/)** ·
**[Releasing & CI](https://kyber-frog.gitlab.io/kyberfrog/dev/releasing/)** ·
**[Contributing](https://kyber-frog.gitlab.io/kyberfrog/dev/contributing/)**.

Backlog & tech debt live in [`IMPROVEMENTS.md`](IMPROVEMENTS.md); the working
plan in [`TODO.md`](TODO.md).

## Licence

[AGPL-3.0](LICENSE) · © Tristan Perrault · Source on
[GitLab](https://gitlab.com/kyber-frog/kyberfrog).
