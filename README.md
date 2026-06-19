# KyberFrog 🐸

> **KyberFrog.exe lets you create transmitters and clients — from its web UI on
> `:7700` — to send Spout sources between Windows PCs with very low latency.**

A polyvalent orchestration layer on top of [Kyber](https://kyber.stream):
publish **any source** as one of **N independent transmitters** and supervise
the viewers, for low-latency, source-agnostic streaming over LAN — a drop-in
replacement for NDI.

KyberFrog is **one app, installed on every machine**. There is no separate
"server" and "client" build: the role — **emit**, **receive**, or **both** — is
set entirely by the config and the web UI.

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

The motivating setup (VJing): **Resolume Arena** on the regie machine publishes
several **Spout** outputs; each is streamed over LAN (QUIC) to display machines
running `kyclient` fullscreen. Today's sources are **Spout** (Windows GPU texture
share) and **screen capture**; the model is designed to grow more input types
without touching the orchestration.

📖 **Full documentation:** <https://kyber-frog.gitlab.io/kyberfrog/>

---

## For users

KyberFrog ships as a **single self-contained Windows installer**. It bundles
`kyberfrog.exe` **and** the Kyber fork binaries it drives (`kycontroller`,
`kyavserver`, `kyclient` + DLLs and the libVLC `plugins\`) — **no separate Kyber
install, no manual PATH step**.

1. Download `KyberFrog-Setup.exe` from the **[Releases page](https://gitlab.com/kyber-frog/kyberfrog/-/releases)**
   (published automatically by CI on every `v*` tag).
2. **Double-click** it (needs admin: Program Files + PATH), accept the licence,
   optionally tick *Launch at logon* on a dedicated display PC.
3. KyberFrog launches with a **system-tray icon**; open
   <http://localhost:7700/> and add transmitters and/or viewers.

> Exiting a fullscreen viewer: a passive display has no quit shortcut by design;
> the escape hatch is **Ctrl+Alt+F** (drops to windowed, releases the keyboard
> grab).

**Read the manual:**

- [Installation](https://kyber-frog.gitlab.io/kyberfrog/user/installation/) — install, silent install, uninstall, autostart/autologon.
- [Getting started](https://kyber-frog.gitlab.io/kyberfrog/user/getting-started/) — first two-machine setup, the web UI, the tray.
- [Troubleshooting](https://kyber-frog.gitlab.io/kyberfrog/user/troubleshooting/) · [FAQ](https://kyber-frog.gitlab.io/kyberfrog/user/faq/)

Install notes also live in [`packaging/windows/INSTALL.md`](packaging/windows/INSTALL.md).

---

## For developers

KyberFrog **orchestrates** pre-built Kyber binaries — it does not reimplement
Kyber. It generates per-instance configs and supervises the fork's processes
under one supervisor, one web UI (`:7700`), one tray, one `kyberfrog.toml`.

| Crate       | Package            | What it is                                                                |
|-------------|--------------------|---------------------------------------------------------------------------|
| `shared`    | `kyberfrog-shared` | Data model (`Config`, `Transmitter`, `Viewer`, `Source`), config generation, paths. No Win32 — tests on Linux. |
| `kyberfrog` | `kyberfrog`        | The single binary: one supervisor for both roles, a system-tray UI and a web dashboard on one port. |

There is **no native Rust toolchain on the dev host** — everything
cross-compiles to Windows via the MinGW `kyber/debian-win64:local` Docker image.
On Windows, mount with **PowerShell**, not git-bash.

```sh
# Fast inner loop (Linux host target; Win32 modules → stubs):
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test

# The single exe → target/x86_64-pc-windows-gnu/release/kyberfrog.exe
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu
```

**Dive into the dev docs:**

- [Architecture](https://kyber-frog.gitlab.io/kyberfrog/dev/architecture/) — crates, the one-config/two-halves model, the supervisor + Job Object, Win32 patterns, gotchas.
- [Building from source](https://kyber-frog.gitlab.io/kyberfrog/dev/building/) — the Docker workflow, the installer, and the cross-repo **fork build model**.
- [Releasing & CI](https://kyber-frog.gitlab.io/kyberfrog/dev/releasing/) — versioning, tags, the `test → build-fork → installer → release` + `pages` pipeline.
- [Contributing](https://kyber-frog.gitlab.io/kyberfrog/dev/contributing/) — workflow, where to put tests, conventions.

Backlog & tech debt: [`IMPROVEMENTS.md`](IMPROVEMENTS.md). Working plan:
[`TODO.md`](TODO.md). The docs site is built from `docs/` by the `pages` CI job.

---

**Licence:** AGPL-3.0 · **Repo:** [gitlab.com/kyber-frog/kyberfrog](https://gitlab.com/kyber-frog/kyberfrog)
