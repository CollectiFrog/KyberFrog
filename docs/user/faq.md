# FAQ

**Is KyberFrog a replacement for NDI?**
For the LAN "send a source to displays" use case, yes — it's a free,
self-hosted alternative built on Kyber's QUIC transport. It is not wire-compatible
with NDI.

**Do I install a different build on the regie and on the displays?**
No. **One** KyberFrog everywhere. The role (emit / receive / both) is set by the
config and the dashboard, not by the binary. A Windows regie and a Linux display
box talk to each other just fine.

**Do I need to install Kyber separately?**
No. Both packages — `KyberFrog-Setup.exe` and the `.deb` — bundle the Kyber fork
binaries (`kycontroller`, `kyavserver`, `kyclient`). Nothing else to install, no
PATH to edit.

**Which sources are supported?**
**Spout** (Windows GPU texture share), **screen capture**, a **webcam**, and a
**capture box** (USB HDMI box, or a Blackmagic DeckLink card). There is also a **Tout envoyer** mode: one transmitter exposing
every source of the machine at once — all monitors *and* all Spout senders —
letting each viewer choose. The model is designed to grow more input types
(SRT/RTSP, NDI in, …) without changing the orchestration.

**Does it work on Linux? On macOS?**
**Linux: yes**, on amd64 and arm64 — there is a released `.deb` for Debian 13 /
Ubuntu 24.04+ PCs and one for Raspberry Pi OS Trixie,
with screen capture, viewers, remote control, mDNS discovery and autostart as a
systemd *user* service. Two things are Windows-only by nature and are simply
hidden on a Linux box: **Spout** (a Windows GPU texture-sharing API) and the
**system tray** — on Linux you drive the app from the browser. Webcams on Linux
(V4L2) are listed in the source picker, and a capture card can be pinned by its
node path (`/dev/video0`) with explicit open options.

**macOS: no**, and none is planned for now.

**My DeckLink card does not show up in the source picker.**
Install Blackmagic **Desktop Video** (the card's driver) on the machine: the
card then appears under **Boîtier de capture**, on Windows and on Linux. Only
its *inputs* are listed — a half-duplex card such as the DeckLink Studio 2
whose connectors are set as outputs in Desktop Video Setup does not appear.
KyberFrog 0.8.0 and earlier cannot capture from a DeckLink card at all: update
to 0.8.1 or later. A USB HDMI box needs none of this:
it is a camera and works with any package.

**Do I need to type the emitter's IP by hand?**
Usually not — KyberFrog auto-discovers transmitters on the LAN (mDNS,
`_kyber._tcp.local.`) and lists them as **Émetteurs détectés** in the viewer
form; clicking one fills the name/IP/port. Manual `IP:port` entry stays
available as a fallback (e.g. across a VLAN, where mDNS doesn't reach). See
[Troubleshooting](troubleshooting.md) if nothing shows up.

**How many streams can one machine publish?**
About **9 transmitters** per machine — `kycontroller`'s internal IPC ports
auto-allocate in `9091..9100`. Run more by spreading across machines.

**How do I exit a fullscreen viewer?**
<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>F</kbd> drops it to windowed and releases the
keyboard grab. See
[Troubleshooting](troubleshooting.md#cant-exit-a-fullscreen-viewer).

**Where is my configuration?**
In `%APPDATA%\kyberfrog\` on Windows, `$XDG_CONFIG_HOME/kyberfrog/` (usually
`~/.config/kyberfrog/`) on Linux: `kyberfrog.toml` holds the machine settings
(tray → *Ouvrir config*), and `setups/` the transmitters and viewers, one file
per saved setup (`setup-default.toml` to start with). On Linux, logs and
instance state are under `$XDG_STATE_HOME/kyberfrog/`. Most day-to-day changes are done from the
dashboard instead.

**Is the stream encrypted? Do I need a password?**
Transport is TLS over QUIC (TOFU by default on a trusted LAN). Auth uses a
transparent default login (`vj` / `kyberfrog`) so you type nothing; you can set
a custom one in the config.

**Can a viewer control a remote machine (remote desktop)?**
Yes. Tick **contrôle à distance** (remote control) on a viewer: it runs windowed
and forwards your keyboard + mouse (grabbing the keyboard) to drive the remote
KyberFrog over QUIC. The remote side runs an Émission with a **screen-capture**
source. Escape with **Ctrl+Alt+F**. It's mutually exclusive with a Spout-out
relay on the same viewer.

**Is it free / open source?**
Yes, **AGPL-3.0**. Source:
[gitlab.com/kyber-frog/kyberfrog](https://gitlab.com/kyber-frog/kyberfrog).
