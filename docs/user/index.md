# User Manual

KyberFrog streams video sources between machines on a local network with low
latency, as a free, self-hosted alternative to NDI. It is built for a **VJ /
live-visuals** setup but works for any "send this screen/output to those
displays over LAN" need.

## The mental model

Every machine runs the **same** KyberFrog — the same app on Windows and on
Linux (amd64 PCs and arm64 Raspberry Pis). What a machine *does* is set by its config, which has two
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
    - **Webcam** — a camera, pinned by its device name (DirectShow on
      Windows, V4L2 card name on Linux) or, on Linux, by a node path such as
      `/dev/video0`. Its *open options* (advanced field, one `key=value` per
      line: `input_format`, `video_size`, `framerate`…) go to FFmpeg as-is.
    - **Capture box** (*Boîtier de capture*) — an HDMI/SDI input: a USB
      capture box (Ugreen, Elgato Cam Link…), which is a camera under the hood
      and takes the same open options — the fix for a box that does not stream
      with the defaults — or a Blackmagic **DeckLink** card (Windows or Linux,
      with a bundle built with DeckLink support), with its connector and capture
      mode. USB boxes are recognised by name; one with a generic name shows
      under **Webcam** and works the same from there.
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

The configuration lives in `%APPDATA%\kyberfrog\` on Windows and
`$XDG_CONFIG_HOME/kyberfrog/` (usually `~/.config/kyberfrog/`) on Linux, in two
kinds of files:

- `kyberfrog.toml` — what belongs to **this machine**: install paths, dashboard
  port, encoder, theme and language, and which setup is loaded;
- `setups/<name>.toml` — a **setup**: the transmitters and viewers of one show,
  with their login, TLS and input settings. The dashboard saves, loads, exports
  and imports setups, so a show moves from one machine to another in one file.

You normally never edit them by hand:

- **The dashboard** — a **native window** on Windows, and always
  reachable in a browser at `http://<this-pc>:7700/`, including from another
  machine on the LAN. Add/remove/restart transmitters and viewers, watch live
  status and logs. Closing the native window only hides it; only the tray's
  *Quitter* stops the app.
- **System tray** (Windows) — the same frequent actions, plus shortcuts to open
  the dashboard, the config file, or the logs. On Linux there is no tray yet:
  the app runs as a systemd *user* service and you drive it from the browser.

**Advanced settings** are **file-only** by design: install dir in
`kyberfrog.toml` (tray → *Ouvrir config*); authentication, base port and the
input/audio/keyboard/TLS flags in the setup file, `setups/setup-default.toml`
unless you saved another. See [Troubleshooting](troubleshooting.md) and the
commented examples, [`kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml) and
[`setups/setup-default.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/setups/setup-default.toml).

---

Ready? → **[Installation](installation.md)** then **[Getting started](getting-started.md)**.
