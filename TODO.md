# TODO — chantiers KyberFrog

Mis à jour le **2026-07-05**. Le backlog canonique (quoi/pourquoi/comment) reste
[`IMPROVEMENTS.md`](IMPROVEMENTS.md) ; les `#N` ci-dessous y renvoient.

## ✅ Terminé

- **#18-A — Webcam Windows (DirectShow)** : `Source::Camera` + pin
  `[kyavserver].camera_device` + picker `GET /cameras` (ffmpeg dshow), après
  7 fixes fork dans le chemin lavd de txproto (jamais testé hardware) + fix
  kyclient fenêtre 0x0. **Validé E2E hardware le 2026-07-05** (PC-LM1E →
  viewer, image OK). Bonus livrés au passage : transmetteurs **éditables**
  après création (`POST /transmitters/:name`, drawer réutilisé), bouton
  **Stop** actif pendant starting/restarting (crash-loops arrêtables).
  Détail : `IMPROVEMENTS.md #18-A`. ✅
- **#17 phase 1 — Remote desktop utilisable (paysage)** : scales X/Y séparés
  dans `VideoLayout` (kynput, +5 tests), accumulateur de deltas fractionnaires
  (kyclient), chaîne fork buildée dans le bundle. **Validé E2E hardware le
  2026-07-05** (remote desktop paysage → paysage, souris + canal input émetteur
  OK) — embarqué en 0.4.0. Restent phases 2–3 (écran vertical B1, Ctrl+Alt+F,
  accélération pointeur) : voir Chantier suivant. ✅
- **#20 — Auto-découverte mDNS des transmetteurs** : KyberFrog annonce chaque
  transmetteur actif (`_kyber._tcp.local.`, crate `mdns-sd`) et browse le LAN
  pour peupler le picker « Émetteurs détectés » du formulaire viewer (clic →
  nom/IP/port pré-remplis), repli sur saisie manuelle inchangé. Zéro
  changement fork — déviation assumée vs l'archi initiale « kycontroller
  announcer » (KyberFrog connaît déjà nom+port de ses transmetteurs). Opt-out
  file-only `mdns = false`. Règle pare-feu NSIS ajoutée. Mergé `dev`
  (`feat/mdns-discovery`, 2026-07-03). Détail : `IMPROVEMENTS.md #20`. ✅
- **CI timeout** `build-fork` 3h → 1h30. ✅
- **Chantier A — Documentation** : README scindé, MkDocs bilingue EN+FR,
  GitLab Pages activé → https://kyber-anysource-b41fc4.gitlab.io/ ✅
- **Chantier B — IHM Web** : React + Vite, design Collecti'Frog, cockpit
  Émission/Réception, remote-control viewer (KyberFrog side). ✅
- **C1 — Job `test`** CI. ✅
- **#18-B — Sélection d'écran source** : côté **réception** (`Viewer::display_idx`
  → arg kyclient `--display-idx`), picker UI via `GET /displays` qui interroge le
  `/enumerate_displays` de l'émetteur, repli saisie manuelle de l'index. Champ mort
  `Source::Screen { display }` retiré. Variante émission (fork) notée dans
  `IMPROVEMENTS.md #18`. ✅
- **#8 — Spout taille native** : `vlc-rs@7393f95` + `kyctl@e114926`, intégrés
  `dev` jusqu'à `kyber-desktop@00bf3bf`, bundle rebuild local. **Validé E2E
  hardware le 2026-07-03** contre Resolume Arena : taille native + couleurs OK,
  resize mid-stream OK (limitation acceptée : le transmetteur se fige et doit
  être redémarré manuellement au changement de résolution — non corrigée, pas
  prévue). Détail : `IMPROVEMENTS.md #8`. ✅
- **#19 — scoping Spout non appliqué à l'énumération** : `enumerate_displays`
  remontait tous les senders Spout live au lieu du seul sender pinné. Corrigé
  côté fork (`kymedia@fix/spout-enumerate-scoping` → `dev`, propagé
  `kysdk@d96b9c4` → `kyber-desktop@368bc00`) + nicety picker UI (retrait du
  bouton auto, ajout refresh). **Validé manuellement 2026-07-03.** Détail :
  `IMPROVEMENTS.md #19`. ✅

## 🐛 Reste à tester — #19 scénarios non liés au bug

Le scoping Spout est corrigé et validé, mais les scénarios écran-seul et
Tout-envoyer n'ont jamais été testés indépendamment :

- [ ] Retester écran-seul (transmetteur `screen` ⇒ moniteurs seuls) et
  Tout-envoyer (⇒ tout) côté validation visuelle.

## 🐛 Reste à tester — #20 validation 2 machines

Validé en solo (annonce + browse + `/discovered` corrects sur une seule
machine, smoke E2E réel). Reste à confirmer en conditions réelles :

- [ ] Machine A émettrice, machine B ouvre le formulaire viewer → l'instance
  apparaît dans « Émetteurs détectés » en quelques secondes.
- [ ] Couper le transmetteur sur A → l'entrée disparaît côté B (paquet goodbye
  / TTL).
- [ ] Vérifier que la règle pare-feu NSIS (UDP 5353) suffit sur une machine
  fraîchement installée (pas de build dev, exe non pré-autorisé manuellement).

## 🧭 Chantier suivant — #17 Remote desktop (phases 2–3)

Phase 1 livrée et **validée E2E paysage → paysage le 2026-07-05** (souris +
canal input émetteur OK, voir Terminé) — embarquée en 0.4.0. Reste :

- [ ] **Phase 2 — rotation écran vertical (B1)** : transpose GPU D3D11 dans
  txproto avant encode. Build fork complet ~1h30 + validation hardware écran
  vertical obligatoire.
- [ ] **Phase 3 — polish** : accélération pointeur Windows (P3b), valider
  **Ctrl+Alt+F** sous keyboard grab actif (B5/#15), logs de diag au resize.

## 🐧 Linux + ARM — tâches Romain Henry

> **Romain Henry** (contributeur) a accès à du hardware ARM. L'objectif est
> d'avoir une release `.deb` fonctionnelle pour AMD64 (x86) **et** ARM64.
> La branche de travail est `feat/linux-arm-support` sur `kyber-frog/kyberfrog`.
> À supprimer : branche/MR `feat/screenbackend-linux` (incluse dans la branche
> linux-arm).

**Tâches pour Romain :**

- [ ] **Build Linux x86 + test** : builder la branche `feat/linux-arm-support`
  en natif sur une machine Linux AMD64, valider que le `.deb` s'installe et
  qu'une source Screen fonctionne.
- [ ] **Build Linux ARM64 + test** : même chose sur hardware arm64 (Pi 4 /
  RK3588 ou équivalent), valider le `.deb` arm64.
- [ ] **Push image Docker arm64** :
  `docker push registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-arm64`
  (nécessaire pour que le job CI `build-fork-linux-arm64` puisse tourner).

**Tâches CI (à faire après les builds Romain) :**

- [ ] Merger la branche `feat/linux-arm-support` (après review + test).
- [ ] Vérifier/finaliser la CI : matrice `{amd64, arm64}`, `.deb` attachés à la
  release, runner arm64 (`saas-linux-medium-arm64` ou self-hosted).
- [ ] Supprimer la branche/MR `feat/screenbackend-linux`.
- [ ] Pin nouveaux SHAs fork dans `packaging/versions.sh` après merge.

## 🔌 Chantier D — #18 Sources & exports étendus *(backlog non planifié)*

Items indépendants, dans l'ordre de complexité croissante *(B et A livrés,
voir Terminé)* :

- [ ] **D — SRT / RTSP input** : variant `Source::Url { url }` dans KyberFrog,
  à valider que txproto accepte une URL `rtsp://`/`srt://` comme entrée.
- [ ] **F — SRT / RTSP output** : sortie réseau d'un flux reçu, via FFmpeg/kyvlcplayer.
- [ ] **C — NDI input** : plugin libndi dans le build fork + `Source::Ndi { name }`.
  *Dépendance lourde (libndi propriétaire).*
- [ ] **E — NDI output** : même dépendance que C.

## 📋 Déféré (pas pour maintenant)

- **C2** — étoffer les tests unitaires (`app.rs`, `kyclient_args`, `gen.rs`).
  À faire quand il y a du token disponible.
- **#2** — SSE log streaming (optimisation, le polling actuel est acceptable).
- **#3** — Gestion credentials dans l'UI (réseau fermé, pas urgent).
- **#1** — Ciblage moniteur de sortie (bloqué upstream kyclient/winit).
- **#8 zero-copy GPU** — output callbacks D3D11 libVLC 4 (post taille native,
  nécessite libVLC 4 côté fork).
- **#16** — ~~Menu clic-droit kyclient~~ **ANNULÉ** (incompatible remote desktop).
- **Étape 3** — App Tauri (après stabilisation remote desktop).

## 🤳 kyberfrog-cast — en attente de définition

Avant toute implémentation, **définir les use cases** et faire un **rétro-planning
des fonctionnalités** :

- [ ] Lister les cas d'usage concrets (VJ cam téléphone, régie distante, autre ?).
- [ ] Prioriser les fonctionnalités (foreground service, rotation, multi-cam,
  écran, uniffi, APK release signé…).
- [ ] Évaluer la faisabilité technique de chaque point.
- [ ] Créer les tâches dans le tracker.

Le cœur technique (E2E caméra → Kyber → PC) est **prouvé**. Ce travail de
définition est le prérequis avant d'attaquer le polish ou de nouvelles features.
