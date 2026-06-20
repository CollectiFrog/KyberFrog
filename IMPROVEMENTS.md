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
- **What:** une doc **utilisateur produit** (présentation, installation, premiers
  pas, dépannage, FAQ) sous forme de site statique **MkDocs Material**, publiée
  sur **GitLab Pages** par la CI (job `pages`), sources dans `docs/user/`.
- **Why deferred:** le README mélange aujourd'hui utilisateur et dev et n'offre
  ni navigation ni recherche pour un opérateur non-dev.
- **How:** `mkdocs.yml` + thème Material ; `docs/user/{index,installation,
  getting-started,troubleshooting,faq}.md` ; job `pages` dans `.gitlab-ci.yml`
  (build sur `main`). Site **statique** → réhébergeable tel quel sur un serveur
  perso plus tard sans rien changer. Le README pointe vers ce site.
- **Status:** ✅ **fait** — `mkdocs.yml`, pages User, job `pages`
  (`mkdocs build --strict`), README scindé User/Dev. Build validé localement.
  Reste : activer Pages côté GitLab (Settings) au 1er run ; décider FR vs EN.

#### 12. Doc technique — section "Dev" du même site, in-repo
- **What:** doc technique pour contributeurs (architecture, modèle de build du
  fork, conventions, contribuer) dans `docs/dev/`, publiée comme **section "Dev"
  du même site Pages** que #11.
- **Why deferred:** garder la doc technique **synchro avec le code** (relue en
  MR, dans le même commit) plutôt qu'un wiki qui dérive. Cohérent avec le `docs/`
  déjà présent.
- **How:** `docs/dev/{architecture,building,releasing,contributing}.md`. **Migrer
  ici** le bloc *« Reference — fork build model »* (bas de ce fichier) et le
  contenu pertinent de `CLAUDE.md` ; y ranger `docs/E2E-spout-output.md`.
- **Status:** ✅ **fait** — pages dev écrites ; le *fork build model* vit
  maintenant dans `docs/dev/building.md`. Le bloc Reference reste en bas de ce
  fichier comme doublon de référence interne (peut être supprimé une fois la
  doc adoptée).

### Web UI

#### 13. Restructuration globale de l'IHM Web (Claude design)
- **What:** refonte complète de l'UI web (`kyberfrog/src/web/index.html` +
  `web.rs`) — design system propre, navigation claire Émission / Réception /
  Logs, responsive, états live lisibles. Conçue avec Claude / outils design.
- **Why deferred:** l'UI actuelle est un POC mono-fichier (~388 lignes HTML
  inline) ; elle doit accueillir de nouvelles fonctions (cf. #10) et devenir
  présentable.
- **Scope inclus dans ce chantier:**
  - **#10 Remote-control viewer** (ci-dessous) est **livré dans cette refonte**.
  - **#2 SSE log streaming** est un bon candidat à intégrer ici.
  - **Titre d'onglet `KyberFrog — [Hostname]`** (au lieu du `<title>KyberFrog</title>`
    statique) — renseigné côté front depuis `/status` (le champ `hostname` est
    déjà servi), pour distinguer plusieurs onglets / machines.
  - Préparer le terrain pour l'app **Tauri** (étape 3 du plan global) : garder
    l'UI encapsulable.
- **How:** à cadrer — maquette → composants → intégration axum. Décider si on
  reste en HTML/JS vanilla servi par axum ou un petit front buildé.

#### 10. Remote-control viewer (desktop takeover) — *KyberFrog side ✅*
- **What:** a per-viewer "remote control" option in the web UI's Réception
  section. A normal viewer is a passive display; a remote-control viewer is a
  **windowed** kyclient that **forwards keyboard + mouse** (and grabs the
  keyboard), so the operator drives a remote KyberFrog from this machine.
- **Use case:** the remote KyberFrog runs an **Émission with a screen-capture
  source** (its whole desktop). This viewer connects to it and takes over —
  remote desktop over Kyber's QUIC transport, no extra tooling.
- **How:** today `forward_inputs` / `keyboard_grab` are **Reception globals**
  (file-only, off by default for video walls). Add a per-viewer override, e.g.
  `Viewer.remote_control: bool` (or `inputs: Option<bool>` + `keyboard_grab:
  Option<bool>`), surfaced as a checkbox on the add form and each viewer row.
  When set, `Globals::kyclient_args()` forces `--inputs true --keyboard-grab
  true` and **not** fullscreen by default (so Ctrl+Alt+F / window chrome stay
  reachable); mutually exclusive with `spout_out` (#8). The escape hatch stays
  **Ctrl+Alt+F** (drops to windowed, releases the grab).
- **Server side:** needs the emitter to publish a screen-capture transmitter
  (KyberFrog Émission already supports `Source::Screen`) and to **accept input
  back** — verify kycontroller/kyavserver serve the input channel for a screen
  source (the `--inputs` plumbing exists in kyclient; confirm the host side
  enables it).
- **Status:** ✅ **KyberFrog side done** (branch `feat/remote-control-viewer`):
  `Viewer.remote_control: bool`; `Globals::kyclient_args()` forces
  `--inputs true --keyboard-grab true` and drops `--fullscreen` when set,
  mutually exclusive with `spout_out` (spout wins if both hand-set); web UI
  checkbox on the add form + each viewer row (greys out fullscreen/spout) and a
  "🎮 contrôle à distance" badge; `op_add_viewer`/`op_update_viewer` carry it;
  `ViewerView` exposes it; 2 unit tests. **Remaining:** the **server-side**
  input-channel check above (needs a screen-source emitter + real input, on
  hardware) — not yet validated end-to-end.

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

#### 15. Bug — la sortie plein écran d'un viewer ne fonctionne pas
- **What:** l'opérateur ne parvient pas à sortir un `kyclient` du plein écran
  depuis sa fenêtre ; l'escape hatch censé rendre la main à Windows ne réagit pas.
- **⚠️ À vérifier en premier (peut-être un faux bug):** l'utilisateur a tapé
  **Alt+Maj+F** (`Alt+Shift+F`), or le raccourci **documenté est `Ctrl+Alt+F`**
  (README, CLAUDE.md). Confirmer le bon combo avant d'investiguer ; si
  `Ctrl+Alt+F` ne marche pas non plus, le binding est réellement cassé.
- **Why important:** c'est *le seul* moyen de reprendre la main sur un viewer
  plein écran (pas de quit par design). Cassé = un display PC est bloqué.
- **How (fork-side, kyclient):** inspecter le handler clavier `winit` de kyclient
  (combo → toggle fullscreen + release du keyboard grab). Tester **avec et sans**
  `keyboard_grab`/`forward_inputs` (#10) : un grab actif peut détourner le combo.
- **Lien:** alternative = le menu clic-droit #16 (mais inopérant si les inputs
  sont captés).

#### 16. Menu contextuel (clic-droit) dans la fenêtre kyclient — façon NDI Studio Monitor
- **What:** un **clic-droit** dans la fenêtre de visualisation ouvre un menu
  (comme **NDI Tools → Studio Monitor**) permettant de :
  - **fermer / quitter le plein écran** du viewer (alternative GUI à #15) ;
  - **(re)configurer la connexion** : choisir l'instance kyberfrog-server
    (quel `kycontroller`). Pas de discovery aujourd'hui → **boîte de dialogue de
    saisie IP + port** (+ auth ?).
- **Why deferred:** fork-side (kyclient/winit), gros morceau ; ouvre un usage
  "moniteur autonome" indépendant de la config centralisée KyberFrog.
- **⚠️ Caveat majeur:** si le viewer **capte les inputs** (`forward_inputs` /
  `keyboard_grab`, cf. #10 remote-control), le clic-droit est intercepté /
  forwardé → le menu **ne s'affichera pas**. #16 ne couvre donc *pas* le mode
  remote-control ; pour celui-là il faut un raccourci clavier fiable (#15).
- **How:** menu natif winit/Win32 au clic-droit ; dialogue de saisie IP/port ;
  Appliquer = reconnecter kyclient à la nouvelle cible. Une **discovery** (mDNS ?)
  serait une amélioration séparée pour éviter la saisie manuelle.
- **Lien:** recoupe #1 (autre changement kyclient), #10, et le chantier IHMWeb
  #13 (mais ici c'est la fenêtre **native kyclient**, pas l'IHM web).

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

#### 8. Spout output — raffinements v1 *(feature livrée, voir Shipped)*
La feature (crate `kyspout`, smem dans `vlc-rs`, `kyvlcplayer`) **et** le câblage
KyberFrog (toggle `spout_out` par viewer, badge UI, run windowless) sont
**livrés et validés E2E contre Resolume**. Restent des raffinements v1 :

> ⚠️ **Côté fork, pas le repo kyberfrog** (`core/kyctl/kyvlcplayer`,
> `…/vlc-rs`). Chaîne de build ~1h + **validation visuelle obligatoire** (taille
> native + couleurs, comme le bug chroma RV32/BGRA trouvé seulement au runtime).
> Plan ci-dessous *investigué et prêt*, non implémenté (non validable sans
> matériel — choix assumé en session autonome 2026-06-19).

- **Taille de sortie fixe 1920×1080** — `setup_spout_output`
  ([`kyvlcplayer/src/player.rs:191`]) la force via `mp.set_video_format("BGRA",
  1920, 1080, 1920*4)` ; libVLC scale le flux. **Plan native-size :**
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
  - **Gotchas :** le `setup` peut être rappelé si la résolution change → re-resize
    sender + buffer ; alignement pitch ; `SpoutSender::new` touche D3D11 → vérifier
    qu'il est OK hors thread principal (il tourne là sur un thread libVLC).
- **Round-trip CPU** — smem donne des frames CPU ré-uploadées sur la texture GPU
  à chaque frame. Zero-copy = output callbacks D3D11 de libVLC 4 (plus gros,
  plus tard ; nécessite libVLC 4 côté fork).

[`kyvlcplayer/src/player.rs:191`]: la canonique est `core/kyctl/kyvlcplayer` ; le
build utilise la copie submodule sous `core/kysdk/**` (cf. *fork build model*).

### CI / tests

#### 14. Tests unitaires KyberFrog dans la CI GitLab
- **What:** un job `test` dans `.gitlab-ci.yml` qui exécute `cargo test`, **+**
  étoffer la couverture.
- **Why deferred:** la CI *build* mais ne *teste* pas. Aujourd'hui 9 tests
  vivent dans `shared/` (6 `config.rs`, 3 `gen.rs`) et ne sont jamais joués en
  CI ; les régressions sur la génération de config / les args kyclient passent
  inaperçues.
- **How:** job `test` (image `$WIN64_IMAGE`, `cargo test`) sur MR + `main`, en
  amont de `installer`. Ajouter des tests sur la logique testable hors-Win32 :
  `app.rs` (`resolve_port`, `resolve_viewer_id`), `config.rs::kyclient_args`,
  cas limites de `gen.rs`. (Le quick win *timeout build-fork 3h→1h30* est suivi
  dans `TODO.md`.)
- **Status:** ✅ le **job `test`** (`cargo test --workspace --locked`, en `needs`
  d'`installer`) est en place ; reste **C2** — étoffer la couverture (`app.rs` /
  `kyclient_args` / `gen.rs`).

#### 15. Import / export de configuration

- **What:** boutons dans l'UI web pour exporter la config actuelle (`kyberfrog.toml`) en JSON/TOML téléchargeable, et importer un fichier de config pour restaurer ou dupliquer un setup sur une autre machine.
- **Why deferred:** utile pour les tournées / changements de matériel — actuellement l'opérateur doit copier manuellement `%APPDATA%\kyberfrog\kyberfrog.toml`. Basse priorité tant que le parc machine est stable.
- **How:** `GET /config/export` → renvoie le TOML brut (header `Content-Disposition: attachment`). `POST /config/import` → reçoit un fichier, valide avec `Config::validate()`, remplace la config courante et redémarre les transmetteurs/viewers concernés. Côté UI : bouton dans `AboutModal` ou dans un panneau Paramètres dédié.
- **Status:** non démarré.

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
