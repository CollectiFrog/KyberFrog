# Backlog archive

Shipped items, one line each. They live here so that `#N` referenced from a
commit message, a merge request or `CLAUDE.md` always resolves to something —
numbers are never renumbered and never reused.

The release-facing story of each version is in
[`CHANGELOG.md`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/CHANGELOG.md);
what is still open is on the [backlog](backlog.md).

Where the CHANGELOG names the item explicitly, the version is given. The
foundation items shipped across the 0.1.0 → 0.3.0 series, which predates the
practice of citing numbers in the changelog.

| ID | What shipped | Version |
|---|---|---|
| #4 | Tray icon embedded in the exe (`winresource`/`windres`, resource ID 1, overridable by a neighbouring file) | 0.1.0 → 0.3.0 |
| #5 | Runtime control over HTTP and from the tray — add/remove/restart transmitters, create/edit/start/stop/remove viewers, persisted to `kyberfrog.toml` through the shared `op_*` functions | 0.1.0 → 0.3.0 |
| #6 | Single-click Windows installer (NSIS): Program Files, PATH > 1024, shortcuts, autostart task, uninstaller, silent `/S` install | 0.1.0 → 0.3.0 |
| #7 | GitLab CI/CD — `build-fork` → `installer` → `release` on a `v*` tag, with the fork bundle cached by SHA in the Generic Package Registry | 0.1.0 → 0.3.0 |
| #9 | Clean release package — a single `KyberFrog-Setup.exe` | 0.1.0 → 0.3.0 |
| #10 | Remote-control viewer — taking over the remote desktop from a viewer | 0.2.0 |
| #11 | User manual — MkDocs site on GitLab Pages *(announced as EN+FR; the FR half is still missing, tracked as #44)* | 0.2.0 |
| #12 | Developer documentation — the `docs/dev/` section of the same site | 0.2.0 |
| #13 | Web cockpit restructure — React + Vite front end, Collecti'Frog design | 0.2.0 → 0.3.0 |
| #14 | Unit tests in GitLab CI — the `test` job *(writing more of them is #38)* | 0.2.0 |
| #15 | Configuration import / export — `GET /setups/export` and friends | 0.3.0 → 0.4.0 |
| #16 | ~~kyclient right-click context menu~~ — **cancelled**, incompatible | — |
| #8 | Spout output from a viewer — fork feature (`kyspout` crate, smem in `vlc-rs`, `kyvlcplayer`) plus the KyberFrog wiring, validated E2E against Resolume Arena. Later refined to negotiate the **native size** instead of forcing 1920×1080. **Known limitation, accepted:** the transmitter freezes and needs a manual restart when the captured screen changes resolution mid-stream | 0.4.0 |
| #17-P1 | Remote desktop phase 1 — separate X/Y scales in `local_to_host`, fractional delta accumulator, `VideoLayout` unit tests. Validated E2E landscape → landscape. *Phases 2–3 are still open* | 0.4.0 |
| #18-A | Windows webcam (DirectShow) — `Source::Camera` with a device picker, CRC pinning. Validated E2E on hardware; 7 latent bugs fixed in txproto's lavd path along the way | 0.4.0 |
| #18-B | Source screen selection — `display_idx` plus a picker fed by `GET /displays`, with manual entry as a fallback | 0.4.0 |
| #19 | Per-transmitter source scoping, plus the global *Send all* mode. *Two scenarios remain to be exercised — see the [validation queue](backlog.md#validation-queue-no-code-just-a-run)* | 0.4.0 |
| #20 | mDNS auto-discovery of transmitters — `_kyber._tcp.local.`, *Detected emitters* in the viewer form, `mdns = false` opt-out, UDP 5353 firewall rule in the installer. *Two-machine validation still pending — see the [validation queue](backlog.md#validation-queue-no-code-just-a-run)* | 0.4.0 |
| #21 | Native Windows application (Tauri/WebView2) — native window on the same embedded server, close-hides, tray left-click opens the dashboard, no console anywhere, WebView2 bootstrapped by the installer. Architecture and gotchas: [plan](plan-tauri-shell.md) | 0.5.0 |
| #22 | Cockpit polish — vertical responsive layout, header rework, Options modal, red delete button. *The global hover pass is still open* | 0.5.0 |
| #28-2 | Zero-copy D3D11 on reception — libVLC renders straight into the shared Spout texture, GPU→GPU, and it is the default on Windows. Detail: [plan](plan-spout-zerocopy.md) | 2026-07-18 |
| — | **Linux amd64** — a standalone `.deb` built and released next to the Windows installer, validated end to end on a Debian 13 / Xfce VM. Detail: [plan](plan-linux-amd64.md) · [status](todo-linux.md) | unreleased |
