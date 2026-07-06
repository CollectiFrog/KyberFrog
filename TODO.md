# TODO — chantiers KyberFrog

Mis à jour le **2026-07-06**. Le backlog canonique (quoi/pourquoi/comment) reste
[`IMPROVEMENTS.md`](IMPROVEMENTS.md) ; les `#N` ci-dessous y renvoient.
L'historique des livraisons (v0.1.0 → v0.4.0) est dans
[`CHANGELOG.md`](CHANGELOG.md).

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

Phase 1 livrée et **validée E2E paysage → paysage le 2026-07-05**, embarquée
en 0.4.0 (voir CHANGELOG.md). Reste :

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

## 🆕 Nouvelles pistes (2026-07-06, à prioriser)

- [ ] **#21** — App Windows native (Tauri) autour de l'IHM React+Vite
  existante, plus de fenêtre console visible. Après stabilisation #17.
- [ ] **#22** — Polish UI cockpit web (responsive vertical, hover boutons
  cohérent, rework du header) — passe d'archi/design à faire avant tout code.
- [ ] **#23** — Drawers → Modals : phase de réflexion/comparaison d'abord, pas
  de code avant décision.
- [ ] **#24** — Simplifier l'environnement de dev (sortir de la chaîne de
  forks imbriqués kyber-desktop/kysdk/… ?).
- [ ] **#25** — Réduire la divergence des forks kyber/vlc, évaluer une
  contribution upstream. Lié à #24.
- [ ] **#26** — Variante #18-B (écran figé côté émetteur) : pure réflexion,
  pas urgent — #18-B actuel fonctionne bien et remplit le use case.

## 🔧 Rebase fork sur kyber upstream

- [ ] Rebaser la chaîne de forks (kyber-desktop, kysdk et sous-modules) sur
  **kyber 0.27.0** upstream.

## 🔌 Chantier D — #18 Sources & exports étendus *(backlog non planifié)*

Items indépendants, dans l'ordre de complexité croissante *(A webcam et B
sélection d'écran livrés en 0.4.0/0.1.0, voir CHANGELOG.md)* :

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
- **#21** — App Tauri (après stabilisation remote desktop, voir Nouvelles
  pistes ci-dessous).

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
