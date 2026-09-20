# Linux amd64 — architecture du portage

KyberFrog tourne sous Linux amd64 avec **la même application, la même chaîne de
forks et le même tag de release** que sous Windows. Le livrable est un paquet
Debian, `kyberfrog_<version>_amd64.deb`, publié à côté de
`KyberFrog-Setup.exe`. L'état fonctionnalité par fonctionnalité est dans
[todo-linux.md](todo-linux.md) ; le mode d'emploi utilisateur dans
[Installation](../user/installation.md#linux-debian-ubuntu-amd64).

**Périmètre : amd64.** Le cœur — transmetteur écran, viewer, remote control,
découverte mDNS, autostart — est validé de bout en bout sur une VM Debian 13 /
Xfce / lightdm.

## Application

- **Capture écran : KyberFrog écrit toujours `grab_backend`.** Le défaut du
  fork est `nvfbc`, qui ne capture rien hors NVIDIA. Le réglage
  `UserConf::screen_backend` vaut `auto` par défaut (non écrit dans
  `kyberfrog.toml`) et se **résout à chaque démarrage de transmetteur**
  (`ScreenBackendChoice::resolve`) : `WAYLAND_DISPLAY` / `XDG_SESSION_TYPE` →
  `wlroots`, `DISPLAY` → `xcb`, sinon `drm`. Les variables de session viennent
  de l'environnement de KyberFrog, ou de `systemctl --user show-environment`
  si le service a démarré avant le bureau (`kyberfrog/src/session.rs`) ; elles
  sont aussi transmises aux enfants. Réglage **de la machine**, jamais embarqué
  dans un setup exporté ; une valeur explicite dans `kyberfrog.toml` force le
  backend.
- **Supervision** : les deux équivalents Linux du Job Object Windows —
  `LD_LIBRARY_PATH` des enfants vers le `lib/` du bundle (ce que faisaient les
  wrappers `run_*.sh`), et `PR_SET_PDEATHSIG` pour que la mort du superviseur
  emporte ses enfants.
- **IPC** : `preflight_ipc_dir` détecte un `/tmp/kyber` inutilisable (résidu
  root) et l'explique, au lieu d'un échec de bind opaque.
- **Chemins XDG** : `$XDG_CONFIG_HOME/kyberfrog` (config, setups) et
  `$XDG_STATE_HOME/kyberfrog` (logs, instances).
- **Un seul front, filtré par plateforme** : `/status.platform` masque les tuiles
  Spout et le mode « Tout envoyer » (`spout_sender` et `all_sources` sont
  `cfg(windows)` dans le fork et seraient ignorés en silence).
- **Pas de tray ni de fenêtre native** : les stubs font tourner l'app en headless,
  le dashboard s'ouvre au navigateur (#34).
- **Fork** : `camera_device` en `cfg(linux)` (`kymedia` `7f0360f`, V4L2 via
  lavd, pin par CRC comme sous Windows).

## Paquet `.deb`

| Chemin | Contenu |
|---|---|
| `/usr/lib/kyberfrog/bin` | `kyberfrog`, `kycontroller`, `kyavserver`, `kyclient`, l'UI web |
| `/usr/lib/kyberfrog/lib` | bibliothèques du bundle, enregistrées par `/etc/ld.so.conf.d/kyberfrog.conf` |
| `/usr/bin/kyberfrog` | lien symbolique |
| `/usr/lib/systemd/user/kyberfrog.service` | autostart, activé pour tous les utilisateurs |
| `/usr/lib/udev/rules.d/99-kyberfrog-uinput.rules` + `modules-load.d` | accès `/dev/uinput` pour le groupe `input` |

- **Service systemd *user*, `WantedBy=default.target`.** Un viewer doit ouvrir
  sa fenêtre dans la session graphique, et un transmetteur écran a besoin de son
  display. `default.target` est atteint quel que soit le bureau, alors que
  `graphical-session.target` (`RefuseManualStart=yes`) n'est activé que par les
  gestionnaires de session GNOME/KDE. `DISPLAY` et `XAUTHORITY` sont présents
  dans l'environnement systemd user d'une vraie session graphique.
  `KillMode=control-group` emporte kycontroller et kyclient à l'arrêt.
- **Remote control via uinput.** kynput injecte le *mouvement* du pointeur par
  X11/XCB, mais les *clics* et le *clavier* par `/dev/uinput`, livré
  `root:root 0600` par Debian. La règle udev ouvre le device au groupe `input` ;
  le postinst rappelle `/usr/sbin/usermod -aG input` (chemin complet :
  `/usr/sbin` n'est pas dans le PATH d'un utilisateur normal).
- **Dépendances calculées** par `dpkg-shlibdeps` sur le bundle réel (~70
  paquets). Le calcul exige les bibliothèques système de l'image de build ;
  s'il échoue, `build-deb.sh` **s'arrête** et affiche la cause plutôt que de
  produire un paquet aux dépendances inventées.
- **`lintian --fail-on error`** en fin de job CI. Permissions normalisées par
  `build-deb.sh`, `changelog.gz` minimal. Findings conservés volontairement :

| Finding | Raison |
|---|---|
| `embedded-library` (ffmpeg/libav/zlib) | le bundle embarque son propre build patché, comme sous Windows : un seul paquet, rien à installer à côté |
| `maintainer-script-calls-systemctl`, `maintscript-calls-ldconfig` | pratique standard d'un `.deb` hors archive officielle |
| `command-with-path-in-maintainer-script` | `/usr/sbin/usermod` en chemin complet, voir ci-dessus |
| `no-manual-page` | cosmétique pour une appli daemon |
| `privacy-breach-generic` | Google Fonts dans `ui/dist/index.html`, commun à Windows — #29 |

## Build et CI

- **Image `debian-linux` reproductible depuis le dépôt**
  (`ops/docker-images/debian-linux/`) : dépendances apt de kyber-desktop,
  `apt-get build-dep vlc`, meson ≥ 1.10 par pip, Rust 1.89.0 + `cargo-c`. Le job
  `image-debian-linux` la construit (Kaniko) et la pousse.
- **Boucle de dev locale** : `packaging/linux/build-fork-local.sh`, ~20 min sur
  le poste contre ~1 h 30 en CI — voir [Building](building.md#building-for-linux-amd64).
- **CI** : `build-fork-linux` (cache par SHA fork) → `deb` → `release-deb` sur
  tag. Sur un tag, la chaîne Linux est `allow_failure` : elle ne retient jamais
  la release Windows — voir [Releasing](releasing.md#the-linux-chain-never-holds-back-a-windows-release).

## Cible d'exécution

Exigences mesurées sur le bundle produit (`objdump -T`) : `kycontroller` et
`kyclient` en `GLIBC_2.39`, `kyavserver` en `GLIBC_2.34`, bibliothèques en
`GLIBC_2.38`. **Plancher : glibc ≥ 2.39**, hérité de l'image
`debian:trixie-slim` (glibc n'est compatible que vers l'avant).

| Distro | glibc | |
|---|---|---|
| Debian 13 Trixie | 2.41 | ✅ |
| Ubuntu 24.04 LTS | 2.39 | ✅ |
| Debian 12 Bookworm | 2.36 | ❌ |
| Ubuntu 22.04 | 2.35 | ❌ |

**Session : X11**, backend `xcb` (capture SHM, sans accès KMS). Le backend
`wlroots` parle *wlroots screencopy*, que GNOME et KDE n'implémentent pas (ils
passent par xdg-desktop-portal/PipeWire).

## Bilan

**Pour**

- **Une seule base de code** : mêmes crates, même UI, même chaîne de forks ; les
  différences sont confinées aux stubs et à `/status.platform`.
- **Release couplée sans dépendance** : un tag publie `.exe` et `.deb`, un échec
  Linux ne bloque pas Windows.
- **Paquet autonome** : dépendances système calculées, bibliothèques média
  embarquées, `lintian` vérifié à chaque build.
- **Image de build reproductible** et boucle locale rapide.
- **Autostart indépendant du bureau** (`default.target`).

**Contre**

- **Plancher glibc 2.39** : Debian 12 et Ubuntu 22.04 exclus.
- **Wayland GNOME/KDE non capturé** — la session par défaut de Debian et Ubuntu.
- **Pas de tray ni de fenêtre native** ; pas de Spout ni de « Tout envoyer ».
- **Dépendance amont non forkée** pour `/tmp/kyber` (`kyutil`, #30) et un abort
  de `libpulse` sans serveur audio (#31).
- Deux builds fork lourds par pipeline, absorbés par le cache par SHA.

## Limites connues

- Énumération des caméras V4L2 absente (#32).
- VAAPI non câblé, `scale=w=1920` en dur sur le chemin x264 Linux (#33).

## arm64

**Périmètre : `kyberfrog_<version>_arm64.deb` installable sur Raspberry Pi OS
Lite Trixie**, construit sur un bundle fork arm64, publié par la CI à côté du
`.deb` amd64. C'est la phase **S1** de [KyberFrog
Satellite](backlog.md#item-46) (#35), et elle se fait ici, côté app.

### La machine arm64 : l'arbitrage

Deux contraintes interdisent de copier la chaîne amd64 telle quelle.

* **Kaniko ne cross-compile pas** : il produit l'image de l'architecture sur
  laquelle il tourne. Le runner de ce projet est l'exécuteur Kubernetes, où
  `docker:dind` n'obtient pas le pod privilégié qu'il lui faudrait (`Cannot
  connect to the Docker daemon`, essayé).
* **Les runners SaaS coupent à 3 h.** Le build fork amd64 natif coûte déjà
  ~1 h 30 ; le seul runner ARM du tier gratuit est `saas-linux-small-arm64`
  (2 vCPU / 8 Go), donc plus lent — et émulé, il ne finit pas.

D'où un partage, une arch par besoin plutôt qu'un runner pour tout :

| Étape | Où | Pourquoi |
|---|---|---|
| Bundle fork arm64 | **sur le poste**, conteneur `linux/arm64` émulé (qemu) | seul endroit sans limite de 3 h ; ne change qu'au bump de `versions.sh` |
| `build-fork-linux-arm64` | runner partagé amd64 | un cache hit, c'est un `curl` et un `tar` : aucune arch requise |
| `deb-arm64` | `saas-linux-small-arm64` | `dpkg-shlibdeps` doit résoudre les ~70 dépendances contre de vrais paquets arm64 ; un binaire Rust + `dpkg-deb`, ça tient largement en 3 h |
| `image-debian-linux-arm64` | `saas-linux-small-arm64` | Kaniko en natif, même Dockerfile, second tag `latest-arm64` |

Le bundle est poussé **une fois par SHA kyber-desktop** dans le Generic Package
Registry ; la CI ne fait plus que le cache hit. C'est la logique de
`build-fork-local.sh` poussée d'un cran : côté amd64 le build local *accélère*
la boucle, côté arm64 il la *remplace*. Corollaire assumé : sur un cache miss,
`build-fork-linux-arm64` échoue vite en imprimant la commande à lancer, au lieu
de démarrer un build qui ne finira pas.

**Toute la branche arm64 est `allow_failure`**, et pas seulement sur un tag
comme la chaîne amd64 : entre un bump de `versions.sh` et l'upload du bundle
elle est rouge par construction, et le `.deb` n'a encore été installé sur aucun
Pi. Elle devient bloquante le jour où ces deux points tombent.

### La chaîne de forks

`build-linux.sh` codait en dur `rootfs-x86_64-linux-gnu`, `kyber-linux-x86_64`
et les chemins multiarch `x86_64-linux-gnu` — un build arm64 déposait ses
artefacts là où personne ne les cherchait. Les trois commits qui dérivent
`ARCH_TRIPLET` de `uname -m` sont cherry-pickés (jamais mergés : ils ont été
écrits sur la base 0.26, les merger ramènerait cette base) sur
`feat/arm64-triplet` dans chaque dépôt :

| Dépôt | Origine | Sur `feat/arm64-triplet` |
|---|---|---|
| `kyber-desktop` | `a325609` | `bce3676` + bump kysdk `5b58c5e` |
| `kysdk` | — | bump kyctl + kymedia `2fd3e89` |
| `kysdk/kyctl` | `204d173` | `facd80c` |
| `kysdk/kymedia` | `e2d58e2` | `cfedffa` + `d0001e6` |

Deux écarts que les commits d'origine ne pouvaient pas connaître, rattrapés
dans `d0001e6` : depuis `build/linux: switch to meson`, `contrib/build-linux.sh`
n'existe plus, et **le gating x86 qu'il portait n'a jamais atteint la base
meson**. `subprojects/packagefiles/ffmpeg/meson.build` tirait NVENC
(`nv-codec-headers`, `ffnvcodec`, `--enable-nvenc`) et oneVPL pour tout CPU dès
lors que le système était Linux : ni l'un ni l'autre n'a de cible aarch64, et un
Pi n'a ni GPU NVIDIA ni GPU Intel. Un `_is_x86` unique
(`host_machine.cpu_family()`) les gate tous ; libdrm, vulkan, x264 et opus
restent actifs sur toutes les arches. Second écart : `PKG_CONFIG_LIBDIR`, ligne
de l'ère meson, pinnait encore `x86_64-linux-gnu`.

`packaging/versions.sh` pointe ce nouveau SHA `kyber-desktop`. Le cache des
deux jobs fork étant keyé par ce SHA, **le bump coûte un rebuild Windows et
amd64 complets** au premier pipeline qui le voit, alors qu'aucun de ces commits
n'est atteint sur un hôte x86_64.

### Cible d'exécution

Le bundle est construit sur `debian:trixie-slim` arm64, **glibc 2.41** — la
même que Raspberry Pi OS Lite Trixie. Le job `deb-arm64` vérifie les deux
preuves sur le paquet qu'il vient de produire, pour qu'une régression de
toolchain rougisse le pipeline plutôt qu'un Pi :

```sh
file kyavserver                     # ELF 64-bit LSB … ARM aarch64
objdump -T kyavserver | grep GLIBC  # rien au-dessus de GLIBC_2.41
```

| Distro | glibc | |
|---|---|---|
| Raspberry Pi OS Lite Trixie | 2.41 | ✅ |
| Debian 13 Trixie arm64 | 2.41 | ✅ |
| Ubuntu 24.04 LTS arm64 | 2.39 | ❌ (plancher arm64 plus haut qu'en amd64) |

### Durées mesurées

Le plan Satellite porte la durée d'un build arm64 comme « à confirmer ». Ce
qui est mesuré à ce jour, sur le poste dev (12 cœurs, conteneur `linux/arm64`
émulé par qemu) :

| Étape | Durée | |
|---|---|---|
| Image `debian-linux` arm64 (`docker build`) | **> 35 min** | apt + `build-dep vlc` ~11 min, puis `cargo install cargo-c` qui domine le reste |
| Bundle fork arm64 (`build-linux.sh -p`) | **à confirmer** | à froid ; référence amd64 native : ~20 min sur ce poste, ~1 h 30 en CI |

L'image ne se reconstruit qu'au changement du Dockerfile, le bundle qu'au bump
de `packaging/versions.sh` — les deux sont hors de la boucle de dev courante.

### Ce qui reste à confirmer

* **L'installation sur un Pi 5** : le matériel n'est pas assemblé côté
  Satellite. Les deux preuves ELF/glibc sont vérifiées en conteneur, la
  troisième ne l'est pas.
* **La performance** : backend `drm` headless, encodeur **x264 logiciel
  uniquement** (pas de VAAPI, rkmpp non supporté par `kyavservice`) — c'est le
  go / no-go S0 du Satellite, pas une promesse de cette phase.
* **L'énumération V4L2** (#32), prérequis de l'étage capture du Satellite.
* **La disponibilité de `saas-linux-small-arm64`** sur le plan du projet :
  `deb-arm64` et `image-debian-linux-arm64` sont taggés pour lui, et c'est le
  premier pipeline qui le dira.
