# FAQ

**Is KyberFrog a replacement for NDI?**
For the LAN "send a source to displays" use case, yes — it's a free,
self-hosted alternative built on Kyber's QUIC transport. It is not wire-compatible
with NDI.

**Do I install a different build on the regie and on the displays?**
No. **One** `kyberfrog.exe` everywhere. The role (emit / receive / both) is set
by the config and the web UI, not by the binary.

**Do I need to install Kyber separately?**
No. `KyberFrog-Setup.exe` bundles the Kyber fork binaries (`kycontroller`,
`kyavserver`, `kyclient`) and adds itself to PATH. Nothing else to install.

**Which sources are supported?**
Today: **Spout** (Windows GPU texture share) and **screen capture**. The model
is designed to grow more input types (video files, NDI in, …) without changing
the orchestration.

**Does it work on macOS / Linux?**
The app targets **Windows** (Spout, the tray, Job Objects, the bundled binaries
are all Windows). The pure data-model crate is cross-platform, but there is no
supported non-Windows build.

**How many streams can one machine publish?**
About **9 transmitters** per machine — `kycontroller`'s internal IPC ports
auto-allocate in `9091..9100`. Run more by spreading across machines.

**How do I exit a fullscreen viewer?**
<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>F</kbd> drops it to windowed and releases the
keyboard grab. See
[Troubleshooting](troubleshooting.md#cant-exit-a-fullscreen-viewer).

**Where is my configuration?**
`%APPDATA%\kyberfrog\kyberfrog.toml`. Edit it via tray → *Ouvrir config*. Most
day-to-day changes are done from the web UI instead.

**Is the stream encrypted? Do I need a password?**
Transport is TLS over QUIC (TOFU by default on a trusted LAN). Auth uses a
transparent default login (`vj` / `kyberfrog`) so you type nothing; you can set
a custom one in the config.

**Can a viewer control a remote machine (remote desktop)?**
Not yet — a remote-control viewer that forwards keyboard/mouse is on the roadmap
(`IMPROVEMENTS.md` #10).

**Is it free / open source?**
Yes, **AGPL-3.0**. Source:
[gitlab.com/kyber-frog/kyberfrog](https://gitlab.com/kyber-frog/kyberfrog).
