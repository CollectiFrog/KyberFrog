# E2E test — Spout output from a viewer (Amélioration 2)

Goal: validate the full path **TouchDesigner → Spout → KyberFrog (emit) → QUIC
→ kyclient `--spout-out` → Spout → Resolume Arena**, i.e. that a windowless
`kyclient` re-publishes the received video as a Spout sender other apps consume.

This document covers the **direct kyclient test** (no KyberFrog needed) — the
fastest way to prove the fork change works. The KyberFrog UI wiring is step (b),
tracked in `IMPROVEMENTS.md` #8.

## The built artifact (ready)

A self-contained bundle was cross-compiled from the `feat/spout-output` fork
chain and is ready to test:

```
C:\Users\trist\Workspace\Kyber\apps\kyber-desktop\kyberfrog-spout-e2e.zip   (~71 MB)
```

Unzip anywhere on the **Windows test machine** (keep everything together —
libVLC needs its `plugins\` folder next to `kyclient.exe`). It contains:
- `kyclient.exe` — **with the `--spout-out` flag** (verified: the binary carries
  the flag and the help string "Windowless: publish the decoded video as a Spout
  sender …").
- `kyclient.dll`, `kynput.dll`, `libvlc.dll` + `libvlccore.dll` + `plugins\`
  (323 VLC plugin DLLs incl. `codec`, `access`, `d3d11`), all ffmpeg/txproto/
  SDL2 DLLs, MinGW runtime DLLs.
- **Bonus — an emitter too:** `kycontroller.exe`, `kyavserver.exe`, plus
  `kyber_config.toml` and test TLS certs, so the same bundle can play both ends
  of the E2E on one or two machines.

> Build provenance: native deps (libVLC/ffmpeg/txproto) + libkyclient (with the
> `set_spout_out` C-API, confirmed in the generated `kyclient.h`) + the
> `kyclient.exe` winit binary, all from the pinned `feat/spout-output` submodule
> chain (`vlc-rs` f91eb1f → `kymedia` → `kyctl` 81e2818 → `kysdk` 3bd1ff8 →
> `kyber-desktop` 1f1349e). Reproducible via `kyber-desktop/build-win32.sh`.
- A running **emitter** producing video over QUIC. Two options:
  - the existing KyberFrog **Émission** panel with a Spout transmitter fed by
    TouchDesigner, **or**
  - any `kycontroller` instance you already use for the regie.
- **Resolume Arena** (or any Spout receiver: Spout's own `SpoutReceiver` demo,
  MadMapper, TouchDesigner `Syphon Spout In` TOP) on the **same machine** as the
  windowless kyclient — Spout is local-only (shared D3D11 texture).

## Step 1 — sanity: kyclient still works with a window

Confirm the build runs normally before testing the new path:

```
kyclient.exe --fullscreen <EMITTER_IP> --port <CONTROL_PORT> ^
  --auth-username vj --auth-password kyberfrog
```

You should see the stream fullscreen. Ctrl+Alt+F drops to windowed. Quit.

## Step 2 — the windowless Spout path

```
kyclient.exe --spout-out "KyberFrog" <EMITTER_IP> --port <CONTROL_PORT> ^
  --auth-username vj --auth-password kyberfrog
```

Expected:
- **No window opens** (windowless relay). The console stays up with logs.
- The log shows `Spout output enabled: running windowless` then the normal
  connect/stream sequence. (Log file: `%LOCALAPPDATA%\Kyber\log\kyclient.log`.)
- `--spout-out` conflicts with `--fullscreen` (clap rejects both together).

## Step 3 — verify the Spout sender in a receiver

In Resolume Arena → add a **Spout** source. A sender named **`KyberFrog`**
(the `--spout-out` value) must appear in the source list, showing the live
video from the emitter.

Quick alternative without Resolume: run the official Spout `SpoutReceiver.exe`
demo (from the Spout SDK release) — it lists active senders and previews them.

## What to look for (v1 limitations, see IMPROVEMENTS.md #8)

- **Colours.** If the image looks colour-swapped (red/blue inverted), the chroma
  fourcc is wrong — switch `RV32` → `RGBA` in `kyvlcplayer`'s
  `setup_spout_output` and rebuild. This is the #1 thing to verify on first run.
- **Resolution.** v1 forces **1920×1080** (libVLC scales the stream to it). A
  non-1080p emitter will be rescaled, not native. Native size needs
  `set_video_format_callbacks` in vlc-rs (deferred).
- **The sender appears but is black / not updating.** Suspect the share-handle
  semantics in `kyspout` (legacy `MISC_SHARED` vs NT handle) or the
  `SpoutSenderNames` / `MaxSenders` memory layout — these are the runtime-
  unvalidated parts. Cross-check against the Spout SDK's `SpoutSenderNames`.
- **CPU round-trip.** Each frame goes smem (CPU BGRA) → GPU texture upload. Some
  latency/CPU cost is expected in v1; zero-copy is future work.

## Pass criteria

✅ kyclient runs windowless, connects, streams.
✅ A Spout sender `KyberFrog` is visible in Resolume/SpoutReceiver.
✅ It shows the live emitter video with correct colours.

If all three hold, the fork side of Amélioration 2 is validated and we can wire
the per-viewer Spout toggle into KyberFrog (step b).
