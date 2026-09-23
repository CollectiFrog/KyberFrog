# Chaîne de forks — cartographie et divergence (#25)

*Ce que le fork de Kyber contient, où, et ce qui peut remonter en amont. La
manière dont la chaîne est maintenue est décrite dans
[plans-fork-restructure.md](plans-fork-restructure.md).*

## En bref

- La chaîne compte **13 repos** sur 4 niveaux de submodules, mais seuls
  **7 sont forkés** ; les 6 autres sont des pins upstream purs : `vlc`, `winit`,
  `kymux`, `kyutil`, `libvlcjni`, externals kynput.
- La divergence réelle est d'**environ 45 commits de code**. Le reste des
  commits fork est de la plomberie : bumps de pointeurs de submodules et URLs de
  `.gitmodules`.
- Une **douzaine de commits sont des bugfixes purs**, candidats directs à une
  contribution upstream.
- **Il n'y a aucune divergence FFmpeg ni VLC (C)** : les patches de
  `kymedia/contrib/ffmpeg/` sont upstream, et la « divergence vlc » se résume aux
  bindings `vlc-rs`.
- KyberFrog ne **linke aucun crate du fork** : la dépendance est exclusivement
  *build-time* (un bundle de 3 binaires + bibliothèques).

## 1. Cartographie

```
kyber-desktop  (kyber-frog) ──────────────── FORK   5 commits de code
├── external/winit            (kyber.stream) upstream pur
└── kysdk          (kyber-frog) ──────────── FORK   plomberie seule
    ├── kyctl          (kyber-frog) ──────── FORK   10 commits de code
    ├── kymedia        (kyber-frog) ──────── FORK    9 commits de code
    │   ├── subprojects/txproto (kyber-frog)  FORK   11 commits de code
    │   ├── subprojects/vlc-rs  (kyber-frog)  FORK    3 commits de code
    │   ├── subprojects/vlc     (kyber.stream) upstream pur
    │   └── subprojects/libvlcjni (kyber.stream) upstream pur
    ├── kynput         (kyber-frog) ──────── FORK    1 commit de code
    │   └── external/{keycode,libudev-sys,
    │        rust-sdl2,vigem-client}         upstream purs (tags exacts)
    ├── kymux          (kyber.stream) ────── upstream pur
    └── kyutil         (kyber.stream) ────── upstream pur
```

| Repo | Rôle | Produit |
|---|---|---|
| `kyber-desktop` | Umbrella : app `kyclient` (fenêtre winit, CLI) + `build-win32.sh` / `build-linux.sh` | `kyclient` |
| `kysdk` | Méta-repo : `.cargo/config.toml` `[patch.crates-io]` qui mappe les crates inter-workspaces vers les checkouts locaux, + les pins de submodules. Zéro code. | — |
| `kyctl` | Client/contrôleur : `kycontroller`, `kyclient-rs`/`-sys`, `kyvlcplayer`, `kyspout`, `kyservice`, `kyc` | `kycontroller`, `kyclient.dll` |
| `kymedia` | Serveur AV : `kyavserver`, `kyavservice`, `txproto-rs`, `kyaudioreg` + `contrib/` (build natif FFmpeg/VLC/libplacebo) | `kyavserver`, bibliothèques FFmpeg/VLC |
| `txproto` | C — capture et encodage (DXGI, Spout, lavd/DirectShow) | `libtxproto` |
| `kynput` | Input remote desktop | `kynput.dll`, `kynputserver` |
| `vlc-rs` | Bindings Rust libVLC (smem et output callbacks D3D11) | linké dans kyclient |
| `kymux` / `kyutil` | Transport QUIC / utilitaires — upstream purs | linkés |

**Bases upstream** : `kyber-desktop` 0.27.1, `kysdk` 0.27.1, `kyctl` 0.27.0
(gitlink pinné par kysdk@0.27.1), `kymedia` 0.27.1, `txproto` kyber-0.27.1,
`kynput` 0.27.0, `vlc-rs` kyber-250902. Branche fork : **`kyberfrog-dev`** dans
les 7 repos.

## 2. Divergence — inventaire des commits de code

« Bugfix pur » = correction d'un défaut upstream, sans opinion KyberFrog →
**candidat upstream direct**. SHAs sur `kyberfrog-dev`.

### Bugfixes purs

| Sujet | Repo | Commits | Nature |
|---|---|---|---|
| lavd / DirectShow | `txproto` | `367d24c` src_lavd jamais enregistré, `cceb33f` + `f787ab0` init COM (STA), `7a30818` identifiant de pin, `7c33227` IOType jamais taggé, `bfe3c30` noms lisibles, `d4d5889` chaîne d'ouverture par format, `d93bcec` ordre du typedef, `12f1e1f` périphériques non vidéo exclus | l'iosys lavd upstream ne fonctionne pas sur hardware sans eux |
| Souris remote desktop | `kynput` | `e4e62d9` scales X/Y séparés | bugfix pur |
| Souris remote desktop | `kyber-desktop` | `7c5a46f` accumulateur fractionnaire des deltas | bugfix pur |
| Robustesse | `kyber-desktop` | `cc6966f` sources énumérées en 0×0 | bugfix pur |

### Features génériques

Utiles à tout utilisateur de Kyber, donc proposables upstream.

| Feature | Commits | Notes |
|---|---|---|
| `--fullscreen` kyclient | `kyber-desktop` `4e80430` | trivial |
| Capture Spout | `txproto` `21655b5`, `7c45a17` · `kymedia` `d49d94b` (pin `spout_sender`), `41ff25e` + `ab41934` (scoping par transmetteur), `6ad0779` | backend iosys complet — gros morceau, mais Spout est un standard VJ |
| Webcam (`camera_device`) | `kymedia` `05c3d04` (Windows), `9e12149`, `04026e0` (`all_sources`), `7f0360f` (Linux, V4L2) | s'appuie sur la série lavd |
| Sortie Spout d'un viewer | `kyctl` `7e2c0f3`, `e64525b`, `8994fd2`, `a1e88fe`, `44dff1e`, `13cd79f`, `5d769a0`, `b124295`, `b29f8f4` · `vlc-rs` `f91eb1f`, `7393f95`, `bfb68fc` · `kyber-desktop` `4278064`, `b7ca4bc` | la plus spécifique à KyberFrog, mais générique. `8994fd2` et `b7ca4bc` corrigent du code **ajouté par cette feature** : ils ne remontent pas seuls |
| Shims `KYBER_CONFIG_PATH` | `kyctl` `3af8d7f` · `kymedia` `9a6d3d4` | upstream 0.27 fournit `KYBER_CONFIG` ; ces alias disparaissent avec #36 |

### Plomberie

- **Bumps de pointeurs** (`deps: bump…`, `chore(submodules): bump…`) dans
  `kyber-desktop`, `kysdk` et `kymedia` : un changement en feuille de chaîne
  exige un bump à chaque niveau parent.
- **`build(submodules)`** durables — `kyber-desktop` `4bd789d`, `kysdk`
  `c2ee013`, `kymedia` `dada818`, `kynput` `c41829a` : forks en URL kyber-frog,
  repos non forkés en URL kyber.stream absolue. **Règle : un changement de
  `.gitmodules` ne va jamais dans un commit `deps…bump`**, que `rebase-fork.sh`
  écarte au replay.
- `kyctl` `e303633` : régénération de `Cargo.lock`.

## 3. Ce que KyberFrog consomme

**Aucun lien Cargo.** La dépendance est un bundle produit par
`build-win32.sh -p` (ou `build-linux.sh -p`) :

- spawnés par KyberFrog : `kycontroller` (1 par transmetteur), `kyclient`
  (1 par viewer) ;
- spawnés par kycontroller : `kyavserver` (+ `kynputserver` pour le remote
  desktop) ;
- bibliothèques embarquées : `kyclient`, `kynput`, `libtxproto`, FFmpeg,
  `libvlc`/`libvlccore` + `plugins/`, SDL2, `kyaudioreg`, runtime MinGW sous
  Windows ;
- non utilisés : `kyservice` et ses scripts de service Windows (KyberFrog
  supervise lui-même), `txproto`/`ffmpeg` (outils de debug), certs de test.

La CI (`build-fork`, `build-fork-linux`) clone `kyber-desktop` au SHA du gitlink
`vendor/kyber-desktop` (lu par `packaging/versions.sh`) et met le bundle en cache par SHA
dans le Generic Package Registry.

## 4. Remontée amont (#25)

Ordre de facilité :

1. **Bugfixes purs** : la série lavd, les scales X/Y, les deltas fractionnaires,
   les sources 0×0. Chacun tient dans une MR courte.
2. **Petites features génériques** : `--fullscreen`.
3. **Grosses features** : capture Spout et scoping, webcam, sortie Spout — une
   fois le contact établi par 1 et 2.

Chaque MR acceptée retire définitivement du poids au prochain rebase. Les
questions à trancher avant d'ouvrir les MRs sont listées dans
[plans-fork-restructure.md](plans-fork-restructure.md#remontee-amont-25).
