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
kyber-desktop  (kyber-frog) ──────────────── FORK   6 commits de code
├── external/winit            (kyber.stream) upstream pur
└── kysdk          (kyber-frog) ──────────── FORK   plomberie seule
    ├── kyctl          (kyber-frog) ──────── FORK   11 commits de code
    ├── kymedia        (kyber-frog) ──────── FORK   12 commits de code
    │   ├── subprojects/txproto (kyber-frog)  FORK   11 commits de code
    │   ├── subprojects/vlc-rs  (kyber-frog)  FORK    3 commits de code
    │   ├── subprojects/vlc     (kyber.stream) upstream pur
    │   └── subprojects/libvlcjni (kyber.stream) upstream pur
    ├── kynput         (kyber-frog) ──────── FORK    2 commits de code
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

**Bases upstream** (rebase du 2026-09-25) : `kyber-desktop` 0.28.0, `kysdk`
0.28.0, `kyctl` 0.28.0, `kymedia` 0.28.0, `txproto` kyber-0.28.0, `kynput`
0.28.0, `vlc-rs` kyber-250902 (inchangé : kymedia 0.28 pinne toujours ce tag).
Branche fork : **`kyberfrog-dev`** dans les 7 repos.

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
- `kyctl` `0356c2f` : régénération de `Cargo.lock` (remplace `e303633`, sauté au
  rebase 0.28.0 puis régénéré contre les checkouts 0.28).
- Portage ARM (`build-linux` `ARCH_TRIPLET`, NVENC/oneVPL gatés x86, `va_list`
  aarch64 dans `txproto-rs`) : un commit dans `kyber-desktop`, `kyctl`,
  `kynput`, trois dans `kymedia`. Candidats upstream eux aussi.

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

## 5. Rebase 0.28.0 (2026-09-25)

Les SHAs des tableaux ci-dessus sont ceux **d'avant** ce rebase ; la table
ci-dessous donne leur équivalent sur `kyberfrog-dev` rebasé.

Ce qui a demandé plus qu'un replay :

- **Ligne hotfix 0.27.1 non remergée** : 0.28.0 part de 0.27.0, donc les
  commits upstream de `0.27.x-branch` (CI retargetée, « Version 0.27.1 », bumps
  txproto) tombaient dans la plage. `rebase-fork.sh` écarte désormais tout
  commit accessible depuis `upstream/*` et les liste au dry-run.
- **kyavservice restructuré** en 0.28 (boucle de service dans `txproto/run.rs`,
  `Config` dérive `Default`, backend `libavconv` choisi au runtime — défaut
  `txproto`) : les 9 commits kymedia ont été portés à la main. L'épinglage
  Spout/caméra et le scoping ne valent que pour le backend `txproto`.
- **Rust 2024** partout : `e64525b` (kyctl) porté (`#[unsafe(no_mangle)]`, blocs
  `unsafe`).
- **Submodules** : `libavconv` ajouté par upstream (URL https absolue, gardée) ;
  `kynput/external/keycode` retiré par upstream (crate crates.io).
- **`va_list`** : upstream couvre macOS aarch64, pas Linux aarch64 — notre
  commit ARM le complète.

| Repo | Avant | Après | Commit |
|---|---|---|---|
| `kyber-desktop` | `4e80430` | `2f2412d` | kyclient: Add --fullscreen startup flag |
| `kyber-desktop` | `4278064` | `a7d6132` | kyclient: Add --spout-out for windowless Spout output |
| `kyber-desktop` | `4bd789d` | `4519ad7` | build(submodules): absolute URLs for kysdk (fork) + winit (upstream) |
| `kyber-desktop` | `b7ca4bc` | `e0941d3` | kyclient: fix Spout windowless 403 — use a real host display id |
| `kyber-desktop` | `7c5a46f` | `c98f400` | fix(inputs): accumulate fractional mouse deltas before i16 cast |
| `kyber-desktop` | `cc6966f` | `ab27579` | kyclient: handle sources enumerating with unknown (0x0) dimensions |
| `kyber-desktop` | `bce3676` | `c26c453` | build-linux.sh: Derive ARCH_TRIPLET from `uname -m` (ARM-ready) |
| `kysdk` | `c2ee013` | `dcb8238` | build(submodules): fork-aware submodule URLs |
| `kyctl` | `3af8d7f` | `6808da5` | kycontroller: honor legacy KYBER_CONFIG_PATH as config override |
| `kyctl` | `7e2c0f3` | `23d5dae` | feat(spout-output): add Spout sender output from a viewer |
| `kyctl` | `e64525b` | `396c31b` | feat(spout-output): expose spout_out through the C-API and kyclient-rs |
| `kyctl` | `8994fd2` | `b908b84` | fix(spout): output BGRA chroma to fix blue tint and bogus transparency |
| `kyctl` | `a1e88fe` | `5b34d04` | feat(spout): publish the Spout output at the stream's native size |
| `kyctl` | `44dff1e` | `3218df5` | feat(kyspout): expose shared texture as a libVLC D3D11 render target |
| `kyctl` | `13cd79f` | `0ce72f2` | feat(kyvlcplayer): wire libVLC 4 D3D11 zero-copy Spout output |
| `kyctl` | `5d769a0` | `eeef266` | fix(kyspout): add D3D11_CREATE_DEVICE_VIDEO_SUPPORT for zero-copy device |
| `kyctl` | `b124295` | `00ee4ff` | fix(kyvlcplayer): describe the Spout render target, not the source, in update_output |
| `kyctl` | `b29f8f4` | `992cffb` | feat(kyvlcplayer): make D3D11 zero-copy the default Spout output path |
| `kyctl` | `facd80c` | `9cbcfdf` | build-linux.sh: Derive ARCH_TRIPLET from `uname -m` (ARM-ready) |
| `kyctl` | `e303633` | `0356c2f` | chore(deps): regenerate Cargo.lock for kyspout (native-size Spout build) |
| `kymedia` | `d49d94b` | `3e29deb` | kyavservice: Add Windows spout_sender source pinning |
| `kymedia` | `9a6d3d4` | `ed60686` | kyavservice: honor legacy KYBER_CONFIG_PATH as config override |
| `kymedia` | `41ff25e` | `c43dc39` | feat(kyavservice): scope Windows capture sources per transmitter |
| `kymedia` | `ab41934` | `c412602` | fix(kyavservice): scope Spout enumeration to the pinned sender (#19) |
| `kymedia` | `05c3d04` | `bf3f798` | kyavservice: Add Windows camera_device source pinning (DirectShow via lavd) |
| `kymedia` | `9e12149` | `91c84e8` | kyavservice: skip hwdownload for pinned camera sources (x264 graph) |
| `kymedia` | `dada818` | `4b153fe` | build(submodules): fork-aware submodule URLs on the subprojects/ layout |
| `kymedia` | `6ad0779` | `306b961` | txproto-rs: expose the SPClass type of iosys entries |
| `kymedia` | `04026e0` | `83e540e` | kyavservice: use the camera CPU graph for lavd sources in all_sources |
| `kymedia` | `7f0360f` | `ecb91d2` | kyavservice: Add Linux camera_device source pinning (V4L2 via lavd) |
| `kymedia` | `cfedffa` | `a7d8f1d` | build-linux: Derive ARCH_TRIPLET from `uname -m` (ARM-ready) |
| `kymedia` | `d0001e6` | `22b2cee` | build(linux): gate NVENC and oneVPL on x86, port ARCH_TRIPLET to the meson era |
| `kymedia` | `fd0b23e` | `bdd53d9` | fix(txproto-rs): porter le binding va_list et le buffer c_char sur aarch64 |
| `kynput` | `e4e62d9` | `ad837f9` | fix(video_layout): use separate X/Y scales in local<->host projection |
| `kynput` | `c41829a` | `27a5b03` | build(submodules): absolute URLs for external deps |
| `kynput` | `9720bd6` | `b68d656` | build-linux: Derive ARCH_TRIPLET from `uname -m` (ARM-ready) |
| `txproto` | `21655b5` | `b071bef` | iosys: Add Spout capture source (Windows) |
| `txproto` | `7c45a17` | `4427e60` | io: Treat spout as a video backend in grab_backend filtering |
| `txproto` | `367d24c` | `699d7a6` | iosys: register src_lavd in sp_compiled_apis (was compiled but never wired) |
| `txproto` | `cceb33f` | `c00a705` | iosys_lavd: initialize COM before DirectShow device enumeration (Windows) |
| `txproto` | `7a30818` | `f68e70c` | iosys_lavd: fix pin identifier mismatch + drop harmful COM wrapper |
| `txproto` | `f787ab0` | `f6706a7` | iosys_lavd: re-add COM init around update_entries, using STA not MTA |
| `txproto` | `7c33227` | `b9911af` | iosys_lavd: tag entries with their IOType (was always SP_IO_TYPE_NONE) |
| `txproto` | `bfe3c30` | `b490b1d` | iosys_lavd: class-name entries by their human-readable name |
| `txproto` | `d4d5889` | `7d0d666` | iosys_lavd: open devices by a per-format input string, not the entry name |
| `txproto` | `d93bcec` | `b9ff059` | iosys_lavd: move LavdEntryPriv typedef above its first use |
| `txproto` | `12f1e1f` | `d655487` | iosys_lavd: keep non-video devices out of the entry list |
| `vlc-rs` | `f91eb1f` | `f91eb1f` | feat(smem): add set_video_format and set_video_callbacks |
| `vlc-rs` | `7393f95` | `7393f95` | feat(smem): negotiate output format via libvlc_video_set_format_callbacks |
| `vlc-rs` | `bfb68fc` | `bfb68fc` | feat(video): wrap libVLC 4 D3D11 GPU output callbacks |
