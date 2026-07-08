# Architecture greenfield — et si KyberFrog intégrait ses dépendances au plus bas niveau ?

*Analyse du 2026-07-08, en réponse à la remise à plat de #24 : « si on
recommençait kyberfrog de zéro, quelle architecture embarquerait le
moins ? ». Complète [plans-fork-restructure.md](plans-fork-restructure.md)
(plans A/B sur la chaîne actuelle) par les options de rupture.*

## Ce que le stack actuel embarque, et pourquoi

Le bundle fork pèse **171 MB** :

| Composant | Poids | Sert à |
|---|---|---|
| Plugins libVLC (325 fichiers) | 60 MB | Décodage/rendu côté viewer (kyvlcplayer joue une URI locale via libVLC) |
| FFmpeg (7 DLLs) | ~35 MB | Capture + filtrage + encodage côté émetteur (via txproto) |
| kycontroller.exe | 21 MB | Session/auth/API d'une instance émettrice |
| libVLC core + libplacebo + SPIRV | ~16 MB | Rendu VLC |
| kyclient.exe/.dll, kyavserver, kynput*, txproto, SDL2, MinGW… | ~35 MB | Le reste de la chaîne |

Constat : **l'essentiel du poids est le moteur média généraliste**
(VLC lit n'importe quoi, FFmpeg capture/encode n'importe quoi). Or le
pipeline KyberFrog est **fermé** — on contrôle les deux bouts. La
tolérance universelle de formats ne sert à rien : l'émetteur produit
exactement ce que le récepteur attend.

Deuxième constat (audit) : les seules briques upstream **jamais forkées**
sont précisément les briques *transport* : `kymux` (QUIC, backend quinn)
et `kyutil`. Toute la divergence fork vit dans la couche média/capture —
celle que Windows sait faire nativement.

## Le spectre des architectures

### α — Orchestrateur de binaires (actuel)

kyberfrog spawne kycontroller/kyavserver/kyclient. Embarque 171 MB,
maintient 7 forks, mais **zéro code média à écrire**. C'est l'archi qui
achète la maturité de Kyber (latence, jitter, reconnexion) au prix de la
chaîne de forks.

### β — Crates du fork linkés in-process (rejetée)

Un seul binaire qui linke `kyclient-rs`, `kyavservice`, etc. Cumule les
deux inconvénients : toute la chaîne de forks *dans* le build kyberfrog
(fin de l'isolation build-time constatée par l'audit) **et** toujours
~150 MB de DLLs (libVLC/FFmpeg restent des DLLs). Gain marginal
(suppression de la génération de configs et des process). À écarter.

### γ — Natif Windows minimal : « qui embarque le moins »

Réponse littérale à la question : **un seul exe (~10–25 MB), zéro DLL
média embarquée** — tout le média est fourni par l'OS.

| Fonction | Aujourd'hui (fork) | Natif Windows |
|---|---|---|
| Capture écran | txproto DXGI (rotation cassée, #17 B1) | **Windows.Graphics.Capture** (WinRT — gère rotation, HDR, occlusion) |
| Capture Spout | txproto iosys_spout (fork #19) | D3D11 shared textures + registre senders (protocole Spout ≈ 500–1000 lignes windows-rs, ou Spout SDK statique) |
| Webcam | 7 fixes lavd + DirectShow (fork #18-A) | **Media Foundation** IMFCaptureEngine |
| Encodage H.264 | FFmpeg x264 (AMF crashe sur RX 7800 XT) | **MFT encoder** (négocie NVENC/AMF/QSV nativement — chemin différent du AMF FFmpeg ; fallback logiciel openh264/x264 statique) |
| Décodage | libVLC (60 MB de plugins) | MFT decoder → texture D3D11 |
| Rendu viewer | VLC + libplacebo | Swapchain D3D11 fullscreen (winit) |
| Sortie Spout viewer (#8) | vlc-rs smem + copie CPU | **Gratuite** : la texture D3D11 décodée *est* partageable Spout |
| Audio | kyaudioreg + VLC | WASAPI + Opus (libopus statique ~300 KB) |
| Transport | kymux (via kycontroller/kyavserver) | **kymux, le crate, en direct** (γ1) ou quinn + protocole maison (γ2) |
| Session/auth | kycontroller (21 MB, IPC 9091–9100, auth basic hashée) | In-process : certs TLS de kymux ; la limite ~9 instances et l'auth transparente `vj/kyberfrog` disparaissent |
| Remote desktop | kynput + kynputserver | Crate kynput réutilisable (upstream-pur possible), injection SendInput |

**γ1 (recommandée dans γ)** : garder `kymux` + `kyutil` comme dépendances
crates — upstream purs, jamais forkés, pins exacts. On conserve l'ADN
QUIC/latence de Kyber sans hériter d'un seul fork. γ2 (quinn nu +
protocole maison) ne gagne presque rien et jette le tuning transport.

Effets de bord notables de γ :

- Les chantiers fork ouverts **disparaissent par construction** : #17
  phase 2 (rotation — WGC la gère), les fixes lavd (MF remplace), le
  crash AMF (autre chemin d'encodage), Amélioration 2 Spout-out (texture
  déjà en D3D11), la limite de 9 instances (plus d'IPC kycontroller).
- `gen.rs` (génération de kyber_config.toml), l'auth transparente, la
  résolution PATH des binaires, le Job Object multi-process : tout ce
  code kyberfrog se simplifie ou disparaît (le supervisor supervise des
  *tasks* tokio, plus des process).
- Le cockpit (web UI, tray, config TOML, supervisor) **reste tel quel** —
  c'est le moteur qu'on remplace, pas l'orchestration.

**Le prix.** On réécrit un pipeline AV temps réel : horodatage, pacing,
jitter buffer, synchro A/V, renégociation résolution, reconnexion —
c'est la maturité de Kyber qu'on abandonne (kymux n'apporte *que* le
transport ; kycontroller/kyavserver portent la logique de session et de
pipeline). Estimation réaliste : **3–5 mois** pour parité E2E
(émission ~1 mois, réception ~1 mois, audio ~2 sem., remote desktop
~2 sem., durcissement/latence ~1 mois+), validation hardware continue.

**La contradiction stratégique : Linux/ARM.** WGC, MF, WASAPI, D3D11 =
Windows uniquement. Le chantier `feat/linux-arm-support` (Romain Henry,
`.deb` AMD64+ARM64) est incompatible avec γ pur : il faudrait soit
assumer **Windows-only**, soit ajouter un second backend média Linux
(PipeWire/VAAPI) — et on ré-embarque un stack.

### δ — FFmpeg-en-bibliothèque : le compromis cross-platform

Même mouvement que γ (un binaire, kymux en crate, VLC/txproto/kyctl/
kymedia/kyber-desktop supprimés), mais le média passe par **libav\* linké
directement** (`ffmpeg-next`) au lieu des APIs Windows :

- Capture : avdevice (gdigrab/dshow sous Windows ; v4l2/x11grab/kmsgrab
  sous Linux) + Spout en windows-rs (comme γ — Spout n'existe pas sous
  Linux de toute façon).
- Encode/décode : avcodec (h264_nvenc/amf/qsv/x264 sous Windows,
  vaapi/v4l2m2m sous Linux/ARM).
- Rendu : wgpu ou D3D11/Vulkan via winit.

Embarque **~35–40 MB** (FFmpeg stock upstream, zéro patch — l'audit a
montré que les 4 patches contrib sont upstream kyber, pas nécessaires
hors txproto). Un seul code de pipeline pour Windows *et* Linux/ARM.
C'est « γ moins minimal, mais compatible avec le chantier Romain ».

## Synthèse

| | α actuel | β in-process | γ natif Windows | δ FFmpeg-lib |
|---|---|---|---|---|
| Embarqué | 171 MB | ~150 MB | **~10–25 MB (exe seul)** | ~40 MB |
| Forks à maintenir | 7 | 7 (dans le build !) | **0** | **0** |
| Code média à écrire/maintenir | 0 | 0 | maximal | moyen (FFmpeg fait le gros) |
| Linux/ARM (chantier Romain) | oui (branche en cours) | oui | **non** (ou 2ᵉ backend) | **oui** |
| Chantiers fork ouverts (#17 P2, AMF…) | à faire dans le fork | idem | disparaissent | disparaissent en grande partie |
| Compat protocole Kyber | totale | totale | transport seul (kymux) | transport seul (kymux) |
| Délai avant bénéfice | immédiat | ~semaines | 3–5 mois | 2–4 mois |

**Réponse directe aux questions posées :**

- *« Intégrer directement les dépendances au plus bas niveau ? »* Oui,
  c'est cohérent — et l'audit le rend possible : la seule dépendance
  irremplaçable (transport QUIC) est justement celle qui n'a jamais eu
  besoin d'être forkée (`kymux`). Tout ce qui est forké est la couche
  média, substituable par l'OS (γ) ou FFmpeg stock (δ).
- *« Qui embarque le moins ? »* **γ1** : un exe, kymux+kyutil+windows-rs
  en crates, le média fourni par Windows.
- *« La mieux ? »* Dépend d'un arbitrage que seul l'opérateur peut
  faire : **si Linux/ARM reste un objectif produit → δ ; si Windows-only
  est assumable → γ1.** À décider *avant* d'investir davantage dans le
  chantier linux-arm actuel (qui construit la chaîne de forks pour
  Linux, c'est-à-dire l'exact opposé de γ/δ).

## Conséquences sur les plans A et B

- **Plan A (MRs upstream) : inchangé, toujours utile.** Les bugfixes
  profitent à la période transitoire (des mois) et à l'écosystème Kyber
  même après. Rien à jeter.
- **Plan B (mono-repo) : à geler si γ/δ est retenu.** Restructurer une
  chaîne qu'on va éteindre est un investissement perdu — pendant la
  transition, geler le fork en l'état (il marche) et ne faire que les
  fixes critiques. Si γ/δ est rejeté (délai, risque), B redevient le
  plan directeur.

## Prochain pas proposé : spike de dé-risquage (~1–2 semaines)

Avant tout engagement : un prototype jetable qui valide les deux
inconnues techniques majeures, hors de kyberfrog :

1. **Encodage MF sur le RX 7800 XT** (γ) ou **h264_amf/x264 via
   ffmpeg-next** (δ) : capture WGC 1080p60 → encode → mesure
   charge/latence. C'est le point qui a déjà mordu (crash AMF FFmpeg).
2. **Chaîne kymux minimale** : émetteur → kymux/quinn → récepteur →
   décode → affichage, latence bout-en-bout mesurée vs chaîne Kyber
   actuelle (~x ms). Valide que la logique de session réécrite tient.

Si le spike passe : stratégie strangler — remplacer d'abord le **viewer**
(réception native, Spout-out gratuit, le supervisor spawne le viewer
natif au lieu de kyclient), puis l'émission, et retirer le bundle
morceau par morceau. Le fork reste en production pendant toute la
transition.
