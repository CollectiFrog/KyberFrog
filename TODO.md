# TODO — chantiers KyberFrog

Mis à jour le **2026-07-01**. Le backlog canonique (quoi/pourquoi/comment) reste
[`IMPROVEMENTS.md`](IMPROVEMENTS.md) ; les `#N` ci-dessous y renvoient.

## ✅ Terminé

- **CI timeout** `build-fork` 3h → 1h30. ✅
- **Chantier A — Documentation** : README scindé, MkDocs bilingue EN+FR,
  GitLab Pages activé → https://kyber-anysource-b41fc4.gitlab.io/ ✅
- **Chantier B — IHM Web** : React + Vite, design Collecti'Frog, cockpit
  Émission/Réception, remote-control viewer (KyberFrog side). ✅
- **C1 — Job `test`** CI. ✅

## 🔥 Chantier prioritaire — #8 Spout taille native

L'image Spout est aujourd'hui forcée à 1920×1080 → Resolume reçoit une image
déformée si la source n'est pas en 1080p, et doit ré-étaler dans Arena. À
corriger.

**Plan prêt** (voir `IMPROVEMENTS.md #8`) :

- [ ] **Fork `vlc-rs`** : ajouter `set_video_format_callbacks(setup, cleanup)`
  (wrapper sûr autour du FFI `libvlc_video_set_format_callbacks` déjà présent).
- [ ] **Fork `kyvlcplayer`** : dans `setup_spout_output`, remplacer
  `set_video_format("BGRA", 1920, 1080, 1920*4)` par le callback `setup` qui
  lit la taille native du flux, crée/resize le `SpoutSender` + buffer `SpoutCtx`.
- [ ] **Bump submodules** (`vlc-rs` → `core/kysdk/kymedia/external/vlc-rs`,
  puis `kysdk` → `apps/kyber-desktop/kysdk`), build complet, MR.
- [ ] **Validation visuelle** sur hardware (taille native + couleurs — le bug
  chroma BGRA ne se voit qu'au runtime).

## 🧭 Chantier suivant — #17 Remote desktop (rework)

La feature existe côté KyberFrog mais est **inutilisable** :

- [ ] Diagnostiquer l'**inversion X/Y des contrôles** (kyclient / kynput /
  kyavserver — à identifier).
- [ ] Valider **Ctrl+Alt+F** sous keyboard grab actif (#15).
- [ ] Valider le **canal input côté émetteur** (source Screen + inputs retours)
  end-to-end sur hardware.

Tout est fork-side (kyclient + kynput + kyavserver). Build ~1h + validation
hardware requise.

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

Items indépendants, dans l'ordre de complexité croissante :

- [ ] **B — Sélection d'écran** : champ `display: Option<u32>` dans `Source::Screen`,
  injecté dans `gen.rs`, picker UI depuis `/enumerate_displays`. *Pas de changement
  fork — tout dans KyberFrog.* (le plus rapide)
- [ ] **D — SRT / RTSP input** : variant `Source::Url { url }` dans KyberFrog,
  à valider que txproto accepte une URL `rtsp://`/`srt://` comme entrée.
- [ ] **F — SRT / RTSP output** : sortie réseau d'un flux reçu, via FFmpeg/kyvlcplayer.
- [ ] **A — Webcam Windows** : iosys `dshow` dans txproto + `camera_device` Windows
  dans kyavservice + `Source::Camera` dans KyberFrog. *Fork txproto requis.*
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
