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
