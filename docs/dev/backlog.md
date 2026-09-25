---
hide:
  - navigation
  - toc
---

# Backlog

<p class="kf-lede">Everything still to do on KyberFrog, on one board. A column is
where an item stands, a colour is the part of the product it touches, and every
card links to the doc that gives the context. Shipped work is in the
<a href="https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/CHANGELOG.md">CHANGELOG</a>.</p>

<div class="kf-stats">
  <a class="kf-stat" href="#col-run"><b>3</b><span>to run — no code</span></a>
  <a class="kf-stat" href="#col-laptop"><b>8</b><span>ready · laptop</span></a>
  <a class="kf-stat" href="#col-fork"><b>8</b><span>ready · fork &amp; hardware</span></a>
  <a class="kf-stat" href="#col-progress"><b>2</b><span>in progress</span></a>
  <a class="kf-stat" href="#col-waiting"><b>12</b><span>waiting</span></a>
</div>

<div class="kf-areas">
  <div class="kf-areabar" aria-hidden="true">
    <i class="kf-core" style="flex-grow:6"></i><i class="kf-ui" style="flex-grow:5"></i><i class="kf-fork" style="flex-grow:9"></i><i class="kf-linux" style="flex-grow:10"></i><i class="kf-proj" style="flex-grow:4"></i>
  </div>
  <a class="kf-core" href="#product-core-emission-and-reception">Product core <b>6</b></a>
  <a class="kf-ui" href="#web-ui">Web UI <b>5</b></a>
  <a class="kf-fork" href="#fork-chain-and-latency">Fork chain &amp; latency <b>9</b></a>
  <a class="kf-linux" href="#linux">Linux <b>10</b></a>
  <a class="kf-proj" href="#project-wide">Project-wide <b>4</b></a>
</div>

<div class="kf-board" markdown>

<section class="kf-col" id="col-run" markdown>
<header class="kf-col-head"><span>🧪 To run</span><b>3</b></header>
<p class="kf-col-note">Built, never exercised. No code — a machine and ten minutes.</p>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#17-B5</span><span>🎛️ remote session</span></p>
<p class="kf-card-title"><kbd>Ctrl+Alt+F</kbd> under keyboard grab</p>
<p class="kf-card-what">Does the fullscreen escape still work while a remote session holds the keyboard?</p>
<p class="kf-card-links" markdown="span">[Remote desktop](plan-remote-desktop.md)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#33 check</span><span>🎛️ Linux box</span></p>
<p class="kf-card-title">Is VAAPI usable?</p>
<p class="kf-card-what"><code>vainfo</code> decides whether GPU encoding on Linux is worth writing.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#41</span><span>🎛️ Linux VM</span></p>
<p class="kf-card-title">Linux viewer flags</p>
<p class="kf-card-what">Fullscreen really goes fullscreen; <code>--display-idx</code> picks the right screen.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

</section>

<section class="kf-col" id="col-laptop" markdown>
<header class="kf-col-head"><span>📋 Ready · laptop</span><b>8</b></header>
<p class="kf-col-note">Rust, React and the MinGW image — nothing else. <strong>Start here.</strong></p>

<div class="kf-card kf-ui" id="item-29" markdown>
<p class="kf-card-head"><span>#29</span><span>💻</span></p>
<p class="kf-card-title">Self-host the web fonts</p>
<p class="kf-card-what">The dashboard loses its typefaces on a venue LAN with no internet. The smallest item on the board.</p>
<details class="kf-more" markdown>
<summary>Why, where, done when</summary>

**Why** `ui/index.html` fetches Londrina Solid and Inter from
`fonts.googleapis.com` at every open. KyberFrog is LAN-only and runs in venues
with no internet; it is also a privacy leak `lintian` flags on the `.deb`
(`privacy-breach-generic`).

**Where** `ui/index.html` and the built `ui/dist/index.html`; the font files go
next to the other UI assets, served by the embedded axum server.

**Done when** the dashboard renders correctly with the network cable unplugged
and no request leaves for `fonts.gstatic.com` — on Windows and Linux alike.

</details>
</div>

<div class="kf-card kf-ui" id="item-2" markdown>
<p class="kf-card-head"><span>#2</span><span>💻</span></p>
<p class="kf-card-title">Live log streaming</p>
<p class="kf-card-what">The log panel receives lines as they are written, instead of re-fetching on a timer.</p>
<details class="kf-more" markdown>
<summary>Why, where, done when</summary>

**Why** the UI re-fetches the last N lines on a timer: the view jumps and every
tick starts over. Poll and SSE read the same log files, so this is purely
additive.

**Where** `kyberfrog/src/web.rs` for the route, `ui/src/api.ts` to swap
`setInterval` + `fetch` for an `EventSource`. The log-reading helper is reused.

**Done when** `GET /logs/stream` serves `text/event-stream`, the UI no longer
polls, and the old endpoint still works as a fallback. Test: `cargo test` and
the app running locally.

</details>
</div>

<div class="kf-card kf-ui" id="item-3" markdown>
<p class="kf-card-head"><span>#3</span><span>💻</span></p>
<p class="kf-card-title">Credentials in the UI</p>
<p class="kf-card-what">Optional login fields that override the transparent <code>vj / kyberfrog</code> default.</p>
<details class="kf-more" markdown>
<summary>Why, where, done when</summary>

**Why** the transparent default is right on a trusted LAN, but overriding it
today means editing the TOML by hand.

**Where** `shared/src/config.rs` (optional fields), `shared/src/gen.rs`
(emission), the kyclient argument builder (reception), the two forms in the UI.

**Done when** empty fields keep today's behaviour exactly and filled ones
override both halves. Test with the round-trip tests in `shared` — and keep the
positional server IP **last** in the kyclient arguments.

</details>
<p class="kf-card-links" markdown="span">[Transparent auth](architecture.md#transparent-auth)</p>
</div>

<div class="kf-card kf-ui" id="item-22" markdown>
<p class="kf-card-head"><span>#22</span><span>💻</span></p>
<p class="kf-card-title">Hover on every button</p>
<p class="kf-card-what">The cockpit has no hover feedback at all. One button system, ~35 buttons, 5 commits.</p>
<details class="kf-more" markdown>
<summary>Why, where, done when</summary>

**Why** every button is styled inline (`style={{…}}`), which cannot express
`:hover`.

**Where** state tokens in `global.css`, `ui/src/buttons.css` with a `.kf-btn`
base and four variants, a small `<Btn>` component — migrated in five commits.

**Done when** the ~35 buttons across 8 files go through `<Btn>` and the local
`iconBtnStyle` / `textBtnStyle` / `tbBtn` / `BarBtn` constants are gone. Test
by eye; keyboard focus comes for free.

</details>
<p class="kf-card-links" markdown="span">[Design](plan-ui-hover.md)</p>
</div>

<div class="kf-card kf-proj" id="item-38" markdown>
<p class="kf-card-head"><span>#38</span><span>💻</span></p>
<p class="kf-card-title">More unit tests</p>
<p class="kf-card-what">Cover the pure <code>shared</code> crate, where tests are cheap and gate the installer.</p>
<details class="kf-more" markdown>
<summary>Why, where, done when</summary>

**Why** `shared` has no Win32, so its tests run on the Linux container target —
exactly where CI runs.

**Where** `Globals::kyclient_args()` ordering and flags, `render_config()`
layering edge cases, `resolve_port` / `resolve_viewer_id` in `app.rs`.

**Done when** those three have tests running in
`docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test --workspace --locked`.

</details>
<p class="kf-card-links" markdown="span">[Where to put tests](contributing.md#where-to-put-tests)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#42</span><span>💻</span></p>
<p class="kf-card-title">mDNS firewall on Linux</p>
<p class="kf-card-what">Document that nothing is needed, or ship the rule in the <code>.deb</code>.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

<div class="kf-card kf-proj" markdown>
<p class="kf-card-head"><span>#44</span><span>💻</span></p>
<p class="kf-card-title">French user manual</p>
<p class="kf-card-what">The user manual readable in French as well as English. Dev docs stay English.</p>
<p class="kf-card-links" markdown="span">[User manual](../user/index.md)</p>
</div>

<div class="kf-card kf-proj" markdown>
<p class="kf-card-head"><span>#45</span><span>💻</span></p>
<p class="kf-card-title">Pin the docs build image</p>
<p class="kf-card-what">The site builds on <code>mkdocs-material:latest</code> and can break without a commit of ours.</p>
<p class="kf-card-links" markdown="span">[Docs site](releasing.md#documentation-site)</p>
</div>
</section>

<section class="kf-col" id="col-fork" markdown>
<header class="kf-col-head"><span>📋 Ready · fork &amp; hardware</span><b>7</b></header>
<p class="kf-col-note">Needs the fork chain (~1 h 30 build) and sometimes a specific machine.</p>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#25</span><span>🔧 🧭</span></p>
<p class="kf-card-title">Push fixes upstream</p>
<p class="kf-card-what">Wave 1 is ready on rebased branches. Waits on the call on contribution identity and licence.</p>
<p class="kf-card-links" markdown="span">[Inventory](audit-fork-chain.md) · [Process](plans-fork-restructure.md#remontee-amont-25)</p>
</div>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#17-P3</span><span>🔧</span></p>
<p class="kf-card-title">Remote desktop polish</p>
<p class="kf-card-what">Pointer acceleration, the <kbd>Ctrl+Alt+F</kbd> hook, resize diagnostics.</p>
<p class="kf-card-links" markdown="span">[Remote desktop](plan-remote-desktop.md#phase-3-polish)</p>
</div>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#36</span><span>🔧</span></p>
<p class="kf-card-title"><code>KYBER_CONFIG</code> migration</p>
<p class="kf-card-what">Use upstream's native variable and drop two legacy fork shims.</p>
<p class="kf-card-links" markdown="span">[The fork](architecture.md#relationship-to-the-kyber-fork)</p>
</div>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#37</span><span>🔧</span></p>
<p class="kf-card-title">Clean <code>local-0.27</code> build image</p>
<p class="kf-card-what">A proper derived MinGW image instead of patching meson in with pip.</p>
<p class="kf-card-links" markdown="span">[Building](building.md)</p>
</div>

<div class="kf-card kf-core" markdown>
<p class="kf-card-head"><span>#18-E</span><span>🔧 🧭</span></p>
<p class="kf-card-title">NDI output</p>
<p class="kf-card-what">A viewer republishes its stream as an NDI source for OBS, vMix or a switcher — the Spout relay's CPU path with an NDI sender plugged in. Needs the call on the NDI SDK licence.</p>
<p class="kf-card-links" markdown="span">[Design](plan-sources-exports.md#18-e-ndi-output-sur-le-chemin-de-la-sortie-spout)</p>
</div>

<div class="kf-card kf-core" markdown>
<p class="kf-card-head"><span>#18-D/F</span><span>🔧</span></p>
<p class="kf-card-title">SRT / RTSP in and out</p>
<p class="kf-card-what">IP cameras as a source, re-streaming as an export. FFmpeg already speaks both.</p>
<p class="kf-card-links" markdown="span">[Sources &amp; exports](plan-sources-exports.md)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#33</span><span>🔧 🎛️ Linux</span></p>
<p class="kf-card-title">VAAPI encoding</p>
<p class="kf-card-what">GPU encoding on Linux and no more forced 1920 scale — once the check says yes.</p>
<p class="kf-card-links" markdown="span">[Linux architecture](plan-linux-amd64.md)</p>
</div>
</section>

<section class="kf-col" id="col-progress" markdown>
<header class="kf-col-head"><span>🚧 In progress</span><b>3</b></header>
<p class="kf-col-note">Someone is on it — check the linked MR before starting.</p>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#32</span><span>🎛️ Linux VM</span></p>
<p class="kf-card-title">V4L2 cameras</p>
<p class="kf-card-what">Webcam picker and camera pinning on Linux. Code done on both sides (KyberFrog + kymedia); end-to-end run on a VM with v4l2loopback left. Branch <code>feat/v4l2-cameras</code>.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#28-1</span><span>💻 🎛️ AMD GPU</span></p>
<p class="kf-card-title">GPU encoder by default <span class="kf-badge">0.6.0</span></p>
<p class="kf-card-what">Encoder setting in Options (Auto = AMF / NVENC from the main GPU, else x264). Measured ~4 ms Spout → Spout instead of ~26 ms. Branch <code>feat/gpu-encoder-default</code>.</p>
<p class="kf-card-links" markdown="span">[Latency](plan-latency.md#1-emission-encodeur-gpu-28-1)</p>
</div>

<div class="kf-card kf-core" markdown>
<p class="kf-card-head"><span>#27</span><span>🎛️ dev box</span></p>
<p class="kf-card-title">Spout passthrough <span class="kf-badge">beta</span></p>
<p class="kf-card-what">One switch publishes every local Spout sender as its own transmitter — Kyber used like NDI from Resolume or TouchDesigner.</p>
<p class="kf-card-links" markdown="span">[Design &amp; beta plan](plan-spout-passthrough.md)</p>
</div>
</section>

<section class="kf-col" id="col-waiting" markdown>
<header class="kf-col-head"><span>⏸ Waiting</span><b>12</b></header>
<p class="kf-col-note">Do not start these: each one waits on something outside the code.</p>

<p class="kf-sub">⏳ Blocked</p>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#17-P2</span><span>🎛️ vertical screen</span></p>
<p class="kf-card-title">Remote control on vertical screens</p>
<p class="kf-card-what">Clicks land 90° off. The fix is known; it needs the hardware.</p>
<p class="kf-card-links" markdown="span">[Remote desktop](plan-remote-desktop.md#phase-2-rotation-b1)</p>
</div>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#1</span><span>upstream</span></p>
<p class="kf-card-title">Per-monitor output</p>
<p class="kf-card-what">Choose which monitor a viewer fullscreens on. Needs a kyclient / winit change upstream.</p>
<p class="kf-card-links" markdown="span">[Detail](#fork-chain-and-latency)</p>
</div>

<div class="kf-card kf-core" markdown>
<p class="kf-card-head"><span>#18-C</span><span>🔧 after #18-E</span></p>
<p class="kf-card-title">NDI input</p>
<p class="kf-card-what">An NDI source into Kyber needs a new txproto capture path — FFmpeg has none. Starts once #18-E has settled the licence.</p>
<p class="kf-card-links" markdown="span">[Sources &amp; exports](plan-sources-exports.md)</p>
</div>

<div class="kf-card kf-linux" id="item-35" markdown>
<p class="kf-card-head"><span>#35</span><span>🎛️ Pi 5</span></p>
<p class="kf-card-title">arm64 — S1 of #46</p>
<p class="kf-card-what">The aarch64 bundle is built and published, the <code>.deb</code> too: <code>ARM aarch64</code>, nothing above <code>GLIBC_2.39</code>. Waiting on a Pi 5 to confirm the install.</p>
<p class="kf-card-links" markdown="span">[arm64](plan-linux-amd64.md#arm64)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#30</span><span>upstream kyutil</span></p>
<p class="kf-card-title"><code>/tmp/kyber</code> is hard-coded</p>
<p class="kf-card-what">A shared IPC directory that one user can block for all the others.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#31</span><span>🔧</span></p>
<p class="kf-card-title">No sound server, no transmitter</p>
<p class="kf-card-what"><code>libpulse</code> aborts on a silent headless box and takes kyavserver with it.</p>
<p class="kf-card-links" markdown="span">[Linux status](todo-linux.md)</p>
</div>

<p class="kf-sub">🧭 Decision</p>

<div class="kf-card kf-fork" markdown>
<p class="kf-card-head"><span>#28-3</span><span>🧭</span></p>
<p class="kf-card-title">Single-session mode</p>
<p class="kf-card-what">Lowest latency, but a second client gets a 409 — in tension with #27.</p>
<p class="kf-card-links" markdown="span">[Latency](plan-latency.md#3-protocole-session-unique-28-3)</p>
</div>

<div class="kf-card kf-linux" markdown>
<p class="kf-card-head"><span>#34</span><span>🧭</span></p>
<p class="kf-card-title">Linux tray and native window</p>
<p class="kf-card-what">webkit2gtk + libappindicator, or the browser stays the UI on Linux.</p>
<p class="kf-card-links" markdown="span">[Linux architecture](plan-linux-amd64.md)</p>
</div>

<div class="kf-card kf-linux" id="item-46" markdown>
<p class="kf-card-head"><span>#46</span><span>🧭 🎛️ Pi 5</span></p>
<p class="kf-card-title">KyberFrog Satellite</p>
<p class="kf-card-what">A flash-and-plug Pi 5 + C790 image: any 1080p60 HDMI source becomes a transmitter. Five calls, then a no-build go / no-go on software encoding.</p>
<p class="kf-card-links" markdown="span">[Plan](https://gitlab.com/kyber-frog/kyberfrog-satellite/-/blob/main/docs/plan.md)</p>
</div>

<div class="kf-card kf-ui" markdown>
<p class="kf-card-head"><span>#23</span><span>🧭</span></p>
<p class="kf-card-title">Drawers → modals</p>
<p class="kf-card-what">A product call to make before any code.</p>
</div>

<div class="kf-card kf-proj" markdown>
<p class="kf-card-head"><span>#39</span><span>🧭</span></p>
<p class="kf-card-title">kyberfrog-cast use cases</p>
<p class="kf-card-what">Phone camera → Kyber → PC already works. What is the product?</p>
</div>

<p class="kf-sub">🧊 Icebox</p>

<div class="kf-card kf-core" markdown>
<p class="kf-card-head"><span>#26</span><span>🔧</span></p>
<p class="kf-card-title">Emitter-pinned screen</p>
<p class="kf-card-what">A transmitter imposing its screen on every client. No expressed need.</p>
<p class="kf-card-links" markdown="span">[Recipe](plan-sources-exports.md#26-ecran-source-fige-cote-emetteur-icebox)</p>
</div>
</section>

</div>

## How to read this

**Columns are states, colours are areas.** The top-right corner of a card says
what you need to have in order to take it — on this project that is the real
filter, far more than difficulty. A card with a *Why, where, done when* fold is
self-contained: open it and you can start.

| State | Meaning |
|---|---|
| 🧪 **to run** | built, needs one run on the right machine — see the [validation queue](#validation-queue-no-code-just-a-run) |
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

**Taking one?** Open an issue from the *Backlog item* template, move the card to
🚧 *In progress* with a link to the issue, and flip its row below — the flow is
in [Contributing → taking an item](contributing.md#taking-an-item). This page
stays the source of truth; the tracker only carries what is actively being
worked on.

**Changing an item** means two edits on this page: move its card on the board,
and update its row in the detail below. Keep the counts at the top in step.

## Validation queue — no code, just a run

These are **not development tasks**. Every one of them is a thing that was built
and needs to be exercised once, by someone with the right machine in front of
them. They are listed separately because they clear fast and they unblock
honest status elsewhere on the board.

| ID | What to run | Needs | What to record |
|---|---|---|---|
| #17-B5 | `Ctrl+Alt+F` while keyboard grab is active — it has **never been proven broken**, only assumed | Windows + a remote session | works / does not work, and with which exact combo |
| #33-check | Is VAAPI available and usable on the Linux box (Intel/AMD amd64)? | Linux machine | `vainfo` output — the fix is only worth writing if the answer is yes |
| #41 | Linux viewer: fullscreen actually goes fullscreen, and `--display-idx` picks the right screen | Linux VM, 2 screens ideally | pass / fail per flag |
| #32 | Linux camera end to end: `modprobe v4l2loopback card_label="KF Test Cam" exclusive_caps=1`, feed it `ffmpeg -re -f lavfi -i testsrc=size=1280x720:rate=30 -pix_fmt yuv420p -f v4l2 /dev/videoN`, then add a webcam transmitter and view it | Linux VM, `.deb` of `feat/v4l2-cameras` | the picker lists `KF Test Cam`; the viewer's source list holds the camera only, no screen; the test pattern is received |

Once a line here is done, tick it off the board and — if it changes a state —
move the item. Nothing else on this page depends on writing code to be true.

## Detail by area

### Product core — emission and reception

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #27 | Spout passthrough — **beta test** | 🚧 in progress | 🎛️ dev box *(Resolume and TD are installed there, loopback via `is_self`)* | one switch, **emitter side**: every local Spout sender becomes its own transmitter. Done when the beta validation plan passes | [plan](plan-spout-passthrough.md) |
| #18-D/F | SRT / RTSP input and output | 📋 ready | 🔧 fork chain | txproto accepts an `rtsp://` / `srt://` URL, `Source::Url` variant exists — FFmpeg already supports both, so expect little fork code | [plan](plan-sources-exports.md) |
| #18-E | NDI output | 📋 ready | 🔧 fork chain + 🧭 operator | a viewer's *Redirection NDI* shows up as an NDI source in OBS or NDI Studio Monitor. It reuses the Spout relay's CPU path (smem BGRA frames) with an NDI sender, loading the machine's NDI runtime. Before the release: the operator's call on the NDI SDK licence | [plan](plan-sources-exports.md#18-e-ndi-output-sur-le-chemin-de-la-sortie-spout) |
| #18-C | NDI input | ⏳ blocked | 🔧 fork chain | FFmpeg has no NDI input, so this is a new txproto iosys on the NDI SDK. Waits on #18-E settling the licence question | [plan](plan-sources-exports.md) |
| #26 | Emitter-pinned source screen | 🧊 icebox | 🔧 | no expressed need — #18-B covers the use case today. The fork recipe is written down in case the field ever asks for it | [plan](plan-sources-exports.md) |

### Web UI

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #22 | Consistent hover state on every button | 📋 ready | 💻 | the ~35 buttons across 8 files go through `.kf-btn`; nothing blocks it since #21 shipped | [card](#item-22) |
| #29 | Self-host the web fonts | 📋 ready | 💻 | `ui/index.html` no longer calls `fonts.googleapis.com` — a venue LAN with no internet keeps its typefaces | [card](#item-29) |
| #2 | Live log streaming (SSE) instead of polling | 📋 ready | 💻 | `GET /logs/stream` pushes new lines, the UI drops its `setInterval` | [card](#item-2) |
| #3 | Credentials in the UI | 📋 ready | 💻 | optional per-viewer / per-transmitter fields override the transparent default | [card](#item-3) |
| #23 | Drawers → modals | 🧭 decision | 🧭 operator | **do not write code before the call is made** | — |

### Fork chain and latency

> Latency is the project's **number one priority**. Anything here that reduces it
> outranks a feature.

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #28-1 | GPU encoder by default: AMF / NVENC, encoder setting in Options | 🚧 in progress (`feat/gpu-encoder-default`, release 0.6.0) | 💻 + 🎛️ AMD GPU | no fork change needed: kyavservice already supports `amf` / `nvenc`. Bench: ~4 ms Spout → Spout vs ~26 ms with x264; the old AMF crash is not reproduced with the current bundle. Still to check: visual quality, several transmitters, screen source | [plan](plan-latency.md) · [bench](bench-latency.md) |
| #28-3 | `multi_client=false` — single session, lowest latency | 🧭 decision | 🧭 operator | tension with #27: a second client gets a 409 | [plan](plan-latency.md) |
| #17-P2 | Vertical-screen rotation (GPU transpose) | ⏳ blocked | 🎛️ **a vertical screen** + 🔧 ~1 h 30 | root cause is already traced — this needs the hardware, not the analysis | [plan](plan-remote-desktop.md) |
| #17-P3 | Pointer acceleration, `Ctrl+Alt+F`, resize diagnostics | 📋 ready | 🔧 | — | [plan](plan-remote-desktop.md) |
| #25 | Reduce fork divergence, push fixes upstream | 📋 ready | 🔧 + 🧭 operator | wave 1 is prepared on rebased branches (the lavd series, X/Y scale, fractional deltas, 0×0 sources) — based on 0.27, to rebase onto 0.28.0 like the chain (2026-09-25). Before the MRs: a validation build, the GitLab fork relation, and the operator's call on contribution identity and licence. There is **no FFmpeg or VLC (C) divergence at all** | [inventory](audit-fork-chain.md) · [process](plans-fork-restructure.md#remontee-amont-25) |
| #36 | Migrate `KYBER_CONFIG_PATH` → `KYBER_CONFIG` | 📋 ready | 🔧 | upstream 0.27 implements it natively; the two legacy shims can then be dropped | — |
| #37 | A clean `local-0.27` build image | 📋 ready | 🔧 | a proper derived image instead of patching meson in with pip | — |
| #1 | Per-monitor output targeting | ⏳ blocked | upstream kyclient/winit | kyclient's `set_fullscreen` uses `Fullscreen::Borderless(None)`, so it can only ever fullscreen on the *current* monitor. The fix is upstream: enumerate `available_monitors()`, add `--output-monitor <idx>`, place the window, then `Borderless(Some(monitor))` — after which the UI gets a dropdown. **Not the same thing as #18-B**, which picks the *source* screen on the emitter | — |

### Linux

Linux amd64 ships as a `.deb` built and released alongside the Windows
installer. What is listed here is what the port does not cover yet.
Detail: [architecture](plan-linux-amd64.md) · [per-feature status](todo-linux.md).

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #32 | V4L2 camera enumeration | 🚧 in progress (`feat/v4l2-cameras`) | 🎛️ Linux VM | `cameras.rs` returns real devices, and `EnumerateDisplays` honours a pinned camera as it does on Windows. Step S3 of #46 (the C790 is a V4L2 device) | — |
| #33 | VAAPI encoding, and the hardcoded `scale=w=1920` | 📋 ready | 🎛️ Linux box | run the check first (validation queue), then drop the forced scale | — |
| #42 | mDNS firewall rule, Linux equivalent | 📋 ready | 💻 | either nothing is needed and it is documented, or the `.deb` ships the rule | — |
| #41 | Viewer fullscreen and `--display-idx` | 📋 ready | 🎛️ Linux VM | see the validation queue | — |
| #34 | Desktop integration: tray and native window | 🧭 decision | 🧭 operator | wry/webkit2gtk + libappindicator, or "the browser is the UI on Linux" — pick one | — |
| #30 | `/tmp/kyber` is hardcoded | ⏳ blocked | upstream **`kyutil`** *(not one of our forks)* | the real fix is `$XDG_RUNTIME_DIR/kyber` upstream — related to #25 | — |
| #31 | `libpulse` aborts with no audio server | ⏳ blocked | 🔧 | blocking for a headless, silent box | — |
| #46 | KyberFrog Satellite — Pi 5 + C790 HDMI-in transmitter image | 🧭 decision | 🧭 operator + 🎛️ Pi 5 + C790 | the five open calls of the study are made, and the S0 go / no-go (sustained 1080p60 x264 `ultrafast` on the Pi, no hardware encoder) is recorded. Taking it on pulls #35 out of the icebox and makes #32 a prerequisite | [plan](https://gitlab.com/kyber-frog/kyberfrog-satellite/-/blob/main/docs/plan.md) |
| #35 | arm64 — S1 of #46 | ⏸ blocked | 🎛️ Pi 5 | the chain is built: the three `ARCH_TRIPLET` commits are cherry-picked onto `feat/arm64-triplet` in the fork repos, the `vendor/kyber-desktop` gitlink pins the new SHA, and `build-fork-linux-arm64` → `deb-arm64` produce and check the package (`file` = ARM aarch64, no symbol above `GLIBC_2.41`). What is missing is a Pi: the install itself is unconfirmed, and so is the x264-only performance that gates #46 S0 | [plan](plan-linux-amd64.md#arm64) |

### Project-wide

| ID | Item | State | Access | Done when | Detail |
|---|---|---|---|---|---|
| #38 | Broader unit-test coverage | 📋 ready | 💻 | the targets listed in [Contributing](contributing.md#where-to-put-tests) have tests | [card](#item-38) |
| #45 | Pin the docs build image | 📋 ready | 💻 | the `pages` job runs `squidfunk/mkdocs-material:latest`, so the site build can break without a single commit on our side — and upstream has announced that MkDocs 2.0 removes plugins and theme overrides outright. Pin a version and bump it deliberately. *(The theme deliberately uses no template override, so only the plugin list is exposed.)* | — |
| #44 | Bilingual documentation site (EN + FR) | 📋 ready | 💻 | the **user manual** is readable in French and in English. The site is English only today (`language: en`, no i18n plugin). Developer docs stay English-only on purpose | — |
| #39 | kyberfrog-cast — define the use cases | 🧭 decision | 🧭 operator | the concrete use cases are written down and the features ranked. The technical core (phone camera → Kyber → PC) is **already proven**; this is a scoping job, not an engineering one | — |

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
