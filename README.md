# KyberFrog 🐸

A polyvalent orchestration layer on top of [Kyber](https://kyber.stream):
publish **any source** as one of **N independent transmitters** and supervise
the viewers, for low-latency, source-agnostic streaming over LAN (a drop-in
replacement for NDI).

KyberFrog is **one app, installed on every machine**. There is no separate
"server" and "client" build: the role — **emit**, **receive**, or **both** — is
set entirely by the config and the web UI. Today the supported sources are
**Spout** (Windows GPU texture share) and **screen capture**; the model is
designed to grow more input types (video files, NDI, …) without touching the
orchestration.

The motivating setup (VJing): **Resolume Arena** on the regie machine publishes
several **Spout** outputs; each is streamed over LAN (QUIC) to display machines
running `kyclient` fullscreen.

```
            ┌──────────── Regie PC (KyberFrog) ───────────┐
  Resolume ─Spout A─▶  emission ─▶ kycontroller :8080 ─┐   │
  Resolume ─Spout B─▶           ─▶ kycontroller :8081 ─┤   │
            └──────────────────────────────────────────│───┘
                                                        │ LAN (QUIC)
                          ┌─────────────────────────────┘
                          ▼                     ▼
              Display A (KyberFrog)   Display B (KyberFrog)
                reception → kyclient    reception → kyclient
                 fullscreen viewers      fullscreen viewers
```

Every machine runs the same `kyberfrog.exe`; the regie one has transmitters
configured, the display ones have viewers. A machine can do both at once.

## Workspace

| Crate       | Package            | What it is                                                                |
|-------------|--------------------|---------------------------------------------------------------------------|
| `shared`    | `kyberfrog-shared` | Data model (`Config`, `Transmitter`, `Viewer`, `Source`), config generation, paths. No Win32 — tests on Linux. |
| `kyberfrog` | `kyberfrog`        | The single binary: one supervisor for both roles, a system-tray UI and a web dashboard on one port. |

## How it works

KyberFrog owns one source of truth per machine,
`%APPDATA%\kyberfrog\kyberfrog.toml` (see
[`examples/kyberfrog.toml`](examples/kyberfrog.toml)), with two halves:

**Émission** — for each `[[emission.transmitter]]` it:
1. Generates a self-contained `%APPDATA%\kyberfrog\instances\<name>\kyber_config.toml`
   from your `[emission.defaults]` plus the transmitter's `port` and `source`.
2. Spawns `kycontroller.exe` with `KYBER_CONFIG_PATH` pointing at that file and
   the working directory set to the Kyber install, so all instances share one
   set of binaries.
3. Supervises the process, restarting it with capped backoff if it exits.

A **Spout** source pins kyavserver to a sender name (the client's requested
display is ignored). A **Screen** source is a plain desktop grabber. New
transmitters get the lowest free port at or above `base_port` (default `8080`).

**Réception** — for each `[[reception.viewer]]` it spawns and supervises one
`kyclient` connected to a remote transmitter (`server` = the emitter's IP,
`port` = its transmitter port). `enabled` viewers relaunch on boot.

Both halves run under **one supervisor** and, on Windows, one **Job Object**: if
KyberFrog exits for any reason, every child it spawned (kycontroller *and*
kyclient) is terminated — no orphans.

This relies on small upstream changes already landed on the `kyber-frog` forks:
`KYBER_CONFIG_PATH` env override (N instances share one install), `spout_sender`
pinning + the `iosys_spout` source in txproto, and the `--fullscreen` flag on
kyclient.

## Web UI & tray

Browse `http://<this-pc>:7700/` (default `web_port`, bound on the LAN). One
dashboard, both halves:

- **Émission** — see each transmitter's live status; add a Spout source from a
  **live sender picker** or a screen capture, optionally choosing the port (it's
  auto-allocated otherwise); restart / remove.
- **Réception** — add a viewer (optional name, transmitter `IP:port`,
  fullscreen), Start / Stop / Restart, edit + Apply (hot relaunch, including
  **renaming** the viewer), remove.
- **Logs** — the app's own log plus each child's (`kycontroller` / `kyclient`).

`GET /transmitters` returns the transmitter list as JSON for discovery by other
machines and tooling.

The **system tray** mirrors the frequent actions (add Spout via the live picker,
start/stop/restart/remove on both halves) and opens the dashboard, the config
file, or the logs. **Advanced settings** (auth, encoder, install dir, base port,
input/audio/keyboard/TLS flags) are **file-only**: edit `kyberfrog.toml`
directly (tray → "Ouvrir config").

## Prerequisites

KyberFrog drives the **kyber-frog** fork of Kyber: it spawns and supervises the
Kyber binaries for you, so those binaries must be present and reachable on
**PATH** on every machine. This is the *only* prerequisite — do it once per
machine and you never touch it again.

A machine needs `kycontroller.exe` + `kyavserver.exe` to emit, and
`kyclient.exe` to receive; the fork ships them together, so installing the whole
fork covers both roles.

### Step by step

1. **Download** the latest Windows x64 build of the kyber-frog fork from its
   releases page:
   👉 https://gitlab.com/kyber-frog/kyber/-/releases
   (grab the `kyber-frog-win64.zip` asset of the newest release).

2. **Extract** it to a permanent folder, e.g. `C:\Program Files\kyber\`. All the
   `.exe` files (and their DLLs) must stay together in that folder.

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
   kycontroller --version
   kyclient --version
   ```
   If you see version numbers, you're done.

## Installation

No build required — grab the prebuilt `kyberfrog.exe` from the releases page
(published automatically by GitLab CI):
👉 https://gitlab.com/kyber-frog/kyberfrog/-/releases

1. Put it wherever you like, e.g. `C:\Program Files\KyberFrog\`. The app icon is
   baked into the exe — nothing else to copy.
2. **Run it** — double-click, or from a terminal:
   ```powershell
   .\kyberfrog.exe
   ```
   On first launch it writes a default `%APPDATA%\kyberfrog\kyberfrog.toml` and
   shows a system-tray icon.
3. Open `http://localhost:7700/` and add transmitters and/or viewers.

> Make sure the [Prerequisites](#prerequisites) are done first, otherwise the
> app launches but can't start any Kyber process.

## Autostart at logon

For a hands-off machine (especially a display PC) register the logon task, in
the session of the auto-login user:

```powershell
.\install\install-kyberfrog.ps1 -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
```

Task Scheduler launches KyberFrog at every logon and relaunches it if it ever
dies (KyberFrog keeps its children alive). Remove it later with `-Uninstall`.

KyberFrog is a console app; its window sits behind any fullscreen viewers and is
only visible if a viewer is dropped to a window.

### Autologon (manual, per-site)

For a truly hands-off display PC the machine must reach the interactive desktop
without someone typing a password. This is **not** scripted (it stores a
credential — a deliberate security trade-off). Two common ways:

- **Sysinternals Autologon** (recommended): stores the password LSA-encrypted.
- `netplwiz` → untick *"Users must enter a user name and password"*.

Pair autologon with the logon task and the PC boots straight into the streams.

> **Exiting a fullscreen viewer:** a passive display has no quit shortcut by
> design; the escape hatch is **Ctrl+Alt+F** (drops kyclient to windowed and
> releases the keyboard grab, giving Windows back).

## Build (from source)

For development only — end users use [Installation](#installation). There's no
native Rust toolchain on the dev env, so cross-compile to Windows via the same
mingw Docker image used for the rest of Kyber:

```sh
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu
```

This produces `target/x86_64-pc-windows-gnu/release/kyberfrog.exe`. Run the
tests with `cargo test` (they live in `shared/` and run on the Linux container
target). The tray icon
([`kyberfrog/assets/kyberfrog.ico`](kyberfrog/assets/kyberfrog.ico), the
Collecti'Frog logo) is embedded in the exe at build time; dropping a
`kyberfrog.ico` next to the exe overrides it.
