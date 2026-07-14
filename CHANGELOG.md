# Changelog

Tous les changements notables de KyberFrog, version par version. Le format
suit l'esprit de [Keep a Changelog](https://keepachangelog.com/fr/) ; la
**version fait foi via le tag git** (`v*`, injectée au build — pas de bump
`Cargo.toml`). Le backlog vit dans [`IMPROVEMENTS.md`](IMPROVEMENTS.md) et
[`TODO.md`](TODO.md) ; les `#N` ci-dessous y renvoient.

## [Non publié]

### Ajouté
- **Application Windows native** (#21, phases 0+1, branche `feat/tauri`) : le
  cockpit s'ouvre dans une vraie fenêtre (Tauri/WebView2) au lieu du
  navigateur — même serveur embarqué sur 7700, zéro réécriture d'UI, le
  navigateur reste utilisable en parallèle. Fermer la fenêtre la cache
  seulement ; l'app ne s'arrête que par « Quitter » du tray. Clic **gauche**
  sur l'icône tray = ouvrir/focus le dashboard, clic droit = menu. **Plus
  aucune invite de commande** : ni au lancement (`windows_subsystem`), ni au
  (re)démarrage des enfants (`CREATE_NO_WINDOW` sur kycontroller / ffmpeg —
  vérifié jusqu'à kyavserver). Icône de fenêtre/taskbar = logo embarqué.
  Restent : bootstrap WebView2 dans l'installeur, E2E manuel (TODO.md).

### Modifié
- **Polish du cockpit web** (#22 — reste le hover global, planifié après #21) :
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

[0.4.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.3...v0.3.0
[0.2.3]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.2...v0.2.3
[0.2.2]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.1...v0.2.2
[0.2.1]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.2.0...v0.2.1
[0.2.0]: https://gitlab.com/kyber-frog/kyberfrog/-/compare/v0.1.0...v0.2.0
[0.1.0]: https://gitlab.com/kyber-frog/kyberfrog/-/tags/v0.1.0
