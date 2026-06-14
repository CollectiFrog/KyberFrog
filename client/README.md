# KyberFrog Client (client-agent)

The client-side half of KyberFrog 🐸. It runs on each **client machine**
(Client A, Client B, …), serves a small **web UI**, and supervises **N
`kyclient` viewers** — each connected to one transmitter published by a Server.

`kyclient` reconnects on its own; when it finally gives up and exits (server
gone, transmitter removed, network drop) the client relaunches it with capped
exponential backoff (1 s → 15 s, reset after 30 s of healthy uptime). Instances
are persisted, so a client PC comes back on its own after a reboot or a server
restart with no operator action.

Each viewer's `kyclient` is launched **video-only** (no input/audio, keyboard
free) so Alt+Tab and the Windows key keep working; to reach Windows, drop a
viewer to a window with kyclient's own shortcut.

## Web UI

Browse `http://<client-pc>:<web_port>/` (default port **7701**, bound on the LAN).
It shows the machine's **name and IP**, the **logs** (the client's own and each
viewer's), and lets you manage the viewers live:

- add a viewer (transmitter `IP:port`, fullscreen on/off),
- **Start / Stop / Restart**, edit a viewer and **Apply** (hot relaunch),
- remove a viewer.

Every change is written to `client-agent.toml`, so it survives a reboot. Picking
*which monitor* a viewer fullscreens on is a planned improvement (needs a small
kyclient change — see [`../IMPROVEMENTS.md`](../IMPROVEMENTS.md)).

## Config

`%APPDATA%\kyberfrog\client-agent.toml`, created with defaults on first run. You
normally edit it from the web UI, but it is plain TOML:

```toml
kyclient_path = 'kyclient.exe'
web_port = 7701

# Transparent login (Server default) + passive-display flags, applied to every
# viewer. Not exposed in the web UI for now (see IMPROVEMENTS.md).
auth_username = "vj"
auth_password = "kyberfrog"
forward_inputs = false
audio = false
keyboard_grab = false
tls_tofu = true

[[instance]]
id = "instance-1"
server = "192.168.1.10"   # transmitter host
port = 9000               # transmitter control-plane port
fullscreen = true
enabled = true            # "should be running" → relaunched on boot
```

> Upgrading from an older build? A legacy `scene-agent.toml` is renamed to
> `client-agent.toml` automatically on first launch — no action needed.

Each viewer runs:

```
kyclient.exe <server> --port <port> --tls-tofu \
    --auth-username vj --auth-password kyberfrog [--fullscreen] \
    --inputs false --audio false --keyboard-grab false
```

## Install (autostart at logon)

1. Copy `kyberfrog-client.exe` to the client machine and place it wherever you
   like (e.g. `C:\Program Files\KyberFrog\`).
2. Run it once to generate the config, then add viewers from the web UI
   (`http://localhost:7701/`) or by editing the TOML.
3. Register the logon task (run in the session of the auto-login user):
   ```powershell
   .\install\install-client-agent.ps1 -ExePath "C:\Program Files\KyberFrog\kyberfrog-client.exe"
   ```
   Remove it later with `-Uninstall`.

The client is a console app; its window sits behind the fullscreen viewers and is
only visible if a viewer is dropped to a window — fine for a client PC.

## Autologon (manual, per-site)

For a truly hands-off client PC the machine must reach the interactive desktop
without someone typing a password. This is **not** scripted here because it
stores a credential and is a security trade-off you should make deliberately.
Two common ways:

- **Sysinternals Autologon** (recommended): stores the password LSA-encrypted
  rather than in plaintext. Run it once, enter the client user's credentials.
- `netplwiz` → untick *"Users must enter a user name and password"*.

Pair autologon with this logon task and the client PC boots straight into the
fullscreen streams.
