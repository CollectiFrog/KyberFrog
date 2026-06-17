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

### 8. Spout output from a viewer (Amélioration 2, in 2 sub-steps)
- **What:** let a viewer re-publish the received video as a **Spout sender** so
  other local apps (Resolume, MadMapper) can consume it — instead of only
  drawing it in a window.
- **Why deferred:** depends partly on a fork-side change (kyclient / txproto
  needs a Spout output sink). To be done after the unified-app work (Amélioration
  1), before the Tauri desktop app (Amélioration 3). See CLAUDE.md.
- **How:** TBD — split into (a) the fork-side Spout output sink, then (b) a
  per-viewer "Spout out" toggle + sender name in the config/UI.

## Release & distribution

### 9. Package release propre & simple d'utilisation (à réfléchir + CI/CD)
- **What:** prendre le temps de concevoir une **distribution propre et simple**
  pour l'utilisateur final : une CI/CD qui build et publie automatiquement, et un
  artefact d'installation aussi simple que possible (idéalement double-clic, sans
  étape PATH manuelle). Regroupe et cadre les pistes #7 (CI/CD release) et #6
  (installeur Windows) ci-dessous — à arbitrer ensemble une fois les
  Améliorations 1–2 stabilisées.
- **Why deferred:** aujourd'hui l'exe est buildé à la main et le fork Kyber doit
  être installé + ajouté au PATH manuellement. Pas bloquant tant qu'il n'y a pas
  d'utilisateurs (projet non publié), mais prérequis avant toute diffusion.
- **How:** décider du périmètre (CI seule → exe en release ; ou CI + installeur
  bundlant le fork) puis implémenter — voir #7 et #6 pour le détail technique.

### 7. CI/CD GitLab — build et publication automatique des releases
- **What:** un pipeline GitLab CI qui, à chaque tag `v*`, cross-compile l'exe
  Windows unique (`kyberfrog.exe`) via l'image Docker `kyber/debian-win64:local`,
  crée une Release GitLab et publie l'exe en asset téléchargeable depuis la page
  releases du projet (`gitlab.com/kyber-frog/kyberfrog/-/releases`).
- **Why deferred:** l'exe est aujourd'hui buildé manuellement et copié à la main.
  La CI est indispensable pour que le README § Installation soit réellement
  utilisable (le lien de téléchargement pointe vers les releases).
- **How:**
  - `.gitlab-ci.yml` avec un job `build` (image `kyber/debian-win64:local`,
    `cargo build --release --target x86_64-pc-windows-gnu`) qui produit l'exe en
    artifact.
  - Job `release` (règle `if: $CI_COMMIT_TAG =~ /^v/`) qui utilise `release-cli`
    pour créer la Release GitLab et attache l'exe comme asset (via l'API `links`
    de release-cli ou le Generic Package Registry).

### 6. Windows installer (single-click setup)
- **What:** a `KyberFrog-Setup.exe` (Inno Setup) that installs everything to
  `C:\Program Files\KyberFrog\`, adds to PATH, creates Start Menu shortcuts, and
  registers an uninstaller — so operators double-click and go.
- **Why deferred:** prerequisite for real-world deployment; skipped while the
  feature set is still stabilising. Today users must install the Kyber fork
  manually and add it to PATH (see README § Prerequisites).
- **How:** one Inno Setup script bundling `kyberfrog.exe` + the Kyber fork
  binaries (`kycontroller`, `kyavserver`, `kyclient`) + their MinGW runtime DLLs.
  One install covers both roles (role is set later from the UI/config).

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
