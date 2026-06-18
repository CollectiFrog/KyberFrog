# KyberFrog — install notes

`KyberFrog-Setup.exe` is a single self-contained installer. It bundles
`kyberfrog.exe` **and** the Kyber fork binaries it drives (`kycontroller`,
`kyavserver`, `kyclient` + their DLLs and the libVLC `plugins\`), so there is
**no separate Kyber install and no manual PATH step**.

## Interactive install

Double-click `KyberFrog-Setup.exe` and follow the wizard:

1. Accept the licence (AGPL-3.0).
2. Choose the install folder (default `C:\Program Files\KyberFrog`).
3. **Options:**
   - *Launch KyberFrog at logon* — registers the autostart task; recommended on
     a dedicated display PC, leave off on a regie/laptop you start by hand.
   - *Launch KyberFrog when the installer finishes*.
4. Finish — optionally open the dashboard at <http://localhost:7700/>.

The installer adds the install folder to the **machine PATH**, so `kyclient` and
`kycontroller` resolve in any new terminal. Settings live in
`%APPDATA%\kyberfrog\kyberfrog.toml` (created on first launch).

## Silent install

```bat
KyberFrog-Setup.exe /S                 :: silent, no autostart task
KyberFrog-Setup.exe /S /AUTOSTART=1    :: silent + register the logon task
KyberFrog-Setup.exe /S /D=C:\KyberFrog :: custom dir (/D must be last, unquoted)
```

`/S` requires administrator rights (Program Files + machine PATH).

## Uninstall

*Apps & features* → **KyberFrog**, or run `uninstall.exe` in the install folder.
The uninstaller stops KyberFrog and its children, removes the autostart task and
the PATH entry, and asks whether to keep your data — both `%APPDATA%\kyberfrog`
(config, logs, per-instance dirs) and `%LOCALAPPDATA%\kyber` (kyclient
known_hosts + logs). Silent: `uninstall.exe /S` (keeps data;
`/KEEPCONFIG=0` to wipe both).

## Autostart (hands-off display PC)

The optional logon task launches KyberFrog at every logon and relaunches it if
it exits. It is **not** the same as autologon: reaching the interactive desktop
without typing a password is a per-site security choice (Sysinternals Autologon
or `netplwiz`) — see the project README. The task can be (re)registered later:

```powershell
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
& "C:\Program Files\KyberFrog\install-kyberfrog.ps1" -Uninstall   # remove it
```
