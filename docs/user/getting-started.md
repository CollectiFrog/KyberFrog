# Getting started

This walks through a minimal two-machine setup: one **regie** PC publishing a
Spout output, one **display** PC showing it fullscreen. Both have KyberFrog
[installed](installation.md).

## 0. Before you start

- Both machines on the **same LAN**, able to reach each other.
- On the regie PC, your source app (e.g. **Resolume Arena**) is running and
  publishing a **Spout** output.

!!! tip "You usually don't need to note the IP"
    KyberFrog auto-discovers transmitters on the LAN (mDNS) and offers them as
    a clickable list on the viewer form — see step 2. Noting the **regie PC's
    IP** (`ipconfig` → e.g. `192.168.1.10`) is only needed as a fallback if
    nothing shows up there.

Open the dashboard on each machine. On Windows it opens **by itself** in a
native window at startup — a left click on the tray icon brings it back. Either
way, and on Linux, it is also at `http://<that-pc>:7700/` in a browser,
including from another machine on the LAN.

## 1. Regie PC — publish a transmitter (Émission)

1. In the dashboard, go to the **Émission** section.
2. Pick a source:
    - **Spout** — from the **live picker**, which lists the senders currently
      active on the machine;
    - **screen capture** — the viewer picks *which* display when it connects.
      On a Windows machine with no monitor, pick **+ Écran virtuel** to stream
      a virtual screen instead (see
      [Installation → Virtual screen](installation.md#virtual-screen-for-a-machine-with-no-monitor));
    - **webcam** — from the detected device list;
    - **capture box** — a USB HDMI box or a DeckLink card, from the detected
      devices (DeckLink needs a bundle built with its SDK — see the
      [FAQ](faq.md));
    - or **Tout envoyer**, one transmitter exposing every monitor *and* every
      Spout sender at once, letting each viewer choose.
3. Optionally set a **port** (otherwise the lowest free port from `9000` is
   auto-allocated).
4. The transmitter starts and shows a **live status**. Note its **port**.

Its card also says what it is sending — for Spout, the sender's size and
format (`1280×720 · RGBA 16 bits float`) — and, in plain words, what stops a
source from sending: sender not found or silent, unreadable format, other
graphics card, device lost, no DeckLink signal. **Edit** changes a transmitter
after the fact, including its **name**; it restarts under the new name.

!!! tip "Discover transmitters from another machine"
    `GET http://<regie-ip>:7700/transmitters` returns the transmitter list as
    JSON, for tooling or to remind yourself which port is which.

## 2. Display PC — add a viewer (Réception)

1. In the dashboard, go to the **Réception** section.
2. **Add a viewer:**
    - Pick the transmitter from **Émetteurs détectés** (auto-discovered over
      the LAN) — this fills the name, IP and port for you; or type the
      transmitter's **`IP:port`** manually — e.g. `192.168.1.10:9000` — if
      nothing is detected (see the tip in step 0),
    - **fullscreen** on/off,
    - **Écran de sortie** — on a Windows machine with several screens, which
      local screen the window opens and goes fullscreen on (the main screen by
      default).
3. **Start** it. A `kyclient` opens and shows the stream.

Use **Start / Stop / Restart** on each viewer. **Edit + Apply** hot-relaunches a
viewer (including **renaming** it). An **enabled** viewer relaunches automatically
on boot — set this on a dedicated display PC. **Closing a viewer's window stops
it**, like the Stop button; it stays in the config, ready to start again.

### When the transmitter or the network goes away

A viewer never freezes on the last image. If the transmitter stops, crashes or
the network cable is pulled, the window **goes black within about 2 s**, stays
open, and the picture comes back in the same window as soon as the
transmitter is reachable again — nothing to click. A Spout-out relay publishes
black the same way. On the emitting side, a source that disappears (Spout
sender closed, camera unplugged) holds its last image for 1 s, then sends
black until it comes back.

## 3. Exit a fullscreen viewer

A passive display has **no quit shortcut by design**. The escape hatch is:

<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>F</kbd>

It drops `kyclient` to **windowed** and releases the keyboard grab, giving you
the desktop back. (Then close the window or use the tray / dashboard.)

!!! note
    It is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>F</kbd> — not Alt+Shift+F. If even
    the correct combo does nothing, see
    [Troubleshooting → Can't exit fullscreen](troubleshooting.md#cant-exit-a-fullscreen-viewer).

## 4. The system tray (Windows)

A **left click** (single or double) on the tray icon opens/focuses the
**dashboard window** — closing that window never stops anything, it just hides
it. A **right click** opens the menu: it mirrors the frequent actions (add a
Spout transmitter via the live picker, start/stop/restart/remove on both
halves), opens the **dashboard**, the **config file** or the **logs**, and
**Quitter** is the only thing that actually stops KyberFrog. Child status
shows as monochrome glyphs:
`○` starting · `●` running · `◐` restarting · `✗` stopped.

## What's next

- **Advanced settings** (auth, base port, input/audio/keyboard/TLS) are
  **file-only**, in the setup file (`setups/setup-default.toml` next to the
  config that tray → *Ouvrir config* opens). See the commented
  [`setups/setup-default.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/setups/setup-default.toml) and
  [`kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml).
- Hitting a wall? → [Troubleshooting](troubleshooting.md) and the [FAQ](faq.md).
