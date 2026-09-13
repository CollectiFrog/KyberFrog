# Installation

KyberFrog ships as **one self-contained package per platform**: a Windows
installer, `KyberFrog-Setup.exe`, and a Debian/Ubuntu package,
`kyberfrog_<version>_amd64.deb`. Both bundle `kyberfrog` **and** the Kyber fork
binaries it drives (`kycontroller`, `kyavserver`, `kyclient` + their libraries
and the libVLC plugins). There is **no separate Kyber install and no manual PATH
step**.

## Download

Grab the artefact for your platform from the releases page (published
automatically by GitLab CI on every `v*` tag):

[:octicons-download-24: Releases](https://gitlab.com/kyber-frog/kyberfrog/-/releases){ .md-button .md-button--primary }

---

## Windows

### Interactive install

1. **Double-click** `KyberFrog-Setup.exe`. It needs administrator rights
   (Program Files + machine PATH).
2. Accept the licence (AGPL-3.0).
3. Choose the install folder (default `C:\Program Files\KyberFrog`).
4. **Options:**
    - *Launch KyberFrog at logon* — registers the autostart task; recommended
      on a dedicated display PC, leave off on a regie/laptop you start by hand.
    - *Launch KyberFrog when the installer finishes*.
5. Finish. KyberFrog launches with its **dashboard window** and a
   **system-tray icon**; on first run it writes a default
   `%APPDATA%\kyberfrog\kyberfrog.toml`.
6. Add transmitters and/or viewers from the dashboard
   (see [Getting started](getting-started.md)). The same UI is also reachable
   from any browser at <http://localhost:7700/> (or the machine's LAN IP).

The installer adds its folder to the machine **PATH**, so `kyclient` /
`kycontroller` resolve in any new terminal, and registers an uninstaller
(*Apps & features* → KyberFrog).

### Silent install

```bat
KyberFrog-Setup.exe /S                 :: silent, no autostart task
KyberFrog-Setup.exe /S /AUTOSTART=1    :: silent + register the logon task
KyberFrog-Setup.exe /S /D=C:\KyberFrog :: custom dir (/D must be last, unquoted)
```

`/S` requires administrator rights.

### Uninstall

*Apps & features* → **KyberFrog**, or run `uninstall.exe` in the install folder.
The uninstaller stops KyberFrog and its children, removes the autostart task and
the PATH entry, and asks whether to keep your data — both `%APPDATA%\kyberfrog`
(config, logs, per-instance dirs) and `%LOCALAPPDATA%\kyber` (kyclient
`known_hosts` + logs). Silent: `uninstall.exe /S` (keeps data;
`/KEEPCONFIG=0` to wipe both).

### Autostart at logon (hands-off display PC)

For a dedicated display PC, tick *Launch at logon* during install. The logon
task launches KyberFrog at every logon and relaunches it if it ever dies
(KyberFrog keeps its children alive). To (re)register or remove it later, run
the bundled script in the session of the auto-login user:

```powershell
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -Uninstall   # remove it
```

#### Autologon (manual, per-site)

!!! warning "Security trade-off"
    A truly hands-off display PC must reach the interactive desktop **without
    someone typing a password**. This is **not** scripted because it stores a
    credential. Two common ways:

    - **Sysinternals Autologon** (recommended): stores the password LSA-encrypted.
    - `netplwiz` → untick *"Users must enter a user name and password"*.

Pair autologon with the logon task and the PC boots straight into the streams.

KyberFrog opens no console. Its dashboard window sits **behind** any fullscreen
viewers; closing it only hides it (the streams and the tray keep running), and
a **left click on the tray icon** brings it back.

---

## Linux (Debian / Ubuntu, amd64)

!!! info "What is supported"
    - **Architecture:** amd64 only (arm64 is not built yet).
    - **glibc ≥ 2.39**, inherited from the Debian 13 build image and only
      forward-compatible — so **Debian 13 (Trixie)** and **Ubuntu 24.04 LTS** or
      newer. Debian 12 (glibc 2.36) and Ubuntu 22.04 (2.35) will not run it.
    - **Session: X11.** Screen capture is validated on X11 (`xcb`). Wayland
      capture goes through the *wlroots screencopy* protocol, which sway,
      Hyprland and river implement — but **GNOME and KDE do not** (they expose
      xdg-desktop-portal/PipeWire instead), and GNOME/Wayland is the default
      session on both Debian and Ubuntu.
    - Spout, the system tray and the native window are Windows-only; on Linux
      the dashboard opens in your browser and the Spout tiles are hidden.

### Install

```sh
sudo apt install ./kyberfrog_<version>_amd64.deb
```

Use `apt`, not `dpkg -i`: the package declares its ~70 system dependencies
(GL/EGL, VA-API, ALSA, xcb, libinput…) and `apt` resolves them. With `dpkg -i`
you must follow up with `sudo apt -f install`.

What lands where:

| Path | Contents |
|---|---|
| `/usr/lib/kyberfrog/bin` | `kyberfrog` + `kycontroller` / `kyavserver` / `kyclient`, and the web UI next to them |
| `/usr/lib/kyberfrog/lib` | the bundled Kyber/FFmpeg/VLC libraries (registered through `/etc/ld.so.conf.d/kyberfrog.conf`) |
| `/usr/bin/kyberfrog` | symlink, so `kyberfrog` resolves in any shell |
| `/usr/lib/systemd/user/kyberfrog.service` | the autostart unit, enabled for every user |
| `/usr/lib/udev/rules.d/99-kyberfrog-uinput.rules` | `/dev/uinput` access for remote control |

### First run

The package installs a systemd **user** service, enabled for all users, so
KyberFrog starts on its own at the next graphical login. Right after the
install you can start it without logging out:

```sh
systemctl --user start kyberfrog
systemctl --user status kyberfrog
journalctl --user -u kyberfrog -f     # live logs
```

Then open <http://localhost:7700/> (or the machine's LAN IP from another
machine) and add transmitters and viewers — see
[Getting started](getting-started.md).

It is a *user* service, not a system one, on purpose: a viewer's `kyclient`
must run inside the graphical session to open its fullscreen window, and a
screen transmitter needs that session's display too.

### Remote control needs the `input` group

Taking over this machine from a remote viewer injects clicks and keystrokes
through `/dev/uinput`, which Debian ships as `root:root 0600`. The package adds
a udev rule granting the **`input`** group write access, but your user still has
to be in that group:

```sh
sudo /usr/sbin/usermod -aG input $USER    # then log out and back in
```

!!! tip "The symptom when it is missing"
    The remote pointer **moves** but never clicks or types, and nothing is
    logged. Pointer movement goes through X11/XCB and works without any
    permission; buttons and keyboard go through `/dev/uinput` and fail
    silently.

### Screen-capture backend

KyberFrog detects the capture API from the session it starts in and writes it
into `kyberfrog.toml` as `screen_backend`. To force it, edit that key and
restart the service:

| Value | Use it for |
|---|---|
| `xcb` | X11 sessions — the validated default. SHM capture, no KMS access needed. |
| `drm` | DRM/KMS scanout — works with no display server at all, for a headless box. |
| `wlroots` | Wayland, **wlroots compositors only** (sway, Hyprland, river). |
| `nvfbc` | NVIDIA Framebuffer Capture, fastest where it exists. |

This is a **per-machine** setting: it describes the session this box runs in, so
it never travels inside a saved setup.

### Where your data lives

Standard XDG locations, never touched by install, upgrade or removal:

- `~/.config/kyberfrog/` — `kyberfrog.toml` and the `setups/` you save
  (`$XDG_CONFIG_HOME` if you set it);
- `~/.local/state/kyberfrog/` — logs and per-instance directories
  (`$XDG_STATE_HOME` if you set it).

### Headless display box

For a box with no keyboard that must come up streaming, give it an **autologin
into a graphical seat** — the user's systemd instance (and therefore the
service) only starts at login. Capture can then use `drm`, which needs no
display server at all.

!!! warning "No sound server, no transmitter"
    The fork always adds `pulse` to the capture API list, and `libpulse`
    **aborts** when no PulseAudio/PipeWire server is running, taking
    `kyavserver` with it. A silent headless box still needs a running sound
    server today. Tracked in the [Linux port
    dashboard](../dev/todo-linux.md).

### Upgrade and removal

```sh
sudo apt install ./kyberfrog_<newer>_amd64.deb   # in-place upgrade
sudo apt remove kyberfrog                        # remove the package
sudo apt purge kyberfrog                         # + drop /etc/ld.so.conf.d entry
```

Neither touches your config, setups or logs under `$HOME`.

### Known limits

- **A stale `/tmp/kyber` blocks startup.** The Kyber IPC directory is hard-coded
  and shared by all users, with no per-user component: whoever starts first owns
  it, and a leftover owned by `root` leaves every normal user with
  `IPC couldn't bind address /tmp/kyber/0`. KyberFrog detects the case and says
  so; the fix is `sudo rm -rf /tmp/kyber` with the app stopped. The real
  correction belongs upstream, in Kyber.
- **No tray icon, no native window.** On Linux the app runs headless and the
  dashboard opens in your browser; the tray and the Tauri window are Windows
  features for now.
- **Camera sources are not enumerated yet** — the screen and viewer paths are
  the validated v1.
