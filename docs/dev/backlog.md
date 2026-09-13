# Backlog

Everything that is open, in one table. If you are looking for what already
shipped, that is [`CHANGELOG.md`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/CHANGELOG.md);
if you are looking for *why* a chantier is designed the way it is, that is the
`plan-*.md` linked from the item.

## How to read this

Every item carries two labels. **State** is the kanban column. **Access** is what
you need to *have* in order to take it — which is the real filter on this
project, far more than difficulty.

| State | Meaning |
|---|---|
| 📋 **ready** | someone can pick this up today |
| 🚧 **in progress** | someone is on it — check the linked MR before starting |
| ⏳ **blocked** | waiting on an upstream change or on hardware nobody has yet |
| 🧭 **decision** | writing code before the call is made would be a mistake |
| 🧊 **icebox** | kept on purpose, no expressed need — do not start it |

| Access | What you need |
|---|---|
| 💻 **laptop** | Rust + React and the MinGW Docker image. Nothing else. |
| 🔧 **fork chain** | the 7 nested fork repos, ~1 h 30 for a full build — see [Building from source](building.md) |
| 🎛️ **hardware** | a second machine, a vertical screen, an AMD GPU, a Linux VM… |
| 🧭 **operator** | a product call, not an implementation |

**New here?** Start with an item that is `📋 ready` **and** `💻 laptop`. There
are five, and each one has a card at the [bottom of this page](#take-one-of-these-first)
telling you why it matters, which files to open, and how you know you are done.

**Taking one?** Open an issue from the *Backlog item* template, link it from the
row here and set the state to `🚧 in progress` — the flow is described in
[Contributing → taking an item](contributing.md#taking-an-item). This page stays
the source of truth; the tracker only carries what is actively being worked on.

## Validation queue — no code, just a run

These are **not development tasks**. Every one of them is a thing that was built
and needs to be exercised once, by someone with the right machine in front of
them. They are listed separately because they clear fast and they unblock
honest status elsewhere on the board.

| ID | What to run | Needs | What to record |
|---|---|---|---|
| #19 | Screen-only transmitter shows monitors only, and *Send all* shows everything | Windows dev box | pass / fail per scenario, screenshot if it fails |
| #20 | Machine A transmits, machine B sees it in *Detected emitters* within seconds; stopping A makes the entry disappear; the NSIS firewall rule (UDP 5353) is enough on a **freshly installed** machine — no dev build, no manual allow | **2 Windows machines**, one with a clean install | the three checks, and how long discovery took |
| #28-4 | Latency baseline: a `Spout In TOP` on the Kyber stream vs a `Spout In TOP` **direct** on the source sender, side by side in TouchDesigner | dev box (TD is installed) | the gap in frames — it *is* the cost of the Kyber chain |
| #17-B5 | `Ctrl+Alt+F` while keyboard grab is active — it has **never been proven broken**, only assumed | Windows + a remote session | works / does not work, and with which exact combo |
| #33-check | Is VAAPI available and usable on the Linux box (Intel/AMD amd64)? | Linux machine | `vainfo` output — the fix is only worth writing if the answer is yes |
| #40 | Kill KyberFrog on Linux and confirm no child process survives | Linux VM | `ps` before / after |
| #41 | Linux viewer: fullscreen actually goes fullscreen, and `--display-idx` picks the right screen | Linux VM, 2 screens ideally | pass / fail per flag |
| #43 | The Linux CI jobs and `release-deb` have **never run in a real pipeline or on a real tag** — only replayed locally in the CI image | nothing, just a push and a tag | pipeline URL, and whether the `.deb` lands on the release |
| [issue #1](https://gitlab.com/kyber-frog/kyberfrog/-/issues/1) | A webcam picked from a *Tout envoyer* transmitter opened a blank kyclient window and never lit the camera. **Believed fixed** — the fork-side fix shipped in 0.5.0 and the bundle pinned since 2026-08-17 contains it, but the report was filed on 2026-07-06, *before* the fix, and never re-tested | Windows box + the webcam | if it works, **close the issue**; if not, it becomes a real backlog item |

Once a line here is done, tick it off the board and — if it changes a state —
move the item. Nothing else on this page depends on writing code to be true.

## Board

### Product core — emission and reception

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #27 | Spout passthrough — **beta test** | 🚧 in progress | 🎛️ dev box *(Resolume and TD are installed there, loopback via `is_self`)* | one switch, **emitter side only**: every local Spout sender becomes its own transmitter. Done when the beta validation plan passes. The receiving half ("receive every Kyber stream") is **not retained** | [plan](plan-spout-passthrough.md) |
| #19 | Re-test screen-only and *Send all* | 📋 ready | 🎛️ dev box | see the validation queue | — |
| #20 | mDNS discovery across two machines | 📋 ready | 🎛️ **2 machines** | see the validation queue | — |
| #18-D/F | SRT / RTSP input and output | 📋 ready | 🔧 fork chain | txproto accepts an `rtsp://` / `srt://` URL, `Source::Url` variant exists — FFmpeg already supports both, so expect little fork code | [plan](plan-sources-exports.md) |
| #18-C/E | NDI input and output | ⏳ blocked | 🔧 + proprietary libndi | libVLC has an NDI plugin, which is the lead worth exploring before touching txproto | [plan](plan-sources-exports.md) |
| #26 | Emitter-pinned source screen | 🧊 icebox | 🔧 | no expressed need — #18-B covers the use case today. The fork recipe is written down in case the field ever asks for it | [plan](plan-sources-exports.md) |

### Web UI

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #22 | Consistent hover state on every button | 📋 ready | 💻 | the ~35 buttons across 8 files go through `.kf-btn`; nothing blocks it since #21 shipped | [card](#22-consistent-hover-state) |
| #29 | Self-host the web fonts | 📋 ready | 💻 | `ui/index.html` no longer calls `fonts.googleapis.com` — a venue LAN with no internet keeps its typefaces | [card](#29-self-host-the-web-fonts) |
| #2 | Live log streaming (SSE) instead of polling | 📋 ready | 💻 | `GET /logs/stream` pushes new lines, the UI drops its `setInterval` | [card](#2-live-log-streaming) |
| #3 | Credentials in the UI | 📋 ready | 💻 | optional per-viewer / per-transmitter fields override the transparent default | [card](#3-credentials-in-the-ui) |
| #23 | Drawers → modals | 🧭 decision | 🧭 operator | **do not write code before the call is made** | — |

### Fork chain and latency

> Latency is the project's **number one priority**. Anything here that reduces it
> outranks a feature.

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #28-4 | Measure before optimising | 📋 ready | 🎛️ **zero code** | a number exists for the cost of the Kyber chain | [plan](plan-latency.md) |
| #28-1 | GPU encoder: AMF / NVENC with `zerolatency` | 📋 ready | 🔧 + 🎛️ AMD GPU | the default is no longer x264 on CPU (the FFmpeg patch is already in the tree) | [plan](plan-latency.md) |
| #28-3 | `multi_client=false` — single session, lowest latency | 🧭 decision | 🧭 operator | tension with #27: a second client gets a 409 | [plan](plan-latency.md) |
| #17-P2 | Vertical-screen rotation (GPU transpose) | ⏳ blocked | 🎛️ **a vertical screen** + 🔧 ~1 h 30 | root cause is already traced — this needs the hardware, not the analysis | [plan](plan-remote-desktop.md) |
| #17-P3 | Pointer acceleration, `Ctrl+Alt+F`, resize diagnostics | 📋 ready | 🔧 | — | [plan](plan-remote-desktop.md) |
| #25 | Reduce fork divergence, push fixes upstream | 📋 ready | 🔧 + upstream contact | first wave: the ~15 pure bugfixes already inventoried (the lavd series ×7, X/Y scale, fractional deltas, BGRA, 0×0). There is **no FFmpeg or VLC (C) divergence at all** — the "vlc divergence" is two commits in vlc-rs | [audit § 2](audit-fork-chain.md) · [plans](plans-fork-restructure.md) |
| #36 | Migrate `KYBER_CONFIG_PATH` → `KYBER_CONFIG` | 📋 ready | 🔧 | upstream 0.27 implements it natively; the two legacy shims can then be dropped | — |
| #37 | A clean `local-0.27` build image | 📋 ready | 🔧 | a proper derived image instead of patching meson in with pip | — |
| #24 | Get out of the nested fork chain | 🧭 decision | 🧭 operator | audit and plans A/B/C delivered 2026-07-07, recommended sequence A → C → B? — **waiting on the call since** | [audit](audit-fork-chain.md) · [plans](plans-fork-restructure.md) |
| #1 | Per-monitor output targeting | ⏳ blocked | upstream kyclient/winit | kyclient's `set_fullscreen` uses `Fullscreen::Borderless(None)`, so it can only ever fullscreen on the *current* monitor. The fix is upstream: enumerate `available_monitors()`, add `--output-monitor <idx>`, place the window, then `Borderless(Some(monitor))` — after which the UI gets a dropdown. **Not the same thing as #18-B**, which picks the *source* screen on the emitter | — |

### Linux

Linux amd64 shipped: a `.deb` is built and released alongside the Windows
installer. What is listed here is what did **not** make the first iteration.
Chantier detail: [plan](plan-linux-amd64.md) · [per-feature status](todo-linux.md).

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #32 | V4L2 camera enumeration | 📋 ready | 🔧 | `cameras.rs` returns real devices, and `EnumerateDisplays` honours a pinned camera as it does on Windows | — |
| #33 | VAAPI encoding, and the hardcoded `scale=w=1920` | 📋 ready | 🎛️ Linux box | run the check first (validation queue), then drop the forced scale | — |
| #42 | mDNS firewall rule, Linux equivalent | 📋 ready | 💻 | either nothing is needed and it is documented, or the `.deb` ships the rule | — |
| #40 | No orphan children when the supervisor dies | 📋 ready | 🎛️ Linux VM | see the validation queue | — |
| #41 | Viewer fullscreen and `--display-idx` | 📋 ready | 🎛️ Linux VM | see the validation queue | — |
| #34 | Desktop integration: tray and native window | 🧭 decision | 🧭 operator | wry/webkit2gtk + libappindicator, or "the browser is the UI on Linux" — pick one | — |
| #30 | `/tmp/kyber` is hardcoded | ⏳ blocked | upstream **`kyutil`** *(not one of our forks)* | the real fix is `$XDG_RUNTIME_DIR/kyber` upstream — related to #25 | — |
| #31 | `libpulse` aborts with no audio server | ⏳ blocked | 🔧 | blocking for a headless, silent box | — |
| #35 | arm64 | 🧊 icebox | 🎛️ arm64 hardware | out of scope for this iteration — § 7 of the Linux plan | [plan](plan-linux-amd64.md) |

### Project-wide

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #38 | Broader unit-test coverage *(was `C2`)* | 📋 ready | 💻 | the targets listed in [Contributing](contributing.md#where-to-put-tests) have tests | [card](#38-broader-unit-test-coverage) |
| #43 | First real run of the Linux CI jobs and `release-deb` | 📋 ready | 🎛️ a push and a tag | see the validation queue | [releasing](releasing.md) |
| #45 | Pin the docs build image | 📋 ready | 💻 | the `pages` job runs `squidfunk/mkdocs-material:latest`, so the site build can break without a single commit on our side — and upstream has announced that MkDocs 2.0 removes plugins and theme overrides outright. Pin a version and bump it deliberately. *(The theme deliberately uses no template override, so only the plugin list is exposed.)* | — |
| #44 | Bilingual documentation site (EN + FR) | 📋 ready | 💻 | the **user manual** is readable in French and in English. `#11` was closed as "MkDocs site EN+FR", but the site is English only: `language: en`, no i18n plugin, not one French page. Developer docs stay English-only on purpose | — |
| #39 | kyberfrog-cast — define the use cases | 🧭 decision | 🧭 operator | the concrete use cases are written down and the features ranked. The technical core (phone camera → Kyber → PC) is **already proven**; this is a scoping job, not an engineering one | — |

## Take one of these first

The five items below need nothing but a laptop. Each one is self-contained and
touches a part of the app you can see working.

### #2 — Live log streaming

**Why** the UI re-fetches the last N lines on a timer. It works, but the view
jumps and every tick starts over. Poll and SSE read the same source — the log
files — so this is purely additive and can't regress the existing path.
**Where** `kyberfrog/src/web.rs` for the new route, `ui/src/api.ts` to swap
`setInterval` + `fetch` for an `EventSource`. The log-reading helper is reused
unchanged.
**Done when** `GET /logs/stream` serves `text/event-stream` and pushes new lines
as they are written, the UI no longer polls, and the old endpoint still works as
a fallback.
**How to test** `cargo test` plus the app running locally. No hardware.

### #3 — Credentials in the UI

**Why** every generated config uses the transparent default (see `DEFAULT_AUTH_*`
in `shared`). That is the right default for a trusted LAN — the operator types
nothing — but there is no way to override it without editing the TOML by hand.
**Where** `shared/src/config.rs` for the optional fields, `shared/src/gen.rs`
for the emission side, the kyclient argument builder for the reception side,
plus the two forms in the web UI.
**Done when** leaving the fields empty keeps today's behaviour exactly, and
filling them overrides the default on both halves.
**How to test** `cargo test` — the round-trip tests in `shared` are the natural
home. Watch the **kyclient argument order**: the positional server IP goes last.

### #22 — Consistent hover state

**Why** the cockpit has no hover feedback at all, because every button is styled
with inline `style={{…}}`, which cannot express `:hover`.
**Where** the architecture was settled on 2026-07-14 and is written out in full
in [plan-ui-hover.md](plan-ui-hover.md): state tokens in `global.css`, a
`ui/src/buttons.css` with a `.kf-btn` base and four variants, and a small `<Btn>`
component. Migrate in five steps, one commit each — the app stays coherent
between them.
**Done when** the ~35 buttons across 8 files go through `<Btn>`, and the local
`iconBtnStyle` / `textBtnStyle` / `tbBtn` / `BarBtn` constants are gone.
**How to test** by eye, in the app. Keyboard navigation comes along for free via
`:focus-visible`.

### #29 — Self-host the web fonts

**Why** `ui/index.html` fetches Londrina Solid and Inter from
`fonts.googleapis.com` every time the dashboard opens. KyberFrog is a
LAN-only, self-hosted app that regularly runs in venues with no internet — where
it silently loses its typefaces. It is also a privacy leak flagged by `lintian`
(`privacy-breach-generic`) on the Debian package.
**Where** `ui/index.html` and the built `ui/dist/index.html`; the font files
themselves go next to the other UI assets and get served by the embedded axum
server.
**Done when** the dashboard renders correctly with the machine's network cable
unplugged, and no request leaves for `fonts.gstatic.com`.
**How to test** open the dashboard offline, or watch the network tab. This is
the smallest item on the board and it affects **Windows and Linux alike**.

### #38 — Broader unit-test coverage

**Why** the `shared` crate is pure — no Win32 — so it compiles and runs on the
Linux container target, which is exactly where CI runs. Test coverage there is
cheap and it gates the `installer` job.
**Where** the targets are already listed in
[Contributing → where to put tests](contributing.md#where-to-put-tests):
`Globals::kyclient_args()` ordering and flags, `render_config()` layering edge
cases, and the testable logic in `app.rs` (`resolve_port`, `resolve_viewer_id`).
**Done when** those three have tests and they run in the container.
**How to test** `docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test --workspace --locked`.

## Numbering, and where shipped items go

`#N` are **stable identifiers**. They are referenced from `CLAUDE.md`, from
commit messages and from merge requests, so:

- a new item takes the next free number — never a recycled one;
- a number is **never renumbered**, even when the item is rewritten or split;
- when an item ships it leaves this page for
  [`backlog-archive.md`](backlog-archive.md), keeping its number, and the
  release-facing story goes in the `CHANGELOG.md`.

Sub-items use the parent number with a suffix (`#18-D`, `#28-1`, `#17-P2`) and
never a namespace of their own.

!!! warning "`#N` here is a backlog item, not a GitLab issue"
    GitLab renders `#1` in a commit message or a description as a link to
    **issue** #1, while `#1` in this project's docs and history has always meant
    **backlog item** #1. The two numbering spaces are independent and they do
    overlap today. Write `issue #N` (or paste the URL) whenever you mean the
    tracker, and keep bare `#N` for the board.
