# Changelog

Tous les changements notables de KyberFrog, version par version. Le format
suit l'esprit de [Keep a Changelog](https://keepachangelog.com/fr/) ; la
**version fait foi via le tag git** (`v*`, injectée au build — pas de bump
`Cargo.toml`). Le backlog vit dans
[`docs/dev/backlog.md`](docs/dev/backlog.md) et les items livrés dans
[`docs/dev/backlog-archive.md`](docs/dev/backlog-archive.md) ; les `#N`
ci-dessous y renvoient.

## [Non publié]

### Ajouté
- Linux : le choix de source liste les webcams V4L2 (nom de carte, via le `ffmpeg` du bundle) au lieu de « Aucune caméra détectée » (#32).
- Caméra : options d'ouverture par transmetteur (`input_format`, `video_size`, `framerate`…), éditables dans l'UI (champ avancé), l'API et `kyberfrog.toml`, transmises telles quelles au démuxeur FFmpeg (#32).
- Linux : une caméra s'épingle aussi par chemin de nœud V4L2 (`/dev/video0`, lien udev), pour les cartes dont tous les nœuds portent le même nom de carte (Pi 5 `rp1-cfe`, #46).
- Fork (`txproto`) : une caméra dont le pilote ne donne aucune cadence prend la `framerate` demandée, au lieu de laisser l'encodeur la déduire de la base de temps.

### Modifié
- Chaîne de forks rebasée sur Kyber **0.28.0** (pin `kyber-desktop` `bfff933`) : tous les commits fork conservés ; les commits upstream de la ligne hotfix 0.27.1 écartés.
- `kymedia` : épinglage Spout/caméra, scoping par transmetteur et shim `KYBER_CONFIG_PATH` portés sur le kyavservice restructuré d'upstream (backend `txproto`).
- `kyctl` : sortie Spout portée en Rust 2024 ; `Cargo.lock` régénéré pour `kyspout`.
- `txproto-rs` : `va_list` Linux aarch64 ajouté à la gestion par arch d'upstream.
- `libavconv-rs` (nouveau en 0.28) : buffer `c_char` d'`av_strerror` portable, sans quoi la chaîne ne compile pas sur arm64.
- `rebase-fork.sh` écarte d'office les commits accessibles depuis une branche upstream et les nomme au dry-run.

### Corrigé
- Linux, fork (`kymedia`) : le client d'un transmetteur caméra ne se voit plus proposer les écrans, seulement la caméra épinglée (#32).
- Linux : sous systemd, l'émetteur n'est plus annoncé `<source>@unknown` en mDNS — le nom d'hôte vient de `gethostname`, plus de la variable bash `HOSTNAME`.
- `fork-lint.sh` vérifiait txproto et vlc-rs sous `external/`, disparu depuis 0.27 : ils passaient « clean » sans être lus.
- `build-fork-local.sh` : git cassé dans le volume avec le layout `vendor/` (gitfiles) ; `-c` masquait l'échec de cargo.

## [0.6.1] — 2026-09-23

### Ajouté
- **Paquet `.deb` arm64** (#35, S1 de KyberFrog Satellite) : `kyberfrog_<version>_arm64.deb` pour Raspberry Pi OS Lite Trixie (plancher mesuré : glibc 2.39).
- Jobs CI `build-fork-linux-arm64`, `deb-arm64` et `image-debian-linux-arm64`, `allow_failure` de bout en bout — ils ne retiennent ni Windows ni amd64.
- `build-fork-local.sh -a arm64` : bundle fork construit sur le poste en conteneur `linux/arm64` émulé, puis poussé dans le Generic Package Registry (la CI ne fait que le cache hit).
- `deb-arm64` vérifie ses propres preuves : binaires `ARM aarch64`, aucun symbole au-dessus de `GLIBC_2.41`.
- `release-deb` attache désormais tous les `.deb` produits, pas seulement l'amd64.
- `./dev.sh` (`.\dev` sous PowerShell/cmd) : l'environnement de dev s'installe en une commande (`setup`), puis `test`, `check`, `installer`, `deb`, `docs` sans taper de `docker run`.
- `packaging/fork-bundle.sh` : `build-installer.sh` et `build-deb.sh` téléchargent seuls le bundle fork du pin depuis le registry public quand `-f` est absent.

### Modifié
- Chaîne de forks : `build-linux.sh` dérive son triplet de `uname -m` dans les quatre dépôts qui en ont un (kyber-desktop, kyctl, kymedia, kynput) au lieu de coder `x86_64-linux-gnu` en dur.
- Les wrappers `run_*.sh` livrés dans le bundle lisent leur triplet à l'exécution, et non plus celui de la machine de build.
- `kymedia` gate NVENC et oneVPL sur x86 dans le contrib meson : ni l'un ni l'autre n'a de cible aarch64.
- `txproto-rs` porte `va_list` (tableau sur x86_64, struct sur aarch64) et le signe de `c_char` par `cfg(target_arch)` ; sans quoi les crates Rust ne compilent pas sur ARM.
- `packaging/versions.sh` pointe `38d64eb` (`feat/arm64-triplet`) — ce bump invalide le cache des builds fork Windows et amd64 une fois.
- Le pin `kyber-desktop` est le gitlink du submodule `vendor/kyber-desktop`, vide dans un clone simple, et non plus un SHA écrit dans `versions.sh`.

### Corrigé
- `.deb` construit en local : toute l'UI web (`.js`, `.css`, `.html` compris) reçoit des permissions normalisées.

### Limitations connues
- Le `.deb` arm64 exige Debian 13 / Pi OS Trixie : sur bookworm, 19 dépendances (glibc 2.39, `libstdc++6` 13, paquets `*t64`) le refusent.
- Sur un Pi headless, le service utilisateur ne démarre qu'avec `loginctl enable-linger` (non fait par le paquet).
- Performance arm64 non mesurée : encodeur x264 logiciel uniquement, backend `drm` — c'est le go / no-go S0 de #46.
- Le bundle fork arm64 est produit hors CI : après un bump du pin, la chaîne arm64 est rouge tant qu'il n'est pas poussé.
- Un bundle fork arm64 coûte ~4 h sur le poste (émulation qemu), contre ~20 min en amd64 natif.

### CI / build
- Jobs CI légers répartis entre le poste et tfgl-goat (tag commun `kyberfrog-self`) ; `build-fork`, `build-fork-linux`, `installer` et `deb` restent sur le poste.
- `packaging/rebase-fork.sh` corrigé pour la cascade vers kyber 0.28 (chemins de submodules renommés, bumps `chore(submodules)` filtrés) — rebase de la chaîne pas encore lancé.

## [0.6.0] — 2026-09-20

### Ajouté
- **Encodeur GPU par défaut** (#28-1) : réglage **Options → Encodage vidéo**, `auto` = AMF / NVENC selon la carte, ~4 ms au lieu de ~26 ms en x264.
- **Repli automatique sur x264** quand l'encodeur GPU échoue (ex. webcam en AMF), signalé sur la tuile.
- **Support Linux amd64** : paquet `.deb` pour Debian 13 / Ubuntu 24.04, service systemd utilisateur, remote control via `/dev/uinput`.
- **Backend de capture Linux** `screen_backend` : `auto` (X11 → `xcb`, Wayland → `wlroots`, sinon `drm`), forçable dans `kyberfrog.toml`.
- **Chemins XDG sous Linux** : `~/.config/kyberfrog` et `~/.local/state/kyberfrog`.

### Modifié
- L'encodeur est un réglage de la machine : une clé `encoder` dans un setup est ignorée.
- Sortie Spout zero-copy par défaut (#28-2) : `KYSPOUT_SMEM=1` restaure le chemin CPU.
- Tuiles Spout et « Tout envoyer » masquées quand le serveur n'est pas Windows.

### Corrigé
- `ui/dist` recopié à côté de l'exe à chaque build : plus de dashboard périmé.
- `http://localhost:7700` joignable sous Windows (écoute aussi en IPv6).
- La fenêtre de l'app affiche la nouvelle UI après une mise à jour (`Cache-Control: no-cache`).

### Limitations connues
- Linux : pas de capture d'écran sous Wayland GNOME / KDE — utiliser une session X11.
- Linux : liste des webcams V4L2 pas encore alimentée (#32).
- Linux : un serveur PulseAudio / PipeWire est requis (#31).
- NVENC jamais testé (couvert par le repli x264) ; la caméra passe par ce repli.

### CI / build
- Chaîne Linux `build-fork-linux` → `deb` → `release-deb`, sans jamais bloquer la release Windows.
- Pipelines limités aux MR, à `dev`, à la branche par défaut et aux tags ; plus de job manuel.
- Banc de latence KyberFrog vs NDI (`bench/`) et site de documentation restylé.

## [0.5.1] — 2026-07-18

### Corrigé
- Build strict du site de documentation (lien sortant de `docs/`).

## [0.5.0] — 2026-07-18

### Ajouté
- **Application Windows native** (#21) : le cockpit s'ouvre dans une vraie
  fenêtre (Tauri/WebView2) au lieu du navigateur — même serveur embarqué sur
  7700, zéro réécriture d'UI, le navigateur reste utilisable en parallèle.
  Fermer la fenêtre la cache seulement ; l'app ne s'arrête que par
  « Quitter » du tray. Clic **gauche** sur l'icône tray = ouvrir/focus le
  dashboard, clic droit = menu. **Plus aucune invite de commande** : ni au
  lancement (`windows_subsystem`), ni au (re)démarrage des enfants
  (`CREATE_NO_WINDOW` sur kycontroller / ffmpeg — vérifié jusqu'à
  kyavserver). Icône de fenêtre/taskbar = logo embarqué. Installeur :
  vérif/installation automatique du runtime **WebView2** (bootstrapper
  Evergreen embarqué, détection registre) et livraison de
  `WebView2Loader.dll` (obligatoire en windows-gnu). **Validé E2E sur
  machine réelle le 2026-07-15** (install silencieuse, fenêtre native,
  AtLogOn, cycle close/tray/quit).
- **Feedback au téléchargement d'une config** (#21) : la fenêtre native n'a
  pas la barre de téléchargement d'un navigateur — « télécharger une config »
  laissait le `.toml` arriver silencieusement dans `Téléchargements` sans que
  rien ne semble se passer. Le fichier est désormais **révélé dans
  l'Explorateur** en fin de téléchargement (handler `on_download` du shell,
  destination WebView2 inchangée).

### Modifié
- **Polish du cockpit web** (#22 — le hover global cohérent reste à venir) :
  - Header repensé : bloc Hostname/IP empilé (IP copiable au clic), LED d'état
    respirante avec « En ligne » en tooltip — l'indicateur-pilule qui
    ressemblait à un bouton disparaît.
  - La modale « À propos » devient **« Options »** (roue crantée dans le
    header) : les choix de thème (clair/sombre) et de langue (FR/EN) y migrent
    depuis le header ; libellés de la modale traduits FR/EN.
  - Responsive vertical : en fenêtre étroite, les sections Émission/Réception
    se dimensionnent sur leur contenu (suppression du vide forcé, la page
    défile).
  - Le bouton **Supprimer** des tuiles est peint en rouge — il paraissait
    désactivé (gris éteint) alors qu'il est actif.
- Formulaires : les protocoles « à venir » (NDI, SRT, Syphon, redirection NDI,
  Enregistrement) ne sont plus proposés dans les pickers de sources/récepteurs ;
  un nouveau récepteur est créé **fenêtré** par défaut (plein écran décoché) ;
  libellés « Tout envoyer » traduits FR/EN.

### CI / build
- `build-fork` : fetch du SHA pinné au lieu de `clone --branch`, meson ≥ 1.10
  installé (requis par kymedia 0.27), python3-pip ajouté (absent de l'image
  registry).
- Outillage rebase de la chaîne de forks : `packaging/rebase-fork.sh` +
  `fork-lint.sh` + skill Claude versionnée, audit complet dans
  `docs/dev/audit-fork-chain.md` (plans de restructuration inclus).

## [0.4.0] — 2026-07-06

### Ajouté
- **Webcam Windows (DirectShow)** (#18-A) : nouvelle source `Camera` avec
  picker de périphérique (`GET /cameras`), épinglage par CRC identique au
  Spout. Validé E2E hardware. Côté fork : 7 bugs latents corrigés dans le
  chemin lavd de txproto (enregistrement du backend, schéma d'identifiants,
  init COM/STA, tagging IOType, noms lisibles, syntaxe d'ouverture dshow,
  graphe caméra CPU pour x264).
- **Auto-découverte mDNS des transmetteurs** (#20) : chaque transmetteur actif
  est annoncé en `_kyber._tcp.local.` ; le formulaire viewer liste les
  « Émetteurs détectés » et pré-remplit nom/IP/port au clic. Opt-out
  `mdns = false` (file-only). Règle pare-feu UDP 5353 gérée par l'installeur.
- **Sources scindées par transmetteur + mode « Tout envoyer »** (#19) : un
  transmetteur n'expose que les sources de son type (Écran → moniteurs,
  Spout → le sender épinglé) ; nouveau mode global « Tout envoyer ». Côté
  fork : l'énumération Spout est scopée au sender épinglé (le picker listait
  tous les senders live du système).
- **Sélection de l'écran source côté viewer** (#18-B) : champ `display_idx` +
  picker alimenté par `GET /displays` (interroge l'émetteur distant), repli
  sur saisie manuelle.
- **Transmetteurs éditables après création** (`POST /transmitters/:name`,
  drawer réutilisé).
- Viewer renommable depuis le formulaire d'édition.

### Corrigé
- **Caméra en « Tout envoyer »** : créer un récepteur sur une caméra d'un
  transmetteur « Tout envoyer » ouvrait une fenêtre kyclient vide (pipeline
  vidéo en échec à l'init : graphe `hwdownload` écran/Spout appliqué aux
  frames CPU de la caméra) et la caméra ne démarrait jamais. Côté fork :
  kyavservice résout désormais le type de la source demandée (SPClass via
  txproto-rs) et route les entrées lavd vers le graphe x264 CPU de #18-A ;
  l'énumération lavd ne liste plus les périphériques audio (micros tagués
  par `media_types`) ni les grabbers legacy (gdigrab/VfW) comme écrans.
  Validé E2E hardware (webcam DirectShow PC-LM1E).
- **Spout taille native** (#8) : le flux garde la résolution du sender (plus
  de rescale) ; couleurs corrigées. Validé E2E contre Resolume Arena.
  Limitation connue : redémarrer le transmetteur si le sender change de
  résolution en cours de stream.
- **Remote desktop — phase 1** (#17) : scales X/Y séparés dans la projection
  souris (kynput) + accumulation des deltas fractionnaires (kyclient). Validé
  E2E paysage → paysage. Restent ouverts : écrans verticaux (rotation),
  Ctrl+Alt+F sous keyboard grab, accélération pointeur Windows.
- Bouton **Stop** actif pendant starting/restarting — les crash-loops
  peuvent être arrêtées.
- kyclient : les sources énumérées en 0x0 (dimensions inconnues avant
  ouverture du device) ouvrent une fenêtre 1280×720 par défaut.
- Énumération caméras robuste (indépendante du tag de log `[dshow @ ...]`).
- Config : `Emission.send_all` sérialisé avant les tables (TOML valide).

### CI / build
- Le bundle fork est buildé depuis la branche d'intégration `dev` de
  kyber-desktop ; URLs de sous-modules corrigées pour le clone récursif CI.
- Timeout `build-fork` 1h30 → 3h ; sortie de compilation redirigée vers
  l'artefact `fork-build.log` (la limite de trace GitLab de 4 Mo tronquait
  les logs) + heartbeat 1 ligne/min.

## [0.3.0] — 2026-06-26

### Ajouté
- **Design V2 du cockpit web** (thème Collecti'Frog) appliqué à toute l'IHM.
- **Setups** : config scindée machine/spectacle, sauvegarde et chargement de
  setups nommés.
- Easter egg : thème Collecti'frog après 5 clics sur le logo.

### Corrigé
- UI : boutons toggle violets, bordure parasite sur la barre d'actions des
  tuiles ; liens documentation du README.

## [0.2.3] — 2026-06-21

### Modifié
- La version provient du tag git (injectée au build) — plus de bump manuel
  de `Cargo.toml`.

## [0.2.2] — 2026-06-21

### Corrigé
- Installeur : l'UI web est embarquée (le dashboard renvoyait 404 sur une
  installation propre).

## [0.2.1] — 2026-06-20

### Modifié
- Version de rodage du pipeline de release (aucun changement fonctionnel).

## [0.2.0] — 2026-06-20

### Ajouté
- **Refonte complète de l'IHM web** en React + Vite.
- **Remote-control viewer** (#10) : prise de contrôle du bureau distant
  depuis un viewer.
- Site de documentation MkDocs (manuel utilisateur EN/FR) publié sur GitLab
  Pages ; README produit ; licence AGPL-3.0.

### CI
- Job `test` (cargo test) ; job Pages.

## [0.1.0] — 2026-06-19

Première release.

### Ajouté
- **App unifiée** : un binaire pour l'émission (N kycontroller) et la
  réception (N kyclient), un superviseur avec redémarrage à backoff, une UI
  web (port 7700), un tray, une config `%APPDATA%\kyberfrog\kyberfrog.toml`.
- **Sortie Spout par viewer** : re-publication du flux reçu en sender Spout
  (Resolume/MadMapper), validée contre Resolume.
- **Installeur Windows single-file** (NSIS) embarquant les binaires du fork,
  + pipeline de release GitLab (tag `v*`).
- Job Object Windows : tous les enfants sont tués si KyberFrog meurt.
- Icône embarquée dans l'exe ; statuts tray par forme (`○●◐✗`).

[0.5.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.4.0...v0.5.0
[0.4.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.3...v0.3.0
[0.2.3]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.2...v0.2.3
[0.2.2]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.1...v0.2.2
[0.2.1]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.0...v0.2.1
[0.2.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.1.0...v0.2.0
[0.1.0]: https://gitlab.com/kyber-frog/kyberfrog/-/tags/v0.1.0
