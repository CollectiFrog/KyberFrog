# Backlog archive

Shipped items, one line each. They live here so that `#N` referenced from a
commit message, a merge request or `CLAUDE.md` always resolves to something —
numbers are never renumbered and never reused.

The release-facing story of each version is in
[`CHANGELOG.md`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/CHANGELOG.md);
what is still open is on the [backlog](backlog.md).

Where the CHANGELOG names the item explicitly, the version is given; the
foundation items shipped across the 0.1.0 → 0.3.0 series.

| ID | What shipped | Version |
|---|---|---|
| #4 | Tray icon embedded in the exe (`winresource`/`windres`, resource ID 1, overridable by a neighbouring file) | 0.1.0 → 0.3.0 |
| #5 | Runtime control over HTTP and from the tray — add/remove/restart transmitters, create/edit/start/stop/remove viewers, persisted to `kyberfrog.toml` through the shared `op_*` functions | 0.1.0 → 0.3.0 |
| #6 | Single-click Windows installer (NSIS): Program Files, PATH > 1024, shortcuts, autostart task, uninstaller, silent `/S` install | 0.1.0 → 0.3.0 |
| #7 | GitLab CI/CD — `build-fork` → `installer` → `release` on a `v*` tag, with the fork bundle cached by SHA in the Generic Package Registry | 0.1.0 → 0.3.0 |
| #9 | Clean release package — a single `KyberFrog-Setup.exe` | 0.1.0 → 0.3.0 |
| #10 | Remote-control viewer — taking over the remote desktop from a viewer | 0.2.0 |
| #11 | User manual — MkDocs site on GitLab Pages *(French version: #44)* | 0.2.0 |
| #12 | Developer documentation — the `docs/dev/` section of the same site | 0.2.0 |
| #13 | Web cockpit — React + Vite front end, Collecti'Frog design | 0.2.0 → 0.3.0 |
| #14 | Unit tests in GitLab CI — the `test` job *(more tests: #38)* | 0.2.0 |
| #15 | Configuration import / export — `GET /setups/export` and friends | 0.3.0 → 0.4.0 |
| #16 | *Number retired* — a kyclient right-click context menu, incompatible with kyclient | — |
| #8 | Spout output from a viewer — fork feature (`kyspout` crate, `vlc-rs`, `kyvlcplayer`) plus the KyberFrog wiring, at the stream's **native size**, validated E2E against Resolume Arena. **Known limitation:** the transmitter freezes and needs a manual restart when the captured screen changes resolution mid-stream | 0.4.0 |
| #17-P1 | Remote desktop mouse path — separate X/Y scales in `local_to_host`, fractional delta accumulator, `VideoLayout` unit tests. Validated E2E landscape → landscape. *Phases 2–3 are open* | 0.4.0 |
| #18-A | Windows webcam (DirectShow) — `Source::Camera` with a device picker, CRC pinning, on top of the txproto lavd fixes. Validated E2E on hardware | 0.4.0 |
| #18-B | Source screen selection — `display_idx` plus a picker fed by `GET /displays`, with manual entry as a fallback | 0.4.0 |
| #19 | Per-transmitter source scoping, plus the global *Send all* mode. Both scenarios (screen-only, *Send all*) validated on Windows on 2026-09-23 | 0.4.0 |
| #20 | mDNS auto-discovery of transmitters — `_kyber._tcp.local.`, *Detected emitters* in the viewer form, `mdns = false` opt-out, UDP 5353 firewall rule in the installer. Validated between two machines on 2026-09-23 | 0.4.0 |
| #24 | Fork chain organisation — Kyber's structure kept as is, with `fork-lint.sh`, `rebase-fork.sh` and the `/rebase-fork` skill; the 0.27.1 rebase went through it. Detail: [process](plans-fork-restructure.md) | 0.5.0 |
| #21 | Native Windows application (Tauri/WebView2) — native window on the same embedded server, close-hides, tray left-click opens the dashboard, no console anywhere, WebView2 bootstrapped by the installer. Architecture: [plan](plan-tauri-shell.md) | 0.5.0 |
| #22 | Cockpit polish — vertical responsive layout, header rework, Options modal, red delete button. *The global hover pass is open* | 0.5.0 |
| #28-2 | Zero-copy D3D11 on reception — libVLC renders straight into the shared Spout texture, GPU→GPU, the default on Windows. Architecture: [plan](plan-spout-zerocopy.md) | unreleased |
| #28-4 | Latency baseline — manual bench, 3.9 ms median Spout → Spout on the GPU encoder, 25.8 ms on x264, NDI 15-26 ms. Automated campaign out of scope. [Bench, results](bench-latency.md) | 0.6.0 |
| #43 | First real Linux pipeline — the tag pipeline of v0.6.0 went green and `release-deb` attached `kyberfrog_0.6.0_amd64.deb` to the release | 0.6.0 |
| #32 | V4L2 cameras on Linux — picker by card name (`ffmpeg -sources v4l2`), `EnumerateDisplays` honouring the pin, pin by node path (`/dev/…`, symlinks followed) and `camera_options` for the demuxer. Validated end to end on the Pi 5 + C790 on 2026-09-26: the HDMI source reaches a Windows viewer (throughput is #49) | unreleased |
| issue #1 | Webcam inside *Send all* — re-tested on 2026-09-23 with the 0.5.0 fork fix: the webcam comes through | 0.5.0 |
| #40 | No orphans on Linux — checked on 2026-09-23 on the Pi 5 (Trixie, 0.6.0 `.deb`): `kycontroller` and `kyavserver` are gone after a `SIGKILL` of `kyberfrog` both under systemd and launched by hand (`PR_SET_PDEATHSIG`), and after `systemctl --user stop` | 0.6.0 |
| — | **Linux amd64** — a standalone `.deb` built and released next to the Windows installer, validated end to end on a Debian 13 / Xfce VM. Architecture: [plan](plan-linux-amd64.md) · [status](todo-linux.md) | unreleased |
| #45 | Docs build image pinned — `squidfunk/mkdocs-material:9.7.6` in `pages` and `dev.sh`, plus a `docs-check` job that builds the site strictly in the MR whenever the docs or the CI change | unreleased |
