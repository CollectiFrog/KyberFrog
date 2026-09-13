# Latence bout-en-bout — chaîne et leviers (#28)

**La latence est la priorité n°1 du projet.** Ce document décrit la chaîne
complète — capture → encodage → QUIC → décodage → Spout → hôte — et les leviers
pour la réduire, afin que les arbitrages d'architecture s'y réfèrent. L'état de
chaque levier est sur le [board](backlog.md).

Tous les leviers sont **côté fork** (kymedia / kyctl) : KyberFrog consomme le
bundle de binaires tel quel.

## Leviers, par gain estimé

### 1. Émission — encodeur GPU *(#28-1)*

L'encodeur par défaut est **x264 (CPU)**, parce qu'AMF crashe en boucle
silencieuse sur la RX 7800 XT (`gen.rs:62-63`). Un encodeur CPU impose un
download GPU→CPU **plus** une latence d'encodage bien supérieure au chemin
Spout : c'est probablement le poste le plus coûteux de la chaîne.

Cible : AMF / NVENC avec `zerolatency` et `intra_refresh`. Le patch FFmpeg
`0001-nvenc-Patch-SPS-when-zerolatency-is-enabled.patch` est déjà dans l'arbre
(`kymedia/subprojects/ffmpeg.wrap`).

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
([plan-bench-latency.md](plan-bench-latency.md)), en lisant la chaîne et lors
d'un smoke test (source écran 2560 × 1440, x264, boucle locale — chiffres
**indicatifs**). Aucune n'est engagée : Phase A mesure la config livrée. Chacune
se teste en A/B sur le banc validé (Phase A+), **mesurer avant d'optimiser**
(levier 4).

| # | Constat | Piste | Où | Coût | Gain attendu |
|---|---|---|---|---|---|
| B1 | **x264 = 72 % de la médiane** (`encoding → encoded` p50 21,2 ms / 29,5 ms) | confirme le levier 1 (AMF/NVENC `zerolatency`) comme priorité, chiffres à l'appui | kymedia + config | voir levier 1 | le plus gros, et de loin |
| B2 | Le commentaire de `create_x264` affirme que `x264-params=force-cfr=0` « ajoute 1 à 2 frames de latence » (contournement AWS) ; or `force-cfr` est déjà désactivé par défaut dans x264, l'effet réel est douteux | A/B **sans code** : `[kyavserver.video_encoder_config]` remplace tout le dictionnaire (y remettre `preset`, `tune`, `b`, `flags=+global_header`) | `kyber_config.toml` | nul | 0 à 33 ms — à mesurer, pas à supposer |
| B3 | `hwdownload,format=bgra,format=nv12` : téléchargement BGRA 32 bpp puis conversion **CPU** (`acquired → encoding` p50 4,1 ms) | convertir en NV12 **sur le GPU** avant le téléchargement (filtre D3D11 de FFmpeg 8.1, à vérifier) : ~2,7× moins d'octets et plus de conversion CPU | `kyavservice/src/video.rs` `create_x264` (chaîne de filtre) | faible, fork | quelques ms (disparaît de toute façon avec B1) |
| B4 | QUIC compte **~30 paquets perdus/s en boucle locale** côté serveur (0 côté client) ; queue p99 portée par `sent → received` et `received → decoding` | explorer buffers UDP (`SO_RCVBUF`/`SO_SNDBUF`) et config quinn de kyproto ; en `Reliable`, chaque perte = retransmission | kymux/kyproto (upstream non forké) via kyctl | moyen | queue p99 (80 ms au smoke test) |
| B5 | **Backlog de démarrage** : les ~6 premières secondes sortent avec 4 à 6 s de retard (frames encodées avant que le client soit prêt, puis rattrapage) | jeter plutôt que mettre en file tant que le client n'est pas prêt, ou vider à l'abonnement | txproto `packet_sink` (FIFO 60), kydup, kycontroller | moyen, fork | UX : image à jour dès la connexion d'un viewer |
| B6 | Scrutation Spout `av_usleep(500)` = `Sleep(0)` sous MinGW (déduit) : attente active possible, un cœur par émetteur Spout | attendre sur l'événement de synchro du SDK Spout plutôt que scruter (existence à vérifier), ou `Sleep(1)` + `timeBeginPeriod(1)` | txproto `iosys_spout.c:833-844` | faible, fork | CPU ; latence de détection inchangée ou meilleure |
| B7 | kydup (mode Multi) est dans le chemin même avec un seul viewer ; **tout** `sent → received` ne fait que 0,24 ms en médiane en boucle locale | `multi_client=false` (levier 3, #28-3) : gain médian probablement négligeable en local — **dépriorisé par la mesure**, à re-mesurer en Phase B | config | nul | faible en local |
| B8 | L'installeur **v0.5.0** déployé n'a **pas** le zero-copy Spout (le levier 2 « livré » n'est que dans le bundle de test) | publier une release dont le bundle contient `kyctl` ≥ `b29f8f4`, et ajouter au CI une vérification (chaîne `D3D11 zero-copy` dans `kyclient.dll`) | packaging / CI | faible | levier 2 enfin chez les utilisateurs |
| B9 | KyberFrog ne sait pas passer `--metrics` ; tous les kyclient partagent un même fichier de log | un interrupteur « diagnostic latence » par viewer (flag + `metrics.json` par viewer + p50 dans l'UI) | `shared/src/config.rs` `kyclient_args`, UI | faible, 💻 | observabilité terrain sans banc |
| B10 | Bus de messages kyclient à 32 places, `try_send` : métriques **et** messages de contrôle jetés si plein (0 perte au smoke test) | canal dédié aux métriques | `kyclient/src/message_bus.rs` | faible, fork | robustesse, pas latence |
