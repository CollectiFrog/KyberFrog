# Résolutions et formats d'image, de la source au récepteur — audit (tours 1-2)

*Étude, pas de code. Point de départ : un projet TouchDesigner qui sort en
Spout à une résolution « custom » n'arrive jamais au récepteur, et une entrée
HDMI sur une DeckLink non plus. Question posée : d'où viennent ces blocages, et
comment corriger par une architecture plutôt que par des rustines.*

*Tour 1 — 2026-09-28. Fork lu au pin `kyber-desktop` `3455934`
(`txproto` `15d03e1`, `kymedia` `d1b955d`, FFmpeg n8.1). Tests sur le PC de dev
(RX 7800 XT, pilote AMF, ffmpeg du bundle).*

Légende : **constaté** (lu dans le code ou mesuré), **déduit** (la lecture ne
laisse guère de doute, pas encore reproduit), **à confirmer**.

## En bref

- **Personne ne négocie la taille ni le format.** La source impose les siens,
  l'encodeur s'ouvre sur la première image reçue, le récepteur décode ce qui
  arrive. Aucune étape ne ramène l'image à ce que l'encodeur sait prendre.
- **Le blocage le plus probable côté TD n'est pas la résolution mais le format
  de pixel.** La capture Spout n'accepte que le RGBA/BGRA 8 bits ; tout autre
  format (10 bits, 16 ou 32 bits flottant) boucle en silence. C'est **constaté**
  dans tes logs : « UE Test » (Unreal, 2038×782) a répété
  `Unsupported sender texture format: 24` 327 fois le 2026-08-21 sans jamais
  envoyer une image. Le Spout Out TOP de TD partage par défaut le format de son
  entrée, et deux Kinect TOP du projet sont en `rgba16float`.
- **L'encodeur AMF H.264 a une plage de 128 à 4096 px par côté** (constaté).
  Hors plage, KyberFrog bascule seul sur x264 — ça marche, mais x264 ne tient
  plus 60 fps au-delà de la 4K environ.
- **Un changement de taille en cours de flux gèle le transmetteur** sur les
  chemins qui passent par un filtre (x264, caméras) : l'encodeur se recrée, le
  filtre non. C'est la cause probable de la limite connue de #8.
- **Les erreurs de capture sont muettes** : le transmetteur reste « Running »,
  le viewer reste noir, et le repli x264 ne se déclenche pas.
- **DeckLink sous Windows n'est pas supporté du tout** (pas un problème de
  résolution) → nouvelle carte #52. Sous Linux, la carte ne reconnaît que les
  modes broadcast : une sortie PC à résolution « PC » n'accroche pas.
- **Téléphone tourné en paysage (KyberFrog Cast)** : le flux reste portrait,
  l'image paysage y est réduite avec des bandes noires. La taille de capture
  est figée au démarrage côté téléphone, et la fenêtre kyclient l'est à la
  connexion.
- **Le fil commun** : à chaque étage (capture, filtre, démuxeur DeckLink,
  fenêtre du viewer), la taille est décidée **une fois** et jamais renégociée.
- **Tour 2 — partie 1 faite (formats Spout)** : épinglée (`4c5099f`), toute
  source Spout lisible arrive désormais au viewer, convertie en BGRA 8 bits
  sur le GPU ; mesuré sur 13 formats, en AMF comme en x264. Le banc a trouvé
  deux défauts de plus, déjà présents dans 0.7.0 : le RGBA 8 bits ne passait
  pas non plus (C1b), et x264 décale les couleurs saturées (C9). Voir
  [partie 1](#6-partie-1-formats-spout).

## 1. Le chemin d'une image

```mermaid
flowchart LR
  subgraph TX["PC émetteur — kyavserver (txproto)"]
    S["Capture<br/>Spout · écran DXGI · caméra dshow/v4l2 · DeckLink"]
    F["Filtre FFmpeg<br/>(seulement x264 / caméra)"]
    E["Encodeur<br/>AMF · NVENC · x264"]
    S -->|image GPU ou CPU,<br/>taille et format de la source| F
    S -.->|chemin direct AMF/NVENC| E
    F --> E
  end
  E -->|H.264 par défaut| N["QUIC / kymux"]
  subgraph RX["PC récepteur — kyclient"]
    D["VLC · décodage D3D11VA"]
    O["Fenêtre plein écran<br/>ou sortie Spout"]
    D --> O
  end
  N --> D
```

Aucune boîte ne choisit une taille. Le seul redimensionnement du code est
Linux-only et forcé à 1920 de large (`scale=w=1920:h=-1`, #33,
`kymedia` `kyavservice/src/txproto/video.rs:645`). Le codec est choisi par le
**client** (`--video-codec`, défaut `h264`, `kyclient/src/main.rs:212-216`) et
KyberFrog ne le règle pas. L'encodeur est choisi par la **marque du GPU**
(`shared/src/encoder.rs`), jamais en fonction de la source.

## 2. Constats

### C1 — Spout : quatre formats 8 bits acceptés, tout le reste boucle en silence

**Constaté.** `txproto` `src/iosys_spout.c:145-160` ne traduit que
`B8G8R8A8_*` et `R8G8B8A8_*` (plus le 0 historique). Tout autre format :
`bind_shared_texture()` logue `Unsupported sender texture format: N`
(`:713-720`) et la boucle de capture réessaie toutes les 100 ms, **indéfiniment
et sans événement d'erreur** (`:817-822`).

| Format DXGI | N° dans le log | Qui l'envoie | Accepté ? |
|---|---|---|---|
| `B8G8R8A8_UNORM` | 87 | Resolume, OBS, TD (senders Kinect du PC de dev) | oui |
| `R8G8B8A8_UNORM` | 28 | toute app qui partage du RGBA 8 bits | **non**, voir C1b |
| `R10G10B10A2_UNORM` | 24 | Unreal (constaté), TD 10 bits | **non** |
| `R16G16B16A16_FLOAT` | 10 | TD 16 bits float | **non** |
| `R32G32B32A32_FLOAT` | 2 | TD 32 bits float | **non** |
| formats mono (`R8`, `R16`…) | 61, 56… | TD mono, si Spout les partage | **non** (à confirmer côté TD) |

TouchDesigner : le paramètre *Pixel Format* du Spout Out TOP vaut « Use Input »
par défaut, et « Spout can send textures in various pixel formats up to and
including 32-bit float RGBA »
([doc Derivative](https://docs.derivative.ca/Syphon_Spout_Out_TOP)). Le format
dépend donc de toute la chaîne de TOP en amont.

Preuve dans les logs du PC de dev (`instances/ue-test/kycontroller.log`) :
sender « UE Test » 2038×782 enregistré, encodeur x264 créé, puis 327 lignes
`Unsupported sender texture format: 24` et aucune image.

### C1b — Spout : même le RGBA 8 bits ne passe pas (trouvé au tour 2)

**Constaté, mesuré.** Le RGBA 8 bits (`R8G8B8A8`, DXGI 28) était « accepté »
par la table des formats, mais le pool de textures D3D11 de FFmpeg n'a pas de
`rgba` : `av_hwframe_ctx_init()` échoue (`Unsupported pixel format: rgba`) et
la capture réessaie dix fois par seconde, sans fin. Reproduit sur le bundle
installé (pin `3455934`) avec le banc de formats ([partie 1](#6-partie-1-formats-spout)).
Seul le BGRA 8 bits passait donc réellement.

### C2 — AMF H.264 : de 128 à 4096 px par côté

**Constaté** sur la RX 7800 XT, avec le ffmpeg du bundle, en reproduisant le
chemin Kyber (images D3D11 BGRA → `h264_amf`, `ultralowlatency`, `bf=0`) :

| Taille | h264_amf | hevc_amf | libx264 |
|---|---|---|---|
| 1920×1080, 2038×782, 3000×1000 | ok | ok | ok |
| impaires (1279×719, 1281×720, 1920×1081) | ok | ok | ok |
| 3840×2160, 4096×2304, 1080×4096 | ok | ok | ok |
| 4098×1080 … 8192×1080 (triple HD, ultra-large) | **échec** `Init() failed with error 5` | ok (≤ 8192) | ok |
| 1920×4352 | **échec** | ok | ok |
| < 128 px sur un côté (80×60, 160×90, 1920×100) | **échec** | **échec** | ok |

Les tailles impaires ne posent **aucun** problème, contrairement à l'intuition.
NVENC n'a pas pu être testé ici (pas de GPU NVIDIA) : sa limite H.264 documentée
est 4096×4096, **à confirmer**.

Hors plage, la ligne `ERROR txproto::lavc:h264_amf …` déclenche le repli
KyberFrog vers x264 (`shared/src/encoder.rs:143`,
`kyberfrog/src/supervisor.rs:782`). Le flux finit donc par passer — sur le CPU.

### C3 — Le repli x264 ne tient pas 60 fps en très grand

**Constaté, mesure pessimiste** (le générateur de test et l'upload GPU sont
comptés dans le temps). Chaîne proche de Kyber : téléchargement BGRA,
conversion CPU vers NV12, x264 `ultrafast` sur 2 threads — ce que txproto impose
(`src/encode.c:89`) tant que `encoder_threads` n'est pas réglé.

| Taille | chaîne actuelle | conversion NV12 sur GPU puis téléchargement |
|---|---|---|
| 1920×1080 | 148 fps | — |
| 3840×2160 | 24 fps | 32 fps |
| 5760×1080 | 32 fps | 44 fps |
| 7680×1080 | 32 fps | — |

Pour mémoire, x264 coûtait ~26 ms de latence contre ~4 ms pour AMF au banc #28.

### C4 — Changement de taille à chaud : l'encodeur suit, le filtre non

**Constaté pour l'encodeur, déduit pour le gel.** L'encodeur compare chaque
image à sa configuration et se recrée si la taille change
(`src/encode.c:598-607`, `:639`, `:653`). Le graphe de filtre, lui, est
configuré **une fois**, sur la première image (`src/filter.c:403-454`), et
jamais reconstruit. Or ce filtre existe sur deux chemins :

- **x264** : `hwdownload,format=bgra,format=nv12` (`video.rs:635`) — le
  téléchargement garde la taille du premier pool de textures ;
- **caméra → AMF/NVENC** : `format=nv12` avant l'encodeur (`video.rs:277`).

Seul le chemin direct Spout/écran → AMF suit un changement de taille. C'est la
cause probable de la limite connue de #8 (« the transmitter freezes and needs a
manual restart when the captured screen changes resolution mid-stream ») et de
la limite notée dans [plan-spout-passthrough](plan-spout-passthrough.md#limites-connues).
À reproduire au tour 2 (TD sait changer sa résolution en direct).
Accessoirement, un changement de **format** seul (BGRA ↔ RGBA) n'est pas
détecté (`video_config_changed` ne regarde que taille et rotation).

### C5 — Les erreurs de capture ne remontent pas

**Constaté.** Trois échecs de capture ne produisent aucun événement d'erreur
vers kyavservice :

- Spout, format refusé ou `OpenSharedResource()` en échec (PC multi-GPU) :
  boucle de réessai (`iosys_spout.c:817-822`) ;
- caméra/DeckLink, `av_read_frame` en erreur : le thread s'arrête sur
  `goto end` (`src/iosys_lavd.c:108-109`), sans `SP_EVENT_ON_ERROR` ;
- côté KyberFrog, le détecteur ne reconnaît que les erreurs d'**encodeur
  matériel** (`txproto::lavc:*_amf|_nvenc|_qsv|_vaapi`), pas `txproto::spout`.

Résultat : transmetteur « Running », viewer noir. Et si aucune image n'arrive
dans les 5 s, la session est perdue pour de bon (#51, `OUTPUT_READY_TIMEOUT`).

### C6 — DeckLink

**Windows — non supporté (constaté).** Le ffmpeg du bundle Windows n'est pas
compilé avec `--enable-decklink` (voir `ffmpeg -version`), et la carte vue par
DirectShow (« Decklink Video Capture ») s'annonce en `(none)`, que
`kyberfrog/src/cameras.rs` écarte (seuls les `(video)` sont gardés). La carte
n'apparaît donc nulle part. → **#52**.

**Linux (#47)** — le démuxeur FFmpeg (`libavdevice/decklink_dec.cpp`) :

- sans `format_code`, détecte le mode pendant **3 s maximum** (`:1049-1057`),
  sinon `Cannot Autodetect input stream or No signal` et l'ouverture échoue
  (`:1211-1215`) ;
- la carte ne reconnaît que des **modes broadcast** (720p, 1080i, 1080p, 2K/4K
  selon le modèle) : une sortie PC en 1920×1200, 1280×1024 ou résolution custom
  n'accroche pas (fiches Blackmagic, aucune résolution PC listée) ;
- le 1080p50/60 dépend du modèle : listé pour les Mini Recorder HD et 4K
  ([tech specs](https://www.blackmagicdesign.com/products/decklink/techspecs)),
  **à confirmer** pour la Mini Recorder d'origine de Romain ;
- un changement de mode en cours de flux est seulement noté
  (`VideoInputFormatChanged`, `:1015-1025`), le flux n'est pas rouvert → gel
  (déduit).

### C7 — Côté récepteur

**Constaté sur ce PC** : le décodage H.264 D3D11VA de la RX 7800 XT passe
jusqu'en 8192×4320 (test ffmpeg, même API que VLC). **À confirmer** sur les PC
récepteurs réels : un iGPU Intel plafonne souvent à 4096×2304 en H.264, et VLC
retombe alors en décodage logiciel (déduit). La sortie Spout du viewer suit la
taille native du flux ([plan-spout-zerocopy](plan-spout-zerocopy.md)).

### C8 — Téléphone tourné : le flux reste portrait (KyberFrog Cast)

Symptôme rapporté : partage d'écran depuis KyberFrog Cast, viewer kyclient
vertical ; en tournant le téléphone, la fenêtre reste verticale et l'image
paysage s'y affiche réduite, sa largeur limitée à celle du cadre portrait.

**Émetteur (constaté, lecture).** `CastService` lit
`resources.displayMetrics` **au démarrage** du partage
(`android/…/CastService.kt:114-116`, dépôt `kyberfrog-cast`) et crée le
`VirtualDisplay` à cette taille (`VideoSource.kt:229-234`), en
`VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR`. Rien n'écoute la rotation (aucun
`DisplayListener`, aucun `VirtualDisplay.resize()`, sur aucune branche) :
Android recopie l'écran devenu paysage dans un cadre resté portrait, en le
réduisant avec des bandes noires. L'encodeur, lui, reçoit toujours du
1080×2230. Log réel du 2026-09-28 19:09 (`kyclient-Nothing-Phone-2.log`) :
`Got displays: [Display { id: 0, height: 2230, width: 1080 … }]`, puis aucune
mise à jour de taille.

**Récepteur (constaté, lecture).** La fenêtre kyclient prend sa taille une
seule fois, à la connexion : taille de l'écran source ÷ 1,5
(`kyclient/src/event_loop.rs:241-250`). Un `DisplayListUpdated` ne met à jour
que le calage de la souris (`window/window.rs:243-259`,
`windows_statemachine.rs:490-500`), jamais la taille de la fenêtre. Même avec
un téléphone qui annoncerait du paysage, **une fenêtre non plein écran
resterait portrait**. En plein écran, la fenêtre est le moniteur et VLC ajuste
le ratio : là, corriger l'émetteur suffirait (déduit).

**Ce qui existe déjà** : `f4740be` (branche `feat/pilot-input-channel` de
kyberfrog-cast) sait déjà annoncer un changement de taille au contrôleur
(`DisplayListUpdated`). Il manque la rotation qui le déclenche, et un nouvel
encodeur à la nouvelle taille. **À vérifier** : que le paquet de config
(SPS/PPS, `docs/DESIGN.md:100`, envoyé « à la connexion ») repart bien vers
une session déjà ouverte.

## 3. Tes deux cas, relus

**TD en Spout, résolution inconnue.** Une seule ligne de
`%APPDATA%\kyberfrog\instances\<transmetteur>\kycontroller.log` tranche :

| Ligne dans le log | Hypothèse |
|---|---|
| `Spout: New sender registered: "<nom>" (L×H)` | donne **la résolution** que tu cherches |
| `Unsupported sender texture format: N` | C1, format de pixel (le plus probable) |
| `h264_amf … Init() failed with error 5` puis relance | C2, taille hors plage AMF → repli x264 |
| `Configuration change detected: L×H` puis plus rien | C4, changement de taille à chaud |
| `OpenSharedResource() failed` | TD et KyberFrog sur deux GPU différents |

Dans `Kinect-Cam_NDI-Spout.toe`, le sender `kinect-contour` descend d'un Kinect
TOP `playerindex` en **80×60** : sous le minimum AMF (repli x264 attendu), et au
format de pixel hérité en cascade, illisible sans TD lancé.

**HDMI → DeckLink.** Sous Windows : non supporté (#52). Sous Linux : vérifier
que la source sort un mode broadcast (1080p50/60 selon le modèle), pas une
résolution PC.

## 4. Piste de ré-architecture (esquisse, à arbitrer)

L'idée : **une étape de normalisation explicite entre la capture et
l'encodeur**, pilotée par ce que l'encodeur choisi sait prendre.

```mermaid
flowchart LR
  S["Capture<br/>accepte tout format lisible<br/>(8/10/16/32 bits)"] --> N["Normalisation GPU<br/>scale_d3d11 → NV12<br/>taille bornée, ratio gardé"]
  C["Table de capacités<br/>AMF H.264 128–4096<br/>HEVC ≤ 8192 · x264 ∞"] -.-> N
  N --> E["Encodeur<br/>direct (AMF/NVENC)<br/>ou téléchargement NV12 (x264)"]
  N -. "reconstruite à chaque<br/>changement de taille/format" .-> N
  S -. "erreur = événement<br/>visible dans l'UI" .-> U["Statut KyberFrog"]
```

Briques déjà vérifiées sur ce PC avec le FFmpeg 8.1 du bundle :

- `scale_d3d11=width=…:height=…:format=nv12` convertit et réduit sur le GPU
  (BGRA 5760×1080 → NV12 3840×720 → `h264_amf` : ok) ;
- AMF accepte en entrée `x2bgr10` (= format 24 d'Unreal) : ok ; FFmpeg déclare
  aussi `rgbaf16` (= TD 16 bits float), **non testable** au ffmpeg en ligne de
  commande ;
- la même conversion GPU sert x264 : 2,7× moins d'octets à télécharger (c'est
  aussi l'item B3 de [plan-latency](plan-latency.md)).

Ce que ça règle : C1 (tout format lisible entre), C2 et C3 (la taille est
bornée avant l'encodeur, ou HEVC choisi), C4 (graphe reconstruit à chaque
changement), C5 (erreurs remontées).

C8 montre que la normalisation seule ne suffit pas : il faut un **chemin de
renégociation de bout en bout**. La source annonce sa nouvelle taille
(rotation, resize TD, changement de mode DeckLink), le pipeline se reconstruit,
le contrôleur publie `DisplayListUpdated` (déjà existant), et le viewer adapte
sa fenêtre s'il n'est pas en plein écran. Où ça vit : `kymedia` `video.rs`
(construction du graphe), `txproto` `filter.c` (reconfiguration),
`iosys_spout.c` (table des formats), KyberFrog (statut, choix encodeur/codec).

Décisions ouvertes : que faire d'une source trop grande (réduire ou passer en
HEVC, qui exige des récepteurs compatibles), et qui porte le changement de
codec (le client le demande aujourd'hui).

## 5. Pas vérifié à ce tour

- Le format réel du Spout de ton projet TD : **vu au tour 2**, le projet Kinect
  ouvert sur le PC de dev publie trois senders en BGRA 8 bits (128×128 sans
  Kinect branchée, 1280×720), avec des doublons `_1` (noms orphelins d'une
  session précédente restés dans le registre Spout).
- `rgbaf16` dans AMF : ffmpeg ne sait pas générer ce format pour le test ;
  sans objet depuis la partie 1 (tout est converti en BGRA avant l'encodeur).
- NVENC : pas de GPU NVIDIA ici.
- DeckLink : pas de carte sur ce PC.
- Le gel au redimensionnement de bout en bout : il faut un sender qui change
  de taille en direct (TD fait l'affaire).
- VLC : ses propres limites de décodage matériel, lues seulement via ffmpeg.
- C8 sur téléphone : lecture du code seulement, pas de téléphone branché ; la
  version de l'APK installée n'est pas connue.

## 6. Partie 1 — formats Spout

*Tour 2, 2026-09-28. Fork `txproto` `1975f44` (`kyberfrog-dev`), épinglé
dans KyberFrog au pin `kyber-desktop` `4c5099f`.*

**Choix d'architecture.** La conversion de format vit à la frontière de
capture, avec un contrat simple : **une source D3D11 livre toujours du BGRA
8 bits**, comme la capture d'écran DXGI. La taille (C2/C3) et la
reconstruction à chaud (C4) iront dans le pipeline, dans les parties
suivantes. Mettre dès maintenant un `scale_d3d11` dans le pipeline aurait
ajouté un filtre sur le chemin AMF direct, que C4 aurait figé au premier
changement de taille : ce serait une régression.

**Ce qui change** (`txproto` `src/iosys_spout.c`) :

- BGRA 8 bits : copie directe, inchangée ;
- tout autre format lisible par un shader (RGBA 8 bits, BGRX, sRGB, 10 bits,
  11-11-10, 16/32 bits flottant ou entier, deux canaux, mono) : la texture
  du sender est copiée telle quelle sous son mutex, puis dessinée dans le
  pool BGRA par un pixel shader, pixel pour pixel, bornée à 0..1 ; un canal
  seul est montré en gris. Le dessin se fait sous `ID3D10Multithread`,
  parce que l'encodeur partage le device ;
- les shaders sont compilés une fois par `d3dcompiler_47.dll`, chargée à
  l'exécution (présente depuis Windows 10) ; aucune dépendance de build ;
- un format toujours refusé (entier, profondeur, multi-échantillonné) n'est
  plus loggé qu'une fois par texture.

**Banc de test.** `bench/format_check.py` (voir le [README du banc](https://gitlab.com/kyber-frog/kyberfrog/-/blob/dev/bench/README.md)) :
`kybench pattern` publie quatre barres de couleur dans le format voulu, la
chaîne complète (kycontroller épinglé → kyclient `--spout-out`) les
transporte, et `kybench check` compare le centre de chaque barre à la valeur
attendue. Résultats bruts dans `bench/runs/2026-09-28-formats-*`.

**Résultats** (1280×720, PC de dev, RX 7800 XT). « Avant » = bundle 0.7.0
installé (pin `3455934`) ; « après » = le même, avec `kyavserver.exe` et
`libtxproto-0.dll` reconstruits sur la branche.

| Format du sender | DXGI | Avant (AMF) | Après (AMF) | Après (x264) |
|---|---|---|---|---|
| BGRA 8 bits | 87 | image, couleurs exactes | idem | idem (décalage C9) |
| BGRA 8 bits sRGB | 91 | — | image | image |
| **RGBA 8 bits** | 28 | **aucune image** (C1b) | image | image |
| RGBA 8 bits sRGB | 29 | — | image | image |
| BGRX 8 bits | 88 | — | image | image |
| **RGB 10 bits** (Unreal) | 24 | **aucune image** | image | image |
| **RGBA 16 bits flottant** (TD) | 10 | **aucune image** | image | image |
| RGBA 16 bits entier | 11 | — | image | image |
| RGBA 32 bits flottant (TD) | 2 | — | image | image |
| **Mono 8 bits** | 61 | **aucune image** | image, en gris | image, en gris |
| Mono 16 bits, 16 f, 32 f | 56, 54, 41 | — | image, en gris | image, en gris |

« Image » = 60 images/s en sortie, et chaque barre à ±3/255 de la valeur
attendue en AMF. En x264, toutes les barres saturées ont le même écart que la
référence BGRA : c'est C9, pas la conversion. « — » = format non mesuré avant
(rejeté par la même table que ses voisins).

**Pas couvert par la partie 1** : C2 à C5 (taille, reconstruction à chaud,
remontée des erreurs), un sender sur un autre GPU, et les valeurs HDR au-delà
de 1,0 (écrêtées, ce que montrerait de toute façon un récepteur 8 bits).

### C9 — En x264, les couleurs saturées sont décalées (trouvé au tour 2)

**Constaté par le banc, sur le bundle installé.** Le même BGRA 8 bits ressort
exact en AMF (écart ≤ 3/255), mais décalé en x264 : vert pur reçu à
`(0, 216, 0)`, rouge pur à `(255, 24, 0)`, bleu pur à `(0, 14, 255)`. Le gris
est juste. C'est la signature d'une matrice couleur différente à l'encodage
(BT.601, défaut de swscale pour une image sans étiquette) et au décodage
(BT.709, défaut pour la HD) — **déduit**, à confirmer en lisant les
étiquettes du flux. Concerne tout transmetteur en x264, donc aussi le repli
automatique de C2. Hors périmètre de la partie 1.

## Annexe — reproduire les mesures

Avec le ffmpeg du bundle (`<install>\ffmpeg.exe`), depuis Git Bash :

```sh
FF=ffmpeg.exe; H="-init_hw_device d3d11va=d3d -filter_hw_device d3d"
# chemin Spout/écran → AMF
$FF $H -f lavfi -i testsrc2=s=5760x1080:r=60 -vf format=bgra,hwupload \
    -c:v h264_amf -usage ultralowlatency -bf 0 -frames:v 10 -f null -
# normalisation GPU puis AMF
$FF $H -f lavfi -i testsrc2=s=5760x1080:r=60 \
    -vf format=bgra,hwupload,scale_d3d11=width=3840:height=720:format=nv12 \
    -c:v h264_amf -frames:v 10 -f null -
```
