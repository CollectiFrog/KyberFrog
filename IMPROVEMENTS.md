# KyberFrog — deferred improvements & tech debt

Things intentionally left out of the current POC, to revisit on their own
branches. Each entry says *what*, *why deferred*, and *how* so we don't forget.

> Note: KyberFrog is now **one app** (one supervisor, one web UI on 7700, one
> `kyberfrog.toml`). "the web UI" / "the tray" below mean that single UI.

## Web UI

### 1. Per-monitor output targeting (needs an upstream kyclient change)
- **What:** let an operator pick *which physical monitor* a viewer fullscreens
  on, from a dropdown of detected monitors.
- **Why deferred:** kyclient can't target an output monitor today. Its
  `set_fullscreen` uses `winit::window::Fullscreen::Borderless(None)` →
  fullscreen on the *current* monitor only. (`--display-idx`/`--display-count`
  are about the **source** display on the server, not the client's output.)
- **How:** in kyclient, enumerate `available_monitors()`, add an
  `--output-monitor <idx>` flag, place the window on that monitor then
  `Fullscreen::Borderless(Some(monitor))`. Then expose the dropdown in the web
  UI. v1 ships with fullscreen-on-current / windowed only.

### 2. Real-time log streaming (SSE) — currently polling
- **What:** stream the app + per-child logs live in the UI.
- **Why deferred:** v1 uses simple periodic polling of the last N lines (fine
  for a POC). **Confirmed non-blocking:** poll and SSE share the same source
  (the log files), so this is purely additive.
- **How:** add `GET /logs/stream` (`text/event-stream`) that tails the file and
  pushes new lines; swap the frontend from `setInterval`+`fetch` to
  `EventSource`. The log-reading helper is reused unchanged.

## Auth

### 3. Surface credential management to the user
- **What:** let the operator set per-viewer / per-transmitter username +
  password in the web UI and tray, instead of the baked-in transparent login.
- **Why deferred:** today every config uses the transparent default
  (`vj` / `kyberfrog`, see `DEFAULT_AUTH_*` in `shared`), which is enough for a
  trusted LAN and means the operator types nothing.
- **How:** optional credential fields that, when set, override the transparent
  default in the generated config (emission) / the kyclient args (reception).

## Features (planned, not tech debt)

### 8. Spout output from a viewer (Amélioration 2)
- **What:** let a viewer re-publish the received video as a **Spout sender** so
  other local apps (Resolume, MadMapper) can consume it — a windowless relay
  (use case: TouchDesigner → Spout → KyberFrog → Spout → Resolume Arena).
- **Design:** the desktop client renders video via **libVLC** (`kyvlcplayer`).
  Spout output = a VLC *smem* video callback (CPU BGRA frames) pushed into a
  Spout **sender** (new `kyspout` crate), the mirror of txproto's `iosys_spout.c`
  *receiver*. Fork-side flag `--spout-out <name>`; "Spout only, no window".
- **Status — fork side, branch `feat/spout-output`:**
  - ✅ **kyspout** (new crate `core/kyctl/kyspout`): Spout2 *sender* — D3D11 BGRA
    `MISC_SHARED` texture + `SpoutSenderNames`/per-sender info mapping/access
    mutex/frame semaphore. **Compiles** for `x86_64-pc-windows-gnu`.
  - ✅ **vlc-rs** (fork `kyber-frog/vlc-rs`): safe `set_video_format` +
    `set_video_callbacks` (the smem *video* callbacks; only the audio ones existed,
    and `MediaPlayer.ptr` is `pub(crate)`).
  - ✅ **kyvlcplayer** (`core/kyctl/kyvlcplayer`): when `VideoConfig.spout_out` is
    set, skip `set_hwnd` and route RV32/BGRA frames into a `kyspout::SpoutSender`
    (`setup_spout_output`).
  - ✅ **kyclient lib** (`core/kyctl/kyclient`): `VideoPlayerConfig.spout_out`
    threaded into `player::VideoConfig` (capi default + kymux + rtp backends).
  - ✅ **kyber-desktop** (`apps/kyber-desktop/kyclient` = the `kyclient.exe`
    binary): `--spout-out <name>` clap arg (conflicts with `--fullscreen`) →
    windowless run (no winit window), one video config on the first host display
    routed through a windowless `VideoPlayerConfig` carrying the Spout name,
    inputs disabled. The C-API / `kyclient-rs` plumbing (`set_spout_out`,
    NULL-window `default`) was added in `kyctl` to thread it through the FFI.
    **Built end-to-end**: a self-contained `kyclient.exe` + DLLs/plugins bundle
    was cross-compiled from the pinned fork chain — see
    `docs/E2E-spout-output.md` for the artifact and test procedure.
- **Build wiring done (CI-ready).** The whole fork submodule chain is pinned to
  `feat/spout-output` and pushed: `vlc-rs` (f91eb1f, smem on `kyber-master`) →
  `kymedia` (b628db4) → `kyctl` (81e2818) → `kysdk` (3bd1ff8) → `kyber-desktop`
  (1f1349e). Submodule URLs were made absolute where no fork exists
  (kymux/kynput/kyutil/vlc/winit → `kyber.stream`; kyctl/kymedia/vlc-rs/txproto →
  `kyber-frog`). See #6.
- **E2E validated against Resolume** ✅ — windowless `kyclient --spout-out` is
  seen as a live Spout sender in Resolume Arena. Two bugs found and fixed during
  the first real run:
  - **403 on `start_session`** — the windowless path sent `display_id = index`
    (0); the controller validates display ids against the host list and 403s
    unknown ones (kyber-desktop `de339ff`: capture the real `displays[idx].id`).
  - **Blue tint + brightness-keyed transparency** — `"RV32"` smem output is
    laid out X,R,G,B; copied into the BGRA texture the 0xFF pad hit the blue
    channel and the blue value hit alpha. Fixed by requesting `"BGRA"` (kyctl
    `53df4ad`); VLC then emits B,G,R,A with opaque alpha. **So the chroma is
    settled: `BGRA`, not RV32/RGBA.**
- **v1 limitations still to refine (next chat):**
  - **Fixed output size 1920×1080** — `setup_spout_output` forces it via
    `libvlc_video_set_format`; libVLC scales the stream. Native size needs a
    `set_video_format_callbacks` wrapper in vlc-rs (negotiate w/h at runtime).
  - **CPU round-trip**: smem gives CPU frames, re-uploaded to the GPU texture each
    frame. Zero-copy would use libVLC 4's D3D11 output callbacks — bigger, later.
- ✅ **Step (b) — KyberFrog side done:** `Viewer.spout_out: Option<String>`
  (sender name, empty = off); `Globals::kyclient_args()` emits `--spout-out`
  *instead of* `--fullscreen` when set (they conflict); web UI has a Spout-name
  field on the add form and each viewer row, greying out fullscreen when filled;
  `op_add_viewer`/`op_update_viewer` carry it (trim, empty → None). A viewer with
  a Spout name runs windowless and shows a "Spout · name" badge.
- **Build:** see #6 — the change spans 3 fork repos and must be wired through the
  kysdk/kyber-desktop submodule + `[patch]` chain.

## Release & distribution

### 9. Package release propre & simple d'utilisation — ✅ done
- **Decision (arbitrage du périmètre):** distribution = **un seul
  `KyberFrog-Setup.exe`** (NSIS) bundlant `kyberfrog.exe` + les binaires fork,
  installé en double-clic **sans étape PATH manuelle**, + une **CI/CD GitLab**
  qui le build et le publie en Release sur tag `v*`. Périmètre complet (#6 *et*
  #7), pas la CI seule.
- **Choix techniques:** NSIS (et non Inno) pour réutiliser l'infra éprouvée de
  `apps/kyber-installer` (gestion PATH > 1024, désinstalleur). Tout le build
  tourne dans `kyber/debian-win64:local` (cargo **et** `makensis` y sont).
- **Implémentation:**
  - `packaging/build-installer.sh` — one-shot dev : cargo build + assemble bundle
    + makensis → `dist/KyberFrog-Setup-<ver>.exe`.
  - `packaging/windows/kyberfrog.nsi` + `INSTALL.md`, `packaging/versions.sh`.
  - `.gitlab-ci.yml` — `build-fork` (bundle fork, cache par SHA) → `installer`
    (exe + makensis) → `release` (release-cli sur tag `v*`).
  - Fix `default_install_dir()` → dossier de l'exe : un install bundlé trouve
    `kycontroller.exe` à côté de `kyberfrog.exe` sans éditer la config.
- **Reste (optim, non bloquant):** trimmer le bundle fork (embarqué tel quel :
  `ffmpeg.exe`/`txproto.exe`/`kyservice.exe`/`kynputserver.exe` + les `*.bat`
  service ne servent pas à KyberFrog) ; signer le `.exe` (SmartScreen) ; pinner
  `KYBER_DESKTOP_REF` sur un SHA pour des releases reproductibles.
- **Log kycontroller en lecture seule (contourné).** kycontroller écrit son log
  log4rs dans `<dossier_exe>\log\` (relatif à l'exe via `current_exe()`, pas au
  cwd — cf. `kycontroller/src/main.rs:627` + `utils.rs::get_resource_path`). Dans
  `C:\Program Files` ça panique en `PermissionDenied` (exit 101) sans admin, ce
  qui casserait aussi l'autostart non-élevé. **Contournement packaging:**
  l'installeur pré-crée `$INSTDIR\log` et donne *Modify* au groupe Users dessus.
  **Fix propre (fork, plus tard):** aligner kycontroller sur kyclient et logger
  dans `%LOCALAPPDATA%\Kyber\log` ; supprime le besoin du `icacls`. Idem à
  vérifier pour kyavserver. (kycontroller résout tout le reste — certs, sous-exes
  — relativement à l'exe : lecture seule OK, seul le log écrit.)

### 7. CI/CD GitLab — ✅ done (voir #9)
- `.gitlab-ci.yml` : `build-fork` clone+build `kyber-frog/kyber-desktop` (cache
  Generic Package Registry par SHA résolu), `installer` produit le setup, et
  `release` (`if: $CI_COMMIT_TAG =~ /^v/`) l'upload puis crée la Release GitLab
  avec l'asset via le bloc `release:` + lien package. La page releases du README
  pointe dessus.

### 6. Windows installer (single-click setup) — ✅ done (voir #9)
- `packaging/windows/kyberfrog.nsi` : installe dans `C:\Program Files\KyberFrog`,
  bundle `kyberfrog.exe` + binaires fork + DLLs + `plugins\`, ajoute au PATH
  (PowerShell .NET, sûr au-delà de 1024), raccourcis menu Démarrer, tâche
  autostart optionnelle, désinstalleur (garde `%APPDATA%\kyberfrog` par défaut).
  Install silencieuse `/S [/AUTOSTART=1]`. NSIS retenu plutôt qu'Inno pour
  réutiliser l'infra `apps/kyber-installer`.

- **Fork build model (so a future chat can build the binaries to bundle, with
  the fewest changes):** KyberFrog only *orchestrates* pre-built Kyber binaries;
  building them means building the **fork**, which is a nest of separate git
  repos wired by cargo `[patch.crates-io]` + git submodules. The forks live in
  the GitLab group **`kyber-frog`** (the old name `kyberFAS` is dead — every
  `fork` remote was updated). Upstream is `kyber.stream`. Layout in this
  workspace (each dir = its own repo):

  - **Build root for `kyclient.exe`:** `apps/kyber-desktop` (`kyber-frog/kyber-desktop`).
    Its `kyclient` crate owns the CLI (`clap`: `--port`, `--fullscreen`, …) and
    the `winit` window, and reaches the client engine via `kyc` + `kyclient-rs`.
    - submodules: `kysdk` → `core/kysdk`, `external/winit` → `deps/winit`.
    - `[patch.crates-io]`: `kyc`/`kyclient-rs`/`kynput-rs`/`kynput-sys` →
      `kysdk/kyctl/…` & `kysdk/kynput/…`; `winit` → `external/winit`.
  - **SDK meta-repo:** `core/kysdk` (submodules: `kyctl`, `kymedia` — itself with
    `external/vlc-rs` + `external/txproto` —, `kynput`, `kymux`, `kyutil`).
    `core/kysdk/.cargo/config.toml` holds the `[patch.crates-io]` that redirects
    cross-crate deps to those submodule paths, **including
    `vlc-rs = { path = "./kymedia/external/vlc-rs" }`**.
  - **Client video path:** `kyber-desktop/kyclient` (bin) → `kyclient-rs` (FFI)
    → **libkyclient** (C ABI, built from `kyctl/kyclient` Rust lib with the
    `capi` feature; `kyclient-sys/build.rs` finds it via **pkg-config**) →
    **kyvlcplayer** (libVLC, via the patched `vlc-rs`) → window / Spout.
  - **Key consequence:** the standalone checkouts `core/kyctl`, `deps/vlc-rs` are
    the *canonical* fork repos, but the **build uses the submodule copies under
    `core/kysdk/**` and `apps/kyber-desktop/kysdk`**. A change in a sub-repo only
    reaches a build after the submodule pointers are bumped *up the chain*.

  **Minimal steps to land a cross-repo change (e.g. the Spout-output feature #8):**
  1. Push the `feat/spout-output` branch to each fork: `kyctl`, `vlc-rs`,
     `kyber-desktop`.
  2. In `core/kysdk`: bump the `kyctl` and `kymedia/external/vlc-rs` submodules to
     those commits, commit (on a branch).
  3. In `apps/kyber-desktop`: bump the `kysdk` submodule, apply the CLI change,
     build libkyclient (kyctl `capi`) then `cargo build` the binary.
  No `.cargo/config.toml` change is needed for `vlc-rs` (the patch already points
  at its submodule — just update that submodule to the fork branch) nor for the
  new `kyspout` crate (a plain path-dep of `kyvlcplayer`, resolved locally).

### 4. Embed the tray icon in the exe — ✅ done
- `kyberfrog/build.rs` embeds `kyberfrog.ico` as Windows resource ID 1 via
  `winresource` (calls `windres`). The tray loads it at runtime with
  `GetModuleHandleW` + `LoadImageW(.., 1, IMAGE_ICON, ..)`, falling back to a
  file next to the exe (override) then a stock icon. Deployment is a single
  self-contained binary.

### 5. Runtime control over HTTP — ✅ done
- The web UI and tray now add / remove / restart transmitters and create / edit /
  start / stop / restart / remove viewers at runtime, all persisted to
  `kyberfrog.toml`. Both front-ends go through the shared `op_*` functions in
  `kyberfrog/src/app.rs`.
