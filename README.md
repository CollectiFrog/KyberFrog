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
transmitters get the lowest free port at or above `base_port` (default `9000`).

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

## Installation

Grab `KyberFrog-Setup.exe` from the releases page (published automatically by
GitLab CI on every `v*` tag):
👉 https://gitlab.com/kyber-frog/kyberfrog/-/releases

It's a **single self-contained installer**: it bundles `kyberfrog.exe` **and**
the Kyber fork binaries it drives (`kycontroller`, `kyavserver`, `kyclient` +
their DLLs and the libVLC `plugins\`). There is **no separate Kyber install and
no manual PATH step** — that was the old way.

1. **Double-click** `KyberFrog-Setup.exe` (it needs admin: Program Files + PATH).
2. Accept the licence, pick the folder (default `C:\Program Files\KyberFrog`),
   and on the Options page optionally tick *Launch at logon* (see
   [Autostart](#autostart-at-logon)).
3. Finish — KyberFrog launches and shows a system-tray icon; on first run it
   writes a default `%APPDATA%\kyberfrog\kyberfrog.toml`.
4. Open `http://localhost:7700/` and add transmitters and/or viewers.

The installer adds its folder to the machine **PATH**, so `kyclient` /
`kycontroller` resolve in any new terminal, and registers an uninstaller (*Apps &
features* → KyberFrog). Silent install: `KyberFrog-Setup.exe /S [/AUTOSTART=1]`.
Full notes: [`packaging/windows/INSTALL.md`](packaging/windows/INSTALL.md).

## Autostart at logon

For a hands-off machine (especially a display PC), tick *Launch at logon* on the
installer's Options page. To (re)register it later, run the bundled script from
the install folder, in the session of the auto-login user:

```powershell
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
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

### Build the installer

`packaging/build-installer.sh` does the whole release locally — builds
`kyberfrog.exe`, stages it with the fork binaries bundle, and runs `makensis`
(both `cargo` and `makensis` live in the image). **Mount the workspace root**
(not just `apps/KyberFrog`) so the sibling fork bundle is reachable:

```sh
# from the workspace root (the dir that contains apps/, core/, …)
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  bash apps/KyberFrog/packaging/build-installer.sh
```

Out comes `apps/KyberFrog/dist/KyberFrog-Setup-<version>.exe`. By default it
reuses the prebuilt fork bundle at `apps/kyber-desktop/kyberfrog-spout-e2e[.zip]`
(build it with `apps/kyber-desktop/build-win32.sh -p`); pass `-f <dir|zip>` to
point elsewhere, `-v <version>` to set the version, `-s` to skip the cargo build.
On a `v*` tag, [`.gitlab-ci.yml`](.gitlab-ci.yml) runs the same script in CI and
publishes the setup to the releases page.

### Versioning & releasing

The version is **one source of truth**: `version` under `[workspace.package]` in
[`Cargo.toml`](Cargo.toml). A `v<version>` git tag cuts a release. Local dev
builds without an exact tag are named `<cargo-version>-<short-sha>`.

To cut a release:

1. Bump `version` in `Cargo.toml` (e.g. `0.1.0` → `0.2.0`), commit.
2. Tag it **matching the Cargo version** and push the tag:
   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```
3. CI builds and attaches `KyberFrog-Setup-v0.2.0.exe` to the
   [GitLab Release](https://gitlab.com/kyber-frog/kyberfrog/-/releases).

The CI `installer` job **fails fast if the tag ≠ the Cargo version**, so the two
can't drift. Follow [SemVer](https://semver.org): bump patch for fixes, minor for
features, major for breaking config/CLI changes.
