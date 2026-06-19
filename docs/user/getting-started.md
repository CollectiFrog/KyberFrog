# Getting started

This walks through a minimal two-machine setup: one **regie** PC publishing a
Spout output, one **display** PC showing it fullscreen. Both have KyberFrog
[installed](installation.md).

## 0. Before you start

- Both machines on the **same LAN**, able to reach each other.
- Note the **regie PC's IP** (e.g. `ipconfig` → `192.168.1.10`).
- On the regie PC, your source app (e.g. **Resolume Arena**) is running and
  publishing a **Spout** output.

Open the dashboard on each machine at `http://<that-pc>:7700/`.

## 1. Regie PC — publish a transmitter (Émission)

1. In the dashboard, go to the **Émission** section.
2. Add a **Spout** source:
    - Pick the Spout sender from the **live picker** (it lists active senders),
      or
    - add a **screen capture** instead.
3. Optionally set a **port** (otherwise the lowest free port from `9000` is
   auto-allocated).
4. The transmitter starts and shows a **live status**. Note its **port**.

!!! tip "Discover transmitters from another machine"
    `GET http://<regie-ip>:7700/transmitters` returns the transmitter list as
    JSON, for tooling or to remind yourself which port is which.

## 2. Display PC — add a viewer (Réception)

1. In the dashboard, go to the **Réception** section.
2. **Add a viewer:**
    - *(optional)* a **name**,
    - the transmitter's **`IP:port`** — e.g. `192.168.1.10:9000`,
    - **fullscreen** on/off.
3. **Start** it. A `kyclient` opens and shows the stream.

Use **Start / Stop / Restart** on each viewer. **Edit + Apply** hot-relaunches a
viewer (including **renaming** it). An **enabled** viewer relaunches automatically
on boot — set this on a dedicated display PC.

## 3. Exit a fullscreen viewer

A passive display has **no quit shortcut by design**. The escape hatch is:

<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>F</kbd>

It drops `kyclient` to **windowed** and releases the keyboard grab, giving
Windows back. (Then close the window or use the tray / dashboard.)

!!! note
    It is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>F</kbd> — not Alt+Shift+F. If even
    the correct combo does nothing, see
    [Troubleshooting → Can't exit fullscreen](troubleshooting.md#cant-exit-a-fullscreen-viewer).

## 4. The system tray

The tray mirrors the frequent actions (add a Spout transmitter via the live
picker, start/stop/restart/remove on both halves) and opens the **dashboard**,
the **config file**, or the **logs**. Child status shows as monochrome glyphs:
`○` starting · `●` running · `◐` restarting · `✗` stopped.

## What's next

- **Advanced settings** (auth, encoder, base port, input/audio/keyboard/TLS) are
  **file-only** — tray → *Ouvrir config*. See the commented
  [`examples/kyberfrog.toml`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/examples/kyberfrog.toml).
- Hitting a wall? → [Troubleshooting](troubleshooting.md) and the [FAQ](faq.md).
