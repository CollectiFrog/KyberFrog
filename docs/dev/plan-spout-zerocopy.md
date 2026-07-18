# Plan — Spout output zero-copy (D3D11, libVLC 4)

> **État 2026-07-18 — implémenté, en attente de build+validation.** Les 3
> couches sont codées et `cargo check` (Docker MinGW, cible windows-gnu) est
> vert, sur les branches `feat/spout-zerocopy` (off `kyberfrog-dev`) :
> vlc-rs `bfb68fc` (FFI+wrapper) · kyctl `44dff1e` (kyspout render-target) +
> `13cd79f` (câblage kyvlcplayer) · bumps de chaîne kymedia `f072fc6` →
> kysdk `62ffd75` → kyber-desktop `d2c418d`. La `libvlc.dll` du builddir
> exporte déjà `libvlc_video_set_output_callbacks` (vérifié) → **aucun rebuild
> VLC/meson requis**, seulement libkyclient + kyclient.exe.
> **Défaut = smem** ; le zero-copy s'active avec `KYSPOUT_ZEROCOPY=1`
> (env de kyclient). Restent : build léger du bundle, E2E hardware
> (Resolume/TD), bascule du défaut, merges `feat/…` → `kyberfrog-dev` + push.

> Cible : supprimer l'aller-retour CPU du chemin de sortie Spout en réception
> (#28 levier 2 / la déferral zero-copy du #8). **Travail côté fork**
> (`kyber-desktop/kysdk` : vlc-rs + kyctl), pas côté KyberFrog. Ce dépôt
> consomme le bundle tel quel.

## TL;DR

**Le blocage annoncé dans IMPROVEMENTS.md (#8, #28.2) — « nécessite libVLC 4
côté fork », présenté comme un gros chantier de migration — est PÉRIMÉ.**
Vérifié le 2026-07-18 :

- Le VLC vendoré par kymedia (`subprojects/vlc`, submodule
  `kyber.stream/deps/vlc`) est **déjà `4.0.0-dev`**, épinglé à la branche Kyber
  `kyber-260626` (26 juin 2026), et il **build/ship** déjà (le #8 tourne
  dessus). `AC_INIT([vlc], [4.0.0-dev])`.
- Son header expose déjà **toute** l'API zero-copy :
  `libvlc_video_set_output_callbacks`, `libvlc_video_engine_d3d11`, et les
  structs `libvlc_video_setup_device_info_t` / `libvlc_video_render_cfg_t` /
  `libvlc_video_output_cfg_t`
  (`subprojects/vlc/include/vlc/libvlc_media_player.h:541-935`).
- **L'ancienne API smem ET la nouvelle coexistent** dans libVLC 4
  (`libvlc_video_set_callbacks` + `set_format_callbacks` toujours présents,
  header:499/508). → la migration est **additive**, pas un rip-and-replace de
  la dépendance VLC.

Donc il n'y a **aucune migration de libVLC** à faire. Le vrai manque, borné, est
en **3 couches du fork**, aucune dans VLC lui-même :

1. **vlc-rs** — le binding Rust ne wrappe que smem ; il faut ajouter le FFI +
   un wrapper sûr pour les output callbacks D3D11.
2. **kyspout** — exposer sa texture partagée comme **render target de VLC** au
   lieu d'uploader un buffer CPU (`send_bgra`).
3. **kyvlcplayer** (`setup_spout_output`) — câbler les callbacks D3D11 à la
   place de smem.

Aucune branche fork n'a commencé ce travail (grep `spout|d3d11|zero.?copy|
output.?callback` sur kymedia/kyctl : rien ; `feat/spout-source` = pinning
source, sans rapport).

## État des lieux (vérifié dans le code)

### Le défaut actuel — GPU → CPU → GPU

- **Réception, smem CPU :**
  `kyvlcplayer/src/player.rs:208-257` (`setup_spout_output`) installe des
  callbacks `set_video_format_callbacks` demandant du **BGRA CPU** :
  `fmt.chroma = *b"BGRA"`, libVLC télécharge la frame décodée dans un
  `Vec<u8>` (`SpoutCtx.buffer`).
- **Ré-upload GPU :**
  `kyspout/src/lib.rs:244-288` (`send_bgra`) recopie ce buffer CPU dans la
  texture D3D11 partagée via `UpdateSubresource` + `Flush`.

Soit, par frame : décodage HW (**GPU**) → download (**CPU**) → upload
(**GPU**). Le coût est **dans kyclient**, pas dans Spout (Spout = partage
GPU→GPU, négligeable). Le commentaire d'origine l'anticipe déjà
(`kyspout/src/lib.rs:38-39` : *« A later optimization can take a GPU texture
directly (zero-copy) »*).

### Ce que le binding expose (et pas)

- `vlc-rs/src/sys.rs` est un **FFI écrit à la main** (pas bindgen) : il déclare
  `libvlc_video_set_callbacks` (sys.rs:412) et `libvlc_video_set_format_callbacks`
  (sys.rs:419) — **rien** sur `libvlc_video_set_output_callbacks` ni les structs
  D3D11.
- Côté sûr, `media_player.rs:150-236` expose `set_video_callbacks` /
  `set_video_format_callbacks` (le pattern à mimer : `Box<dyn Fn>` dans un
  `VideoCallbacksData`, `Box::into_raw`, trampolines `extern "C"`).

## Cible — modèle « output callbacks D3D11 » de libVLC 4

Signature (header:920-935) :

```c
bool libvlc_video_set_output_callbacks(mp, engine,
    setup_cb, cleanup_cb, window_cb,
    update_output_cb, swap_cb, makeCurrent_cb, getProcAddress_cb,
    metadata_cb, select_plane_cb, opaque);
```

Flux pour notre relais headless (engine = `libvlc_video_engine_d3d11`) :

| Callback | Ce que kyspout/kyvlcplayer fait |
|---|---|
| **setup** (`header:585`) | Renvoie à VLC un `ID3D11DeviceContext*` (`out->d3d11.device_context`) **+** un `context_mutex` (`out->d3d11.context_mutex`). |
| **update_output** (`header:669`) | VLC donne `width/height` (`render_cfg`). kyspout (re)crée sa **texture partagée BGRA** à cette taille + une **RTV** dessus, et remplit `output->dxgi_format = DXGI_FORMAT_B8G8R8A8_UNORM` (87). |
| **select_plane** (`header:892`) | Renvoie la RTV de la texture partagée (`*output = rtv`). Une seule plane RGBA. |
| **makeCurrent(enter)** | `enter=true` : prendre le `SpoutAccessMutex`. `enter=false` : le relâcher. Protège le receiver contre une frame à moitié rendue. |
| **swap** | La frame est **déjà** dans la texture partagée (zéro copie) → bump du `_Count_Semaphore` Spout. |
| window / getProcAddress / metadata | `None` (getProcAddress = GL only ; window/metadata inutiles pour un relais). |

Structs à FFI-er (header:541-650) :

```c
libvlc_video_setup_device_cfg_t  { bool hardware_decoding; }
libvlc_video_setup_device_info_t { union { struct { void* device_context;
                                                     void* context_mutex; } d3d11; ... } }
libvlc_video_render_cfg_t        { unsigned width, height, bitdepth; bool full_range;
                                   colorspace; primaries; transfer; void* device; }
libvlc_video_output_cfg_t        { union { int dxgi_format; ... }; bool full_range;
                                   colorspace; primaries; transfer; orientation; }
```

## Travail par couche

### 1. vlc-rs — FFI + wrapper sûr

- **`src/sys.rs`** : déclarer les 3 structs (`#[repr(C)]`, unions incluses),
  les typedefs de callbacks, et `libvlc_video_set_output_callbacks`. Miroir des
  déclarations smem existantes.
- **`src/media_player.rs`** : `set_video_output_callbacks_d3d11(setup, cleanup,
  update_output, swap, make_current, select_plane)` prenant des `Box<dyn Fn>`
  Rust, stockés dans un `OutputCallbacksData` (comme `VideoCallbacksData`),
  `Box::into_raw`, trampolines `extern "C"` qui reconstruisent l'opaque. Passer
  `None` pour window/getProcAddress/metadata.
- Types Rust confort : `D3d11DeviceInfo { device_context, context_mutex }`,
  `RenderCfg { width, height, .. }`, `OutputCfg { dxgi_format, .. }`.
- Compile contre les headers 4.0 déjà vendorés — **rien à builder côté VLC**.

### 2. kyspout — la texture partagée devient la render target de VLC

`SpoutSender` crée **déjà** exactement la bonne texture
(`kyspout/src/lib.rs:308-343` : BGRA, `D3D11_RESOURCE_MISC_SHARED`,
`BIND_RENDER_TARGET | BIND_SHADER_RESOURCE`). Changements :

- **Device** : garder le device kyspout et le **donner** à VLC via setup.
  Deux exigences du header (header:578-583) :
  - *« The ID3D11Device … must have multithreading enabled »* → activer
    `ID3D11Multithread::SetMultithreadProtected(TRUE)` sur le context.
  - Comme kyspout utilise le context **hors** des callbacks VLC (création de
    texture, sync Spout) → fournir un **`context_mutex`** non-NULL.
- **Nouvelle API** (remplace `send_bgra`) :
  - `device_context() -> (*mut c_void, HANDLE)` pour le setup.
  - `update_output(width, height) -> dxgi_format` : (re)crée texture partagée +
    RTV + republie le registry (nom/actif/info block — déjà fait par `open`).
  - `render_target() -> *mut c_void` (la RTV) pour select_plane.
  - `publish()` : `ReleaseSemaphore` dans swap.
- `send_bgra` : **conservé** (chemin CPU de secours + non-Windows via le stub).

### 3. kyvlcplayer — câbler D3D11 au lieu de smem

- `setup_spout_output` (`player.rs:189-261`) : sur Windows, remplacer le bloc
  `set_video_format_callbacks` par `set_video_output_callbacks_d3d11`, en
  routant setup/update_output/select_plane/makeCurrent/swap vers le `SpoutCtx`
  (qui ne porte plus de `buffer` CPU).
- **Garder un toggle runtime smem ↔ d3d11** (ex. var d'env ou champ
  `VideoConfig`) le temps de la validation, pour A/B mesurable.
- Hors Windows : inchangé (smem / stub).

## Points de conception & risques

- **Modèle single-device (zéro copie vrai).** kyspout possède le device, le
  prête à VLC ; VLC rend **directement** dans la texture partagée kyspout →
  aucune copie inter-device. (L'exemple officiel `doc/libvlc/d3d11_player.cpp`
  utilise 2 devices + texture partagée **parce qu'il présente aussi dans une
  swapchain** ; nous n'avons pas de swapchain — juste la texture, que Spout
  partage ensuite au receiver via le legacy shared handle.)
- **Format BGRA.** Demander `dxgi_format = 87` (`DXGI_FORMAT_B8G8R8A8_UNORM`),
  ce que le receiver Spout attend et ce que kyspout crée déjà. À valider : VLC 4
  rend bien dans une RTV BGRA (format RTV standard, a priori OK).
- **⚠️ Risque principal — RTV sur texture legacy `MISC_SHARED`.** Certains
  drivers refusent une RTV sur une texture `D3D11_RESOURCE_MISC_SHARED` (legacy,
  handle 32-bit) selon le format. **Plan B si ça coince :** VLC rend dans une
  texture **intermédiaire** possédée par kyspout (RTV standard), puis
  `CopyResource` GPU→GPU vers la texture partagée dans `swap`. Toujours **zéro
  aller-retour CPU** — une copie GPU→GPU, du même ordre que Spout lui-même
  (négligeable). C'est d'ailleurs le pattern de la plupart des intégrations
  Spout. → prototyper Plan A, se rabattre sur B sans drame.
- **Sync / tearing.** Tenir le `SpoutAccessMutex` sur la fenêtre de rendu VLC
  (makeCurrent enter → acquire ; swap → release + semaphore). Plus le
  `context_mutex` fourni à VLC pour sérialiser l'accès au context partagé.
- **Resize mid-stream.** `update_output` est le hook de changement de taille →
  recréer texture + RTV là (kyspout le fait déjà côté CPU sur changement de
  dims). **Opportunité :** vérifier si ce chemin corrige aussi le gel du #8
  (transmetteur figé quand la résolution source change) au lieu d'en hériter.
- **`setup` peut être rappelé** plusieurs fois (doc) → idempotence.

## Validation

1. **Build** : rebuild fork complet via Docker MinGW, puis bump de la chaîne de
   submodules `vlc-rs → kymedia → kysdk → kyber-desktop` (cf. `bump-fork.sh`).
2. **Mesure avant/après** : l'API C de kyclient expose déjà les timestamps par
   frame `FrameAcquired → FrameDisplayed` (`kyclient/src/capi.rs:1639`) →
   chiffrer le gain réel du zero-copy plutôt que le supposer (cf. #28 levier 4).
   Mesure terrain : `Spout In TOP` sur flux Kyber vs `Spout In TOP` direct.
3. **E2E hardware obligatoire** (comme #8) : Resolume Arena + TouchDesigner
   `Spout In TOP` — couleurs, taille native, resize mid-stream.

## Séquencement (1 commit cohérent par étape)

1. **vlc-rs** : FFI (`sys.rs`) + wrapper (`media_player.rs`) → compile seul.
2. **kyspout** : API render-target (+ garder `send_bgra`). Parties testables
   unitairement isolées.
3. **kyvlcplayer** : câblage + toggle smem↔d3d11 pour l'A/B.
4. **Bump chaîne** + build + mesures + E2E hardware.

## Corrections à porter dans IMPROVEMENTS.md (findings de l'enquête)

- **#8 (Shipped)** et **#28 levier 2** : reformuler le blocage. libVLC 4 est
  **déjà vendoré et buildé** (`subprojects/vlc` @ `4.0.0-dev`, `kyber-260626`).
  Le reste n'est **pas** une migration VLC mais le câblage vlc-rs + kyspout +
  kyvlcplayer décrit ici. Retirer « gros chantier, nécessite libVLC 4 ».
