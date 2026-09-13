# Architecture greenfield — piste prospective

!!! note "Piste non planifiée"
    L'architecture en place est l'orchestrateur de binaires (α), maintenu selon
    [plans-fork-restructure.md](plans-fork-restructure.md). Ce document décrit
    les ruptures possibles si la chaîne de forks devenait trop coûteuse. Aucune
    n'est engagée.

La question : **quelle architecture embarquerait le moins**, en intégrant les
dépendances au plus bas niveau plutôt qu'en orchestrant les binaires du fork ?

## Ce que le stack actuel embarque

Bundle Windows du fork : **171 MB**.

| Composant | Poids | Sert à |
|---|---|---|
| Plugins libVLC (325 fichiers) | 60 MB | Décodage/rendu côté viewer |
| FFmpeg (7 DLLs) | ~35 MB | Capture + filtrage + encodage côté émetteur (via txproto) |
| kycontroller.exe | 21 MB | Session/auth/API d'une instance émettrice |
| libVLC core + libplacebo + SPIRV | ~16 MB | Rendu VLC |
| kyclient, kyavserver, kynput, txproto, SDL2, MinGW… | ~35 MB | Le reste de la chaîne |

Deux constats orientent les options :

- **L'essentiel du poids est le moteur média généraliste**, alors que le pipeline
  KyberFrog est **fermé** : on contrôle les deux bouts, la tolérance universelle
  de formats ne sert à rien.
- **Les seules briques upstream jamais forkées sont les briques transport**
  (`kymux`, `kyutil`). Toute la divergence vit dans la couche média/capture.

## α — Orchestrateur de binaires *(en place)*

KyberFrog spawne kycontroller/kyavserver/kyclient. Embarque 171 MB, maintient
7 forks, **zéro code média à écrire** : l'architecture achète la maturité de
Kyber (latence, jitter, reconnexion) au prix de la chaîne de forks. Windows et
Linux amd64 sont livrés sur ce modèle.

## γ — Natif Windows minimal

**Un exe (~10–25 MB), zéro DLL média** : tout le média est fourni par l'OS.

| Fonction | Aujourd'hui (fork) | Natif Windows |
|---|---|---|
| Capture écran | txproto DXGI (rotation non gérée, #17-P2) | **Windows.Graphics.Capture** (rotation, HDR, occlusion) |
| Capture Spout | txproto `iosys_spout` | textures partagées D3D11 + registre des senders (windows-rs ou Spout SDK statique) |
| Webcam | DirectShow via lavd | **Media Foundation** `IMFCaptureEngine` |
| Encodage H.264 | FFmpeg x264 | **MFT encoder** (NVENC/AMF/QSV négociés par l'OS ; repli openh264/x264 statique) |
| Décodage + rendu | libVLC + libplacebo | MFT decoder → swapchain D3D11 (winit) |
| Sortie Spout viewer | libVLC → texture partagée | la texture décodée *est* partageable |
| Audio | kyaudioreg + VLC | WASAPI + Opus (libopus statique) |
| Transport | kymux via kycontroller/kyavserver | **crate `kymux` en direct** |
| Session/auth | kycontroller (IPC 9091–9100) | in-process — la limite ~9 instances disparaît |
| Remote desktop | kynput + kynputserver | crate kynput, injection `SendInput` |

**Pour** : le plus petit livrable possible ; plus aucun fork (`kymux` et
`kyutil` restent des crates upstream pures) ; les chantiers fork ouverts
disparaissent par construction (rotation, encodeur GPU, plafond d'instances) ;
le supervisor supervise des tâches tokio au lieu de process ; le cockpit (UI,
tray, config, supervisor) reste tel quel.

**Contre** : réécrire un pipeline AV temps réel — horodatage, pacing, jitter
buffer, synchro A/V, renégociation de résolution, reconnexion — soit **3 à 5
mois** pour la parité E2E ; compatibilité protocole Kyber limitée au transport ;
**Windows uniquement**, ce qui exige un second backend média pour garder Linux.

## δ — FFmpeg en bibliothèque

Même mouvement que γ (un binaire, `kymux` en crate, VLC/txproto/kyctl/kymedia
retirés), mais le média passe par **libav\* linké directement**
(`ffmpeg-next`) :

- capture : avdevice (dshow/gdigrab sous Windows ; v4l2/x11grab/kmsgrab sous
  Linux) + Spout en windows-rs ;
- encode/décode : avcodec (nvenc/amf/qsv/x264 ; vaapi/v4l2m2m sous Linux) ;
- rendu : wgpu, ou D3D11/Vulkan via winit.

**Pour** : ~35–40 MB embarqués, FFmpeg stock sans patch ; **un seul code de
pipeline pour Windows et Linux** ; plus aucun fork.

**Contre** : même réécriture du pipeline temps réel que γ, un peu allégée par
FFmpeg (**2 à 4 mois**) ; compatibilité protocole limitée au transport.

## Synthèse

| | α en place | γ natif Windows | δ FFmpeg-lib |
|---|---|---|---|
| Embarqué | 171 MB | **~10–25 MB** | ~40 MB |
| Forks à maintenir | 7 | **0** | **0** |
| Code média à maintenir | 0 | maximal | moyen |
| Linux | oui | non (ou 2ᵉ backend) | **oui** |
| Compat protocole Kyber | totale | transport seul | transport seul |
| Délai avant bénéfice | — | 3–5 mois | 2–4 mois |

Avec Linux livré, **δ est la seule rupture qui garde le périmètre produit**.

## Si la piste était engagée

1. **Spike de dé-risquage (~1–2 semaines)**, hors KyberFrog : capture 1080p60 →
   encodage GPU sur la RX 7800 XT → mesure charge/latence ; puis émetteur →
   `kymux` → récepteur → décodage → affichage, latence bout-en-bout comparée à
   la chaîne Kyber.
2. **Remplacement progressif** : le viewer d'abord (réception native, sortie
   Spout directe, le supervisor spawne le viewer natif au lieu de kyclient),
   puis l'émission, en retirant le bundle morceau par morceau. Le fork reste en
   production pendant toute la transition.
