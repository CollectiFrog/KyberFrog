# Suivi du portage Linux

*Tableau de bord du chantier Linux amd64. Le « pourquoi » et le découpage en
phases vivent dans [`plan-linux-amd64.md`](plan-linux-amd64.md) ; ici on suit
l'état réel, fonctionnalité par fonctionnalité.*

Périmètre v1 arbitré : **le cœur d'abord** — un transmetteur écran qui diffuse,
un viewer qui affiche, empaquetés en `.deb`. Le reste est listé pour être porté
ensuite, pas abandonné.

Légende : ✅ fait · 🟡 partiel / à valider · ⬜ à faire · ➖ sans objet sur Linux

---

## Phases (voir le plan)

| Phase | État | Note |
|---|---|---|
| **P0** — compilation Linux du fork | ✅ | **Prouvé deux fois** le 2026-08-18 : bundle de 66 Mo produit en CI *et* en local, `bin/{kycontroller,kyavserver,kyclient}` présents. Le cherry-pick `camera_device` cfg(linux) fonctionne. |
| **P1** — image Docker `debian-linux` | ✅ | Image construite et poussée par la CI (Kaniko), et utilisable en local via `build-fork-local.sh`. |
| **P2** — app native Linux | 🟡 | Code écrit et compilé pour Linux (backend, cycle de vie, XDG, UI filtrée). **Reste l'E2E sur VM.** |
| **P3** — paquet `.deb` | 🟡 | `packaging/linux/` importé verbatim, jamais exécuté. |
| **P4** — jobs CI amd64 | ⬜ | 2 jobs à écrire. |
| **P5** — release sur tag | ⬜ | Asset `.deb`, sans coupler la release Windows. |
| **P6** — doc & clôture | ⬜ | |

---

## Cœur v1 — ce qui doit marcher pour dire « KyberFrog tourne sous Linux »

| Sujet | État | Ce qu'il reste |
|---|---|---|
| Compilation de `kyberfrog` sur cible Linux | ✅ | Le job CI `test` compile déjà tout le workspace sur l'hôte Linux ; les modules Win32 tombent sur leurs stubs. |
| Compilation du fork sur cible Linux | ✅ | Bundle Linux amd64 produit, en CI **et** en local (~20 min sur le poste). |
| Capture écran (`grab_backend`) | 🟡 | Fait : clé écrite **systématiquement** depuis `UserConf::screen_backend`, auto-détectée par session (`XDG_SESSION_TYPE`, sinon `WAYLAND_DISPLAY`/`DISPLAY`, sinon drm). **Non validé sur hardware.** Reste : override depuis l'UI (aujourd'hui, éditer `kyberfrog.toml`). |
| Supervision des enfants | 🟡 | Fait : `LD_LIBRARY_PATH` vers le `lib/` du bundle + `PR_SET_PDEATHSIG`. **Non validé** : reste à vérifier qu'un KyberFrog tué ne laisse aucun orphelin. |
| Chemins de config / logs | ✅ | `$XDG_CONFIG_HOME/kyberfrog` (config, setups) et `$XDG_STATE_HOME/kyberfrog` (logs, instances). |
| Viewer (`kyclient`) | ⬜ | Jamais lancé depuis KyberFrog sous Linux. Vérifier le plein écran et la sélection d'écran (`--display-idx`). |
| Découverte mDNS | 🟡 | `mdns-sd` est pur Rust, aucune dépendance Avahi — devrait marcher tel quel. À vérifier, et voir la question pare-feu ci-dessous. |
| Paquet `.deb` | 🟡 | Dépendances réelles (`dpkg-shlibdeps`), `lintian`, cycle install → upgrade → purge, service systemd **user**. |
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
| Bureau à distance / inputs | ⬜ | `kyclient --inputs --keyboard-grab` passe par kynput ; jamais évalué sous Linux. |
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
| **Déclenchement** | à la demande | **manuel** partout, `allow_failure` partout |

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

### En CI

Les deux jobs sont **manuels** et `allow_failure` : ils ne partent jamais seuls
et ne peuvent pas bloquer la chaîne Windows.

- **`image-debian-linux`** ne s'affiche que si `ops/docker-images/debian-linux/`
  a changé — sinon il polluait le pipeline de toute MR, y compris celles qui ne
  touchent que l'UI React.
- **`build-fork-linux`** : cache d'abord (Generic Package Registry, clé = SHA
  `kyber-desktop` de `versions.sh`), build depuis les sources sinon. Le log
  complet est un artefact conservé **même en échec**, avec un battement d'une
  ligne par minute pour ne pas exploser la limite de trace de 4 Mio.

**À basculer en automatique quand le job `deb` (P3) existera** et consommera le
bundle : la logique devient alors celle de `build-fork`/`installer` côté
Windows. Tant que personne ne consomme l'artefact, l'automatiser ne fait que
brûler des minutes.

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
