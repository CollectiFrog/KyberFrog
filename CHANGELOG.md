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
- **Encodeur GPU par défaut** (#28-1) : nouveau réglage machine `encoder`
  (`auto` par défaut, `x264`, `amf`, `nvenc`, `qsv`), choisi dans **Options →
  Encodage vidéo**, avec le nom de la carte graphique détectée. `auto` prend
  l'encodeur matériel du GPU principal (AMF sur AMD, NVENC sur NVIDIA) et
  retombe sur x264 sinon (Intel/QSV pas encore validé, donc jamais choisi
  automatiquement). Mesuré sur le banc de latence (RX 7800 XT, source Spout
  1080p60, 20 Mbps) : **~4 ms Spout → Spout au lieu de ~26 ms** avec x264,
  tenu 10 min sans perte notable ; le crash AMF « en boucle silencieuse » qui
  avait imposé x264 n'est pas reproduit avec le bundle actuel. Le changement
  s'applique au prochain démarrage de chaque transmetteur.
- **Repli automatique sur x264** si l'encodeur GPU échoue sur un transmetteur
  (par exemple une webcam en AMF : `Unsupported pixel format: yuvj422p`) :
  KyberFrog repère l'erreur dans le log du transmetteur, le relance aussitôt en
  x264 et l'affiche sur sa tuile (« x264 (repli) »). Le repli tient jusqu'au
  prochain changement d'encodage ou redémarrage de l'app.
- **Support Linux amd64** : KyberFrog s'installe sur Debian 13 / Ubuntu 24.04
  (ou plus récent) via un paquet `kyberfrog_<version>_amd64.deb` autonome,
  publié par la CI à côté de l'installeur Windows. Il embarque les binaires du
  fork Kyber et leurs bibliothèques, **calcule** ses ~70 dépendances système,
  installe un service systemd **utilisateur** démarré au login graphique et une
  règle udev donnant au groupe `input` l'accès à `/dev/uinput` (nécessaire au
  remote control). Validé de bout en bout sur une VM Debian 13 / Xfce /
  lightdm : capture d'écran, viewer, souris + clics + clavier à distance,
  découverte mDNS, autostart réel, cycle install → upgrade → purge. Voir la
  section Linux de `docs/user/installation.md`.
- **Backend de capture explicite** (`screen_backend`) : `nvfbc`, `drm`, `xcb`
  ou `wlroots`, auto-détecté depuis la session et écrit dans `kyberfrog.toml`.
  C'est un réglage **machine** : il décrit la session, il ne voyage jamais dans
  un setup sauvegardé.
- **Chemins XDG sous Linux** : `~/.config/kyberfrog` (config et setups),
  `~/.local/state/kyberfrog` (logs et instances).
- **CI** : jobs `build-fork-linux`, `deb` et `release-deb`, et une image de
  build Linux reproductible depuis le dépôt (`ops/docker-images/debian-linux/`).
  Sur un tag, la chaîne Linux ne peut pas retenir la release Windows.

### Modifié
- **L'encodeur n'est plus pris dans le setup** : comme `screen_backend`, c'est
  un réglage de la machine. Une clé `encoder` dans `[emission.defaults.kyavserver]`
  (celle de l'ancien exemple de config, `"x264"`) est ignorée, avec un
  avertissement dans le log ; `[kyavserver.video_encoder_config]` reste
  appliqué tel quel.
- **Sortie Spout zero-copy par défaut** (#28-2, bundle fork mis à jour) : libVLC
  rend directement dans la texture Spout partagée, GPU → GPU, sans aller-retour
  CPU. `KYSPOUT_SMEM=1` restaure le chemin CPU. Voir
  `docs/dev/plan-spout-zerocopy.md`.
- **UI filtrée par plateforme** : les tuiles Spout et « Tout envoyer », sans
  équivalent Linux, sont masquées quand le serveur n'est pas Windows.
- **CI, surface unique** : les pipelines ne se jouent plus que sur les MR, sur
  `dev`, sur la branche par défaut et sur les tags — plus aucun job manuel, et
  plus de pipeline en double quand une branche a une MR ouverte.

### Corrigé
- **Dashboard toujours à jour après un `cargo build`** : le build recopie
  désormais `ui/dist` à côté de l'exe (au lieu d'une copie manuelle oubliée qui
  servait une vieille UI, sans le bouton Options ni le réglage Encodage vidéo),
  et avertit si `ui/dist` manque ou est plus ancien que `ui/src`.
- **`http://localhost:7700` joignable** : l'UI écoute aussi sur la boucle IPv6
  (`[::1]`) ; `localhost` y résout en premier sous Windows et le serveur,
  IPv4 seul, n'était joignable que par l'IP LAN.
- **Fenêtre de l'app à jour après une mise à jour** : l'UI est servie en
  `Cache-Control: no-cache` ; WebView2 gardait sinon l'ancien `index.html` en
  cache (et donc l'ancienne UI) alors qu'un navigateur affichait la nouvelle.
- **Dépendances du `.deb` calculées, jamais inventées** : `build-deb.sh`
  retombait en silence sur `Depends: libc6` quand `dpkg-shlibdeps` échouait.
  Le paquet s'installait proprement puis mourait au démarrage sur une `.so`
  manquante ; le repli est supprimé au profit d'un échec explicite.

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
