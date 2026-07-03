# KyberFrog — backlog & tech debt

Deferred work and tech debt. Each entry says *what*, *why deferred*, and *how*.
Numbers are **stable identifiers** (referenced from `CLAUDE.md`, commits, MRs):
a new item takes the next free number; a shipped item moves to **Shipped** but
keeps its number. The working action plan (sequencing, quick wins) lives in
[`TODO.md`](TODO.md).

> KyberFrog is **one app** (one supervisor, one web UI on 7700, one
> `kyberfrog.toml`). "the web UI" / "the tray" below mean that single UI.

## Active backlog

### Web UI

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

### Réseau

#### 20. Auto-découverte des kycontroller sur le réseau
- **What:** découvrir automatiquement les instances kycontroller (émetteurs)
  disponibles sur le réseau local, au lieu de saisir IP + port à la main dans
  l'UI web (requis aujourd'hui pour le picker d'écran `GET /displays?server=&port=`
  côté viewer, cf. **#18-B**, et pour configurer un transmetteur).
- **Why:** confort opérateur en régie — plus besoin de connaître/chercher
  l'IP de chaque PC émetteur, moins d'erreurs de saisie.
- **How (archi implémentée) :** mDNS/DNS-SD, **entièrement côté KyberFrog**
  (déviation validée vs l'ébauche initiale « kycontroller = announcer » : zéro
  changement fork, un seul repo, et KyberFrog connaît déjà nom+port de chaque
  transmetteur au runtime. Revers assumé : un kycontroller lancé *hors*
  KyberFrog n'est pas découvert — cas inexistant dans le déploiement actuel ;
  si des instances standalone apparaissent un jour, migrer l'annonce dans
  kycontroller reste possible, points d'ancrage repérés : `main.rs:719` de
  kycontroller, pattern `ExpirationTask`).
  - **KyberFrog = announcer** (`kyberfrog/src/discovery.rs`) — publie un
    service `_kyber._tcp.local.` **par transmetteur actif** (instance
    `<tx>@<hostname>`, port = port control-plane, TXT : `version` + `tx` +
    `kind`), re-synchronisé à chaque mutation via `persist_and_refresh`.
  - **KyberFrog = browser** (même module) — map vivante des instances
    entendues, servie par `GET /discovered` ; le formulaire viewer affiche
    « Émetteurs détectés » **en tête du formulaire** et pré-remplit
    nom/serveur/port au clic (nom repris seulement à la création — un
    renommage reste un acte délibéré une fois le viewer créé), avec repli
    sur saisie manuelle si rien trouvé (même pattern que le picker #18-B).
    Les annonces de la machine elle-même sont listées avec badge (viewer
    local légitime). Le champ nom du viewer est désormais éditable aussi en
    édition (le backend supportait déjà le rename via `resolve_viewer_id`,
    seul le front verrouillait).
  - **Crate :** `mdns-sd` (pure Rust, UDP brut) plutôt qu'un binding
    Bonjour/Avahi (type `zeroconf`) — Windows n'a pas de résolveur mDNS natif
    fiable sans Bonjour installé, et ça reste cohérent avec la piste
    ARM/Linux de Romain.
  - **Opt-out :** `mdns = false` dans `kyberfrog.toml` (file-only, défaut on).
  - **Limites connues :** link-local uniquement (pas de traversée
    VLAN/routeur — ok pour un LAN plat de régie) ; pas de sécu native, repose
    sur l'hypothèse LAN de confiance déjà posée par #3 ; règle pare-feu
    Windows « KyberFrog mDNS » (UDP 5353 entrant, scopée exe) ajoutée par
    l'installeur NSIS, supprimée à la désinstallation.
- **Status:** implémenté (backend + UI + installeur), en attente de
  validation E2E 2 machines.

### Viewer / kyclient (côté fork)

#### 16. ~~Menu contextuel clic-droit kyclient~~ — **ANNULÉ**
- **Raison:** incompatible avec le mode remote-control/remote desktop (le
  clic-droit est forwardé au PC distant). Pour garder une UX homogène entre
  viewer passif et remote-control, on ne crée pas deux mécanismes divergents.
  L'escape hatch reste le raccourci clavier (#17).

#### 17. Rework remote desktop (remote-control viewer) ⚠️ **PRIORITAIRE**
- **What:** la feature "remote-control viewer" est codée côté KyberFrog (#10)
  mais **inutilisable en pratique** : inversion X/Y des contrôles, Ctrl+Alt+F
  possiblement cassé sous keyboard grab, canal input côté serveur non validé.
- **Bugs connus :**
  - Inversion des axes X/Y des contrôles souris/clavier (fork-side kyclient ou
    kyavserver, à diagnostiquer).
  - Sortie plein écran `Ctrl+Alt+F` (escape hatch : passage en fenêtré + release
    du keyboard grab) potentiellement avalée par le grab actif — **vérifier
    d'abord** que l'opérateur tape bien `Ctrl+Alt+F` (pas Alt+Maj+F) avant
    d'investiguer le code.
  - Canal input côté émetteur (kycontroller/kyavserver en mode Screen) non
    validé end-to-end.
- **Why important:** le remote desktop est une feature attendue et différenciante
  (remote desktop over QUIC, sans outil tiers). Actuellement inutilisable.

**Audit fait (2026-07-02)** — chemin souris tracé de bout en bout, causes racines
identifiées :

- **B1 (🔴 root cause « inversion X/Y » écrans verticaux)** : rotation non
  appliquée. La capture DXGI livre une texture **non pivotée**
  (`iosys_dxgi.c:855`) et l'encodeur n'attache la rotation qu'en **metadata**
  (`encode.c:519`) ; le chemin desktop natif (kyclient/kyvlcplayer) **ignore
  totalement cette metadata** (seuls les backends web/ws la lisent). Pendant ce
  temps l'énumération renvoie les dims **pivotées** (`DesktopCoordinates`,
  portrait = 1080×1920). Le client projette donc le curseur dans un repère
  portrait sur une image affichée paysage → clics décalés « à 90° ».
- **B2 (🟠 scale souris incohérent, 3 causes)** : (a)
  `VideoLayout::local_to_host` calcule le scale **sur width seul** et
  l'applique à X et Y (`kynput/src/video_layout.rs:147`) ; (b) deltas relatifs
  tronqués `f64 → as i16` — les deltas fractionnaires (<1.0) des souris haute
  fréquence deviennent 0 (`winit_handler/mod.rs:251`) ; (c) injection
  `MOUSEEVENTF_MOVE` relative → l'accélération pointeur Windows de l'hôte
  s'applique, aucune compensation de résolution.
- **B3/B4 (🟡 mineurs)** : arrondis entiers `inject_position` ; race
  `get_virtualscreen()` déjà commentée dans le code.
- **B5 (🟡)** : Ctrl+Alt+F — pas prouvé cassé, tester le bon combo d'abord.

**Plan d'implémentation (3 phases) :**

1. **Phase 1 — quick wins kynput/kyclient** (pas de rebuild txproto) :
   - P2 : scales X/Y séparés dans `local_to_host`/`host_to_local`
     (`kynput/src/video_layout.rs` + copie `kynput-rs` + C-API). ~6 lignes.
   - P3a : accumulateur fractionnaire des deltas relatifs côté kyclient
     (garder le reste f64, envoyer l'entier).
   - P4 : tests unitaires `VideoLayout` (host portrait 1080×1920, property
     test aller-retour local↔host). Zéro test aujourd'hui sur ce module.
   - Build fork léger + validation souris sur écran paysage.
   - **Statut (2026-07-03) :** ✅ codé + poussé — `kynput:feat/remote-desktop-fix`
     (P2 + P4, 5 tests verts), `kyber-desktop:feat/remote-desktop-fix` (P3a +
     bump kysdk). Chaîne rebasée proprement sur `kyberfrog-main` (kysdk,
     kyber-desktop, kynput). ⏳ Build fork + validation manuelle souris pas
     encore faits (interrompu, TLS local à reconfigurer pour le test).
2. **Phase 2 — rotation (fix B1, le gros morceau)** :
   - P1-A (retenu) : transpose GPU D3D11 dans txproto avant encode quand
     `rotation != IDENTITY` — un seul endroit, tous les clients corrigés,
     l'énumération (dims pivotées) devient cohérente avec les pixels.
   - Alternative si coût GPU rédhibitoire : P1-B côté client (propager la
     metadata dans le path RTP natif + rendu pivoté kyvlcplayer + transform
     souris dans VideoLayout) — plus de code, 3 crates.
   - Build fork complet ~1h30 + **validation hardware écran vertical
     obligatoire**.
3. **Phase 3 — polish** :
   - P3b : neutraliser l'accélération Windows (documenter « désactiver
     Enhance pointer precision » ou convertir relatif→absolu server-side).
   - P5 : protocole de test Ctrl+Alt+F ; si cassé, traiter le combo dans le
     hook `WH_KEYBOARD_LL` avant le forward.
   - Diag : logs `host_size/video_size/scale` au resize + exposer `rotation`
     dans `/enumerate_displays`.

- **Scope :** fork-side (kynput + kyclient + txproto). Phase 1 = build léger ;
  Phase 2 = chaîne complète ~1h30.

### Auth

#### 3. Surface credential management to the user
- **What:** let the operator set per-viewer / per-transmitter username +
  password in the web UI and tray, instead of the baked-in transparent login.
- **Why deferred:** today every config uses the transparent default
  (`vj` / `kyberfrog`, see `DEFAULT_AUTH_*` in `shared`), which is enough for a
  trusted LAN and means the operator types nothing.
- **How:** optional credential fields that, when set, override the transparent
  default in the generated config (emission) / the kyclient args (reception).

### CI / tests

#### 14. Tests unitaires KyberFrog dans la CI GitLab
- **Status:** ✅ **C1 en place** — job `test` (`cargo test --workspace --locked`)
  vert sur MR + `main`. Reste **C2** (étoffer la couverture : `app.rs`,
  `kyclient_args`, `gen.rs`) — **déféré**, basse priorité.

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

### 19. Sources scindées par transmetteur + mode « Tout envoyer »

- **What :** un transmetteur n'expose (énumération *et* streaming) que les sources
  de son type — **Écran → moniteurs seuls**, **Spout → son Spout épinglé**, **Tout
  envoyer → tout** (moniteurs + Spout). Corrige le fait qu'un transmetteur écran
  listait/servait aussi les Spout. Mode « Tout envoyer » = un transmetteur global
  unique, ajout des autres bloqué, retour à l'état initial à l'extinction.
- **Statut :** ✅ **KyberFrog livré** (branche `feat/source-selector`) — `Source::All`,
  `Emission.send_all` + transmetteur synthétique `tout-envoyer`, `gen.rs` émet
  `[kyavserver].all_sources`, toggle UI + blocage ajout, endpoint
  `POST /emission/send-all`. ⏳ **Fork intégré `dev` + buildé** (merge
  `kymedia@7c173c7`, dans le même bundle que #8, rebuild du 2026-07-03) :
  l'`api_list` txproto est scindé par config (`["dxgi"]` / `["spout"]` / `[]`),
  défaut = moniteurs seuls.
- **🐛 Bug trouvé en validation (2026-07-03) — scoping Spout non appliqué :**
  un seul transmetteur Spout configuré (kind `spout`, épinglé sur le sender
  Resolume `Arena - LatJar`). Le picker (viewer, `GET /displays?server=&port=`
  → `/enumerate_displays` du transmetteur) proposait **2 choix : `LatJar` ET
  `LatCour`** — ce dernier est un *autre* sender Spout live d'Arena, non
  épinglé à ce transmetteur. Cause : `api_list` scopait bien le backend
  (`["spout"]`), mais le backend spout de txproto remonte tous les senders
  live du système — `enumerate_displays` (kymedia/kyavservice) ne filtrait
  jamais par identifiant.
  **✅ Corrigé** — `kymedia@fix/spout-enumerate-scoping` (mergé `dev`) : filtre
  l'identifiant pinné (même CRC-32 que le pin streaming) dans le callback
  `enumerate()`, initial + hotplug. Propagé `kysdk@d96b9c4` →
  `kyber-desktop@368bc00` (dev). **Validé manuellement 2026-07-03** : un seul
  choix affiché pour un transmetteur Spout pinné.
- **Nicety résolue au passage :** picker écran, viewer côté KyberFrog —
  bouton « Automatique (premier écran) » retiré (pré-sélection directe du
  premier élément de la liste), + icône refresh à côté du titre de section
  pour re-fetch la liste sans rouvrir le formulaire.
- **Reste à faire :** valider indépendamment les scénarios écran-seul et
  Tout-envoyer (jamais testés, non liés à ce bug).

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
  `docs/E2E-spout-output.md` + historique git.
  **Raffinement taille native** (v1 forçait 1920×1080, déformant l'image si la
  source avait une autre résolution) : `vlc-rs@7393f95` (wrapper sûr
  `set_video_format_callbacks(setup, cleanup, lock, unlock, display)` + struct
  `VideoFormat` + fix du typedef FFI `libvlc_video_format_cb`, le retour
  `unsigned` = nb de buffers manquait) + `kyctl@e114926` (`setup_spout_output`
  négocie BGRA à la taille native dans le callback `setup`, resize mid-stream
  couvert par `SpoutSender::send_bgra` qui recréait déjà texture + info block
  au changement de dimensions). Intégré `dev` jusqu'à
  `kyber-desktop@00bf3bf`, bundle fork rebuild local, **validé E2E hardware le
  2026-07-03** : taille native + couleurs OK dans Arena, resize mid-stream OK.
  **Limitation connue acceptée (non corrigée) :** le transmetteur se fige et
  doit être redémarré manuellement quand la résolution de l'écran capturé
  change en cours de stream. Round-trip CPU zero-copy (output callbacks D3D11
  libVLC 4) reste déféré — gros chantier, nécessite libVLC 4 côté fork. ✅
- **#9** Package release propre & simple (un seul `KyberFrog-Setup.exe` NSIS
  bundlant `kyberfrog.exe` + binaires fork, double-clic sans étape PATH, CI qui
  build et publie la Release sur tag `v*`). ✅
- **#10** Remote-control viewer (desktop takeover) — checkbox par viewer,
  `kyclient` fenêtré + `--inputs true --keyboard-grab true`, exclusif de
  `spout_out`, badge UI. KyberFrog-side livré ; bugs d'usage (inversion X/Y,
  Ctrl+Alt+F sous grab, canal input non validé) trackés dans **#17**. ✅
- **#11** User Manual — site MkDocs bilingue EN+FR publié sur GitLab Pages
  (https://kyber-anysource-b41fc4.gitlab.io/), README scindé User/Dev. ✅
- **#12** Doc technique — section `docs/dev/` du même site (même déploiement
  que #11), fork build model migré dans `docs/dev/building.md`. ✅
- **#13** Restructuration IHM Web — front React + Vite, design Collecti'Frog,
  cockpit Émission/Réception, remote-control viewer (#10) inclus, embarqué
  dans le binaire. Reste actif dans le backlog : SSE (#2), ciblage moniteur
  (#1). ✅
- **#15** Import / export de configuration — `GET /setups/export` +
  `POST /setups/import` (`kyberfrog/src/web.rs`, `shared/src/config.rs`),
  vérifié dans le code et testé. ✅

> Détail complet de ces items : historique git + `docs/E2E-spout-output.md`. La
> doc technique #12 absorbera le reste (le bloc Reference ci-dessous notamment).

## Reference — fork build model

> Référence (pas une tâche). À migrer vers `docs/dev/build-fork.md` avec #12.

KyberFrog only *orchestrates* pre-built Kyber binaries; building them means
building the **fork**, a nest of separate git repos wired by cargo
`[patch.crates-io]` + git submodules, under the GitLab group **`kyber-frog`**
(upstream = `kyber.stream`). Since the KyberClean workspace (see its README),
there is exactly **one checkout of each repo** — no more standalone-vs-submodule
duplication. Layout (each dir = its own repo, siblings under `KyberClean/`):

- **Build root for `kyclient.exe`:** `kyber-desktop` (`kyber-frog/kyber-desktop`).
  Its `kyclient` crate owns the CLI (`clap`: `--port`, `--fullscreen`, …) and the
  `winit` window, and reaches the client engine via `kyc` + `kyclient-rs`.
  - submodules: `kysdk`, `external/winit`.
  - `[patch.crates-io]`: `kyc`/`kyclient-rs`/`kynput-rs`/`kynput-sys` →
    `kysdk/kyctl/…` & `kysdk/kynput/…`; `winit` → `external/winit`.
- **SDK meta-repo:** `kyber-desktop/kysdk` (submodules: `kyctl`, `kymedia` —
  itself with `external/vlc-rs` + `external/txproto` —, `kynput`, `kymux`,
  `kyutil`). `kysdk/.cargo/config.toml` holds the `[patch.crates-io]`
  redirecting cross-crate deps to those submodule paths, **including
  `vlc-rs = { path = "./kymedia/external/vlc-rs" }`**.
- **Client video path:** `kyber-desktop/kyclient` (bin) → `kyclient-rs` (FFI) →
  **libkyclient** (C ABI, built from `kyctl/kyclient` Rust lib with the `capi`
  feature; `kyclient-sys/build.rs` finds it via **pkg-config**) → **kyvlcplayer**
  (libVLC, via the patched `vlc-rs`) → window / Spout.
- **Key consequence:** each submodule tree (`kyber-desktop/kysdk/**`) **is** the
  canonical fork repo now — editing in place is enough for a *local* build. A
  change only reaches **other clones / CI** after it's pushed on the sub-repo's
  own branch and the submodule pointers are bumped *up the chain* (see
  `bump-fork.sh` at the KyberClean root).

**`dev` is the integration branch on every repo** (kyctl, vlc-rs, kymedia,
kysdk, kyber-desktop, txproto, kyberfrog) — it's what `versions.sh`
(`KYBER_DESKTOP_REF`) and the CI build. Older `feat/spout-output` lines are
frozen for reproducibility of past releases; new fork work branches off `dev`
and merges back into it.

**Minimal steps to land a cross-repo change (e.g. the Spout-output feature #8):**
1. Commit + push the change on each affected sub-repo's own branch (e.g.
   `kyctl`, `vlc-rs`), merge into that repo's `dev`, push `dev`.
2. In `kyber-desktop/kysdk`: checkout `dev`, `git add kyctl kymedia` (whichever
   moved), commit, push.
3. In `kyber-desktop`: checkout `dev`, `git add kysdk`, commit, push. Build
   libkyclient (kyctl `capi`) then `cargo build`, or run the full
   `contrib/build-win32.sh` for a release bundle (see KyberClean README).

No `.cargo/config.toml` change is needed for `vlc-rs` (the patch already points
at its submodule — just update that submodule to the fork branch) nor for a new
crate that is a plain path-dep (resolved locally).
