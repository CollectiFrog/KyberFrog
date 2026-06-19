# Installation

KyberFrog ships as a **single self-contained Windows installer**,
`KyberFrog-Setup.exe`. It bundles `kyberfrog.exe` **and** the Kyber fork
binaries it drives (`kycontroller`, `kyavserver`, `kyclient` + their DLLs and
the libVLC `plugins\`). There is **no separate Kyber install and no manual PATH
step**.

## Download

Grab the latest `KyberFrog-Setup.exe` from the releases page (published
automatically by GitLab CI on every `v*` tag):

[:octicons-download-24: Releases](https://gitlab.com/kyber-frog/kyberfrog/-/releases){ .md-button .md-button--primary }

## Interactive install

1. **Double-click** `KyberFrog-Setup.exe`. It needs administrator rights
   (Program Files + machine PATH).
2. Accept the licence (AGPL-3.0).
3. Choose the install folder (default `C:\Program Files\KyberFrog`).
4. **Options:**
    - *Launch KyberFrog at logon* — registers the autostart task; recommended
      on a dedicated display PC, leave off on a regie/laptop you start by hand.
    - *Launch KyberFrog when the installer finishes*.
5. Finish. KyberFrog launches and shows a **system-tray icon**; on first run it
   writes a default `%APPDATA%\kyberfrog\kyberfrog.toml`.
6. Open <http://localhost:7700/> and add transmitters and/or viewers
   (see [Getting started](getting-started.md)).

The installer adds its folder to the machine **PATH**, so `kyclient` /
`kycontroller` resolve in any new terminal, and registers an uninstaller
(*Apps & features* → KyberFrog).

## Silent install

```bat
KyberFrog-Setup.exe /S                 :: silent, no autostart task
KyberFrog-Setup.exe /S /AUTOSTART=1    :: silent + register the logon task
KyberFrog-Setup.exe /S /D=C:\KyberFrog :: custom dir (/D must be last, unquoted)
```

`/S` requires administrator rights.

## Uninstall

*Apps & features* → **KyberFrog**, or run `uninstall.exe` in the install folder.
The uninstaller stops KyberFrog and its children, removes the autostart task and
the PATH entry, and asks whether to keep your data — both `%APPDATA%\kyberfrog`
(config, logs, per-instance dirs) and `%LOCALAPPDATA%\kyber` (kyclient
`known_hosts` + logs). Silent: `uninstall.exe /S` (keeps data;
`/KEEPCONFIG=0` to wipe both).

## Autostart at logon (hands-off display PC)

For a dedicated display PC, tick *Launch at logon* during install. The logon
task launches KyberFrog at every logon and relaunches it if it ever dies
(KyberFrog keeps its children alive). To (re)register or remove it later, run
the bundled script in the session of the auto-login user:

```powershell
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -Uninstall   # remove it
```

### Autologon (manual, per-site)

!!! warning "Security trade-off"
    A truly hands-off display PC must reach the interactive desktop **without
    someone typing a password**. This is **not** scripted because it stores a
    credential. Two common ways:

    - **Sysinternals Autologon** (recommended): stores the password LSA-encrypted.
    - `netplwiz` → untick *"Users must enter a user name and password"*.

Pair autologon with the logon task and the PC boots straight into the streams.

KyberFrog is a console app; its window sits **behind** any fullscreen viewers
and is only visible if a viewer is dropped to a window.
