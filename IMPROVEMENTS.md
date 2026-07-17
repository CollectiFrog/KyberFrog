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

#### 22. Polish UI cockpit web — reste le hover global (après #21)
- **Livré le 2026-07-14** (branche `feat/ui-v2.1`) :
  - Responsive vertical : en fenêtre étroite les sections Émission/Réception
    se dimensionnent sur leur contenu (fin du `min-height: 64vh` forcé et du
    scroll interne ; c'est la page qui défile).
  - Header : bloc Hostname/IP empilé (IP copiable au clic, fallback
    `execCommand` pour l'accès http LAN), LED d'état respirante avec
    « En ligne » en tooltip — l'indicateur-pilule qui ressemblait à un bouton
    est supprimé.
  - Modale « À propos » → « Options » (roue crantée dans le header) : les
    choix thème (clair/sombre) et langue (FR/EN) y migrent depuis le header,
    libellés de la modale traduits FR/EN, route `/about` → `/options`.
  - Quick-fix bouton Supprimer des tuiles : peint en `--k-danger` (rouge) au
    lieu de `--k-faint` qui le faisait paraître désactivé. Le hover rouge
    viendra avec la passe globale ci-dessous.
- **What (reste) :** état hover cohérent sur tous les boutons de l'app.
- **Why deferred :** décision 2026-07-14 — à implémenter **après #21
  (Tauri)**, pour ne pas polir deux fois si le wrap fait bouger l'IHM.
  **#21 livré le 2026-07-15 → plus rien ne bloque.**
- **How (archi arrêtée 2026-07-14, analyse) :**
  - *Cause racine :* tout le styling est en `style={{…}}` inline, qui ne peut
    pas exprimer `:hover` — d'où l'absence totale d'états hover aujourd'hui.
  - *Approche retenue :* classes CSS + design tokens, zéro dépendance.
    Écartés : CSS Modules (éparpille le look par composant alors qu'on veut
    mutualiser) et Tailwind/styled-components (réécriture massive +
    dépendance, à éviter avant le wrap Tauri).
  - *3 couches :*
    1. **Tokens d'état** dans `global.css`, déclinés dans les 3 thèmes
       (dark/light/frog) : `--k-hover` (fond hover neutre),
       `--k-accent-hover` (accent éclairci/assombri), `--k-danger-hover`.
    2. **`ui/src/buttons.css`** : base `.kf-btn` (inline-flex, radius 8,
       transition .15s sur background/color/border-color, ring
       `:focus-visible` accent, `:disabled` centralisé opacity .4 +
       not-allowed) + 4 variantes : `--primary` (fond accent →
       `--k-accent-hover`), `--ghost` (bordure, fond transparent → fond
       `--k-hover`), `--quiet` (sans bordure → fond `--k-hover`), `--danger`
       (apparence normale au repos → fond `--k-danger-soft` + texte/icône
       `--k-danger` au hover) ; modificateurs `--icon` (carré) et `--sm`.
    3. **Composant `<Btn variant size icon>`** (~30 lignes) qui assemble les
       classNames et forwarde le reste vers `<button>` ; le `style` inline
       reste pour le layout uniquement. Migration incrémentale : les
       constantes locales `iconBtnStyle`/`textBtnStyle`/`tbBtn`/`BarBtn`
       meurent fichier par fichier.
  - *Ordre de migration* (1 commit par étape, l'app reste cohérente entre
    chaque) : ① tokens + buttons.css + Btn.tsx → ② TopBar + PaneHeader →
    ③ cartes émetteur/récepteur (hover rouge Supprimer ici) → ④ LogDrawer →
    ⑤ drawers/modales (Segmented d'OptionsModal inclus). Bonus au passage :
    hover des `<select>` et des liens, navigation clavier gratuite via
    `:focus-visible`. Volume : ~35 boutons dans 8 fichiers, mécanique une
    fois la couche ① posée.

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

### Latence (chaîne fork)

#### 28. Latence bout-en-bout — analyse et leviers
- **What:** l'analyse de la chaîne complète (capture → encode → QUIC → décode →
  Spout → hôte) et les leviers identifiés pour la réduire. **La latence est la
  priorité n°1 du projet** : cet item existe pour que les arbitrages d'archi
  (dont #27) s'y réfèrent au lieu de re-dériver la chaîne à chaque fois.
- **Why deferred:** travail **côté fork** (kymedia / kyctl), traité par
  l'opérateur ; KyberFrog consomme le bundle de binaires tel quel et ne linke
  aucun crate du fork (voir #24). Rien ici ne bloque #27.
- **How — leviers, par ordre de gain estimé :**
  1. **Émission — le plus gros levier, probablement.** L'encodeur par défaut est
     **x264 (CPU)** *uniquement* parce qu'AMF crashe en boucle silencieuse sur
     la RX 7800 XT (`gen.rs:62-63`). Un encodeur CPU impose un download
     GPU→CPU **plus** une latence d'encodage bien supérieure au coût du chemin
     Spout — c'est-à-dire que le défaut actuel coûte probablement plus cher que
     tout le reste réuni. Pistes : reprendre AMF/NVENC avec `zerolatency` (le
     patch FFmpeg `0001-nvenc-Patch-SPS-when-zerolatency-is-enabled.patch` est
     **déjà dans l'arbre**, `kymedia/subprojects/ffmpeg.wrap`), `intra_refresh`.
  2. **Réception — un aller-retour CPU évitable.** `setup_spout_output`
     (`kyctl/kyvlcplayer/src/player.rs:189-259`) installe des callbacks libVLC
     `smem` demandant du **BGRA CPU**, puis `kyspout` ré-uploade chaque frame
     dans la texture D3D11 partagée. Soit : décodage HW (**GPU**) → **CPU** →
     **GPU**. Cible : output callbacks **D3D11** de libVLC 4 → `SpoutSender`
     prend la texture directement. **Déjà anticipé par l'auteur de kyspout**
     (`kyctl/kyspout/src/lib.rs:37-40` : « *A later optimization can take a GPU
     texture directly (zero-copy)* »). Noter que le coût est **dans kyclient, pas
     dans Spout** — Spout est une copie GPU→GPU, négligeable. Ce correctif
     bénéficie à Resolume **et** TouchDesigner **sans aucun plugin**.
  3. **Protocole.** `multi_client=false` → session unique : *direct forward,
     lowest latency, full backpressure* vers l'encodeur (vs le duplicateur
     `kydup` du mode multi-client, qui est le défaut). Protocoles vidéo
     `Unreliable` / `UnreliableFec` (`kyclient/src/capi.rs:104-109`).
     ⚠️ Tension avec #27 : en session unique, un 2e client sur le même flux
     reçoit un **409 Conflict**.
  4. **Instrumenter avant d'optimiser.** L'API C de kyclient expose **déjà** les
     timestamps par frame `FrameAcquired → FrameDisplayed`
     (`kyclient/src/capi.rs:1639`) : chiffrer chaque étape avant d'engager le
     travail de fork, pour cibler le vrai coupable plutôt que le supposé. Mesure
     de terrain simple, sans code : dans TouchDesigner, comparer un
     `Spout In TOP` sur le flux Kyber vs un `Spout In TOP` **direct** sur le
     sender source — l'écart *est* le coût de la chaîne Kyber.
  5. **Note transverse (à vérifier).** `Source::All` (« Tout envoyer ») met
     `all_sources = true` côté kyavserver → expose *tous* les senders Spout de la
     machine, **y compris les `Kyber - *`** créés par un viewer local. Le
     filtrage anti-boucle de #27 est côté KyberFrog ; en mode « Tout envoyer »
     l'énumération est faite par le fork, donc **hors de portée** de KyberFrog.

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

#### 27. Passthrough Spout (2 modes : émission / réception) — Kyber comme NDI pour Resolume / TouchDesigner
- **What:** **deux interrupteurs indépendants**, un par sens — décision
  opérateur du 2026-07-17, cf. « Pourquoi deux interrupteurs » ci-dessous :

  ```
  ☐ emission.spout_passthrough   « Publier tous les Spout »
      tout sender Spout local (Resolume Advanced Output, Spout Out TOP, OBS…)
      → 1 transmetteur chacun,  Source::Spout { sender }            [zero-copy]

  ☐ reception.spout_passthrough  « Recevoir tous les Kyber »
      tout flux Kyber découvert (mDNS, hors is_self)
      → 1 viewer chacun, { fullscreen: false, spout_out: "Kyber - <tx>@<host>" }
        → Resolume > Sources > Spout Servers · TD > Spout In TOP
  ```

  Rend Kyber utilisable depuis Resolume / TouchDesigner / OBS comme on
  utiliserait NDI, **sans ajouter une seule copie** au chemin natif.

- **Pourquoi deux interrupteurs et pas un seul mode (important) :**
  1. **Ça colle à l'infra réelle.** Les machines ont des rôles **asymétriques** :
     une machine envoie ses flux TD, une autre reçoit tous les flux TD pour les
     injecter dans Resolume Arena. Le ping-pong est l'exception, pas la norme.
  2. **Ça supprime la boucle *par construction*, pas par filtrage.** Une boucle
     exige qu'une **même machine** crée des senders `Kyber - *` (réception)
     **et** publie des senders (émission). En sens unique, il n'y a **rien à
     re-publier** → topologies 1 et 2 **structurellement impossibles**. Le
     filtre anti-boucle ne reste nécessaire que pour la machine qui active les
     **deux** sens — cas légitime (envoyer son TD *et* recevoir les autres),
     mais explicite et minoritaire. Défense en profondeur : archi **puis**
     runtime.
  3. **Ça laisse éviter le côté cher.** Un transmetteur au repos est gratuit
     (kycontroller n'encode qu'à l'ouverture de session) ; **un viewer est actif
     d'office** (session distante + décodage permanents). En un seul mode,
     c'était tout ou rien ; séparés, l'opérateur prend le côté gratuit sans le
     côté coûteux.
  4. **Ça épouse le schéma existant.** La config a **déjà** `[emission]` et
     `[reception]` : un champ par moitié, le placement se déduit du modèle.
  5. **Ça réduit le conflit avec « Tout envoyer ».** `send_all` est un mode
     d'**émission** : seul `emission.spout_passthrough` l'exclut. La réception
     devient orthogonale.

- **Why:** ceci **remplace** la demande initiale « un plugin Resolume (FFGL) +
  un plugin TouchDesigner ». Conclusion de l'exploration du 2026-07-17 :
  **aucun des deux ne doit être écrit.** Les 4 raisons, pour ne pas les
  re-dériver :
  1. **FFGL n'a que Source / Effect / Mixer** — aucun type « output device ».
     Un plugin **ne peut pas** ajouter « Kyber » au dropdown de l'Advanced
     Output ; seul Resolume peut le faire.
  2. **Resolume fait déjà Spout in/out nativement** — les senders apparaissent
     sous *Sources → Spout Servers*, et l'Advanced Output sort en Spout **en
     utilisant le nom de l'écran comme sender name**. C'est le modèle NDI, déjà
     livré.
  3. **TouchDesigner aussi** — `Spout In/Out TOP` natifs et GPU ; le
     `sendername` du Spout In TOP est **déjà un menu des senders vivants**. Un
     `CPlusPlus TOP` ne sort qu'en CPUMem/CUDA → strictement pire.
  4. **Aucune API C n'offrirait un chemin plus court** — `kyclient.dll` n'a
     **aucun callback frame** (`capi.rs:821/895`, seulement `HWND` ou
     `spout_out`), le QUIC (quinn) a **zéro export C**, et tout le fork est
     **MinGW** quand FFGL/TD sont MSVC.

  Un plugin ne ferait donc que **ré-lire le même Spout en ajoutant un blit** —
  soit de la latence, contre la priorité n°1 du projet. Le manque réel n'est pas
  dans les hôtes : c'est que l'opérateur **câble le pont à la main** dans l'UI.

- **How:**
  - **Pattern : dérivé, jamais persisté.** Reprendre `op_set_send_all`
    (`app.rs:349-383`), qui **ne mute jamais les listes** — l'extinction restaure
    l'état d'avant à l'identique. Seuls les 2 booléens (+ leurs excludes) sont
    persistés ; les ressources dérivées n'entrent **pas** dans
    `config.emission.transmitters` / `reception.viewers`, sinon le TOML
    churnerait à chaque sender Spout qui apparaît et l'extinction laisserait des
    déchets. **Corollaire : non modifiable** (comme `send_all`) — la boucle de
    réconciliation écraserait toute édition au tick suivant. L'échappatoire est
    **l'exclusion**, pas l'édition.
  - **Placement des champs — ⚠️ piège de sérialisation TOML.**
    `emission.spout_passthrough: bool` + `emission.spout_passthrough_exclude:
    Vec<String>` doivent être déclarés **avant** `defaults: toml::Table` et
    `transmitters` ; `reception.spout_passthrough` + son exclude **avant**
    `viewers`. Raison déjà documentée pour `send_all` (`config.rs:211-213`) :
    *« TOML requires bare keys before `[table]` / `[[array]]` sections »*. Un
    `Vec<String>` sérialise en tableau **inline** (clé nue) → OK, mais l'ordre
    reste impératif. Tous en `#[serde(default)]`.
  - **Deux boucles de réconciliation indépendantes** (~2 s), une par sens :
    émission ← `spout.rs` (senders locaux, `spout.rs:25-124`) · réception ←
    `discovery.rs` (mDNS, `discovery.rs:32-51`). Idempotentes, via les `op_*`
    (verrou **config avant manager**, `app.rs:604-614`).
  - **Combinaisons — les 2 passthrough sont INDÉPENDANTS et CUMULABLES.**
    Émission et réception ne s'excluent **jamais** l'un l'autre : les 4
    combinaisons sont valides et supportées.

    | `emission.spout_passthrough` | `reception.spout_passthrough` | Rôle | Boucle possible ? |
    |---|---|---|---|
    | ☐ | ☐ | manuel (défaut) | non |
    | ☑ | ☐ | **machine émettrice** (ex. envoie ses flux TD) | **non — par construction** |
    | ☐ | ☑ | **machine réceptrice** (ex. reçoit tout pour Resolume) | **non — par construction** |
    | ☑ | ☑ | bidirectionnelle | oui → **avertir**, filtrer, **ne jamais bloquer** |

    **Le cas ☑/☑ est légitime et doit rester possible** (une machine peut
    vouloir envoyer son TD *et* recevoir les autres). L'UI **avertit**
    (« les 2 sens actifs — risque de boucle, filtre anti-boucle actif ») ;
    elle **n'interdit pas**, ne désactive pas l'autre bascule, et ne demande pas
    de confirmation. Seule combinaison réellement exclue : **`send_all` ⟷
    `emission.spout_passthrough`** — deux stratégies d'*émission* concurrentes
    (cf. ci-dessous). `send_all` n'a **aucun** effet sur la réception.
  - **`send_all` vs `emission.spout_passthrough` — pourquoi ils s'excluent.**
    `send_all` = **1 kycontroller** exposant *toutes* les sources, le client
    choisit à la connexion → 1 seule entrée mDNS (`kind=all`). Passthrough =
    **N kycontroller**, 1 par sender → **N entrées mDNS individuelles**. Or
    `reception.spout_passthrough` **a besoin d'annonces individuelles** pour
    énumérer et s'abonner flux par flux. Les deux stratégies sont donc
    alternatives : l'une privilégie le coût (1 instance), l'autre la
    découvrabilité (N instances, mais plafond ~9).
  - **❓ À trancher — interop avec un voisin en « Tout envoyer ».** Que fait
    `reception.spout_passthrough` face à un transmetteur découvert `kind=all` ?
    Il expose N sources derrière **une** annonce ; s'y abonner « une fois »
    donne une source ambiguë. Proposition : **ignorer `kind=all` en
    réception-passthrough** et le journaliser (l'opérateur crée un viewer
    manuel s'il le veut), plutôt que de deviner. Le TXT `kind` est déjà là
    (`discovery.rs:76-94`).
  - **⚠️ ANTI-BOUCLE — nécessaire uniquement quand les DEUX sens sont actifs
    sur la même machine.**
    Le découpage en 2 interrupteurs rend les topologies 1 et 2 **impossibles par
    construction en sens unique** (rien de créé localement à re-publier). Ce qui
    suit ne s'applique donc qu'à la machine « bidirectionnelle » — cas
    légitime mais explicite. **L'UI doit avertir quand les 2 sens sont
    activés** (« risque de boucle, filtre actif »). Les 3 topologies :

    | # | Topologie | Effet sans parade | Parade |
    |---|---|---|---|
    | 1 | **Machine seule bidirectionnelle** — viewer crée `Kyber - x@A`, l'émission le republie | 1 transmetteur parasite, pollue le LAN. `is_self` empêche le ré-abonnement local, donc **pas** d'explosion | filtre publication |
    | 2 | **Ping-pong A↔B** — exige que **les deux** machines soient bidirectionnelles | **explosion exponentielle** : chaque tour ajoute 1 tx + 1 viewer **par machine**, jusqu'au plafond ~9, sur toute la flotte | **sens unique** (archi) **+** filtre publication |
    | 3 | **Boucle via l'app hôte** — B affiche `Kyber - x@A` sur un layer → Advanced Output → `Kyber Out B` → publié → A s'y abonne → l'affiche → … | boucle réelle, **mais avec des noms 100 % légitimes** | ❌ **indétectable** — voir plus bas |

    **Filtre de publication, en deux couches (les deux, pas l'une ou l'autre) :**
    1. **Identité (primaire, exact).** Exclure les senders dont le nom est
       **exactement** l'un des `spout_out` des viewers dérivés vivants —
       KyberFrog *sait* lesquels il a créés. Exact, donc **zéro faux positif**.
    2. **Préfixe réservé `Kyber - ` (secondaire, défense en profondeur).**
       Rattrape ce que l'identité rate : sender **orphelin d'un kyclient
       crashé** (l'entrée survit en mémoire partagée Spout alors qu'aucun viewer
       ne la revendique plus) et sender d'un **KyberFrog voisin** d'une version
       sans filtre. Sans cette 2ᵉ couche, un crash rouvre la topologie 2.

    **Corollaire : `Kyber - ` devient un préfixe RÉSERVÉ** pour les noms de
    senders Spout locaux. Un écran Resolume nommé `Kyber - Main` serait
    silencieusement non publié → **avertir dans l'UI** quand un sender local
    matche le préfixe sans correspondre à un viewer dérivé, au lieu de l'ignorer
    en silence. (`Kyber Out A` ne matche **pas** `Kyber - ` — les noms de la doc
    opérateur sont sûrs, mais la marge est mince : le dire explicitement.)

    **Défense en profondeur côté réception :** journaliser (et proposer de
    filtrer) les transmetteurs découverts dont le TXT `tx` commence par
    `kyber-` — c'est la signature d'un relais republié, le slugifieur
    (`app.rs:640-648`) transformant `Kyber - main@regie` en
    `kyber-main-regie`. À logger, **pas** à bloquer en dur : un opérateur peut
    légitimement nommer un transmetteur `kyber-*`.

    **Topologie 3 : hors de portée du code, à documenter.** Les noms y sont
    légitimes ; c'est un feedback vidéo, exactement comme filmer l'écran qui
    affiche la caméra. Aucun filtre ne peut le distinguer d'un usage voulu →
    responsabilité opérateur, à écrire noir sur blanc dans la doc.

    **Tests unitaires obligatoires** (`shared`, cible Linux) : un sender égal au
    `spout_out` d'un viewer dérivé n'est jamais publié ; un sender `Kyber - …`
    orphelin non plus ; un sender opérateur `Kyber Out A` **l'est** ; et le
    scénario 2 simulé (A publie, B reçoit) ne crée **aucun** tx chez B.
  - **Anti-self** — ignorer `is_self` côté réception (ne pas se réabonner à ses
    propres transmetteurs). Insuffisant seul : ne couvre **pas** la topologie 2.
  - **Anti-flap (stabilité).** Un sender qui apparaît/disparaît rapidement
    (Resolume qui recharge une compo, TD qui recook) ferait spawner/tuer des
    kycontroller en rafale et entrerait en conflit avec le backoff du
    superviseur. Debounce : publier après **N ticks stables**, temporiser avant
    de démonter.
  - **Plafond** — les ports IPC de kycontroller s'auto-allouent en
    **9091..9100 → ~9 instances max**. Plafonner, journaliser et afficher quand
    ça mord ; jamais échouer en silence. C'est aussi le **dernier rempart** si
    une boucle passe malgré tout : il borne les dégâts. *(À vérifier : les
    kyclient consomment-ils ce budget, ou seulement les kycontroller ?)*
  - **Nommage — les deux champs n'ont pas les mêmes règles :** `viewer.id` →
    `[A-Za-z0-9-]` uniquement (segment d'URL + nom de log, `resolve_viewer_id`
    `app.rs:690`) → `kyber-<tx>-<host>` via le slugifieur `app.rs:640-648`.
    `spout_out` → chaîne libre → **`Kyber - <tx>@<host>`** (vu par l'opérateur
    dans Resolume/TD, sert au regroupement visuel **et** de 2ᵉ couche
    anti-boucle → **préfixe réservé**, cf. ci-dessus).
  - **Exclusion, fichier-only** (cohérent avec « advanced settings are file-only
    by design ») : `[emission] spout_passthrough_exclude = ["Preview", …]`.
    Elle survit à la réconciliation, contrairement à une édition.
  - **UI — une bascule par moitié, dans sa propre section** (Émission /
    Réception), et **chacune ne grise que son côté** : `emission.spout_passthrough`
    grise **`kind:"spout"`** dans la création de transmetteur (screen/camera
    restent offerts) ; `reception.spout_passthrough` grise **`spout_out`** dans
    la création de viewer (fullscreen/remote_control restent offerts). Bandeau
    d'avertissement si les **2** sens sont actifs. Précédent : `op_add_spout`
    refuse déjà quand `send_all` est actif (`app.rs:168-171`) — les endpoints
    doivent renvoyer une **erreur explicite**, jamais un succès silencieux.
    Lister les ressources dérivées en lecture seule **avec le `spout_name`
    résolu** (c'est ce que l'opérateur cherchera dans Resolume/TD).
  - **Fichiers :** `shared/src/config.rs` (champ + exclusions + tests
    round-trip) · `kyberfrog/src/app.rs` (`op_set_spout_passthrough` sur le
    modèle de `op_set_send_all`, + réconciliation) · `src/web.rs` +
    `web/index.html` · `src/tray/` (miroir de la bascule) · doc
    `docs/user/spout-passthrough.md`.
  - **⚠️ Libellés — ne pas réutiliser « Tout envoyer »**, déjà pris par
    `send_all` et de mécanique **différente** (1 transmetteur pour tout vs N
    transmetteurs). Deux modes d'émission aux noms voisins = confusion garantie
    dans le tray et l'UI. Pistes : « **Publier tous les Spout** » /
    « **Recevoir tous les Kyber** », à trancher à l'implémentation (FR/EN, cf.
    #22).
  - **Limites à documenter :**
    - **`Kyber - ` est un préfixe réservé** : ne nommez pas un écran Resolume /
      Spout Out TOP avec ce préfixe, il ne serait pas publié (l'UI avertit) ;
    - **boucle via l'app hôte (topologie 3)** — si B affiche un flux Kyber venu
      de A **et** que ce layer part dans son Advanced Output republié vers A,
      c'est un feedback vidéo que **KyberFrog ne peut pas détecter** (les noms
      sont légitimes). Même nature qu'une caméra filmant son propre écran :
      responsabilité opérateur ;
    - le dropdown Advanced Output de Resolume dira toujours « Spout », jamais
      « Kyber » (impossible sans modification de Resolume) ;
    - **un viewer est actif d'office** — il force une session distante + un
      décodage local **en permanence**, même si personne ne consomme le Spout.
      Contrairement à NDI (qui ne décode que ce qu'on utilise), « recevoir
      tout » a donc un coût GPU/CPU proportionnel au nombre de flux du LAN →
      contenu par le plafond, la liste d'exclusion, et surtout par le fait que
      **ce côté s'active séparément** (une machine purement émettrice ne le paie
      jamais) ;
    - gel du transmetteur si la **résolution de la source change en cours de
      stream** (voir #8, Shipped) — un changement de résolution de composition
      Resolume déclenche ça ;
    - avec `multi_client=false` (le réglage **basse latence**), un 2e client sur
      le même flux reçoit un **409 Conflict** — tension assumée entre « recevoir
      tout » et « latence minimale » (voir #28) ;
    - un sender Spout vivant sur un **autre adaptateur GPU** échoue à
      `OpenSharedResource()` (`iosys_spout.c:66-67`) — PC multi-GPU.

## Shipped (archive — numéros conservés pour les références)

- **#21** Application Windows native (Tauri/WebView2) — **Étape 3** du plan à
  3 temps, livrée et validée E2E opérateur le 2026-07-15 (`feat/tauri` →
  `dev`). Archi : coquille native, jamais un pipeline d'assets — la fenêtre
  (`kyberfrog/src/shell/`) pointe sur `http://localhost:7700` (axum,
  same-origin, zéro changement `api.ts`/`web.rs` ; `KYBERFROG_UI_URL` → vite
  pour l'HMR). Fermer = cacher, seul « Quitter » du tray arrête l'app ; clic
  gauche tray = dashboard, droit = menu. Zéro console : `windows_subsystem`
  (tous builds) + `CREATE_NO_WINDOW` sur les enfants. Installeur NSIS
  conservé + bootstrap WebView2 (détection registre, bootstrapper Evergreen)
  + `WebView2Loader.dll` livrée (**obligatoire en windows-gnu**, pas de link
  statique hors MSVC). Archi/déviations/gotchas :
  [docs/dev/plan-tauri-shell.md](docs/dev/plan-tauri-shell.md). Pistes v2
  non planifiées : bascule `tauri build`, migration du tray vers l'API
  Tauri. ✅
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
