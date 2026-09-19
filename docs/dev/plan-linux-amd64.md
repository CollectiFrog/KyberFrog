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

## arm64 — ce qu'il faudra

1. **Un runner** : le tier gratuit GitLab n'offre que `saas-linux-small-arm64`
   (2 vCPU / 8 Go), trop petit pour le build fork. Options : runner self-hosted
   sur le hardware ARM, tier Premium, ou bundle arm64 construit hors CI et poussé
   une fois dans le Generic Package Registry (le cache par SHA rend cette option
   quasi gratuite).
2. **`ARCH_TRIPLET` dérivé de `uname -m`** dans `build-linux.sh`. Les commits
   existent et se restaurent par SHA : `kyber-desktop` `a325609`, `kyctl`
   `204d173`, `kymedia` `e2d58e2` (à cherry-picker sur la base du moment).
3. **Une image `debian-linux` arm64** (l'image accepte un second tag).
4. **Une validation hardware** (Pi 4 / RK3588) : backend `drm` headless, encodeur
   **x264 logiciel uniquement** (pas de VAAPI, rkmpp non supporté par
   `kyavservice`) — fixer une cible de perf avant toute promesse.

Les jobs CI sont écrits pour qu'arm64 soit une extension de matrice.
