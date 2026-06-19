# Troubleshooting

Where the logs are first, then the common problems.

## Logs

KyberFrog and each child write to `%APPDATA%\kyberfrog\`:

| Log | Path |
|-----|------|
| App | `logs\kyberfrog.log` |
| A viewer (`kyclient`) | `logs\kyclient-<id>.log` |
| A transmitter (`kycontroller`) | `instances\<name>\kycontroller.log` |

The **dashboard → Logs** panel tails all of these live. `kyclient` also keeps
its own log under `%LOCALAPPDATA%\kyber\log\`.

## Can't exit a fullscreen viewer

The escape hatch is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>F</kbd> (drops to
windowed, releases the keyboard grab) — **not Alt+Shift+F**. Check the combo
first.

If the correct combo still does nothing, fall back to:

- the **dashboard / tray** on that PC → **Stop** the viewer, or
- if the keyboard is grabbed and you're locked out, kill the `kyclient` process
  (Task Manager), or quit KyberFrog from the tray — the Job Object takes its
  children down with it.

A native right-click menu to close/reconfigure the viewer window is on the
roadmap (`IMPROVEMENTS.md` #15/#16).

## No video / viewer keeps restarting

Check the viewer log (`logs\kyclient-<id>.log`) and walk down this list:

1. **Wrong IP/port** — does `http://<regie-ip>:7700/transmitters` list the port
   you typed? The viewer's `port` is the transmitter's **control-plane port**,
   not 7700.
2. **Source not published** — on the regie PC, is the Spout sender actually
   live (the source app running and outputting)? A Spout transmitter is pinned
   to a **sender name**; if the name changes, re-add it.
3. **Firewall** — allow `kycontroller` / `kyclient` (and the QUIC/UDP port range)
   through Windows Defender Firewall on both machines.
4. **Auth mismatch** — see below.

## Authentication

`kycontroller` refuses connections without a valid login. By default KyberFrog
uses a **transparent login** (`vj` / `kyberfrog`) on both ends, so you type
nothing on a trusted LAN. If you set a custom login on the emitter, the viewer
side must match (file-only, in `[reception]`). Surfacing credentials in the UI
is deferred (`IMPROVEMENTS.md` #3).

## TLS "unknown host" / certificate errors

Viewers default to **TLS TOFU** (`tls_tofu = true`): they trust the emitter's
self-signed cert on first use, stored in `%LOCALAPPDATA%\kyber\known_hosts`. If
the emitter's cert changed, delete its entry there and reconnect.

## A transmitter won't start / wrong colours / crashes

- **Port clash** — `base_port` is `9000`; busy ports are skipped automatically.
  If `9000` clashes with something else, change `base_port` in the config.
- **Encoder crash (AMD GPUs)** — the AMF hardware encoder crashes in a silent
  loop on some AMD cards (e.g. RX 7800 XT). KyberFrog therefore defaults the
  generated config to **x264**. Don't switch `encoder` to AMF on affected
  hardware.
- **Max ~9 transmitters per machine** — `kycontroller`'s internal IPC ports
  auto-allocate in `9091..9100`, capping concurrent instances at about nine.

## `kyclient` / `kycontroller` not found

The installer adds its folder to the machine **PATH**, but an **already-open**
terminal won't see it — open a new one. If you installed to a custom dir and the
binaries still aren't found, set `kyber_install_dir` in `kyberfrog.toml`.

## Editing the config

Tray → **Ouvrir config** opens `%APPDATA%\kyberfrog\kyberfrog.toml`. Restart
KyberFrog after editing advanced (file-only) settings. The commented
[`examples/kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml)
documents every field.
