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
| Bundle fork arm64 | **sur le poste**, conteneur `linux/arm64` émulé (qemu) | seul endroit sans limite de 3 h ; ne change qu'au bump du pin `vendor/kyber-desktop` |
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
comme la chaîne amd64 : entre un bump du pin et l'upload du bundle
elle est rouge par construction, et le `.deb` n'a encore été installé sur aucun
Pi. Elle devient bloquante le jour où ces deux points tombent.

### La chaîne de forks

`build-linux.sh` codait en dur `rootfs-x86_64-linux-gnu`, `kyber-linux-x86_64`
et les chemins multiarch `x86_64-linux-gnu` — un build arm64 déposait ses
artefacts là où personne ne les cherchait. Les trois commits qui dérivent
`ARCH_TRIPLET` de `uname -m` sont cherry-pickés (jamais mergés : ils ont été
écrits sur la base 0.26, les merger ramènerait cette base) sur
`feat/arm64-triplet` dans chaque dépôt :

| Dépôt | Origine | Sur `feat/arm64-triplet` | Tête |
|---|---|---|---|
| `kyber-desktop` | `a325609` | cherry-pick + wrappers `run_*.sh` + bumps | `38d64eb` |
| `kysdk` | — | bumps des trois sous-modules | `b93bf57` |
| `kysdk/kyctl` | `204d173` | cherry-pick propre | `facd80c` |
| `kysdk/kymedia` | `e2d58e2` | cherry-pick adapté + gate x86 + portage `txproto-rs` | `fd0b23e` |
| `kysdk/kynput` | — | **non couvert par les commits d'origine** | `9720bd6` |

C'est le SHA `kyber-desktop` `38d64eb` que pinne le gitlink `vendor/kyber-desktop`.

**Les trois commits d'origine étaient nécessaires mais loin d'être
suffisants.** Ils portent les *scripts de build* d'un fork de juin ; trois
familles de blocages se sont ajoutées, chacune découverte par un build réel.

*Décalage d'époque (kymedia).* Depuis `build/linux: switch to meson`,
`contrib/build-linux.sh` n'existe plus, et **le gating x86 qu'il portait n'a
jamais atteint la base meson**. `subprojects/packagefiles/ffmpeg/meson.build`
tirait NVENC (`nv-codec-headers`, `ffnvcodec`, `--enable-nvenc`) et oneVPL pour
tout CPU dès lors que le système était Linux : ni l'un ni l'autre n'a de cible
aarch64, et un Pi n'a ni GPU NVIDIA ni GPU Intel. Un `_is_x86` unique
(`host_machine.cpu_family()`) les gate tous ; libdrm, vulkan, x264 et opus
restent actifs partout. Même époque, `PKG_CONFIG_LIBDIR` pinnait encore
`x86_64-linux-gnu`. Vérifié sur le `config.h` produit : `ARCH_AARCH64 1`,
`CONFIG_NVENC 0`, `CONFIG_LIBVPL 0`, `CONFIG_LIBX264 1`, `CONFIG_LIBDRM 1`,
`CONFIG_VULKAN 1` — et `CONFIG_V4L2_M2M 1`, le décodage matériel du Pi
auto-détecté.

*Un dépôt oublié (kynput).* `kynput` a son propre `build-linux.sh`, qu'aucun
commit de juin ne touchait. Le rootfs était correctement nommé
`rootfs-aarch64-linux-gnu`, mais le sous-dossier multiarch *à l'intérieur*
restait `x86_64-linux-gnu` : `kynput-sys` ne trouvait pas le `kynput.pc` qu'on
venait d'installer sous `lib/aarch64-linux-gnu/pkgconfig`. Les wrappers
`scripts/linux/run_*.sh` de kyber-desktop, eux, **partent dans le bundle** et
pointaient `LD_LIBRARY_PATH` vers `lib/x86_64-linux-gnu` — vers rien, sur un
Pi ; ils lisent désormais le triplet à l'exécution.

*Portage Rust (txproto-rs).* Le seul blocage qui ne soit pas un chemin. `va_list`
n'a pas la même forme selon l'arch, et l'écart va jusqu'à l'ABI : sur x86_64
`__gnuc_va_list` est `__va_list_tag[1]`, un **tableau**, donc le paramètre décae
et bindgen émet `*mut __va_list_tag` ; sur aarch64 c'est une **struct** (rendue
`[u64; 4]`) passée par valeur, et aucun `__va_list_tag` n'est généré. S'ajoute
le signe de `c_char`, signé sur x86_64 et non signé sur aarch64, qu'un littéral
`0i8` épinglait. Corrigé par `cfg(target_arch)`, x86_64 gardant exactement les
types qu'il avait.

Ce qui n'a **pas** eu besoin d'être touché : les bindings SDL2 de `kynput`,
pré-générés pour x86_64 et commités tels quels. Ils se compilent sur aarch64 —
ils définissent eux-mêmes `__va_list_tag` — et KyberFrog n'appelle aucune
fonction variadique de SDL. À reprendre le jour où ce serait le cas.

Le gitlink `vendor/kyber-desktop` pointe ce nouveau SHA. Le cache des
deux jobs fork étant keyé par ce SHA, **le bump coûte un rebuild Windows et
amd64 complets** au premier pipeline qui le voit, alors qu'aucun de ces commits
n'est atteint sur un hôte x86_64.

### Cible d'exécution

Le bundle est construit sur `debian:trixie-slim` arm64, la même base que
Raspberry Pi OS Lite Trixie. Le job `deb-arm64` vérifie les deux preuves sur le
paquet qu'il vient de produire, pour qu'une régression de toolchain rougisse le
pipeline plutôt qu'un Pi :

```sh
file kyavserver                     # ELF 64-bit LSB … ARM aarch64
objdump -T kyavserver | grep GLIBC  # rien au-dessus de GLIBC_2.41
```

Relevé sur le bundle du 2026-09-21 (`38d64eb`) :

```
kyavserver: ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV),
            dynamically linked, interpreter /lib/ld-linux-aarch64.so.1, stripped
kyavserver, symbole le plus haut : GLIBC_2.34
bundle entier, symbole le plus haut : GLIBC_2.39
```

`kycontroller` et `kyclient` sont dans le même état.

**Le plancher réel est donc `GLIBC_2.39`**, comme en amd64 : ce qui compte est
le symbole le plus haut *effectivement référencé*, pas la glibc de l'image de
build (2.41). Le contrôle CI est volontairement plus lâche que la mesure — il
échoue au-dessus de 2.41, la glibc du Pi — pour ne pas rougir sur un simple
changement de base d'image qui resterait installable sur la cible.

| Distro | glibc | |
|---|---|---|
| Raspberry Pi OS Lite Trixie | 2.41 | ✅ — la cible du Satellite |
| Debian 13 Trixie arm64 | 2.41 | ✅ |
| Ubuntu 24.04 LTS arm64 | 2.39 | ✅ (tout juste : c'est le symbole le plus haut du bundle) |
| Debian 12 Bookworm arm64 | 2.36 | ❌ |

Même plancher qu'en amd64, et pour la même raison : les symboles `GLIBC_2.39`
viennent de la glibc de la machine de build, pas de l'architecture. L'abaisser
(Debian 12, Ubuntu 22.04) est donc un chantier distinct et commun aux deux
arches — il a son étude, sur une autre branche à ce jour.

### Durées mesurées

Le plan Satellite porte la durée d'un build arm64 comme « à confirmer ». Ce
qui est mesuré à ce jour, sur le poste dev (12 cœurs, conteneur `linux/arm64`
émulé par qemu) :

| Étape | Durée | |
|---|---|---|
| Image `debian-linux` arm64 (`docker build`) | **39 min 51 s** | couches apt en cache ; à froid, ajouter les ~11 min d'`apt` + `build-dep vlc`. `cargo install cargo-c` domine le reste |
| Bundle fork, contrib à froid | **1 h 48 min 04 s** | FFmpeg, VLC, glslang, libplacebo — jusqu'aux crates Rust |
| Bundle fork, cycle sur le volume chaud | **36 min** à **1 h 39 min** | contrib revalidé en ~10 min ; le reste dépend de ce qui est recompilé (36 min pour les crates kymedia seules, 1 h 39 min en incluant le contrib SDL2 de kynput et tout l'arbre de kyclient) |

Soit, cumulé, **~4 h** de la machine nue au bundle, en trois passes dont deux
interrompues par les blocages ci-dessus. Une quatrième passe sur volume chaud,
sans changement de source, coûterait la revalidation seule.

Référence de comparaison : le même bundle en amd64 natif coûte ~20 min sur ce
poste et ~1 h 30 en CI. L'émulation multiplie donc par **5 à 6**, ce qui
confirme l'arbitrage : un runner SaaS plafonné à 3 h n'y arriverait pas, un
poste sans plafond si.

L'image ne se reconstruit qu'au changement du Dockerfile, le bundle qu'au bump
du gitlink `vendor/kyber-desktop` — les deux sont hors de la boucle de dev courante.

### Installé sur un Pi 5 (2026-09-23)

Troisième preuve, obtenue par exécution sur le Pi 5 de test, passé de Debian 12
bookworm à Debian 13 trixie par mise à niveau en place (glibc 2.41) :

```
apt-get install ./kyberfrog_0.2.2~bc1b1a2_arm64.deb   → install ok installed
ldd sur les ELF du paquet                             → aucune bibliothèque introuvable
readelf GNU_STACK                                     → aucune pile exécutable (refusée par dlopen en glibc 2.41)
systemctl --user start kyberfrog ; curl :7700         → active, HTTP 200
```

**Bookworm n'est pas une cible** : sur ce même Pi en Debian 12, `dpkg -i`
refusait 19 dépendances, pas seulement `libc6 (>= 2.39)` — `libstdc++6 (>= 13.1)`,
`libva2 (>= 2.21.0)` et une dizaine de paquets `*t64` (renommage time64 de
Trixie). Le prendre en charge voudrait dire reconstruire tout le bundle sur une
base bookworm, pas retoucher un seuil.

### Ce qui reste à confirmer

* **Le démarrage sans session** : `kyberfrog.service` est un service
  *utilisateur* ; sur un Pi headless sans session ouverte il faut
  `loginctl enable-linger <user>`, pas encore documenté ni fait par le paquet.
* **La performance** : backend `drm` headless, encodeur **x264 logiciel
  uniquement** (pas de VAAPI, rkmpp non supporté par `kyavservice`) — c'est le
  go / no-go S0 du Satellite, pas une promesse de cette phase.
* **L'énumération V4L2** (#32), prérequis de l'étage capture du Satellite.
