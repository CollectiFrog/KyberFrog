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
| **P2** — app native Linux | ⬜ | Le gros morceau. Détail ci-dessous. |
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
| Capture écran (`grab_backend`) | ⬜ | Écrire **systématiquement** la clé — le défaut du fork est `NvFBC`, inutilisable hors NVIDIA. Auto-détection : `WAYLAND_DISPLAY`/`XDG_SESSION_TYPE` → wlroots, `DISPLAY` → xcb, sinon drm. Override manuel dans l'UI. |
| Supervision des enfants | ⬜ | Le Job Object est Windows-only (`supervisor.rs`). Porter les deux équivalents : `LD_LIBRARY_PATH` vers le `lib/` du bundle, et `PR_SET_PDEATHSIG` pour les runs hors systemd. |
| Chemins de config / logs | ⬜ | `paths.rs` retombe sur `$HOME/.config` en dur. Respecter `XDG_CONFIG_HOME` / `XDG_STATE_HOME` (des logs sous `~/.config` sont discutables). |
| Viewer (`kyclient`) | ⬜ | Jamais lancé depuis KyberFrog sous Linux. Vérifier le plein écran et la sélection d'écran (`--display-idx`). |
| Découverte mDNS | 🟡 | `mdns-sd` est pur Rust, aucune dépendance Avahi — devrait marcher tel quel. À vérifier, et voir la question pare-feu ci-dessous. |
| Paquet `.deb` | 🟡 | Dépendances réelles (`dpkg-shlibdeps`), `lintian`, cycle install → upgrade → purge, service systemd **user**. |
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
| Spout | ➖ | Technologie Windows. `spout.rs` rend une liste vide hors Windows — correct. Masquer la tuile côté UI. |
| « Tout envoyer » | ➖ | `all_sources` est `cfg(windows)` dans le fork : la clé serait ignorée en silence. Masquer la tuile. |
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

## Décisions déjà prises

- **amd64 seul** pour cette itération.
- **Cœur d'abord** : écran + viewer ; la caméra suit.
- Le front filtre ses tuiles de source selon la plateforme exposée par
  `/status` — pas de tuile morte sur Linux.
- `packaging/linux/` a été importé verbatim des branches de juin avant leur
  suppression : c'est un point de départ, pas un livrable validé.
