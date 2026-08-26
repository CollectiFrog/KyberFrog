# KyberFrog 🐸

> **KyberFrog lets you create transmitters and clients — from its dashboard on
> `:7700` — to send Spout, screen and webcam sources between machines with very
> low latency. Windows and Linux.**

![The KyberFrog web dashboard — Émission, Réception and live logs on one page](assets/dashboard.png)

A polyvalent orchestration layer on top of [Kyber](https://kyber.stream):
publish **any source** as one of **N independent transmitters** and supervise
the viewers, for low-latency, source-agnostic streaming over LAN — a drop-in
replacement for NDI.

KyberFrog is **one app, installed on every machine**. There is no separate
"server" and "client" build, and no separate Windows and Linux feature set: the
role — **emit**, **receive**, or **both** — is set entirely by the config and
the dashboard.

```
            ┌──────────── Regie PC (KyberFrog) ───────────┐
  Resolume ─Spout A─▶  emission ─▶ kycontroller :9000 ─┐   │
  Resolume ─Spout B─▶           ─▶ kycontroller :9001 ─┤   │
            └──────────────────────────────────────────│───┘
                                                        │ LAN (QUIC)
                          ┌─────────────────────────────┘
                          ▼                     ▼
              Display A (KyberFrog)   Display B (KyberFrog)
                reception → kyclient    reception → kyclient
                 fullscreen viewers      fullscreen viewers
```

The motivating setup (VJing): **Resolume Arena** on the regie machine publishes
several **Spout** outputs; each is streamed over LAN (QUIC) to display machines
running `kyclient` fullscreen.

## Where to go next

<div class="grid cards" markdown>

-   :material-account: **I want to use KyberFrog**

    ---

    Install it, add your first transmitter and viewer, fix common problems.

    [:octicons-arrow-right-24: User Manual](user/index.md)

-   :material-code-braces: **I want to build or contribute**

    ---

    Architecture, building from source (incl. the Kyber fork), releasing, CI.

    [:octicons-arrow-right-24: Developer docs](dev/index.md)

</div>

## At a glance

| | |
|---|---|
| **Sources** | Spout (Windows GPU texture share), screen capture, webcam, or **every source at once** |
| **Transport** | Kyber over QUIC (LAN) |
| **Platforms** | Windows, and Linux amd64 (Debian 13 / Ubuntu 24.04+) |
| **Per machine** | one KyberFrog, one dashboard (native window on Windows, web UI on `:7700` everywhere), one tray, one `kyberfrog.toml` |
| **Install** | a single `KyberFrog-Setup.exe` or a `.deb` — each bundles the Kyber fork binaries, no manual PATH |
| **Licence** | AGPL-3.0 |
| **Repo** | [gitlab.com/kyber-frog/kyberfrog](https://gitlab.com/kyber-frog/kyberfrog) |
