# User Manual

KyberFrog streams video sources between machines on a local network with low
latency, as a free, self-hosted alternative to NDI. It is built for a **VJ /
live-visuals** setup but works for any "send this screen/output to those
displays over LAN" need.

## The mental model

Every machine runs the **same** `kyberfrog.exe`. What a machine *does* is set by
its config, which has two independent halves:

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
    - **Screen** — a plain desktop / monitor capture.
- **Viewer** — one `kyclient` showing a remote transmitter. Has an *id/name*,
  the emitter's *IP : port*, and a *fullscreen* flag.

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

Everything lives in `%APPDATA%\kyberfrog\kyberfrog.toml`. You normally never
edit it by hand:

- **Web UI** — `http://<this-pc>:7700/`. Add/remove/restart transmitters and
  viewers, watch live status and logs.
- **System tray** — the same frequent actions, plus shortcuts to open the
  dashboard, the config file, or the logs.

**Advanced settings** (authentication, encoder, install dir, base port,
input/audio/keyboard/TLS flags) are **file-only** by design — edit the TOML
(tray → *Ouvrir config*). See [Troubleshooting](troubleshooting.md) and the
commented [`examples/kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml).

---

Ready? → **[Installation](installation.md)** then **[Getting started](getting-started.md)**.
