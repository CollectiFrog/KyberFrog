# Latence bout-en-bout — analyse et leviers (#28)

> Spec extraite d'`IMPROVEMENTS.md` le 2026-08-25, **sans une ligne de
> réécriture technique** : le contenu est celui de l'analyse du 2026-07-17. L'état
> d'avancement de l'item ne vit plus ici mais dans le board
> (`docs/dev/backlog.md`) — ce document dit le *pourquoi* et le *comment*.

- **What:** l'analyse de la chaîne complète (capture → encode → QUIC → décode →
  Spout → hôte) et les leviers identifiés pour la réduire. **La latence est la
  priorité n°1 du projet** : cet item existe pour que les arbitrages d'archi
  (dont #27) s'y réfèrent au lieu de re-dériver la chaîne à chaque fois.
- **Why deferred:** travail **côté fork** (kymedia / kyctl), traité par
  l'opérateur ; KyberFrog consomme le bundle de binaires tel quel et ne linke
  aucun crate du fork (voir #24). Rien ici ne bloque #27.
- **How — leviers, par ordre de gain estimé :**
  1. **Émission — le plus gros levier, probablement.** L'encodeur par défaut est
     **x264 (CPU)** *uniquement* parce qu'AMF crashe en boucle silencieuse sur
     la RX 7800 XT (`gen.rs:62-63`). Un encodeur CPU impose un download
     GPU→CPU **plus** une latence d'encodage bien supérieure au coût du chemin
     Spout — c'est-à-dire que le défaut actuel coûte probablement plus cher que
     tout le reste réuni. Pistes : reprendre AMF/NVENC avec `zerolatency` (le
     patch FFmpeg `0001-nvenc-Patch-SPS-when-zerolatency-is-enabled.patch` est
     **déjà dans l'arbre**, `kymedia/subprojects/ffmpeg.wrap`), `intra_refresh`.
  2. **Réception — un aller-retour CPU évitable.** `setup_spout_output`
     (`kyctl/kyvlcplayer/src/player.rs:189-259`) installe des callbacks libVLC
     `smem` demandant du **BGRA CPU**, puis `kyspout` ré-uploade chaque frame
     dans la texture D3D11 partagée. Soit : décodage HW (**GPU**) → **CPU** →
     **GPU**. Cible : output callbacks **D3D11** de libVLC 4 → `SpoutSender`
     prend la texture directement. **Déjà anticipé par l'auteur de kyspout**
     (`kyctl/kyspout/src/lib.rs:37-40` : « *A later optimization can take a GPU
     texture directly (zero-copy)* »). Noter que le coût est **dans kyclient, pas
     dans Spout** — Spout est une copie GPU→GPU, négligeable. Ce correctif
     bénéficie à Resolume **et** TouchDesigner **sans aucun plugin**.
     **✅ LIVRÉ le 2026-07-18 — validé E2E hardware (Resolume/TouchDesigner).**
     Le blocage « nécessite libVLC 4 » était périmé : le VLC vendoré
     (`kymedia/subprojects/vlc`) est déjà `4.0.0-dev` et expose
     `libvlc_video_set_output_callbacks` (engine D3D11) — ce n'était donc pas
     une migration VLC mais le câblage vlc-rs + kyspout + kyvlcplayer. libVLC
     rend désormais **directement dans la texture Spout partagée** (GPU→GPU) ;
     le zero-copy est le **défaut** sur Windows (`KYSPOUT_SMEM=1` restaure le
     chemin CPU). Détail, bugs rencontrés et warnings attendus :
     [`docs/dev/plan-spout-zerocopy.md`](plan-spout-zerocopy.md).
  3. **Protocole.** `multi_client=false` → session unique : *direct forward,
     lowest latency, full backpressure* vers l'encodeur (vs le duplicateur
     `kydup` du mode multi-client, qui est le défaut). Protocoles vidéo
     `Unreliable` / `UnreliableFec` (`kyclient/src/capi.rs:104-109`).
     ⚠️ Tension avec #27 : en session unique, un 2e client sur le même flux
     reçoit un **409 Conflict**.
  4. **Instrumenter avant d'optimiser.** L'API C de kyclient expose **déjà** les
     timestamps par frame `FrameAcquired → FrameDisplayed`
     (`kyclient/src/capi.rs:1639`) : chiffrer chaque étape avant d'engager le
     travail de fork, pour cibler le vrai coupable plutôt que le supposé. Mesure
     de terrain simple, sans code : dans TouchDesigner, comparer un
     `Spout In TOP` sur le flux Kyber vs un `Spout In TOP` **direct** sur le
     sender source — l'écart *est* le coût de la chaîne Kyber.
  5. **Note transverse (à vérifier).** `Source::All` (« Tout envoyer ») met
     `all_sources = true` côté kyavserver → expose *tous* les senders Spout de la
     machine, **y compris les `Kyber - *`** créés par un viewer local. Le
     filtrage anti-boucle de #27 est côté KyberFrog ; en mode « Tout envoyer »
     l'énumération est faite par le fork, donc **hors de portée** de KyberFrog.

