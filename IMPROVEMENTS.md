# KyberFrog — backlog & tech debt

Deferred work and tech debt. Each entry says *what*, *why deferred*, and *how*.
Numbers are **stable identifiers** (referenced from `CLAUDE.md`, commits, MRs):
a new item takes the next free number; a shipped item moves to **Shipped** but
keeps its number. The working action plan (sequencing, quick wins) lives in
[`TODO.md`](TODO.md).

> KyberFrog is **one app** (one supervisor, one web UI on 7700, one
> `kyberfrog.toml`). "the web UI" / "the tray" below mean that single UI.

## Active backlog

### Documentation

#### 11. User Manual — site MkDocs publié sur GitLab Pages
- **Status:** ✅ **entièrement livré** — site bilingue EN + FR en ligne à
  https://kyber-anysource-b41fc4.gitlab.io/. Pages GitLab activé, job `pages`
  vert sur `main`. README scindé User/Dev. Rien à faire.

#### 12. Doc technique — section "Dev" du même site, in-repo
- **Status:** ✅ **entièrement livré** — `docs/dev/` écrit et publié (même
  site que #11). Fork build model migré dans `docs/dev/building.md`.

### Web UI

#### 13. Restructuration globale de l'IHM Web (Claude design)
- **Status:** ✅ **livré** — front React + Vite, design system Collecti'Frog,
  cockpit Émission/Réception, embarqué dans le binaire. Remote-control viewer
  (#10) KyberFrog-side inclus. Tauri préparé.
  Reste dans le backlog : SSE (#2, optimisation, déféré), ciblage moniteur (#1, bloqué upstream).

#### 10. Remote-control viewer (desktop takeover)
- **Status:** ✅ **KyberFrog side done** — checkbox par viewer, `kyclient`
  fenêtré + `--inputs true --keyboard-grab true`, exclusif de `spout_out`,
  badge UI. **Non validé end-to-end** (inversion X/Y des contrôles constatée,
  Ctrl+Alt+F à confirmer fonctionnel). Voir **#17** pour le rework complet.

#### 1. Per-monitor output targeting (needs an upstream kyclient change)
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

#### 2. Real-time log streaming (SSE) — currently polling
- **What:** stream the app + per-child logs live in the UI.
- **Why deferred:** v1 uses simple periodic polling of the last N lines (fine
  for a POC). **Confirmed non-blocking:** poll and SSE share the same source
  (the log files), so this is purely additive. Bon candidat à livrer dans #13.
- **How:** add `GET /logs/stream` (`text/event-stream`) that tails the file and
  pushes new lines; swap the frontend from `setInterval`+`fetch` to
  `EventSource`. The log-reading helper is reused unchanged.

### Viewer / kyclient (côté fork)

#### 15. Bug — sortie plein écran kyclient (Ctrl+Alt+F)
- **What:** vérifier et fixer l'escape hatch `Ctrl+Alt+F` (passage en fenêtré
  + release du keyboard grab). Probablement lié au mode remote-control (#10/#17)
  où le grab actif peut avaler le combo.
- **⚠️ Vérifier d'abord :** l'opérateur tapait peut-être **Alt+Maj+F** — tester
  le bon raccourci `Ctrl+Alt+F` avant d'investiguer le code.
- **How (fork-side, kyclient):** inspecter le handler `winit`, tester avec et
  sans `keyboard_grab`. Traité dans le cadre du rework #17.
- **Lien:** absorbé dans **#17** (remote desktop rework).

#### 16. ~~Menu contextuel clic-droit kyclient~~ — **ANNULÉ**
- **Raison:** incompatible avec le mode remote-control/remote desktop (le
  clic-droit est forwardé au PC distant). Pour garder une UX homogène entre
  viewer passif et remote-control, on ne crée pas deux mécanismes divergents.
  L'escape hatch reste le raccourci clavier (#15).

#### 17. Rework remote desktop (remote-control viewer) ⚠️ **PRIORITAIRE**
- **What:** la feature "remote-control viewer" est codée côté KyberFrog (#10)
  mais **inutilisable en pratique** : inversion X/Y des contrôles, Ctrl+Alt+F
  possiblement cassé sous keyboard grab, canal input côté serveur non validé.
- **Bugs connus :**
  - Inversion des axes X/Y des contrôles souris/clavier (fork-side kyclient ou
    kyavserver, à diagnostiquer).
  - `Ctrl+Alt+F` (sortie plein écran) potentiellement avalé par le grab (#15).
  - Canal input côté émetteur (kycontroller/kyavserver en mode Screen) non
    validé end-to-end.
- **Why important:** le remote desktop est une feature attendue et différenciante
  (remote desktop over QUIC, sans outil tiers). Actuellement inutilisable.
- **How :** investigation fork-side (`kyclient` + `kyavserver`).
  1. Diagnostiquer l'inversion X/Y (comparer les coordonnées envoyées vs
     reçues, vérifier `kynput` et le mapping côté `kyavserver`).
  2. Valider Ctrl+Alt+F sous grab actif (peut nécessiter un hook bas niveau
     distinct du forward, comme pour le LowLevelKeyboard actuel).
  3. Valider end-to-end : émetteur Screen + viewer remote-control + inputs
     retours sur hardware réel.
- **Scope :** fork-side (kyclient + kynput + kyavserver). Build chain ~1h.

### Auth

#### 3. Surface credential management to the user
- **What:** let the operator set per-viewer / per-transmitter username +
  password in the web UI and tray, instead of the baked-in transparent login.
- **Why deferred:** today every config uses the transparent default
  (`vj` / `kyberfrog`, see `DEFAULT_AUTH_*` in `shared`), which is enough for a
  trusted LAN and means the operator types nothing.
- **How:** optional credential fields that, when set, override the transparent
  default in the generated config (emission) / the kyclient args (reception).

### Features — refinements

#### 8. Spout output — taille native ⚠️ **PRIORITAIRE**
La feature (crate `kyspout`, smem dans `vlc-rs`, `kyvlcplayer`) est livrée et
validée E2E. **Problème actuel :** la sortie Spout est forcée à **1920×1080**,
donc Resolume reçoit une image déformée si la source a une résolution différente
(elle doit ré-étaler dans Arena, ce qui est sale). À corriger.

> ⚠️ **Côté fork, pas le repo kyberfrog** (`core/kyctl/kyvlcplayer`,
> `…/vlc-rs`). Chaîne de build ~1h + **validation visuelle obligatoire** (taille
> native + couleurs, comme le bug chroma RV32/BGRA trouvé seulement au runtime).

**Plan investigué et prêt à implémenter :**

1. **vlc-rs** (`media_player.rs`) : ajouter un wrapper sûr
   `set_video_format_callbacks(setup, cleanup)` au-dessus du FFI **déjà présent**
   `libvlc_video_set_format_callbacks` (`sys.rs`). Le callback `setup` a la
   signature `(opaque, chroma[4], *width, *height, *pitches, *lines) -> u32`
   (nb de buffers) : libVLC passe la taille **native** du flux ; on écrit en
   retour `chroma="BGRA"`, `pitches[0]=width*4`, `lines[0]=height`, retourne 1.
2. **kyvlcplayer** : dans `setup_spout_output`, remplacer `set_video_format` par
   ce wrapper ; **créer/redimensionner** le `kyspout::SpoutSender` et le buffer
   `SpoutCtx` à la taille négociée *dans* le callback `setup` (et non plus en
   constantes), puis garder les callbacks lock/display existants (lecture de
   `width/height/pitch` sous le mutex `SpoutCtx`).

**Gotchas :** le `setup` peut être rappelé si la résolution change → re-resize
sender + buffer ; alignement pitch ; `SpoutSender::new` touche D3D11 → vérifier
qu'il est OK hors thread principal (il tourne là sur un thread libVLC).

**Déféré (post taille native) :** round-trip CPU (zero-copy via output callbacks
D3D11 de libVLC 4 — nécessite libVLC 4 côté fork, gros chantier).

[`kyvlcplayer/src/player.rs:191`]: la canonique est `core/kyctl/kyvlcplayer` ; le
build utilise la copie submodule sous `core/kysdk/**` (cf. *fork build model*).

### CI / tests

#### 14. Tests unitaires KyberFrog dans la CI GitLab
- **Status:** ✅ **C1 en place** — job `test` (`cargo test --workspace --locked`)
  vert sur MR + `main`. Reste **C2** (étoffer la couverture : `app.rs`,
  `kyclient_args`, `gen.rs`) — **déféré**, basse priorité.

#### 15. Import / export de configuration

- **What:** boutons dans l'UI web pour exporter la config actuelle (`kyberfrog.toml`) en JSON/TOML téléchargeable, et importer un fichier de config pour restaurer ou dupliquer un setup sur une autre machine.
- **Why deferred:** utile pour les tournées / changements de matériel — actuellement l'opérateur doit copier manuellement `%APPDATA%\kyberfrog\kyberfrog.toml`. Basse priorité tant que le parc machine est stable.
- **How:** `GET /config/export` → renvoie le TOML brut (header `Content-Disposition: attachment`). `POST /config/import` → reçoit un fichier, valide avec `Config::validate()`, remplace la config courante et redémarre les transmetteurs/viewers concernés. Côté UI : bouton dans `AboutModal` ou dans un panneau Paramètres dédié.
- **Status:** non démarré.

### Sources & exports

#### 18. Sources et exports étendus

> Chaque sous-item est indépendant et peut être livré séparément. Complexité
> variable : **B (sélection d'écran) est ✅ livré** ; les items "FFmpeg natif"
> (D/F) sont probablement peu coûteux ; l'item "fork txproto" (A) et "NDI" (C/E)
> demandent plus de travail. Priorité à décider selon les besoins terrain.

**Sources (Émission)**

- **A — Webcam Windows (DirectShow)** : `camera_device` est aujourd'hui
  `#[cfg(target_os = "linux")]` uniquement — pas de caméra sur Windows. Il faut
  un iosys `dshow` dans txproto (ou activer `lavd` côté Windows si déjà
  compilé), puis ajouter `camera_device` dans kyavservice pour Windows, et
  exposer le variant `Source::Camera` dans KyberFrog shared + l'UI.
  *Complexité : moyenne — fork txproto + kyavservice + KyberFrog.*

- **B — Sélection d'écran (quel display capturer)** : ✅ **livré (côté
  réception).** La prémisse initiale était fausse : dans Kyber, le display est
  choisi **par le client au démarrage du flux** (`display_id` dans
  `RtpStartVideo`/`KymuxStartVideo`), pas figé dans la config de l'émetteur —
  `[kyavserver]` n'a aucune clé display, seul `spout_sender` prime. Injecter un
  display dans `gen.rs` aurait été un no-op. La sélection a donc été mise sur le
  **viewer** : champ `Viewer::display_idx` → arg kyclient `--display-idx` (index
  0-based dans la liste d'écrans de l'émetteur). Picker vivant dans l'UI web
  alimenté par un nouvel endpoint `GET /displays?server=&port=` qui interroge le
  `/enumerate_displays` de l'émetteur distant (HTTPS TOFU, GET sans login — pas
  d'éviction de session), avec repli sur une saisie manuelle de l'index si
  l'émetteur est injoignable. Le champ mort `Source::Screen { display }` (jamais
  câblé) a été retiré. *Voir la variante émission différée ci-dessous.*

- **C — NDI input** : ingérer un flux NDI et le re-transmettre en Kyber.
  Côté réception, **libVLC dispose déjà d'un plugin NDI** (
  [tobiasebsen/libndi](https://github.com/tobiasebsen/libndi)) — à explorer
  comme voie d'intégration (kyvlcplayer, même chemin que le Spout output #8)
  plutôt que d'ajouter NDI dans la chaîne txproto/FFmpeg. Nouveau variant source
  `Source::Ndi { name }` dans KyberFrog. Interop avec caméras réseau, switchers
  Tricaster/ATEM, OBS.
  *Complexité : moyenne (si via VLC plugin) à haute (si via FFmpeg txproto).*

- **D — SRT / RTSP input** : ingérer un flux SRT ou RTSP (caméras IP, etc.).
  FFmpeg le supporte nativement (`rtsp://`, `srt://` comme URL d'entrée) ;
  txproto utilise déjà FFmpeg → probablement peu de code fork nécessaire.
  Nouveau variant `Source::Url { url }` dans KyberFrog.
  *Complexité : faible à moyenne — à valider côté txproto.*

**Exports (Réception)**

- **E — NDI output** : re-publier un flux Kyber reçu en NDI (sortie vers
  switchers, OBS, écrans NDI). Même voie que C : plugin VLC NDI
  ([tobiasebsen/libndi](https://github.com/tobiasebsen/libndi)) côté
  kyvlcplayer, similaire au Spout output (#8) mais protocole NDI.
  *Complexité : moyenne (si via VLC plugin).*

- **F — SRT / RTSP output** : sortie réseau d'un flux reçu vers d'autres
  systèmes (enregistrement, re-streaming). Via FFmpeg côté kyvlcplayer ou
  txproto.
  *Complexité : faible à moyenne.*

**Variante différée de B — écran source figé côté émetteur (fork).** La
sélection livrée est côté *réception* : chaque viewer demande l'écran voulu. Si
un jour on veut qu'un **transmetteur** impose son écran à tout client qui s'y
connecte (sémantique « le transmetteur possède l'écran », utile pour un mapping
émetteur→écran unique documenté côté régie), il faut un changement **fork** :
(1) ajouter une clé `display_id: Option<u32>` au `Config` de kyavserver
(`kyavservice/src/config.rs`), (2) la faire primer sur le `display_id` demandé
par le client dans `video_config` (comme `spout_sender` aujourd'hui), (3) la
remonter via kycontroller, (4) *puis* rétablir un champ côté `Source::Screen` +
`gen.rs` + un picker émission. Chaîne de build ~1h + validation visuelle
obligatoire → complexité moyenne, à mettre au niveau de #8/#17, **pas** en
« faible ». Non planifié.

**Ordre conseillé (restant) :** D/F (SRT/RTSP, peu de fork) → A (webcam Windows)
→ C/E (NDI, dépendance lourde).

## Shipped (archive — numéros conservés pour les références)

- **#4** Icône tray embarquée dans l'exe (`winresource`/`windres`, resource ID 1,
  override par fichier voisin). ✅
- **#5** Contrôle runtime via HTTP + tray (add/remove/restart transmitters,
  create/edit/start/stop/restart/remove viewers, persisté dans `kyberfrog.toml`,
  via les `op_*` partagés). ✅
- **#6** Installeur Windows single-click (NSIS : Program Files, PATH > 1024,
  raccourcis, tâche autostart, désinstalleur ; install silencieuse `/S`). ✅
- **#7** CI/CD GitLab (`build-fork` → `installer` → `release` sur tag `v*`,
  cache du bundle fork par SHA dans le Generic Package Registry). ✅
- **#8** Spout output depuis un viewer — feature fork (crate `kyspout`, smem dans
  `vlc-rs`, `kyvlcplayer`) + câblage KyberFrog (toggle `spout_out` par viewer,
  UI, windowless), **validé E2E contre Resolume Arena**. Détail :
  `docs/E2E-spout-output.md` + historique git. Raffinements v1 → #8 ci-dessus. ✅
- **#9** Package release propre & simple (un seul `KyberFrog-Setup.exe` NSIS
  bundlant `kyberfrog.exe` + binaires fork, double-clic sans étape PATH, CI qui
  build et publie la Release sur tag `v*`). ✅

> Détail complet de ces items : historique git + `docs/E2E-spout-output.md`. La
> doc technique #12 absorbera le reste (le bloc Reference ci-dessous notamment).

## Reference — fork build model

> Référence (pas une tâche). À migrer vers `docs/dev/build-fork.md` avec #12.

KyberFrog only *orchestrates* pre-built Kyber binaries; building them means
building the **fork**, a nest of separate git repos wired by cargo
`[patch.crates-io]` + git submodules, under the GitLab group **`kyber-frog`**
(upstream = `kyber.stream`). Layout in this workspace (each dir = its own repo):

- **Build root for `kyclient.exe`:** `apps/kyber-desktop` (`kyber-frog/kyber-desktop`).
  Its `kyclient` crate owns the CLI (`clap`: `--port`, `--fullscreen`, …) and the
  `winit` window, and reaches the client engine via `kyc` + `kyclient-rs`.
  - submodules: `kysdk` → `core/kysdk`, `external/winit` → `deps/winit`.
  - `[patch.crates-io]`: `kyc`/`kyclient-rs`/`kynput-rs`/`kynput-sys` →
    `kysdk/kyctl/…` & `kysdk/kynput/…`; `winit` → `external/winit`.
- **SDK meta-repo:** `core/kysdk` (submodules: `kyctl`, `kymedia` — itself with
  `external/vlc-rs` + `external/txproto` —, `kynput`, `kymux`, `kyutil`).
  `core/kysdk/.cargo/config.toml` holds the `[patch.crates-io]` redirecting
  cross-crate deps to those submodule paths, **including
  `vlc-rs = { path = "./kymedia/external/vlc-rs" }`**.
- **Client video path:** `kyber-desktop/kyclient` (bin) → `kyclient-rs` (FFI) →
  **libkyclient** (C ABI, built from `kyctl/kyclient` Rust lib with the `capi`
  feature; `kyclient-sys/build.rs` finds it via **pkg-config**) → **kyvlcplayer**
  (libVLC, via the patched `vlc-rs`) → window / Spout.
- **Key consequence:** the standalone checkouts `core/kyctl`, `deps/vlc-rs` are
  the *canonical* fork repos, but the **build uses the submodule copies under
  `core/kysdk/**` and `apps/kyber-desktop/kysdk`**. A change in a sub-repo only
  reaches a build after the submodule pointers are bumped *up the chain*.

**Minimal steps to land a cross-repo change (e.g. the Spout-output feature #8):**
1. Push the feature branch to each fork: `kyctl`, `vlc-rs`, `kyber-desktop`.
2. In `core/kysdk`: bump the `kyctl` and `kymedia/external/vlc-rs` submodules to
   those commits, commit (on a branch).
3. In `apps/kyber-desktop`: bump the `kysdk` submodule, apply the CLI change,
   build libkyclient (kyctl `capi`) then `cargo build` the binary.

No `.cargo/config.toml` change is needed for `vlc-rs` (the patch already points
at its submodule — just update that submodule to the fork branch) nor for a new
crate that is a plain path-dep (resolved locally).
