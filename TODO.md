# TODO — chantiers KyberFrog

Mis à jour le **2026-07-18**. Le backlog canonique (quoi/pourquoi/comment) reste
[`IMPROVEMENTS.md`](IMPROVEMENTS.md) ; les `#N` ci-dessous y renvoient.
L'historique des livraisons (v0.1.0 → v0.5.0) est dans
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

- [ ] **#22** — Polish UI cockpit web : responsive vertical, rework header,
  modale Options (thème + langue) et fix bouton Supprimer (rouge actif) livrés
  en 0.5.0 (voir CHANGELOG.md). Reste : **hover cohérent** sur tous les boutons
  — archi arrêtée (voir IMPROVEMENTS.md §22), plus rien ne le bloque (#21 livré).
- [ ] **#23** — Drawers → Modals : phase de réflexion/comparaison d'abord, pas
  de code avant décision.
- [ ] **#24** — Simplifier l'environnement de dev (sortir de la chaîne de
  forks imbriqués kyber-desktop/kysdk/… ?). **Audit fait 2026-07-07**
  (`docs/dev/audit-fork-chain.md`) + plans proposés
  (`docs/dev/plans-fork-restructure.md`) — reste à arbitrer A → C → B?.
- [ ] **#25** — Réduire la divergence des forks kyber/vlc, évaluer une
  contribution upstream. Lié à #24. Inventaire fait (audit §2) : ~15
  bugfixes purs upstreamables en 1ʳᵉ vague.
- [ ] **#26** — Variante #18-B (écran figé côté émetteur) : pure réflexion,
  pas urgent — #18-B actuel fonctionne bien et remplit le use case.

## 🔧 Rebase fork sur kyber upstream

- [x] Rebaser la chaîne de forks sur **kyber 0.27.1** — cascade faite le
  2026-07-08 (branches `rebase/0.27.1` partout, voir
  [audit-fork-chain.md §6](docs/dev/audit-fork-chain.md)). Restant :
  - [ ] Validation : build complet `-p` (image `kyber/debian-win64:local-0.27`,
    meson ≥ 1.10 requis par kymedia 0.27) + smoke E2E.
  - [ ] Pushes `--force-with-lease` (5 repos : kyber-desktop, kysdk, kyctl,
    kymedia, txproto ; kynput et vlc-rs inchangés) — **accord utilisateur**.
  - [ ] Pinner `KYBER_DESKTOP_REF` (SHA `ac3d781…`) dans
    `packaging/versions.sh` + re-lancer `fork-lint.sh`.
- [ ] Migrer kyberfrog de `KYBER_CONFIG_PATH` vers `KYBER_CONFIG` (upstream
  0.27 l'implémente nativement et le ré-exporte aux services spawnés), puis
  dropper les 2 shims légataires kyctl/kymedia au rebase suivant.
- [ ] Image docker de build : l'ops upstream pinne `debian-win64:a3d1e28…`
  (registry non récupérable) ; le dérivé local `local-0.27` ajoute juste
  meson 1.11 via pip — à refaire proprement à l'occasion.

## 🔀 Chantier suivant — #27 Passthrough Spout (2 modes)

**Archi arrêtée le 2026-07-17** (voir IMPROVEMENTS.md §27). Ce chantier
**remplace** la demande initiale « plugin Resolume (FFGL) + plugin
TouchDesigner » : l'exploration a conclu qu'**aucun des deux ne doit être
écrit** — les 2 hôtes font déjà du Spout nativement, FFGL ne peut pas créer de
device de sortie, et un plugin ne ferait qu'ajouter un blit (donc de la latence)
sur le chemin natif.

**2 interrupteurs indépendants, un par sens** (décision opérateur) : l'infra
réelle est asymétrique (une machine envoie ses flux TD, une autre les reçoit
tous pour Resolume). Bénéfice majeur : **en sens unique la boucle est
impossible par construction** — le filtre anti-boucle ne couvre plus que la
machine bidirectionnelle.

- [ ] **Config** (`shared/src/config.rs`) : `emission.spout_passthrough` +
  `reception.spout_passthrough`, chacun avec son `spout_passthrough_exclude:
  Vec<String>` (fichier-only). ⚠️ **Déclarer les 4 champs AVANT** `defaults` /
  `transmitters` / `viewers` — *« TOML requires bare keys before `[table]` /
  `[[array]]` sections »* (raison déjà écrite pour `send_all`,
  `config.rs:211-213`). Tests round-trip.
- [ ] **Ops + 2 réconciliations indépendantes** (`app.rs`) : sur le **modèle
  exact de `op_set_send_all`** (`app.rs:349-383` — dérivé, ne mute jamais les
  listes, l'extinction restaure à l'identique). Émission ← `spout.rs`,
  réception ← `discovery.rs`, ~2 s. **Non modifiable par design** (cf. §27).
- [ ] **Combinaisons** : les 2 passthrough sont **indépendants et cumulables** —
  ils ne s'excluent **jamais**. ☑/☐ = machine émettrice, ☐/☑ = machine
  réceptrice (aucune boucle possible dans ces 2 cas), **☑/☑ = légitime →
  avertir + filtrer, NE JAMAIS BLOQUER** ni décocher l'autre. Seule exclusion
  réelle : `send_all` ⟷ **émission seulement** (`send_all` = 1 tx pour tout ;
  passthrough = N tx individuellement annoncés, ce dont la réception a besoin
  pour énumérer). `send_all` n'affecte **pas** la réception.
- [ ] **❓ Trancher** : que fait la réception face à un voisin `kind=all` ?
  Proposition = l'ignorer + logger, plutôt que deviner la source.
- [ ] **⚠️ Libellés** : ne pas réutiliser « Tout envoyer » (pris par `send_all`,
  mécanique différente) → « Publier tous les Spout » / « Recevoir tous les
  Kyber ».
- [ ] **⚠️ ANTI-BOUCLE — ne concerne que la machine bidirectionnelle** (détail +
  les 3 topologies dans IMPROVEMENTS.md §27). En sens unique il n'y a **rien à
  re-publier** → topologies 1 et 2 impossibles. Reste à couvrir : la machine qui
  active **les 2** sens (cas légitime → **bandeau d'avertissement UI**), où le
  ping-pong A↔B provoquerait une **explosion exponentielle**. Filtre de
  publication en **2 couches, les deux obligatoires** :
  - [ ] **Identité (primaire)** : exclure les senders dont le nom est
    exactement un `spout_out` de viewer dérivé vivant. Exact, zéro faux positif.
  - [ ] **Préfixe réservé `Kyber - ` (secondaire)** : rattrape les senders
    orphelins d'un kyclient crashé et les KyberFrog voisins sans filtre. Sans
    lui, un crash rouvre le ping-pong.
  - [ ] **UI** : avertir si un sender local matche `Kyber - ` sans correspondre
    à un viewer dérivé (sinon une source opérateur disparaît en silence).
  - [ ] **Tests** : sender = `spout_out` dérivé → jamais publié ; `Kyber - …`
    orphelin → jamais publié ; **`Kyber Out A` opérateur → publié** ; scénario
    A→B simulé → **aucun** tx chez B.
- [ ] **Anti-self** (`is_self`) — nécessaire mais **ne couvre pas** le
  ping-pong A↔B.
- [ ] **Anti-flap** : debounce (publier après N ticks stables, temporiser au
  démontage) — sinon Resolume qui recharge une compo fait spawner/tuer des
  kycontroller en rafale, en conflit avec le backoff du superviseur.
- [ ] **Plafond ~9 instances** (ports IPC kycontroller 9091..9100) avec message
  explicite, jamais d'échec silencieux — c'est aussi le **dernier rempart** qui
  borne les dégâts si une boucle passe malgré tout.
- [ ] **UI** : **une bascule par section** (Émission / Réception), chacune ne
  grisant **que son côté** — émission → `kind:"spout"`, réception →
  `spout_out` — avec **erreur explicite** côté endpoints (précédent :
  `app.rs:168-171`) ; lister les ressources dérivées en lecture seule avec le
  `spout_name` résolu ; bandeau si les 2 sens sont actifs. Miroir dans le tray.
- [ ] **Doc** `docs/user/spout-passthrough.md` : les 2 rôles types (machine
  émettrice TD / machine réceptrice Resolume), workflow Resolume (Advanced
  Output → Spout) et TD (Spout In/Out TOP), + limites (préfixe réservé
  `Kyber - `, boucle via l'app hôte = responsabilité opérateur, viewer actif
  d'office, 409 en `multi_client=false`, gel au changement de résolution,
  multi-GPU).
- [ ] **E2E** — testable **sur la seule machine de dev** (Resolume Arena 7.22.9
  et TouchDesigner 2025.32280 y sont installés ; loopback via `is_self`) :
  - [ ] **Rôle émetteur seul** : Spout Out TOP `Kyber Out A` → activer
    l'émission seule → 1 transmetteur, **0 viewer**, et vérifier qu'aucun
    sender `Kyber - …` n'est créé (rien à boucler).
  - [ ] **Rôle récepteur seul** : activer la réception seule → viewer +
    sender `Kyber - …` visible dans Resolume (*Sources → Spout Servers*) et
    dans le menu `sendername` d'un Spout In TOP, **0 transmetteur**.
  - [ ] **Bidirectionnel (le test qui compte)** : activer les 2 → bandeau
    d'avertissement affiché, et **aucun transmetteur créé pour un sender
    `Kyber - …`**.
  - [ ] Vérifier que chaque bascule ne grise **que son côté**, et que
    `send_all` n'entre en conflit qu'avec l'émission.
  - [ ] Extinction → listes manuelles restaurées à l'identique, TOML inchangé.

## ⏱️ #28 Latence bout-en-bout — analyse livrée, exécution côté fork

Analyse consignée le **2026-07-17** (IMPROVEMENTS.md §28). Travail **côté fork**
(kymedia/kyctl), pris en charge par l'opérateur — ne bloque pas #27.

- [ ] **Levier 1 (le plus gros, probablement)** : l'encodeur par défaut est
  **x264 CPU** *uniquement* parce qu'AMF crashe sur la RX 7800 XT
  (`gen.rs:62-63`) → download GPU→CPU + latence d'encodage. Reprendre AMF/NVENC
  + `zerolatency` (patch FFmpeg déjà dans l'arbre) / `intra_refresh`.
- [ ] **Levier 2** : réception — supprimer l'aller-retour GPU→CPU→GPU de
  `smem` (`kyvlcplayer/src/player.rs:189-259`) via les output callbacks D3D11 de
  libVLC 4. **Même chantier que « #8 zero-copy GPU » ci-dessous.** Bénéficie à
  Resolume **et** TD sans aucun plugin.
- [ ] **Levier 3** : `multi_client=false` (session unique = latence mini) —
  ⚠️ implique un **409** pour un 2e client, à arbitrer avec #27.
- [ ] **Mesurer d'abord** : timestamps `FrameAcquired → FrameDisplayed` déjà
  exposés (`capi.rs:1639`). Mesure sans code : dans TD, `Spout In TOP` sur le
  flux Kyber vs `Spout In TOP` direct sur le sender source.

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
  nécessite libVLC 4 côté fork). **= levier 2 de #28** : plus « déféré » depuis
  l'analyse latence du 2026-07-17, c'est le 2ᵉ gain de la chaîne.

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
