# E2E test — Spout output from a viewer

Validates the full path **Spout source → KyberFrog transmitter → QUIC →
windowless `kyclient` → Spout → Resolume Arena / TouchDesigner**: a viewer with
*Spout out* re-publishes the received video as a local Spout sender that other
apps consume. Architecture: [Spout zero-copy](dev/plan-spout-zerocopy.md).

## What you need

- An **emitter**: a KyberFrog transmitter with a Spout source (e.g. a
  TouchDesigner `Spout Out TOP`), or any `kycontroller` you already run.
- On the **receiving** Windows machine: KyberFrog installed (or the fork bundle
  unzipped — libVLC needs its `plugins\` folder next to `kyclient.exe`), and a
  Spout receiver on the **same machine** — Spout is local-only (a shared D3D11
  texture): Resolume Arena, TouchDesigner `Spout In TOP`, or the Spout SDK's
  `SpoutReceiver.exe`.
- Emitter and receiver can be the same box: loopback works.

## Path A — from KyberFrog

1. **Réception** → add a viewer on the emitter (from *Émetteurs détectés* or by
   `IP:port`).
2. Pick the reception type **Redirection Spout**. The sender is named
   `KyberFrog-<viewer name>`.
3. Start the viewer. **No window opens**: the viewer runs windowless.

The generated command is `kyclient --spout-out <name> …`; *fullscreen* and
*remote control* do not apply to a Spout relay (the kyclient flags conflict).

## Path B — kyclient directly

Fastest way to isolate the fork side, without KyberFrog:

```bat
kyclient.exe --spout-out "KyberFrog" --tls-tofu <EMITTER_IP> --port <CONTROL_PORT> ^
  --auth-username vj --auth-password kyberfrog
```

- `--tls-tofu` trusts the emitter's self-signed certificate on first use
  (stored in `%LOCALAPPDATA%\kyber\known_hosts`); KyberFrog passes it by default.
- The server IP is positional and goes **last**.
- The log (`%LOCALAPPDATA%\Kyber\log\kyclient.log`, or `logs\kyclient-<id>.log`
  under KyberFrog) shows `Spout output enabled: running windowless (display id
  Some(…))` — a real host display id, not 0 — then the connect/stream sequence.

## Verify in a receiver

- **Resolume Arena** → *Sources → Spout Servers*: the sender (`KyberFrog-<viewer
  name>`, or the `--spout-out` value) appears and shows the live video.
- **TouchDesigner** → `Spout In TOP` → pick the same sender.

## Pass criteria

- ✅ The viewer runs windowless, connects and streams.
- ✅ The Spout sender is listed in the receiver.
- ✅ Correct colours, opaque output.
- ✅ **Native size**: the sender matches the emitter's resolution (check the
  TOP's info, or Resolume's source properties).

## If something is off

- **Black or missing image**: retry with `KYSPOUT_SMEM=1` in the viewer's
  environment, which switches to the CPU path. If that works, the driver refuses
  the zero-copy render target — report the GPU and driver version.
- **Expected libVLC warnings**, not errors: `window size missing` and
  `external ID3D11DeviceContext mutex not provided`.
- **Freeze when the source changes resolution mid-stream**: known limitation of
  the transmitter (see #8 in the [archive](dev/backlog-archive.md)) — restart
  it.
- **Multi-GPU PC**: a sender living on another adapter cannot be opened.
