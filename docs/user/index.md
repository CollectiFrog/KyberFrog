# User Manual

KyberFrog streams video sources between machines on a local network with low
latency, as a free, self-hosted alternative to NDI. It is built for a **VJ /
live-visuals** setup but works for any "send this screen/output to those
displays over LAN" need.

## The mental model

Every machine runs the **same** KyberFrog — the same app on Windows and on
Linux amd64. What a machine *does* is set by its config, which has two
independent halves:

| Half | What it does | Process per item |
|------|--------------|------------------|
| **Émission** (emit) | Publishes **transmitters** — each takes a *source* and serves it over the LAN. | one `kycontroller` |
| **Réception** (receive) | Runs **viewers** — each connects to a remote transmitter and shows it. | one `kyclient` |

- A **regie / host** PC has transmitters (and usually no viewers).
- A **display** PC has viewers (and usually no transmitters).
- A machine can do **both** at once.

### Key terms

- **Transmitter** — one published stream. Has a *name*, a *port*, and a *source*.
- **Source** — what the transmitter captures:
    - **Spout** — a Windows GPU texture shared by another app (Resolume,
      TouchDesigner, MadMapper…), pinned by its *sender name*.
    - **Screen** — a plain desktop / monitor capture. Which physical display a
      viewer gets is chosen **on the viewer's side**, at connection time.
    - **Webcam** — a capture device, pinned by its device name (Windows
      DirectShow).
    - **Tout envoyer** (send all) — one transmitter exposing **every** source of
      the machine at once, monitors *and* Spout senders, letting each viewer
      pick. Handy for a single "just give me everything" link between two boxes.
- **Viewer** — one `kyclient` showing a remote transmitter. Has an *id/name*,
  the emitter's *IP : port* (auto-filled from the LAN's auto-discovered
  transmitters, or typed manually), and a handful of flags: **fullscreen**,
  which **display** to request from the emitter, **remote control** (keyboard
  and mouse takeover), or **Spout out** to re-publish the incoming stream as a
  local Spout sender instead of showing a window.

## How a stream flows

1. An app (e.g. Resolume) publishes a **Spout** output on the regie PC.
2. KyberFrog's **Émission** spawns a `kycontroller` that captures that Spout
   sender and serves it over QUIC on a port (default `9000`, then the next free
   one).
3. On a display PC, KyberFrog's **Réception** spawns a `kyclient` pointed at
   `regie-ip:9000`; it shows the video fullscreen.

Both halves run under **one supervisor**: if KyberFrog exits for any reason,
every child process it started is terminated — no orphans left behind.

## One config, two front-ends

Everything lives in one `kyberfrog.toml` — `%APPDATA%\kyberfrog\` on Windows,
`$XDG_CONFIG_HOME/kyberfrog/` (usually `~/.config/kyberfrog/`) on Linux. You
normally never edit it by hand:

- **The dashboard** — a **native window** on Windows, and always
  reachable in a browser at `http://<this-pc>:7700/`, including from another
  machine on the LAN. Add/remove/restart transmitters and viewers, watch live
  status and logs. Closing the native window only hides it; only the tray's
  *Quitter* stops the app.
- **System tray** (Windows) — the same frequent actions, plus shortcuts to open
  the dashboard, the config file, or the logs. On Linux there is no tray yet:
  the app runs as a systemd *user* service and you drive it from the browser.

**Advanced settings** (authentication, encoder, install dir, base port,
input/audio/keyboard/TLS flags) are **file-only** by design — edit the TOML
(tray → *Ouvrir config*). See [Troubleshooting](troubleshooting.md) and the
commented [`examples/kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml).

---

Ready? → **[Installation](installation.md)** then **[Getting started](getting-started.md)**.
