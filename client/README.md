# KyberFrog Client (scene-agent)

The client-side half of KyberFrog 🐸. It runs on each **scene machine**
(PCSceneJar, PCSceneCour, …) and keeps exactly one `kyclient` alive, fullscreen,
connected to one transmitter published by the Server on the regie.

`kyclient` already reconnects on its own; when it finally gives up and exits
(server gone, transmitter removed, network drop) the client relaunches it with
capped exponential backoff (1 s → 15 s, reset after 30 s of healthy uptime). A
scene PC therefore recovers on its own from a regie restart with no operator
action.

There is **no quit/maintenance mode**: the operator never voluntarily closes the
viewer. To reach Windows they drop the client to a window with kyclient's own
shortcut (the keyboard grab is off by default here, so Alt+Tab and the Windows
key already work).

## Config

`%APPDATA%\kyberfrog\scene-agent.toml`, created with defaults on first run.
Only `server` is mandatory.

```toml
server = "192.168.1.10"   # regie IP / hostname (REQUIRED)
port = 9000               # transmitter control-plane port to display

kyclient_path = 'D:\soft\kyber\kyclient.exe'

# Defaults to the Server's transparent login, so a stock LAN needs no setup.
auth_username = "vj"
auth_password = "kyberfrog"

# Passive scene display: fullscreen video only, no input/audio, keyboard free.
fullscreen = true
forward_inputs = false
audio = false
keyboard_grab = false
tls_tofu = true

# Anything not modelled above, appended verbatim (codec, bitrate, …).
extra_args = []
```

The resulting command is:

```
kyclient.exe <server> --port <port> --tls-tofu \
    --auth-username <u> --auth-password <p> --fullscreen \
    --inputs false --audio false --keyboard-grab false
```

To switch which transmitter a scene shows, change `port` and restart the client
(`Restart-ScheduledTask`). Remote switching is the job of the future web UI
(step 5).

## Install (autostart at logon)

1. Copy `kyberfrog-client.exe` to the scene machine (e.g. next to the Kyber
   install, `D:\soft\kyber\`).
2. Run it once to generate the config, then set `server`:
   ```powershell
   D:\soft\kyber\kyberfrog-client.exe   # writes the default toml, then exits with a hint
   notepad $env:APPDATA\kyberfrog\scene-agent.toml
   ```
3. Register the logon task (run in the session of the auto-login user):
   ```powershell
   .\install\install-scene-agent.ps1 -ExePath D:\soft\kyber\kyberfrog-client.exe
   ```
   Remove it later with `-Uninstall`.

The client is a console app; its window sits behind the fullscreen viewer and is
only visible if the viewer is dropped to a window — fine for a scene PC.

## Autologon (manual, per-site)

For a truly hands-off scene PC the machine must reach the interactive desktop
without someone typing a password. This is **not** scripted here because it
stores a credential and is a security trade-off you should make deliberately.
Two common ways:

- **Sysinternals Autologon** (recommended): stores the password LSA-encrypted
  rather than in plaintext. Run it once, enter the scene user's credentials.
- `netplwiz` → untick *"Users must enter a user name and password"*.

Pair autologon with this logon task and the scene PC boots straight into the
fullscreen stream.
