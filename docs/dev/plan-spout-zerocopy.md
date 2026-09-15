# Sortie Spout zero-copy (D3D11, libVLC 4) — #28-2

Côté réception, un viewer avec `spout_out` republie le flux décodé en sender
Spout local. **libVLC rend directement dans la texture Spout partagée** : le
chemin est GPU → GPU, sans aucune copie CPU. C'est le **défaut sous Windows**.

Travail **côté fork** (`kyber-desktop/kysdk` : vlc-rs + kyctl) ; KyberFrog
consomme le bundle tel quel.

## Principe

libVLC 4 (`kymedia/subprojects/vlc`, `4.0.0-dev`) expose les **output callbacks
D3D11** (`libvlc_video_set_output_callbacks`, engine
`libvlc_video_engine_d3d11`) à côté de l'ancienne API smem. Le relais utilise un
**modèle single-device** : kyspout possède le device D3D11, le prête à libVLC,
et libVLC rend dans la texture partagée que Spout expose ensuite au receiver par
son handle legacy.

```c
bool libvlc_video_set_output_callbacks(mp, engine,
    setup_cb, cleanup_cb, window_cb,
    update_output_cb, swap_cb, makeCurrent_cb, getProcAddress_cb,
    metadata_cb, select_plane_cb, opaque);
```

| Callback | Rôle dans le relais |
|---|---|
| **setup** | Renvoie à libVLC l'`ID3D11DeviceContext*` de kyspout et un `context_mutex`. Idempotent : libVLC peut le rappeler. |
| **update_output** | libVLC donne `width/height`. kyspout (re)crée sa **texture partagée BGRA** et une **RTV** à cette taille, renvoie `DXGI_FORMAT_B8G8R8A8_UNORM` et décrit la **sortie** : sRGB / BT.709 / full-range. |
| **select_plane** | Renvoie la RTV de la texture partagée (une seule plane). |
| **makeCurrent** | `enter=true` : prend le `SpoutAccessMutex` ; `enter=false` : le relâche. Le receiver ne lit jamais une frame à moitié rendue. |
| **swap** | La frame est déjà dans la texture partagée → incrément du `_Count_Semaphore` Spout. |
| window / getProcAddress / metadata | `None` : inutiles pour un relais sans fenêtre (getProcAddress est GL-only). |

## Implémentation par couche

### vlc-rs

- `src/sys.rs` (FFI écrit à la main) : les structs
  `libvlc_video_setup_device_info_t`, `libvlc_video_render_cfg_t`,
  `libvlc_video_output_cfg_t` (`#[repr(C)]`, unions incluses), les typedefs de
  callbacks et `libvlc_video_set_output_callbacks`.
- `src/media_player.rs` : wrapper sûr `set_video_output_callbacks_d3d11`, sur le
  pattern de `set_video_callbacks` — `Box<dyn Fn>` dans une struct de données,
  `Box::into_raw`, trampolines `extern "C"`. Types de confort
  `D3d11DeviceInfo`, `RenderCfg`, `OutputCfg`.

### kyspout

- La texture partagée existante (BGRA, `D3D11_RESOURCE_MISC_SHARED`,
  `BIND_RENDER_TARGET | BIND_SHADER_RESOURCE`) devient la render target de
  libVLC.
- **Device créé avec `D3D11_CREATE_DEVICE_VIDEO_SUPPORT`** (en plus de
  `BGRA_SUPPORT`) : libVLC décode sur ce device et a besoin
  d'`ID3D11VideoDevice`.
- Multithreading activé (`ID3D11Multithread::SetMultithreadProtected`) et
  `context_mutex` non nul, parce que kyspout utilise aussi le context hors des
  callbacks (création de texture, sync Spout).
- API : `device_context()` pour setup, `update_output(w, h)`,
  `render_target()` pour select_plane, `publish()` dans swap.
- `send_bgra` (chemin CPU) est conservé comme repli et pour le stub non-Windows.

### kyvlcplayer

- `setup_spout_output` câble les callbacks D3D11 vers le `SpoutCtx`, qui ne
  porte plus de buffer CPU.
- **`KYSPOUT_SMEM=1`** restaure le chemin smem CPU (BGRA téléchargé puis
  ré-uploadé) en cas de driver récalcitrant.
- Hors Windows : chemin smem / stub.

## Bilan

**Pour**

- **Zéro copie CPU** : le décodage matériel reste sur le GPU jusqu'au receiver.
- **Taille native** : `update_output` suit la résolution du flux et recrée
  texture et RTV à chaque changement.
- **Aucun plugin côté hôte** : Resolume et TouchDesigner en profitent tels
  quels.
- **Aucune migration de libVLC** : l'API est additive dans le VLC déjà vendoré ;
  seules libkyclient et `kyclient.exe` sont rebuildées.
- **Repli immédiat** par variable d'environnement.

**Contre**

- Le device D3D11 est **partagé entre libVLC et kyspout** : toute utilisation du
  context hors callbacks passe par le mutex.
- Dépend du driver acceptant une RTV sur une texture legacy `MISC_SHARED` (validé
  sur RX 7800 XT) ; sinon, repli smem.
- Windows seulement.

## Warnings libVLC attendus

- `window size missing` : pas de `window_cb`, libVLC déduit la taille de la
  source, ce qui est voulu.
- `external ID3D11DeviceContext mutex not provided` : `null` volontaire côté
  libVLC, tout l'usage du context se fait dans les callbacks.

## Validation

1. **Build** : rebuild fork via Docker MinGW, bump de la chaîne
   `vlc-rs → kymedia → kysdk → kyber-desktop`.
2. **Mesure** : timestamps par frame `FrameAcquired → FrameDisplayed`
   (`kyclient/src/capi.rs:1639`), et `Spout In TOP` sur le flux Kyber vs
   `Spout In TOP` direct (#28-4).
3. **E2E hardware** : Resolume Arena + TouchDesigner — image visible, couleurs
   correctes, taille native, sender enregistré et actif. Procédure :
   [E2E Spout output](../E2E-spout-output.md).
