# KyberFrog — backlog & tech debt

Deferred work and tech debt. Each entry says *what*, *why deferred*, and *how*.
Numbers are **stable identifiers** (referenced from `CLAUDE.md`, commits, MRs):
a new item takes the next free number; a shipped item moves to **Shipped** but
keeps its number. The working action plan (sequencing, quick wins) lives in
[`TODO.md`](TODO.md). Release-facing detail of what shipped in each version
lives in [`CHANGELOG.md`](CHANGELOG.md).

> KyberFrog is **one app** (one supervisor, one web UI on 7700, one
> `kyberfrog.toml`). "the web UI" / "the tray" below mean that single UI.

## Active backlog

### Web UI

#### 1. Per-monitor output targeting (needs an upstream kyclient change)
- **What:** let an operator pick *which physical monitor* a viewer fullscreens
  on, from a dropdown of detected monitors.
- **Why deferred:** kyclient can't target an output monitor today. Its
  `set_fullscreen` uses `winit::window::Fullscreen::Borderless(None)` →
  fullscreen on the *current* monitor only. **Not to be confused with #18-B**
  (shipped): `--display-idx`/`--display-count` pick the **source** screen
  captured on the *emitter*, not which monitor of the *receiving* PC the
  kyclient window lands on — that part is still unsolved.
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

#### 21. Application Windows native (Tauri) autour de l'IHM web existante
- **What:** empaqueter le cockpit web (React + Vite) dans une vraie appli
  Windows au lieu d'ouvrir `http://localhost:7700` dans le navigateur. Plus de
  fenêtre console/invite de commande visible au lancement.
- **Why:** **Étape 3** du plan à 3 temps (voir `CLAUDE.md` § In-flight
  restructuring) — confort opérateur (pas de navigateur à gérer, une seule
  icône), à faire après stabilisation du remote desktop (**#17**).
- **How (piste, pas encore d'archi) :** Tauri en frontend = le build
  React+Vite déjà existant, aucune réécriture d'UI ; masquer la fenêtre
  console au démarrage. Points à creuser avant de se lancer : bundling,
  updates, cohabitation avec le tray existant (`kyberfrog/src/tray/`).

#### 22. Polish UI cockpit web (plusieurs points, archi/design à faire avant d'implémenter)
- **What (pistes en vrac) :**
  - Format vertical : retirer le vide dans les sections Émission/Réception
    quand l'app est étroite (responsive).
  - Boutons : état hover cohérent dans toute l'app — **définir l'archi/design
    system avant d'attaquer**, pas trivial si pas mutualisé aujourd'hui. Le
    bouton Supprimer des tuiles émetteur/récepteur paraît désactivé/grisé
    alors qu'il est actif : lui redonner l'apparence normale des autres
    boutons + un hover rouge (à traiter avec le point hover global ci-dessus).
  - Header : l'indicateur « En ligne » ressemble à un bouton — ne garder que
    la LED, afficher « En ligne » en tooltip au survol. Regrouper Hostname et
    IP l'un sous l'autre (IP copiable au clic), LED d'état (respirante) à
    droite du bloc. Intégrer à la modale « À propos » le choix du thème
    (clair/sombre) et de la langue (FR/EN) ; renommer « À propos » en
    « Options » (ou icône roue crantée).
- **Why:** cohérence visuelle + ergonomie régie.
- **How:** pas d'archi arrêtée — plusieurs points (notamment le hover global)
  demandent une passe de conception avant tout code.

#### 23. Drawers → Modals — à évaluer avant de se lancer
- **What:** remplacer les drawers actuels (formulaires transmetteur/viewer)
  par des modales.
- **Why:** pas acquis que ce soit un gain — à trancher.
- **How:** **ne pas coder avant d'avoir décidé** : commencer par une phase de
  réflexion/comparaison (UX, code, cohérence avec le reste du cockpit) entre
  les deux patterns.

### Environnement de dev & fork model

#### 24. Simplifier l'environnement de dev — sortir de la chaîne de forks imbriqués ?
- **What:** évaluer si kyberfrog peut intégrer directement les dépendances/libs
  nécessaires au lieu de dépendre de la chaîne de forks imbriqués
  `kyber-desktop` → `kysdk` → `kyctl`/`kymedia`/`kynput`/`kymux`/`kyutil`
  (submodules git à plusieurs niveaux).
- **Why:** ne rien embarquer d'inutile et simplifier la chaîne de build/CI —
  cf. les galères de résolution de submodules/URLs rencontrées le 2026-07-06
  en préparant la release 0.4.0.
- **How:** audit fait le 2026-07-07 —
  [docs/dev/audit-fork-chain.md](docs/dev/audit-fork-chain.md) (cartographie,
  divergence réelle ~30 commits de code, ce que kyberfrog consomme) ; plans
  proposés dans
  [docs/dev/plans-fork-restructure.md](docs/dev/plans-fork-restructure.md)
  (A upstream-first, B mono-repo, C manifest plat — séquence recommandée
  A → C → B?). Décisions à arbitrer listées en fin de doc. Lié à **#25**.

#### 25. Repenser l'intégration des modifs kyber/vlc — réduire la divergence des forks
- **What:** revoir comment les modifications apportées aux dépendances Kyber
  et VLC (aujourd'hui portées dans des forks `kyber-frog/*`) sont intégrées.
  Si des changements sont pertinents pour le projet upstream, contacter les
  mainteneurs et/ou ouvrir des issues/MR sur les repos d'origine.
- **Why:** éviter d'entretenir indéfiniment des forks qui divergent de plus en
  plus des repos officiels Kyber (coût de rebase croissant, risque de rater
  des fixes upstream).
- **How:** inventaire fait le 2026-07-07 (voir
  [docs/dev/audit-fork-chain.md](docs/dev/audit-fork-chain.md) §2) : ~15
  bugfixes purs (série lavd ×7, X/Y scale, deltas fractionnaires, BGRA, 0×0)
  + features génériques (KYBER_CONFIG_PATH, --fullscreen, Spout in/out,
  webcam). Aucune divergence FFmpeg ni VLC (C) — la « divergence vlc » se
  réduit à vlc-rs (2 commits). Stratégie de contribution par vagues :
  [docs/dev/plans-fork-restructure.md](docs/dev/plans-fork-restructure.md)
  plan A. Lié à **#24**.

### Viewer / kyclient (côté fork)

#### 17. Rework remote desktop (remote-control viewer) — phase 1 livrée (0.4.0), phases 2–3 ouvertes
- **What:** la feature "remote-control viewer" est codée côté KyberFrog (#10).
  **Phase 1 livrée + validée E2E paysage → paysage le 2026-07-05** (voir
  CHANGELOG.md 0.4.0) — utilisable en configuration paysage → paysage. Restent
  ouverts : écrans **verticaux** (B1, phase 2 — cassé tant que la rotation
  n'est pas gérée), `Ctrl+Alt+F` sous keyboard grab (B5, jamais testé),
  accélération pointeur Windows non compensée (phase 3).
- **Why important:** le remote desktop est une feature attendue et
  différenciante (remote desktop over QUIC, sans outil tiers).

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
  l'applique à X et Y (`kynput/src/video_layout.rs:147`) — **corrigé phase 1**
  (scales X/Y séparés) ; (b) deltas relatifs tronqués `f64 → as i16`
  (`winit_handler/mod.rs:251`) — **corrigé phase 1** (accumulateur
  fractionnaire) ; (c) injection `MOUSEEVENTF_MOVE` relative → l'accélération
  pointeur Windows de l'hôte s'applique, aucune compensation de résolution —
  **reste ouvert, phase 3 (P3b)**.
- **B3/B4 (🟡 mineurs)** : arrondis entiers `inject_position` ; race
  `get_virtualscreen()` déjà commentée dans le code.
- **B5 (🟡)** : Ctrl+Alt+F — pas prouvé cassé, tester le bon combo d'abord.

**Plan d'implémentation (3 phases) :**

1. **Phase 1 — quick wins kynput/kyclient** (pas de rebuild txproto) : ✅
   **livrée, embarquée en 0.4.0** — scales X/Y séparés dans
   `local_to_host`/`host_to_local`, accumulateur fractionnaire des deltas côté
   kyclient, tests unitaires `VideoLayout`. Détail : CHANGELOG.md 0.4.0.
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

### Sources & exports

#### 18. Sources et exports étendus

> Chaque sous-item est indépendant et peut être livré séparément. **A (webcam)
> et B (sélection d'écran) livrés** (voir **Shipped** ci-dessous) ; restent
> C/D/E/F. Les items "FFmpeg natif" (D/F) sont probablement peu coûteux ;
> l'item "NDI" (C/E) demande plus de travail (dépendance libndi propriétaire).
> Priorité à décider selon les besoins terrain.

**Sources (Émission)**

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

**Ordre conseillé (restant) :** D/F (SRT/RTSP, peu de fork) → C/E (NDI,
dépendance lourde).

#### 26. Variante #18-B — écran source figé côté émetteur (réflexion, non urgent)
- **What:** aujourd'hui la sélection d'écran (#18-B) est côté *réception* —
  chaque viewer demande l'écran voulu à la connexion. Piste : un
  **transmetteur** qui impose son écran à tout client qui s'y connecte
  (sémantique « le transmetteur possède l'écran », utile pour un mapping
  émetteur→écran unique documenté côté régie).
- **Why:** pure réflexion — **l'implémentation actuelle (#18-B) fonctionne
  bien et remplit le use case** ; ceci n'est pas un besoin exprimé, juste une
  variante à garder en tête si le terrain en montre le besoin.
- **How (si un jour utile) :** changement **fork** : (1) ajouter une clé
  `display_id: Option<u32>` au `Config` de kyavserver
  (`kyavservice/src/config.rs`), (2) la faire primer sur le `display_id`
  demandé par le client dans `video_config` (comme `spout_sender`
  aujourd'hui), (3) la remonter via kycontroller, (4) *puis* rétablir un champ
  côté `Source::Screen` + `gen.rs` + un picker émission. Chaîne de build ~1h +
  validation visuelle obligatoire → complexité moyenne, à mettre au niveau de
  #8/#17, **pas** en « faible ».

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
- **#14** Tests unitaires KyberFrog dans la CI GitLab — job `test`
  (`cargo test --workspace --locked`) vert sur MR + `main`. Reste **C2**
  (étoffer la couverture `app.rs`/`kyclient_args`/`gen.rs`) — déféré, basse
  priorité, tracké dans TODO.md. ✅
- **#15** Import / export de configuration — `GET /setups/export` +
  `POST /setups/import` (`kyberfrog/src/web.rs`, `shared/src/config.rs`),
  vérifié dans le code et testé. ✅
- **#16** ~~Menu contextuel clic-droit kyclient~~ — **ANNULÉ**, incompatible
  avec le mode remote-control/remote desktop (le clic-droit est forwardé au
  PC distant) ; garder une UX homogène entre viewer passif et remote-control
  plutôt que deux mécanismes divergents. Escape hatch : raccourci clavier
  (#17). ✅
- **#18-A** Webcam Windows (DirectShow) — validé E2E hardware. `Source::Camera
  { device }` → pin `[kyavserver].camera_device`, énumération `GET /cameras`.
  Le vrai travail a été de déboguer 7 bugs latents empilés dans le chemin
  lavd de txproto, jamais testé sur hardware upstream — ces fixes profitent
  gratuitement au `camera_device` Linux/V4L2 de la branche de Romain (même
  chemin, jamais validé non plus). *Limitation cosmétique : résolution
  affichée « 0x0 » dans le picker avant ouverture du device.* Détail :
  CHANGELOG.md 0.4.0. ✅
- **#18-B** Sélection d'écran (quel display capturer) — livré côté réception.
  Le display est choisi **par le viewer** (`Viewer::display_idx` →
  `--display-idx`), pas par l'émetteur : dans Kyber le `display_id` est
  demandé par le client au démarrage du flux, `[kyavserver]` n'a aucune clé
  display. Picker UI alimenté par `GET /displays?server=&port=`. Variante
  émission différée : voir **#26**. *Ne pas confondre avec #1 (ciblage
  moniteur de sortie côté réception, toujours ouvert).* Détail :
  CHANGELOG.md 0.1.0. ✅
- **#19** Sources scindées par transmetteur + mode « Tout envoyer » — un
  transmetteur n'expose (énumération *et* streaming) que les sources de son
  type (Écran → moniteurs, Spout → son sender épinglé, **Tout envoyer** →
  tout). `Source::All` + `Emission.send_all`, `gen.rs` émet
  `[kyavserver].all_sources`, `api_list` txproto scindé par config. **Bug
  trouvé + corrigé en validation :** `enumerate_displays` ne filtrait pas les
  senders Spout par identifiant pinné (tous les senders live du système
  remontaient) — fix `kymedia@fix/spout-enumerate-scoping`, même CRC-32 que
  le pin streaming. Restent les scénarios écran-seul et Tout-envoyer à tester
  indépendamment (voir TODO.md). ✅
- **#20** Auto-découverte mDNS des transmetteurs — chaque transmetteur actif
  s'annonce en `_kyber._tcp.local.` (crate `mdns-sd`) ; le formulaire viewer
  liste les « Émetteurs détectés » et pré-remplit nom/IP/port au clic, repli
  sur saisie manuelle. **Entièrement côté KyberFrog** (déviation assumée vs
  l'ébauche initiale « kycontroller announcer » — zéro changement fork ;
  migration possible plus tard si des instances standalone apparaissent,
  point d'ancrage repéré dans kycontroller : `main.rs:719`, pattern
  `ExpirationTask`). Opt-out file-only `mdns = false` ; règle pare-feu NSIS
  (UDP 5353) gérée par l'installeur. Reste la validation E2E 2 machines (voir
  TODO.md). ✅

> Détail complet de ces items : historique git, CHANGELOG.md (notes de
> version) + `docs/E2E-spout-output.md`. La doc technique #12 absorbera le
> reste (le bloc Reference ci-dessous notamment).

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
