# Latence bout-en-bout — chaîne et leviers (#28)

**La latence est la priorité n°1 du projet.** Ce document décrit la chaîne
complète — capture → encodage → QUIC → décodage → Spout → hôte — et les leviers
pour la réduire, afin que les arbitrages d'architecture s'y réfèrent. L'état de
chaque levier est sur le [board](backlog.md).

Tous les leviers sont **côté fork** (kymedia / kyctl) : KyberFrog consomme le
bundle de binaires tel quel.

## Leviers, par gain estimé

### 1. Émission — encodeur GPU *(#28-1)*

En place depuis 0.6.0. Réglage machine `encoder` (Options → Encodage vidéo) :
`auto` = AMF / NVENC selon le fabricant de l'adaptateur DXGI 0, x264 sinon,
avec repli automatique sur x264 si l'encodeur GPU échoue ; l'`encoder` hérité
d'un setup est ignoré
(`shared/src/encoder.rs`, `kyberfrog/src/gpu.rs`). Kyavservice branche les
frames D3D11 de la capture directement sur `h264_amf` : ni download GPU→CPU ni
conversion.

Banc de latence, source Spout 1080p60, 20 Mbps, boucle locale : **AMF 4,0 ms
p50 (10,9 ms p99) Spout → Spout, contre 25,8 ms en x264** (encodage 19,9 ms,
77 % du total ; x264 est bridé à 2 threads, `txproto/src/encode.c:89`, mais 6
threads ne gagnent que 0,8 ms). Repère : NDI → NDI sur le même poste, 15 à
26 ms. Détail : [bench-latency.md](bench-latency.md).

Le crash AMF « en boucle silencieuse » qui justifiait x264 par défaut n'est plus
reproduit (FFmpeg 8.1, pilote AMD 32.0.31041.1004, 10 min au banc).

**Pour** : ~22 ms de moins, et le CPU libéré. **Contre** : qualité visuelle à
20 Mbps, plusieurs émetteurs simultanés et source écran pas encore contrôlés
(file de validation du backlog, `#28-1 check`) ; NVENC jamais testé, couvert
par le repli x264 ; les sources de capture (webcams, boîtiers) retombent
toujours sur x264, faute de conversion NV12 devant AMF / NVENC (#48-A). Levier restant : `zerolatency` / `intra_refresh` (le patch
FFmpeg `0001-nvenc-Patch-SPS-when-zerolatency-is-enabled.patch` est déjà dans
`kymedia/subprojects/ffmpeg.wrap`).

### 2. Réception — sortie Spout zero-copy *(#28-2, en place)*

libVLC rend directement dans la texture Spout partagée, GPU→GPU, sans
aller-retour CPU ; c'est le défaut sous Windows. Architecture :
[plan-spout-zerocopy.md](plan-spout-zerocopy.md).

### 3. Protocole — session unique *(#28-3)*

`multi_client=false` → session unique : *direct forward, lowest latency, full
backpressure* vers l'encodeur, au lieu du duplicateur `kydup` du mode
multi-client (le défaut). Protocoles vidéo `Unreliable` / `UnreliableFec`
(`kyclient/src/capi.rs:104-109`).

**Contre** : en session unique, un 2ᵉ client sur le même flux reçoit un
**409 Conflict**, ce qui entre en tension avec le passthrough Spout (#27).

### 4. Mesurer avant d'optimiser *(#28-4)*

L'API C de kyclient expose déjà les timestamps par frame
`FrameAcquired → FrameDisplayed` (`kyclient/src/capi.rs:1639`) : chiffrer
chaque étape cible le vrai coupable plutôt que le supposé.

Mesure de terrain, sans code : dans TouchDesigner, un `Spout In TOP` sur le flux
Kyber à côté d'un `Spout In TOP` **direct** sur le sender source — l'écart *est*
le coût de la chaîne Kyber.

## Note transverse — « Tout envoyer » et les boucles

`Source::All` met `all_sources = true` côté kyavserver, qui expose *tous* les
senders Spout de la machine, **y compris les `KyberFrog-*`** créés par un
viewer local en redirection Spout. L'énumération est faite par le fork : le filtrage anti-boucle de
KyberFrog (#27) ne s'applique pas à ce mode.

## Pistes relevées par le banc (2026-09-13)

Relevées pendant l'étude du banc de mesure
([bench-latency.md](bench-latency.md)), en lisant la chaîne et lors
d'un smoke test (source écran 2560 × 1440, x264, boucle locale — chiffres
**indicatifs**). Aucune n'est engagée : Phase A mesure la config livrée. Chacune
se teste en A/B sur le banc validé (Phase A+), **mesurer avant d'optimiser**
(levier 4).

| # | Constat | Piste | Où | Coût | Gain attendu |
|---|---|---|---|---|---|
| B1 | **x264 = 72 % de la médiane** (`encoding → encoded` p50 21,2 ms / 29,5 ms) | confirme le levier 1 (AMF/NVENC `zerolatency`) comme priorité, chiffres à l'appui | kymedia + config | voir levier 1 | le plus gros, et de loin |
| B2 | Le commentaire de `create_x264` affirme que `x264-params=force-cfr=0` « ajoute 1 à 2 frames de latence » (contournement AWS) ; or `force-cfr` est déjà désactivé par défaut dans x264, l'effet réel est douteux | A/B **sans code** : `[kyavserver.video_encoder_config]` remplace tout le dictionnaire (y remettre `preset`, `tune`, `b`, `flags=+global_header`) | `kyber_config.toml` | nul | 0 à 33 ms — à mesurer, pas à supposer |
| B3 | `hwdownload,format=bgra,format=nv12` : téléchargement BGRA 32 bpp puis conversion **CPU** (`acquired → encoding` p50 4,1 ms) | convertir en NV12 **sur le GPU** avant le téléchargement (filtre D3D11 de FFmpeg 8.1, à vérifier) : ~2,7× moins d'octets et plus de conversion CPU | `kyavservice/src/video.rs` `create_x264` (chaîne de filtre) | faible, fork | quelques ms (disparaît de toute façon avec B1) |
| B4 | QUIC compte **~30 paquets perdus/s en boucle locale** côté serveur (0 côté client) ; queue p99 portée par `sent → received` et `received → decoding`. **Étape 4 du banc** (Spout 1080p60, 20 Mbps) : ~70 pertes/s, régulières, **non corrélées** à la queue (\|r\| ≤ 0,35 sur 60 fenêtres de 5 s) ; la queue vient des grosses frames (coupure de contenu : `sent → received` 8 à 11 ms au lieu de 0,25) — coût lié à la taille, cause des pertes toujours inconnue | explorer buffers UDP (`SO_RCVBUF`/`SO_SNDBUF`) et config quinn de kyproto ; en `Reliable`, chaque perte = retransmission | kymux/kyproto (upstream non forké) via kyctl | moyen | queue p99 (80 ms au smoke test) |
| B5 | **Backlog de démarrage** : les ~6 premières secondes sortent avec 4 à 6 s de retard (frames encodées avant que le client soit prêt, puis rattrapage) | jeter plutôt que mettre en file tant que le client n'est pas prêt, ou vider à l'abonnement | txproto `packet_sink` (FIFO 60), kydup, kycontroller | moyen, fork | UX : image à jour dès la connexion d'un viewer |
| B6 | Scrutation Spout `av_usleep(500)` = `Sleep(0)` sous MinGW (déduit) : attente active, un cœur par émetteur Spout. **Constaté à l'étape 4 du banc** : en source Spout un thread de kyavserver tient **un cœur** à 60 fps (0,94–0,98) comme à 1 fps (0,999) ; en source écran aucun thread ne dépasse 0,04 cœur | attendre sur l'événement de synchro du SDK Spout plutôt que scruter (existence à vérifier), ou `Sleep(1)` + `timeBeginPeriod(1)` | txproto `iosys_spout.c:833-844` | faible, fork | CPU ; latence de détection inchangée ou meilleure |
| B7 | kydup (mode Multi) est dans le chemin même avec un seul viewer ; **tout** `sent → received` ne fait que 0,24 ms en médiane en boucle locale | `multi_client=false` (levier 3, #28-3) : gain médian probablement négligeable en local — **dépriorisé par la mesure**, à re-mesurer en Phase B | config | nul | faible en local |
| B8 | L'installeur **v0.5.0** déployé n'a **pas** le zero-copy Spout (le levier 2 « livré » n'est que dans le bundle de test) | publier une release dont le bundle contient `kyctl` ≥ `b29f8f4`, et ajouter au CI une vérification (chaîne `D3D11 zero-copy` dans `kyclient.dll`) | packaging / CI | faible | levier 2 enfin chez les utilisateurs |
| B9 | KyberFrog ne sait pas passer `--metrics` ; tous les kyclient partagent un même fichier de log | un interrupteur « diagnostic latence » par viewer (flag + `metrics.json` par viewer + p50 dans l'UI) | `shared/src/config.rs` `kyclient_args`, UI | faible, 💻 | observabilité terrain sans banc |
| B10 | Bus de messages kyclient à 32 places, `try_send` : métriques **et** messages de contrôle jetés si plein (0 perte au smoke test) | canal dédié aux métriques | `kyclient/src/message_bus.rs` | faible, fork | robustesse, pas latence |
| B11 | **Compteur de frames Spout lu de façon destructive** : `read_frame_count` fait `WaitForSingleObject(0)` puis `ReleaseSemaphore` (motif du SDK Spout), et traite tout `count != last` comme une nouvelle frame. Or le motif n'est pas atomique : un **autre** lecteur du même sender (récepteur SDK, ou sonde du banc avant correction) abaisse le compteur de 1 pendant quelques µs, et l'émetteur SDK lui-même passe par `n − 1` avant `n + 1` (`SetNewFrame` : wait puis release de 2). **Constaté à l'étape 2 du banc** : avec une sonde en scrutation sur le même sender, kyavserver a capturé **~900 « frames »/s** fantômes (copies de la même image), sorti 259 fps et journalisé `Dropping frame` en rafale. Un VJ qui prévisualise le sender dans Resolume/TouchDesigner **en même temps** que Kyber le capture est dans ce cas | lire le compteur sans le toucher (`NtQuerySemaphore`, `SemaphoreBasicInformation`) **et** n'accepter que `count > last` ; ce que fait `kybench` depuis l'étape 2 | txproto `iosys_spout.c` `read_frame_count` | faible, fork | robustesse (frames dupliquées, débit x264 gaspillé) ; latence inchangée |
| B12 | **La sortie Spout de kyclient perd une frame à chaque à-coup** : quand une frame arrive en retard (ici la coupure de fond du générateur, toutes les 128 frames : +5 ms d'encodage x264, publication ~17 ms en retard), la frame suivante est déjà décodée ; la texture partagée contient **déjà n + 1** quand le compteur de n est signalé. Un récepteur Spout ne voit jamais n et voit n + 1 deux fois. **Constaté à l'étape 3 du banc** : 1 frame sur 128 en sortie (0,74 %, 59,55 IDs distincts/s pour 60,0 publications/s), sur 36 000 frames à 20 Mbps ; persiste quand la sonde attend l'exécution GPU de sa copie **sous** le mutex Spout (artefact de sonde exclu). **Précisé à l'étape 4** (399 cas, métriques jointes aux IDs à la µs) : `prepared(n)` finit 0,10 ms avant `decoded(n + 1)`, puis `displayed(n)` tombe 0,003 ms **après** `decoded(n + 1)` et la texture montre n + 1 — l'affichage de n est bloqué par le décodage de n + 1 (qui dure alors 1,6 à 5,7 ms au lieu de 0,16), un verrou partagé décodeur / sortie Spout est probable mais non identifié. La note précédente (« `displayed` de n avant `decoded` de n + 1 ») venait de valeurs arrondies à 0,1 ms |
| B13 | **Boucle d'accept à vide au logout d'un viewer** : `run_task` garde 4 tâches `server.accept()` et les relance dès qu'elles rendent la main ; une fois l'endpoint fermé, chaque `accept` échoue aussitôt (`Kynet accept failed: connection error: Endpoint closed`) jusqu'à l'arrêt de la tâche. **Constaté à l'étape 4 du banc** : ~6 000 lignes WARN en 60 ms à chaque déconnexion propre de kyclient (~800 Ko de log par session) ; invisible tant que les clients étaient tués sans logout | traiter l'erreur « endpoint fermé » comme terminale (sortir de la boucle) ou attendre avant de relancer | kyctl `kycontroller/src/kymux_controller/server.rs` `run_task` / `handle_connection` (identique à `b29f8f4`) | faible, fork | robustesse (logs, CPU bref au logout) ; latence inchangée | localiser dans `video_output.c` / le rendu D3D11 quelle image est réellement dessinée dans la cible au `prepare` ; puis soit publier chaque image rendue, soit sauter proprement n (sans doublon) | kyvlcplayer `setup_spout_output` (callbacks `make_current`/`swap`), VLC `video_output.c` | moyen, fork (VLC upstream non forké) | fluidité en sortie sur contenu à coupures (clips VJ) ; et le critère de perte du banc (§ 6.3) |
