# Audit #24/#25 — chaîne de forks & divergence

*Audit réalisé le 2026-07-07 (état : post-release 0.4.0). Alimente les
chantiers [#24 et #25 d'IMPROVEMENTS.md](../../IMPROVEMENTS.md).*

## TL;DR

- La chaîne compte **13 repos** (4 niveaux de submodules), mais seuls
  **7 sont réellement forkés** ; les 6 autres sont des pins upstream purs
  (zéro divergence) : `vlc`, `winit`, `kymux`, `kyutil`, `libvlcjni`,
  externals kynput.
- La divergence réelle est **~30 commits de code** (le reste — ~60 % des
  commits fork — n'est que des *bumps de pointers de submodules*, du bruit
  mécanique créé par l'imbrication elle-même).
- Environ **la moitié des commits de code sont des bugfixes purs**
  (jamais testés hardware côté upstream) : candidats idéaux à une
  contribution upstream (#25).
- KyberFrog ne **linke aucun crate du fork** : la dépendance est
  exclusivement *build-time* (produire le bundle de 3 binaires + DLLs).
  Tout plan d'archi peut donc se concentrer sur la chaîne de build, sans
  toucher au code de kyberfrog lui-même.

## 1. Cartographie de la chaîne

```
kyber-desktop  (kyber-frog) ──────────────── FORK   22 commits (5 code)
├── external/winit            (kyber.stream) upstream pur (tag kyber-v0.28.7-3)
└── kysdk          (kyber-frog) ──────────── FORK   15 commits (0 code, plomberie)
    ├── kyctl          (kyber-frog) ──────── FORK    8 commits (5 code)
    ├── kymedia        (kyber-frog) ──────── FORK   23 commits (7 code)
    │   ├── external/txproto   (kyber-frog)─ FORK   20 commits (10 code)
    │   ├── external/vlc-rs    (kyber-frog)─ FORK    2 commits (2 code)
    │   ├── external/vlc       (kyber.stream) upstream pur (pin sur kymux.66)
    │   └── external/libvlcjni (kyber.stream) upstream pur (tag kyber-260306)
    ├── kynput         (kyber-frog) ──────── FORK    2 commits (1 code)
    │   └── external/{keycode,libudev-sys,
    │        rust-sdl2,vigem-client}         upstream purs (tags exacts)
    ├── kymux          (kyber.stream) ────── upstream pur (tag 0.16.0)
    └── kyutil         (kyber.stream) ────── upstream pur (tag 0.11.0)
```

Rôle de chaque niveau :

| Repo | Rôle | Produit |
|---|---|---|
| `kyber-desktop` | Umbrella : app `kyclient` (binaire desktop) + `build-win32.sh` (orchestre tout le build natif) | `kyclient.exe` |
| `kysdk` | **Méta-repo de plomberie** : un `.cargo/config.toml` `[patch.crates-io]` qui mappe les crates inter-workspaces vers les checkouts locaux + les pins de submodules. Zéro code. | — |
| `kyctl` | Crates client/contrôleur : `kycontroller`, `kyclient-rs`/`-sys`/`-common`, `kyvlcplayer`, `kyspout`, `kyservice`, `kyc` | `kycontroller.exe`, `kyclient.dll` |
| `kymedia` | Serveur AV : `kyavserver`, `kyavservice(-types)`, `txproto-rs`, `kyaudioreg` + `contrib/` (build natif FFmpeg/VLC/libplacebo…) | `kyavserver.exe`, DLLs FFmpeg/VLC |
| `txproto` | C — capture/encodage (DXGI, Spout, lavd/DirectShow) | `libtxproto-0.dll` |
| `kynput` | Input remote-desktop (`kynput-rs`, `kynputservice-types`) | `kynput.dll`, `kynputserver.exe` |
| `vlc-rs` | Bindings Rust libVLC (smem callbacks pour la sortie Spout) | (linké dans kyclient) |
| `kymux` / `kyutil` | Transport QUIC / utilitaires — **upstream purs** | (linkés) |

Bases upstream actuelles : **0.27.1 partout** depuis le rebase du
2026-07-08 (voir §6) — `kyber-desktop` 0.27.1, `kysdk` 0.27.1,
`kyctl` 0.27.0 (gitlink pinné par kysdk@0.27.1), `kymedia` 0.27.1,
`txproto` kyber-0.27.1, `kynput` 0.27.0, `vlc-rs` kyber-250902 (inchangé).

## 2. Divergence réelle — inventaire des commits de code (#25)

Classés par fonctionnalité. « Bugfix pur » = correction d'un défaut
upstream, sans opinion kyberfrog → **candidat upstream direct**.

### Bugfixes purs (≈ 12 commits, upstream-able tels quels)

| Feature | Repo | Commits | Nature |
|---|---|---|---|
| lavd/DirectShow (#18-A) | `txproto` | `67b4437` registration manquante, `a2380a2`+`36d0f1f` COM init STA, `3627dc6` identifier mismatch, `8a60777` IOType jamais taggé, `4fa7411` noms lisibles, `0e495b3` open string par format, `d52bffc` ordre typedef | 7 bugs latents empilés — l'iosys lavd n'a jamais marché upstream sur hardware |
| Souris #17 P1 | `kynput` | `e4e62d9` scales X/Y séparés | Bugfix pur |
| Souris #17 P1 | `kyber-desktop` | `4fea8e0` accumulateur fractionnaire des deltas | Bugfix pur |
| Robustesse | `kyber-desktop` | `ac17eb4` sources énumérées en 0×0 | Bugfix pur |
| Spout windowless | `kyber-desktop` | `de339ff` 403 — vrai host display id | Bugfix (lié feature Spout out) |

### Features génériques (utiles à tout utilisateur Kyber, proposables upstream)

| Feature | Repo(s) | Commits | Notes |
|---|---|---|---|
| `KYBER_CONFIG_PATH` (N instances / 1 install) | `kyctl` `5229f`, `kymedia` `0389f6a` | 2 commits triviaux | Le plus simple à proposer |
| `--fullscreen` kyclient | `kyber-desktop` `7bf14b8` | 1 commit | Trivial |
| Source Spout (capture) #19 | `txproto` `9669617`+`758b19b` | Backend iosys complet | Gros morceau mais générique (Spout est un standard VJ) |
| Pinning + scoping sources #19 | `kymedia` `605c47b`, `3fcc121`, `536941b` | Dépend du backend Spout | |
| Webcam DirectShow #18-A | `kymedia` `1470046`, `dae97a8` | camera_device pinning + graph x264 | S'appuie sur les fixes lavd |
| Sortie Spout viewer (#8) | `kyctl` `340a79e`+`81e2818`+`53df4ad`+`e114926`, `vlc-rs` `f91eb1f`+`7393f95`, `kyber-desktop` `21ce68a` | C-API spout_out + smem callbacks + flag (le fix BGRA `53df4ad` corrige du code *ajouté par cette feature* — vérifié 2026-07-08, pas upstreamable seul) | La plus « kyberfrog-spécifique », mais reste générique |

### Bruit mécanique (créé par l'architecture actuelle, pas par les features)

- **~25 commits `deps: bump submodule pointers`** répartis sur
  `kyber-desktop` (15), `kysdk` (14), `kymedia` (10) — chaque changement
  d'une ligne dans `txproto` exige 3 commits de bump en cascade
  (`kymedia` → `kysdk` → `kyber-desktop`), d'où l'existence de
  `bump-fork.sh`.
- Fixes d'URLs de submodules (`c41829a`, `20c336c`, `27f48e8`) : les URLs
  relatives (`../kyctl.git`) se résolvent contre l'URL du parent et cassent
  quand un repo est forké dans un autre groupe ; `.gitmodules` committé ≠
  remotes locaux a cassé le pipeline v0.4.0 deux fois (`not our ref`).
- `264e059` (extraction glslang/lua), `65f633a` (Cargo.lock) : entretien du build.

À noter : 2 commits de `txproto` (`6265fc9`, `3a91b1e` — auteur upstream
Anton Khirnov) sont *hérités d'upstream* au moment du fork, pas du travail
kyberfrog. Les 4 patches FFmpeg de `kymedia/contrib/ffmpeg/` sont
également upstream : **il n'y a aujourd'hui aucune divergence FFmpeg ni
VLC (C)** — la « divergence vlc » de #25 se réduit à `vlc-rs` (bindings, 2 commits).

## 3. Ce que KyberFrog consomme réellement

**Aucun lien Cargo** : les manifests de `kyberfrog` ne référencent aucun
crate du fork. La dépendance est un **bundle de binaires** produit par
`build-win32.sh -p` et consommé tel quel :

- Spawnés par kyberfrog : `kycontroller.exe` (1/transmitter),
  `kyclient.exe` (1/viewer).
- Spawnés par kycontroller : `kyavserver.exe` (+ `kynputserver.exe` pour
  le remote desktop).
- Bibliothèques embarquées : `kyclient.dll`, `kynput.dll`,
  `libtxproto-0.dll`, DLLs FFmpeg (×7), `libvlc`/`libvlccore` + `plugins/`,
  `SDL2.dll`, `kyaudioreg.dll`, runtime MinGW.
- Non utilisés par kyberfrog : `kyservice.exe` + scripts service Windows
  (kyberfrog supervise lui-même), `txproto.exe`/`ffmpeg.exe` (outils de
  debug), certs de test.

Le CI (`build-fork`) clone `kyber-desktop@KYBER_DESKTOP_REF` (« dev »,
`packaging/versions.sh`), build ~1h30 (timeout 3h), cache le bundle par
SHA résolu dans le Generic Package Registry.

## 4. Points de douleur constatés

1. **Cascade de bumps** : 1 fix d'une ligne en feuille de chaîne = jusqu'à
   4 commits + 4 pushes coordonnés. C'est le coût dominant (~60 % des
   commits fork) et la source principale d'erreurs (pins désynchronisés).
2. **Fragilité des URLs de submodules** : URLs relatives + `.gitmodules`
   divergeant des remotes locaux = pipelines cassés difficile à
   diagnostiquer (`upload-pack: not our ref`).
3. **Layout de branches confus** : `main` (copie upstream),
   `kyberfrog-main` (base fork), `dev` (intégration, buildé par CI) — les
   branches locales `dev` trackent `origin/kyberfrog-main`, donc
   `git status` ment sur l'état de synchro.
4. **kysdk n'apporte rien** : méta-repo dont la seule fonction est une
   table `[patch.crates-io]` + des pins — mais il impose un niveau de
   bump supplémentaire au milieu de la chaîne.
5. **Rebase upstream coûteux** : 5 repos forkés à rebaser individuellement
   sur 0.27.0, en maintenant la cohérence des pins croisés.
6. **Compréhension** : pour lire « tout ce que le fork change », il faut
   ouvrir 7 repos et filtrer les bumps à la main (cet audit est
   précisément ce travail).

## 5. Pistes d'architecture

Voir [plans-fork-restructure.md](plans-fork-restructure.md) (propositions
détaillées, à arbitrer).

## 6. Addendum — rebase 0.27.1 (2026-07-08)

Cascade `rebase-fork.sh 0.27.1` exécutée ; chaque repo porte une branche
`rebase/0.27.1`. Tips (avant push) :

| Repo | Tip | Commits sur la base | Notes |
|---|---|---|---|
| kyber-desktop | `ac3d781` | 5 code + 1 `build(submodules)` + 1 bump | |
| kysdk | `4f84239` | 1 `build(submodules)` + 1 bump | plus aucun commit de code |
| kyctl | `e303633` | 5 code + 1 chore (Cargo.lock) | base = gitlink 0.27.0 pinné par kysdk@0.27.1 |
| kymedia | `77f5cd9` | 6 code + 1 `build(submodules)` + 1 bump | |
| txproto | `d93bcec` | 10 code | rejoués sans conflit sur kyber-0.27.1 |
| kynput | `c41829a` | 2 (inchangé) | base 0.27.0 = cible, rebase identité — pas de push |
| vlc-rs | `7393f95` | 2 (inchangé) | cible kyber-250902 = base — pas de push |

**Divergence absorbée par upstream 0.27** (le but du plan A, sans même
passer par des MRs) :

- `KYBER_CONFIG_PATH` (kyctl `5229fc7`, kymedia `0389f6a`) : upstream a
  maintenant `--config` > `KYBER_CONFIG` > exe-relative, et kycontroller
  ré-exporte le chemin résolu aux services spawnés. Les 2 commits fork sont
  réduits à des **shims légataires** (alias basse priorité). Prochaine
  étape : faire exporter `KYBER_CONFIG` par kyberfrog, puis dropper les
  shims au rebase suivant.
- `264e059` (fix contrib glslang/lua) : **droppé** — upstream a supprimé
  `contrib/build-win32.sh` (deps natives via subprojects meson).
- `e844f6f` (bump gitlink txproto) : **droppé** — régénéré par la cascade.
- Les 2 commits hérités d'Anton Khirnov (`6265fc9`, `3a91b1e`) sont dans
  l'historique de kyber-0.27.1 — plus dans la divergence.

**Restructurations upstream absorbées** : `kymedia/external/` →
`subprojects/` ; C-API kyclient passée aux `Option<Box<…>>` (le handle NULL
windowless est résolu en `match` dans
`kyclient_video_player_config_default`) ; `connect()` refactoré ;
`CliArgs` upstream a gagné `auto_reconnect`/`close_requested` (nos champs
`spout_out`/`spout_display_id` cohabitent) ; **meson ≥ 1.10 requis** — image
docker locale dérivée `kyber/debian-win64:local-0.27` (pip meson 1.11) tant
que l'image ops upstream n'est pas récupérable.

**Politique `.gitmodules` (leçon du rebase)** : les redirects d'URL du fork
vivaient dans des commits `deps…bump` — que `rebase-fork.sh` droppe par
design. Ils sont maintenant portés par des commits **`build(submodules)`
durables** (kyber-desktop `4bd789d`, kysdk `c2ee013`, kymedia `dada818`,
kynput `c41829a`) : forks en URL kyber-frog (absolue ou relative), repos
non forkés en URL kyber.stream absolue. Règle : ne plus jamais mettre un
changement `.gitmodules` dans un commit `deps…bump`.

**Fixes outillage au passage** : regex du drop des bumps compatible
git ≥ 2.52 (todo `pick <sha> # <sujet>`) ; le fichier d'état
`.rebase-fork.state` n'est plus compté comme saleté par le script.
