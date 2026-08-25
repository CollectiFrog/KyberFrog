# Suivi du portage Linux

*Tableau de bord du chantier Linux amd64. Le « pourquoi » et le découpage en
phases vivent dans [`plan-linux-amd64.md`](plan-linux-amd64.md) ; ici on suit
l'état réel, fonctionnalité par fonctionnalité.*

Périmètre v1 arbitré : **le cœur d'abord** — un transmetteur écran qui diffuse,
un viewer qui affiche, empaquetés en `.deb`. Le reste est listé pour être porté
ensuite, pas abandonné.

Légende : ✅ fait · 🟡 partiel / à valider · ⬜ à faire · ➖ sans objet sur Linux

> **Point d'arrêt du 2026-08-21 (soir)** : **P0→P6 bouclées.** P0→P3 validés de
> bout en bout sur une VM Debian 13 / Xfce / lightdm réelle (voir « Validation
> P3 » plus bas) ; P4/P5 écrits — jobs `deb` et `release-deb`, chaîne Linux
> automatique et CI remise à plat (une seule surface, plus aucun job manuel) ;
> P6 = doc utilisateur, doc dev, CHANGELOG, README, TODO. Le script
> d'empaquetage a été rejoué dans la vraie image CI (69 dépendances calculées,
> `lintian` propre), mais **aucun pipeline GitLab n'a encore joué cette
> configuration** : la MR `feat/linux-support` → `dev` est le test grandeur
> nature. Rien n'est cassé, aucune manip en attente côté VM.

---

## Phases (voir le plan)

| Phase | État | Note |
|---|---|---|
| **P0** — compilation Linux du fork | ✅ | **Prouvé deux fois** le 2026-08-18 : bundle de 66 Mo produit en CI *et* en local, `bin/{kycontroller,kyavserver,kyclient}` présents. Le cherry-pick `camera_device` cfg(linux) fonctionne. |
| **P1** — image Docker `debian-linux` | ✅ | Image construite et poussée par la CI (Kaniko), et utilisable en local via `build-fork-local.sh`. |
| **P2** — app native Linux | ✅ | **Validé sur VM Debian 13 / Xfce le 2026-08-18** : capture `xcb` fonctionnelle, remote control (souris + clics + clavier) fonctionnel, mDNS fonctionnel. |
| **P3** — paquet `.deb` | ✅ | Cycle complet validé sur VM : install → upgrade → purge, service systemd user (autostart réel, pas un lancement à la main), `lintian` propre (hors findings acceptés, voir plus bas). |
| **P4** — jobs CI amd64 | ✅ | Job `deb` écrit (pendant Linux de `installer`, avec garde-fou `lintian`), `build-fork-linux` et `image-debian-linux` sortis du mode manuel. Répété en local dans l'image CI ; **pas encore joué par un vrai pipeline**. |
| **P5** — release sur tag | ✅ | Job `release-deb` : publie le `.deb` et l'attache à la release *déjà créée*, donc sans jamais retenir la release Windows. **Pas encore joué par un vrai tag.** |
| **P6** — doc & clôture | ✅ | Section Linux de `docs/user/installation.md`, section build Linux de `docs/dev/building.md`, `docs/dev/releasing.md` réécrit (pipeline + versioning corrigé), `CHANGELOG.md`, `README.md`, section 🐧 de `TODO.md`. Les branches de juin étaient déjà supprimées (§ 6 du plan). |

---

## Cœur v1 — ce qui doit marcher pour dire « KyberFrog tourne sous Linux »

| Sujet | État | Ce qu'il reste |
|---|---|---|
| Compilation de `kyberfrog` sur cible Linux | ✅ | Le job CI `test` compile déjà tout le workspace sur l'hôte Linux ; les modules Win32 tombent sur leurs stubs. |
| Compilation du fork sur cible Linux | ✅ | Bundle Linux amd64 produit, en CI **et** en local (~20 min sur le poste). |
| Capture écran (`grab_backend`) | ✅ | Clé écrite systématiquement depuis `UserConf::screen_backend`, auto-détectée par session. **`xcb` validé sur VM Debian 13 / Xfce.** Reste, cosmétique : override depuis l'UI (aujourd'hui, éditer `kyberfrog.toml`). |
| Supervision des enfants | 🟡 | Fait : `LD_LIBRARY_PATH` vers le `lib/` du bundle + `PR_SET_PDEATHSIG`. **Non validé** : reste à vérifier qu'un KyberFrog tué ne laisse aucun orphelin. |
| Chemins de config / logs | ✅ | `$XDG_CONFIG_HOME/kyberfrog` (config, setups) et `$XDG_STATE_HOME/kyberfrog` (logs, instances). |
| Viewer (`kyclient`) | 🟡 | Lancé depuis KyberFrog sur VM et affichant bien un flux distant (validé via le remote control). Reste à vérifier explicitement le plein écran et la sélection d'écran (`--display-idx`). |
| Découverte mDNS | ✅ | Validé sur VM : annonce + découverte de soi-même correctes, sans Avahi ni règle pare-feu. |
| Autostart (unité systemd user) | ✅ | **Bug trouvé et corrigé 2026-08-21** : `WantedBy=graphical-session.target` ne démarre jamais sous lightdm+Xfce — ce target a `RefuseManualStart=yes` et n'est activé que par les gestionnaires de session GNOME/KDE, pas par xfce4-session. Rebasculé sur `WantedBy=default.target`, vérifié compatible (`DISPLAY`/`XAUTHORITY` déjà présents dans l'environnement systemd user même sans ce target). **Course confirmée par la suite** (voir § dédié plus bas) — reste néanmoins ✅ pour le parcours réel (login graphique normal), la course ne se produit que via un déclencheur artificiel (SSH avant tout login). |
| Paquet `.deb` | ✅ | Dépendances calculées, libs multiarch, udev uinput, `lintian` propre, cycle install/upgrade/purge validé sur VM (2026-08-21). |
| IPC `/tmp/kyber` | ⬜ | `kyutil/libkypc/.../unix.rs:38` code en dur `/tmp/kyber`, chemin **partagé sans composante utilisateur** : le premier qui démarre le possède, et un résidu root bloque tous les utilisateurs normaux (`IPC couldn't bind address /tmp/kyber/0`). Vécu en validation. KyberFrog détecte et explique le cas depuis `preflight_ipc_dir`, mais la vraie correction est `$XDG_RUNTIME_DIR/kyber` côté fork — **et `kyutil` n'est pas un de nos forks** (submodule pointant l'upstream), donc c'est une contribution amont, pas un patch local. Lié à #25. |
| Serveur audio | ⬜ | `grab_backend_api_list` du fork renvoie toujours `[<backend>, "pulse"]` : sans serveur PulseAudio, libpulse **abort** (`Assertion 'pa_atomic_load...' failed`) et tue kyavserver. Vu en conteneur. Bloquant pour le cas boîtier headless sans audio ; à remonter côté fork. |
| Encodeur | ⬜ | `gen.rs` force `x264` quand l'opérateur n'a rien choisi. Vérifier VAAPI sur amd64 Intel/AMD, et le `scale=w=1920` en dur du chemin x264 Linux. |

---

## À porter ensuite

| Sujet | État | Ce qu'il reste |
|---|---|---|
| Caméra V4L2 | 🟡 | Le pin `camera_device` côté fork est en place (P0). Manque : l'énumération (`cameras.rs` sonde `ffmpeg -f dshow` et rend une liste vide hors Windows) et **`EnumerateDisplays` côté fork, qui sur Linux construit encore son `api_list` depuis le seul `grab_backend`, sans tenir compte d'une caméra épinglée** — Windows le fait, Linux non. |
| Fenêtre native (#21) | ⬜ | `shell/stub.rs` : hors Windows, boucle headless, le dashboard s'ouvre au navigateur. Décision à prendre : shell Tauri Linux (wry/webkit2gtk) ou navigateur assumé. |
| Icône de barre des tâches | ⬜ | `tray/stub.rs` : aucun tray hors Windows, l'app tourne headless. Décision : tray Linux (libappindicator) ou pilotage uniquement par le web + systemd. |
| Règle pare-feu mDNS | ⬜ | L'installeur NSIS ouvre UDP 5353 sur Windows. Équivalent Linux à décider (souvent rien à faire, mais à documenter). |
| Bureau à distance / inputs | ✅ | Validé sur VM 2026-08-18 (souris, clics et clavier). kynput injecte par **deux chemins distincts** : le *mouvement* du pointeur passe par X11/XCB (`host_mouse_position.rs`) et marche d'emblée ; les *clics* et le *clavier* passent par **uinput** (`host_mouse.rs`, `host_keyboard.rs`), donc par `/dev/uinput`, que Debian livre en `root:root 0600` — le pointeur bouge et rien d'autre ne se passe, sans message d'erreur. Le `.deb` livre désormais une règle udev (groupe `input`) + `modules-load.d`, et le postinst rappelle le `usermod -aG input` (chemin complet `/usr/sbin/usermod` : `/usr/sbin` n'est pas dans le PATH d'un utilisateur normal). |
| Ouverture du dossier de logs | ✅ | `web.rs` utilise déjà `xdg-open` hors Windows. |
| Spout | ➖ | Technologie Windows. `spout.rs` rend une liste vide hors Windows, et la tuile est masquée sur un serveur non-Windows via `/status.platform`. |
| « Tout envoyer » | ➖ | `all_sources` est `cfg(windows)` dans le fork : la clé serait ignorée en silence. Le bascule est masqué sur un serveur non-Windows. |
| arm64 | ⬜ | Hors périmètre — voir § 7 du plan pour ce qu'il faudra reprendre. |

---

## Où tourne quoi — local vs CI

Division du travail arbitrée le **2026-08-18**, après un premier aller-retour
CI qui a montré le coût de la boucle : ~1 h 30 par essai, cache keyé sur le SHA
entier de `kyber-desktop`, et aucun accès aux logs intermédiaires.

| | Local (poste) | CI |
|---|---|---|
| **Rôle** | boucle de dev : itérer, débugger, valider vite | produire l'artefact **officiel**, reproductible, depuis le SHA pinné |
| **Outil** | `packaging/linux/build-fork-local.sh` | jobs `image-debian-linux` + `build-fork-linux` |
| **Déclenchement** | à la demande | même surface que la chaîne Windows : MR, `dev`, branche par défaut, tag |

### En local

```bash
# Première fois : construire l'image, puis tout builder (~1h30 à froid)
packaging/linux/build-fork-local.sh -b

# Boucle de dev : juste vérifier que ça compile (quelques minutes)
packaging/linux/build-fork-local.sh -c

# Repartir propre après un rebase de la chaîne de forks
packaging/linux/build-fork-local.sh -f
```

Le build tourne dans un **volume docker**, pas dans le bind mount : sur Windows
l'I/O d'un bind mount est catastrophique pour un arbre contrib de cette taille.
Les sources y sont copiées une fois (artefacts du checkout Windows exclus), puis
le cache cargo/contrib persiste d'un run à l'autre.

### En CI — la chaîne Linux depuis P4/P5

Trois jobs, et une bascule : la chaîne Linux n'est plus une annexe manuelle,
elle partage la surface de la chaîne Windows (`.chain-rules` : MR, branche par
défaut, tag).

| Job | Étape | Ce qu'il fait |
|---|---|---|
| `build-fork-linux` | build | cache d'abord (Generic Package Registry, clé = SHA `kyber-desktop` de `versions.sh`), build depuis les sources sinon. Log complet en artefact **même en échec**, battement d'une ligne par minute pour rester sous la limite de trace de 4 Mio. |
| `deb` | package | pendant Linux de `installer` : `cargo build` + `build-deb.sh`, en consommant `ui/dist` de `build-ui` et le bundle de `build-fork-linux`. Termine par `lintian --fail-on error`. |
| `release-deb` | release | sur un tag `v*` : publie le `.deb` dans le Generic Package Registry et l'attache à la release existante. |

Trois décisions à connaître :

- **Une seule surface, déclarée dans `workflow:rules`** plutôt que job par job :
  MR, `dev`, branche par défaut, tags. Conséquences voulues — rien ne tourne sur
  une branche de travail tant qu'elle n'a pas de MR, il n'y a plus de pipeline
  en double quand une branche a une MR ouverte, et **plus aucun job manuel** :
  ce qui est déclenché l'est automatiquement, ce qui n'a pas lieu d'être
  n'apparaît pas.
- **Sur un tag, la chaîne Linux est `allow_failure`.** Une release Windows ne
  doit jamais être retenue par un build Linux cassé. Ailleurs (MR, `dev`,
  branche par défaut) elle est bloquante, exactement comme la chaîne Windows :
  une régression Linux doit se voir *avant* le merge.
- **`release-deb` est un job séparé, pas un lien de plus dans le bloc
  `release:`.** Ce bloc est statique : impossible d'y omettre conditionnellement
  un asset. En déclarant le `.deb` là-haut, un tag dont la chaîne Linux a échoué
  publierait une release ornée d'un lien mort. Le job séparé, lui, ne s'exécute
  qu'avec un paquet en main.

`image-debian-linux` ne se déclenche que si son Dockerfile change, et jamais sur
un tag — un pipeline de tag n'a pas de base de comparaison, `changes` y vaut
toujours vrai et l'image serait reconstruite à chaque release. À savoir :
`latest-amd64` est un tag flottant partagé, donc une MR qui touche le Dockerfile
le republie pour tout le monde — c'est précisément ce qui permet à cette MR de
tester réellement sa nouvelle image ; si elle est abandonnée, relancer le job
depuis `dev` remet le tag en place. `build-fork-linux` l'attend en
`needs: optional`, donc il build sur l'image qui vient d'être poussée quand il y
en a une, et démarre aussitôt sinon.

### Ce que P4/P5 n'ont pas encore prouvé

Le job `deb` a été **répété en local dans l'image CI exacte**
(`registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-amd64`) : 69
dépendances calculées, `.deb` de 46 Mo, `lintian --fail-on error` vert, et le
chemin d'échec vérifié aussi. Restent deux inconnues, à lever au premier vrai
pipeline :

1. **Le pipeline complet n'a jamais tourné dans cette forme** — enchaînement des
   `needs`, artefacts qui se croisent, cache du bundle. Le premier passage sera
   la MR `feat/linux-support` → `dev`.
2. **L'API Release Links avec un jeton de job** : `release-cli` s'authentifie
   ainsi pour *créer* une release, mais l'ajout d'un lien d'asset via
   `POST /releases/:tag/assets/links` n'a pas été vérifié avec `JOB-TOKEN`. D'où
   `allow_failure: true` sur `release-deb`, et l'URL du paquet imprimée dans la
   trace : au pire, le `.deb` est publié et téléchargeable, seul le lien manque
   sur la page de release.

### Un piège désamorcé au passage : les dépendances calculées

En répétant le job `deb` dans une image de build **périmée** (copie locale de
`latest-amd64` datant de juin, sans les bibliothèques système qu'apporte
`apt build-dep vlc`), `dpkg-shlibdeps` a échoué sur *chaque* SONAME — et
`build-deb.sh` est retombé en silence sur `Depends: libc6`, stderr avalé par un
`2>/dev/null`. Le paquet se construisait, s'installait, et
kyavserver/kyclient seraient morts sur une `.so` manquante : très exactement le
bug que le calcul des dépendances existe pour supprimer, ressuscité sous une
forme muette.

Le repli a été supprimé. `build-deb.sh` échoue maintenant bruyamment, affiche
les premières erreurs de `dpkg-shlibdeps` et nomme la cause la plus probable. Un
paquet aux dépendances inventées ne peut plus sortir de la chaîne.

## Validation P3 — cycle complet sur VM (2026-08-21)

Debian 13 / Xfce / lightdm. Ordre : install → autostart réel → upgrade →
purge → `lintian`.

### Autostart : le vrai bug de la journée

`WantedBy=graphical-session.target` (le design initial) **ne démarre jamais**
sous lightdm+Xfce. Ce n'est pas une manip ratée : ce target a
`RefuseManualStart=yes` et n'est activé nativement que par les gestionnaires de
session GNOME/KDE — lightdm et xfce4-session ne l'activent pas. Le service
restait `enabled` mais `inactive (dead)` en permanence, sans aucune erreur nulle
part.

Rebasculé sur `WantedBy=default.target`, atteint quel que soit le bureau. Vérifié
sain : `DISPLAY`/`XAUTHORITY` sont bien présents dans l'environnement systemd
user d'une vraie session graphique, même sans que `graphical-session.target`
s'active jamais.

**Course confirmée, mais par un déclencheur artificiel.** En testant via SSH
(qui démarre l'instance systemd user de l'utilisateur exactement comme un login
le ferait), une config neuve créée *avant* tout login graphique a figé
`screen_backend = "drm"` au lieu de `"xcb"` — parce que `detect()` ne lit que
l'environnement du moment, et ce moment n'avait pas encore de `DISPLAY`. Dans le
parcours réel (démarrage physique → login lightdm → Xfce), l'import de
`DISPLAY` se fait tôt dans la séquence X11, avant que les unités
`default.target` ne soient dispatchées — non reproduit dans ce cas. Reste une
limite de conception à connaître : un provisionnement qui SSH dans la machine
avant le premier login graphique tomberait dedans. Correctif de fond (probe
d'un serveur X actif plutôt que la confiance aux variables d'env) : hors
périmètre de ce soir, à reprendre si le cas se présente en usage réel.

### `lintian` — 411 findings mécaniques, réduits à ceux qui sont voulus

Le premier passage réel (jamais fait avant faute de VM) a trouvé un tas de
permissions héritées du pipeline Windows/git/docker plutôt que du contenu :

- **392×** `shared-library-is-executable` — tous les `.so` du bundle en 0755
- **10×** `executable-not-elf-or-script` + **2×** `non-standard-executable-perm`
  (jusqu'à 0777) — assets SVG, `.pem`, `.toml`, nos fichiers udev/systemd,
  fichiers de doc
- **1×** `unstripped-binary-or-object` — notre propre binaire `kyberfrog`
  (les binaires du fork sont déjà strippés par leur propre build)
- **1×** `no-changelog` → corrigé, puis **1×** `wrong-name-for-changelog-of-native-package`
  (mauvais nom de fichier pour un paquet sans révision `-N` séparée)

Tout corrigé dans `build-deb.sh` par une passe de normalisation des permissions
et un `changelog.gz` minimal. **Zéro développement de fonctionnalité** — que du
nettoyage de pipeline de build. Depuis P4, le job CI `deb` rejoue
`lintian --fail-on error` sur chaque paquet : ce nettoyage ne peut plus se
défaire sans que le pipeline le dise.

Ce qui **reste**, volontairement :

| Finding | Pourquoi on le garde |
|---|---|
| `embedded-library` (28×, ffmpeg/libav/zlib) | Par conception — le bundle embarque son propre build patché, comme côté Windows. Le supprimer démonterait le modèle « un seul paquet, rien à installer à côté ». |
| `maintainer-script-calls-systemctl`, `maintscript-calls-ldconfig` | Pratique standard pour un `.deb` hors archive Debian officielle. |
| `command-with-path-in-maintainer-script` (`/usr/sbin/usermod`) | Délibéré — chemin complet dans le message d'aide précisément pour éviter le `command not found` rencontré en validation (`/usr/sbin` hors PATH utilisateur). |
| `no-manual-page` | Cosmétique pour une appli desktop/daemon, non prioritaire. |
| **`privacy-breach-generic`** (Google Fonts dans `ui/dist/index.html`) | **Pas un problème Linux** — le même `index.html` sert sous Windows. Le dashboard appelle `fonts.googleapis.com`/`fonts.gstatic.com` à chaque ouverture, ce qui contredit l'esprit « self-hosted, LAN-only » du projet (scénario plausible : LAN de salle sans accès internet). À corriger côté `ui/` (auto-héberger les polices ou les droper), hors périmètre de ce chantier Linux — signalé ici parce que trouvé ici. |

## Cible d'exécution — mesurée, pas supposée

Relevé le 2026-08-18 sur le bundle réellement produit (`objdump -T` des binaires) :

| | Exigence |
|---|---|
| `kycontroller`, `kyclient` | `GLIBC_2.39` |
| `kyavserver` | `GLIBC_2.34` |
| `.so` embarquées | `GLIBC_2.38` |

⇒ **plancher : glibc ≥ 2.39**, hérité de l'image de build `debian:trixie-slim`
(glibc 2.41). glibc n'est compatible que vers l'avant.

| Distro | glibc | |
|---|---|---|
| Debian 13 Trixie | 2.41 | ✅ identique à l'image de build |
| Ubuntu 24.04 LTS | 2.39 | ✅ pile au minimum |
| Debian 12 Bookworm | 2.36 | ❌ |
| Ubuntu 22.04 | 2.35 | ❌ |

**Le README de kyber-desktop annonce « Tested: Debian Bookworm » — c'est faux
depuis notre image Trixie.** À arbitrer en P3 : soit on assume le plancher 2.39
(et le `.deb` déclare `libc6 (>= 2.39)`), soit on rebase l'image de build sur
Bookworm pour élargir la compatibilité — au prix du toolchain (meson ≥ 1.10 et
consorts sont plus pénibles à obtenir sur Bookworm).

**Session : X11.** Le backend `wlroots` du fork parle le protocole *wlroots
screencopy*, que GNOME et KDE n'implémentent pas (ils passent par
xdg-desktop-portal/PipeWire) — or GNOME/Wayland est le défaut sur Debian comme
sur Ubuntu. La VM de validation tourne donc en **Xfce/X11**, backend `xcb`
(capture SHM, pas d'accès KMS requis contrairement à `drm`).

## Décisions déjà prises

- **amd64 seul** pour cette itération.
- **Cœur d'abord** : écran + viewer ; la caméra suit.
- Le front filtre ses tuiles de source selon la plateforme exposée par
  `/status` — pas de tuile morte sur Linux.
- `packaging/linux/` a été importé verbatim des branches de juin avant leur
  suppression : c'est un point de départ, pas un livrable validé.
