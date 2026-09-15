# Banc de mesure de latence KyberFrog vs NDI — Phase A (#28-4)

*Étude du 2026-09-13. Aucune ligne de code écrite dans la chaîne : ce document
est le plan, la cartographie qui le justifie, la spec des instruments, et le
résultat d'un smoke test exécuté le jour même (§ 0). Il complète
[plan-latency.md](plan-latency.md) (les leviers) par la question préalable :
**comment mesurer sans se tromper**.*

> **Branche.** `feat/bench-latency-phase-a` est empilée sur
> `feat/backlog-reorg` (qui porte `plan-latency.md`, absent de `dev`) et encore
> en développement. **Ne pas merger** : rebaser sur `feat/backlog-reorg` quand
> celle-ci bouge, puis sur `dev` une fois qu'elle y est.

## TL;DR

1. **La question bloquante a une réponse positive — lue dans le code, puis
   constatée à l'exécution.** La sortie Spout de kyclient est câblée de bout en
   bout (§ 2.3) ; le 2026-09-13 un `kyclient --spout-out` a publié son sender à
   **59,5 fps** en zero-copy D3D11 (§ 0). Pas de fallback capture d'écran : le
   banc mesure **Spout → Spout**, sur la même frontière pour KyberFrog et NDI.
2. **L'instrumentation interne existe déjà et elle est riche** : dix horodatages
   par frame, clés par PTS, de la capture Spout jusqu'à la publication Spout de
   sortie (§ 3). Le binaire `kyclient --metrics true` les écrit dans
   `metrics.json` — **97,4 % de frames complètes, aucune métrique perdue**
   pendant le smoke test. KyberFrog ne passe pas le flag.
3. **Sur une seule machine, tout le monde parle le même temps** — QPC en
   microsecondes côté kyproto, VLC et txproto, **confirmé** par la config du
   build FFmpeg et par un offset clock-sync mesuré ≤ 19 µs (§ 0, § 3.3).
4. **Phase A ne demande quasiment aucun code dans la chaîne de forks.** Le
   travail est un instrument externe (`kybench`, Rust) + un orchestrateur et une
   analyse (Python). La seule instrumentation fork envisagée est
   **conditionnelle** (étape 10).
5. **Trois pièges trouvés dans le code**, qui fausseraient une mesure naïve :
   métriques jetées silencieusement quand un canal est plein (§ 3.4), métriques
   serveur réservées au *premier* client (§ 3.4), et `received` qui ne date pas
   l'arrivée QUIC mais la lecture du socket local par VLC (§ 3.2).
6. **Le smoke test a ajouté trois constats** : l'installeur v0.5.0 déployé sur
   le poste **n'embarque pas** le zero-copy (seul le bundle `KyberFrog-test` l'a),
   QUIC signale **~30 paquets perdus/s en boucle locale** côté serveur, et les
   ~6 premières secondes d'une session sortent avec **4 à 6 s de retard**
   (backlog de démarrage) — § 0.

---

## 0. Vérifications exécutées (2026-09-13)

Faites dans la foulée de l'étude, parce qu'elles transformaient les deux
affirmations dont tout le plan dépend (sortie Spout, domaine d'horloge) en
constats, pour quelques minutes et zéro code.

### 0.1 Ce qui a été lancé

Instance isolée, sans toucher à la config de l'opérateur : `kycontroller` du
bundle `C:\Users\trist\KyberFrog-test` sur le port **9150** avec un
`kyber_config.toml` de scratch (x264, auth transparente, source **écran**
2560 × 1440 — aucun sender Spout source n'était actif), puis

```
kyclient --port 9150 --tls-skip-verification --auth-username vj --auth-password kyberfrog \
         --spout-out kybench-smoke --inputs false --audio false --keyboard-grab false \
         --metrics true 127.0.0.1
```

~2 min 30 de flux, puis arrêt ; seuls les processus de l'opérateur restaient.
Révisions lues dans les logs : kyclient `0.27.1-8-gd2c418d`, kycontroller
`0.27.0-6-ge303633`.

### 0.2 Résultats

| Vérification | Résultat | Statut |
|---|---|---|
| Sender Spout de sortie enregistré | `kybench-smoke` présent dans `SpoutSenderNames` | **constaté** |
| Chemin de sortie | log : `Spout output enabled (D3D11 zero-copy)`, décodage `D3D11VA` sur la RX 7800 XT | **constaté** |
| Cadence publiée | sémaphore compteur : **59,5 frames/s** | **constaté** |
| Domaine d'horloge | FFmpeg 8.1 compilé `HAVE_CLOCK_GETTIME 0` → `av_gettime_relative_win32()` = QPC µs (même formule que kyproto) ; offset clock-sync mesuré entre −19 et +19 µs sur 17 échantillons ; **0 segment négatif** sur 9 141 frames | **constaté** |
| Complétude des métriques | 9 141 frames complètes / 9 387 acquises (**97,4 %**) ; 0 `Channel full`, 0 `Metric discarded`, 0 `Failed to send Metrics` | **constaté** (à 2560 × 1440 / 60 fps) |
| Doublons de PTS | aucun | **constaté** |

Décomposition en **régime établi** (après 10 s, 8 790 frames), source écran
2560 × 1440, x264 — **indicatif, pas un résultat Phase A** (ni Spout, ni 1080p) :

| Segment | p50 ms | p95 ms | p99 ms | max ms |
|---|---:|---:|---:|---:|
| acquired → encoding (hwdownload + conversion) | 4,13 | 5,82 | 7,03 | 14,7 |
| encoding → encoded (**x264**) | **21,16** | 25,22 | 31,80 | 43,3 |
| encoded → sent | 0,05 | 0,11 | 0,17 | 2,4 |
| sent → received (kycom + kydup + QUIC + kycom) | 0,24 | 0,52 | 9,07 | 44,7 |
| received → decoding | 0,02 | 0,03 | 20,20 | 39,7 |
| decoding → decoded | 0,19 | 0,43 | 3,61 | 33,5 |
| decoded → prepared (rendu dans la texture Spout) | 3,66 | 3,90 | 4,45 | 38,3 |
| prepared → displayed | 0,02 | 0,04 | 0,11 | 1,2 |
| **acquired → displayed** | **29,46** | **36,51** | **80,06** | 122,4 |

### 0.3 Ce que ça change

- **L'encodeur fait 72 % de la médiane.** Le transport loopback est négligeable
  en médiane (0,24 ms) : l'étape 10 (découpe du transport) a peu de chances de
  se déclencher sur la médiane — mais voir la queue ci-dessous.
- **La queue (p99 80 ms) vient du transport et de l'entrée VLC**, pas de
  l'encodeur. En parallèle, `network_remote.packets_lost` (vu du serveur) monte
  **régulièrement de ~30/s** (4 939 en 160 s) en boucle locale, alors que le client en voit 0.
  Hypothèse (déduite, non vérifiée) : débordement du buffer de réception UDP
  côté kyclient ou détection de perte parasite de quinn ; en mode `Reliable`,
  chaque perte coûte une retransmission. **Ajout au plan** : corréler pertes et
  pics de `sent → received` à l'étape 4.
- **Backlog de démarrage** : sur les 10 premières secondes, `acquired →
  displayed` atteint p95 6,1 s (frames encodées avant que le client soit prêt,
  `Dropping filtered frame` dans le log kyavserver). Le préchauffage de 60 s du
  protocole est justifié par la mesure ; c'est aussi une piste produit
  ([plan-latency.md](plan-latency.md), pistes du banc).
- **Bundle à mesurer** : l'installeur **v0.5.0 installé dans
  `C:\Program Files\KyberFrog` n'a pas le zero-copy** (chaîne `D3D11 zero-copy`
  absente de son `kyclient.dll`, binaires du 09/07) ; `KyberFrog-test` l'a
  (`kyclient.dll` du 18/07) mais c'est un **bundle mixte** (kycontroller 09/07,
  kyavserver 15/07). Le contenu de l'installeur v0.5.1 n'est pas vérifié. →
  étape 0 : construire un bundle cohérent aux SHAs `kyberfrog-dev`.
- **Logs kyclient partagés** : tous les kyclient écrivent dans
  `%LOCALAPPDATA%\Kyber\log\kyclient.log` (sans PID dans les lignes), et
  ignorent `KYBER_LOG_DIR`. Le viewer KyberFrog de l'opérateur y écrivait en
  même temps (reconnexion toutes les 15 s vers `192.168.1.15:9000`). →
  condition figée : **KyberFrog arrêté pendant les runs**.
- **Piège CLI** : `--metrics` attend `true`/`false`, le serveur reste
  positionnel en dernier.

---

## 1. Cartographie des branches (révisions lues)

Le code a été lu sur les **têtes d'intégration**, pas sur les worktrees locaux
(plusieurs sont désynchronisés de leur gitlink — `kyctl` est checkouté sur une
vieille `dev`). Snapshot extrait par `git archive` :

| Repo | Révision lue | Branche vivante |
|---|---|---|
| kyber-desktop | `origin/kyberfrog-dev` (2026-08-17) | `kyberfrog-dev` |
| kysdk | `6a92a38` | `kyberfrog-dev` |
| kyctl | `b29f8f4` (zero-copy Spout par défaut) | `kyberfrog-dev` |
| kymedia | `7f0360f` | `kyberfrog-dev` |
| txproto | `12f1e1f` | `kyberfrog-dev` |
| vlc-rs | `bfb68fc` | `kyberfrog-dev` |
| vlc (upstream kyber.stream, non forké) | `dd2db54` — lu dans le checkout de WS1, même SHA | — |
| kymux / kyproto (upstream, non forké) | `831d312` | — |
| kyberfrog | `origin/dev` `2e13d5f` | `dev` |

**Aucune branche ne touche à l'instrumentation ou à la latence.** Écarts
notables (`behind/ahead` contre la branche vivante) :

| Branche | Écart | Ce qui compte pour le banc |
|---|---|---|
| kyctl `KF-cast/player-external` | 66 / 11 | pré-rebase 0.27.1 → **cherry-pick only**. Porte `7e0fb8e` « don't panic when a host offers no metrics endpoints » : le `unwrap()` existe toujours sur `kyberfrog-dev` (`kyclient/src/kymux_backend/mod.rs:345-349`). **Sans effet en Phase A** (kycontroller offre les deux endpoints dès que `metrics: true`), mais à rapatrier si un émetteur tiers (kyberfrog-cast) est un jour mesuré. Porte aussi `d1a3cca` (kycontroller en bibliothèque) et `12b9ce0` (backend AV in-process) — voir § 2.4. |
| kyctl `feat/spout-zerocopy` | 3 / 0 | contenu inclus dans `kyberfrog-dev`, supprimable |
| kyber-desktop `fix/*-0.27` | 12 / 1 | bugfixes d'input, hors sujet |
| kyberfrog `feat/linux-support` | 1 / 0 | déjà mergée dans `dev` |
| kyberfrog `feat/backlog-reorg` | 0 / 13 | **base de cette branche d'étude** (porte `plan-latency.md`, absent de `dev`) |

---

## 2. Chemin d'une frame, fonction par fonction

```mermaid
flowchart LR
  subgraph S["kyavserver (processus 1)"]
    A["iosys_spout.c<br/>spout_capture_thread<br/><b>acquired</b>"] --> B["filtergraph x264<br/>hwdownload,format=nv12"]
    B --> C["encode.c<br/><b>encoding</b> / <b>encoded</b>"]
    C --> D["packet_sink.c<br/>en-tête kymux (PTS)<br/><b>sent</b>"]
  end
  subgraph K["kycontroller (processus 2)"]
    E["kycom (local)"] --> F["kydup<br/>(mode Multi, défaut)"] --> G["kyproto video endpoint<br/>Reliable (défaut)"]
  end
  subgraph R["kyclient (processus 3)"]
    H["kyproto recv"] --> I["kycom (local)"] --> J["VLC access kymux.c<br/><b>received</b>"]
    J --> L["decoder.c<br/><b>decoding</b> / <b>decoded</b>"]
    L --> M["video_output.c Thread0Latency<br/><b>prepared</b> → display → <b>displayed</b>"]
    M --> N["kyspout end_frame<br/>Flush + sémaphore Spout"]
  end
  D --> E
  G -- "QUIC loopback" --> H
```

### 2.1 Émission

| # | Étape | Où (révision § 1) | Notes pour la mesure |
|---|---|---|---|
| 1 | Configuration | `kyberfrog/shared/src/gen.rs` `render_config()` | pose `spout_sender`, encodeur `x264` par défaut (`gen.rs:74`) |
| 2 | Choix du backend | `kyavservice/src/video.rs` `configure()` → `windows_streaming_api_list()` | Spout épinglé → seul `spout` est chargé |
| 3 | Capture Spout | `txproto/src/iosys_spout.c:786` `spout_capture_thread` | **scrute** le sémaphore compteur toutes les ~0,5 ms (`av_usleep(500)`, l. 838) ; sans compteur, cadence fixe 16,7 ms. Copie GPU `CopySubresourceRegion` + `Flush` **puis** horodate `acquired` (l. 906-919) |
| 4 | Téléchargement + conversion | `video.rs:540` `create_x264` : `hwdownload,format=bgra,format=nv12` | aucun horodatage entre `acquired` et `encoding` |
| 5 | Encodage | `txproto/src/encode.c:706` (`encoding`, avant `avcodec_send_frame`) et `:775` (`encoded`, après `receive_packet`) | `preset=ultrafast tune=zerolatency x264-params=force-cfr=0` (« 1 à 2 frames de latence » selon le commentaire l. 579-582) |
| 6 | Sortie paquet | `txproto/src/packet_sink.c:184-213` | en-tête 12 octets : **PTS µs sur 61 bits** + flags ; `sent` après `net_send_all` vers le kycom **local** de kycontroller |

La PTS naît à la capture (`ts - epoch`, base `AV_TIME_BASE_Q`), traverse le
filtre et l'encodeur sans rééchelonnement (`encode.c:87`, `filter.c:601`), et
voyage dans l'en-tête kymux jusqu'à VLC. **C'est l'identifiant de frame interne,
de bout en bout.**

### 2.2 Transport (kycontroller)

- `kycontroller/src/kymux_controller/mod.rs:286` `start_video` : en mode
  **Multi** (défaut, `config.rs:116` — `multi_client` absent ⇒ `true`) le flux
  passe par **kydup** ; en **Mono** l'avservice écrit directement dans l'endpoint
  client via kycom.
- `kydup.rs` : broadcast de 32 paquets, **backpressure globale à 20** — le
  client le plus lent freine l'encodeur. Sur un banc à un récepteur, c'est
  neutre ; à surveiller dès qu'un second client se connecte.
- Protocole vidéo : `Reliable` par défaut côté kyclient
  (`kyber-desktop/kyclient/src/main.rs`, `--kymux-video`).

### 2.3 Réception et sortie Spout — **réponse à la question bloquante**

**Oui, la sortie Spout est opérationnelle**, preuve par le code, maillon par
maillon :

| Maillon | Fichier |
|---|---|
| KyberFrog émet `--spout-out <nom>` (et retire `--fullscreen`) | `kyberfrog/shared/src/config.rs:477-481` |
| kyclient bascule en mode sans fenêtre, fixe le display id | `kyber-desktop/kyclient/src/event_loop.rs:344-352`, `:396-405` |
| Config player → C-API | `event_loop.rs:494-501` → `kyclient-rs/src/lib.rs:656` `set_spout_out` → `kyclient/src/capi.rs:895` |
| Backend kymux desktop → player | `kyclient/src/kymux_backend/desktop.rs:77` |
| Le player route vers Spout au lieu d'une fenêtre | `kyvlcplayer/src/player.rs:521-523` |
| **Zero-copy D3D11 par défaut** (`KYSPOUT_SMEM=1` = chemin CPU) | `player.rs:195-204`, `:225-303` |
| libVLC rend dans la texture partagée ; `swap` → `end_frame` : `Flush` + libère le mutex + **incrémente le sémaphore compteur** | `kyspout/src/lib.rs:354-362` |

Validé E2E matériel le 2026-07-18 (Resolume/TouchDesigner, voir
[plan-spout-zerocopy.md](plan-spout-zerocopy.md)) et contenu dans la chaîne
`kyberfrog-dev` épinglée depuis le 2026-08-17. **Re-constaté le 2026-09-13**
avec le bundle `KyberFrog-test` (§ 0) — mais **absent de l'installeur v0.5.0**
installé sur le poste : le bundle mesuré doit être choisi, pas supposé.

Conséquences directes pour le banc :

- **Le compteur Spout est publié** → une sonde peut détecter chaque nouvelle
  frame sans heuristique (même protocole que `iosys_spout.c:766`).
- **`displayed` est horodaté *après* `vd->ops->display`** (`vlc
  src/video_output/video_output.c:2075`), c'est-à-dire après `end_frame`. C'est
  donc l'heure de publication Spout, vue de l'intérieur — et un point de
  recoupement idéal avec la sonde externe (étape 4).
- Réserve : `Flush` ne garantit pas que le GPU a fini. L'écart
  `sonde − displayed` le mesure.

### 2.4 Rôles réels et état de l'unification serveur/client

| Binaire | Rôle réel aujourd'hui | Processus |
|---|---|---|
| `kyberfrog` | superviseur **unique** des deux rôles, web UI, tray — ne linke aucun crate du fork | 1 |
| `kycontroller` | session, auth, TLS, QUIC (kyproto), kydup, relais métriques, clock-sync ; spawne kyavserver | 1 par émetteur |
| `kyavserver` | pipeline média (txproto/FFmpeg) : capture → encodage → socket kycom local | 1 par session/flux |
| `kyclient` | client QUIC + kycom local + libVLC **in-process** (décodage, rendu, sortie Spout) | 1 par viewer |

**L'unification est faite au niveau orchestration, pas au niveau processus.**
« Une app, deux rôles » (Amélioration 1) est livré : un superviseur, une UI, un
TOML. Mais la chaîne reste **trois processus et deux sauts kycom locaux par
flux**. La seule unification in-process existante vit sur la branche kyctl
`KF-cast/player-external` (kycontroller en bibliothèque, backend AV pluggable),
**pré-rebase, 66 commits de retard**, pour kyberfrog-cast (Android). Les options
de rupture (γ/δ) sont dans [archi-greenfield.md](archi-greenfield.md), non
arbitrées (#24).

**Pour le banc :** Phase A mesure les binaires tels que livrés, lancés
directement (sans le superviseur, qui ne sait pas passer `--metrics`), avec le
`kyber_config.toml` que KyberFrog génère. Aucun impact de latence attendu du
superviseur : il ne voit pas passer les frames.

**Cohabitation de ports :** kycontroller et kyclient allouent tous deux leur
kycom dans `9090..9100` (`desktop.rs:201`). Deux processus sur une machine :
aucun souci, mais ne pas lancer d'autres instances pendant un run.

---

## 3. Instrumentation existante

### 3.1 Les dix horodatages par frame

| Clé | Émis par | Moment |
|---|---|---|
| `acquired` | txproto `iosys_spout.c:919` | copie GPU du sender terminée (après détection) |
| `encoding` | txproto `encode.c:706` | frame remise à l'encodeur |
| `encoded` | txproto `encode.c:775` | paquet sorti de l'encodeur |
| `sent` | txproto `packet_sink.c:210` | paquet écrit dans le socket kycom local |
| `received` | VLC `modules/access/kymux.c:310/405` | **VLC a lu le bloc sur son socket kycom local** |
| `decoding` | VLC `src/input/decoder.c:1808` | bloc remis au décodeur |
| `decoded` | VLC `decoder.c:1570` | image décodée |
| `skipped` | VLC `src/video_output/pic_buf.c:81` | remplacée avant affichage (mode 0-latence) |
| `prepared` | VLC `video_output.c:2062` | après `prepare` |
| `displayed` | VLC `video_output.c:2075` | après `display` (= publication Spout) |

Flux : txproto → `kyavservice/src/metrics.rs:50` (msgpack sur kycom) →
kycontroller (endpoint métriques, *claim-once*) → kyclient
`kymux_backend/metrics.rs:144` → `Metric::Video` → `kyber-desktop/kyclient/src/metrics.rs:36`
écrit **`metrics.json` dans le répertoire courant**, en JSON brut (une ligne
par événement, non consolidé). Le consolidateur `MetricsConsolidator`
(`kyclient/src/metrics.rs`) existe mais **n'est pas branché** par le binaire —
tant mieux : il `assert!`e sur les doublons. La consolidation se fera hors ligne.

Métriques réseau en plus : `network_local` (RTT, pertes, toutes les 5 s),
`network_remote` (vu du serveur, 5 s), `network_ping` (clock-sync, offset et
délai, toutes les 10 s, moyenne de 3).

### 3.2 Ce qui manque, et où

| Trou | Pourquoi ça compte | Qui le comble |
|---|---|---|
| Publication du sender **source** | aucun horodatage avant `acquired` | `kybench` (générateur) |
| Détection de la frame **en sortie**, lue par un tiers | c'est la seule mesure identique pour NDI | `kybench` (sonde) |
| Identifiant de frame **dans les pixels** | NDI ne transporte pas la PTS Kyber | `kybench` (codec d'ID) |
| Entre `acquired` et `encoding` | `hwdownload` + conversion non isolés | déduit (`encoding − acquired`), suffisant en Phase A |
| **Entre `sent` et `received`** | ce segment agrège kycom serveur, kydup, kyproto, QUIC, kyproto client, kycom client — **`received` n'est pas l'arrivée QUIC** | conditionnel, étape 10 (fork) |

### 3.3 Domaines d'horloge

| Composant | Horloge (Windows) | Source |
|---|---|---|
| kyproto (clock-sync) | `QueryPerformanceCounter` → µs | `kyproto/src/clock.rs:34` |
| VLC | `mdate_perf` = QPC → µs | `vlc src/win32/thread.c:500` |
| txproto | `av_gettime_relative()` → `av_gettime_relative_win32()` = `counter × 10⁶ / freq` | **constaté** : FFmpeg 8.1 du build (`builddir-win32-server/subprojects/FFmpeg-n8.1/build/config.h`, idem WS1) a `HAVE_CLOCK_GETTIME 0`, donc branche `_WIN32` de `libavutil/time.c` |

Les trois sont QPC en µs : sur une machine l'`offset` clock-sync vaut ≈ 0
(mesuré ±19 µs, § 0) et les horodatages bruts sont directement comparables à
ceux de `kybench`, qui lira QPC aussi. La porte de l'étape 4 reste, pour
re-vérifier sur le bundle réellement mesuré.

`av_usleep(500)` (scrutation Spout, `iosys_spout.c:838`) passe par `usleep` de
MinGW (`HAVE_USLEEP 1`), qui fait `Sleep(usec / 1000)` = **`Sleep(0)`** —
déduit de la CRT MinGW : la boucle cède la main sans dormir. **Effet constaté à
l'étape 4** : en source Spout, un thread de kyavserver consomme **un cœur
entier**, à 60 fps (0,94–0,98) comme à 1 fps (0,999) ; en source écran, aucun
thread ne dépasse 0,04 cœur (piste B6). `clock_type = "system"`
(`kyavservice/src/config.rs:145`) ne change que l'epoch de la PTS, pas l'horloge
des métriques.

### 3.4 Effets observateur et pertes silencieuses

| Constat | Où | Conséquence |
|---|---|---|
| txproto → forwarder : `try_send` sur un canal de 32 | `kyavservice/src/metrics.rs:62` | métriques serveur jetées sous charge (log `Failed to send Metrics`) |
| Player → bus de messages kyclient : `try_send` sur un canal de **32**, `Channel full: message dropped` | `kyclient/src/message_bus.rs:138-146`, `:208` | ~300 métriques/s à 60 fps ; le **même** bus porte les messages de contrôle |
| Bus → collecteur : `try_send` sur 1024 | `kyclient/src/lib.rs:806` | `Metric discarded` |
| Métriques serveur **réservées au premier client** qui les demande (mode Multi) | `kycontroller/src/kymux_controller/mod.rs:399-441` | un second client (preview, test) vole les métriques |
| Écriture JSON synchrone dans un `BufWriter` depuis le callback | `kyber-desktop/kyclient/src/metrics.rs` | coût à mesurer (porte « overhead `--metrics` », étape 4) |

**Règle du banc :** le chiffre de tête (latence Spout → Spout) ne dépend
**jamais** des métriques internes. Elles servent à la décomposition, avec un
taux de complétude publié.

---

## 4. Principes du banc

### 4.1 Frontière de mesure unique

```
t_pub(n)  : kybench publie la frame n sur le sender SOURCE (QPC, copie GPU exécutée, juste avant ReleaseSemaphore)
t_out(n)  : kybench détecte et décode l'ID n sur le sender de SORTIE (QPC)
latence(n) = t_out(n) − t_pub(n)
```

Même générateur, même sonde, même frontière pour les trois configurations :

| Config | Chemin entre les deux senders |
|---|---|
| **F0** plancher | aucun : la sonde lit directement le sender source |
| **K** KyberFrog | kyavserver → kycontroller → QUIC loopback → kyclient → VLC → Spout |
| **N** NDI pontée | *Spout to NDI* → NDI loopback → *NDI to Spout* ([Spout to NDI, leadedge](https://leadedge.github.io/spout-projects.html)) |
| **NN** NDI → NDI | NDI SDK dans `kybench` : le générateur envoie la frame (buffer CPU BGRA) par `NDIlib_send`, la sonde la reçoit par `NDIlib_recv` — aucun Spout, aucun pont |

Toutes les horloges sont QPC sur une seule machine : pas de synchronisation
réseau, pas de NTP, pas de photodiode.

!!! note "Décision opérateur du 2026-09-15 — NN devient la comparaison de tête, N est reportée"
    Le cas d'usage réel est **Arena / TouchDesigner → sortie NDI native →
    récepteur NDI** contre **Arena / TD → Spout → Kyber → Spout**. Personne ne
    fait transiter un flux Spout par des ponts NDI : la config **N** (ponts
    leadedge) sort de la Phase A et reviendra plus tard pour étoffer le banc.
    La comparaison de tête devient **K vs NN**, où `kybench` joue le rôle de
    l'application : il rend la frame sur le GPU, la relit vers le CPU et
    l'envoie par le **NDI SDK officiel** (6.3.2, celui qu'Arena et TD
    embarquent) ; en réception, SDK officiel puis **libNoIDea** (VideoLAN,
    réception seule, SpeedHQ à décoder par FFmpeg), la bibliothèque prévue pour
    le support NDI de KyberFrog. Une validation « Arena dans la boucle » reste
    possible plus tard. L'arrivée à l'écran (« → display ») reste hors Phase A.

!!! note "Décisions opérateur du 2026-09-15 (suite) — contenu réel et config K-AMF"
    - **Contenu** : le fond synthétique décide à lui seul de K vs NN (NN 15,0 à
      25,7 ms selon le mouvement, K ~26 ms quel qu'il soit —
      `bench/runs/2026-09-15-explore-latency/`). Chiffre de tête sur un **clip
      VJ réel** fourni par l'opérateur (`C:\Users\trist\KyberFrog-bench\clips\`,
      empreinte dans `env.json`), rejoué à l'identique pour toutes les configs ;
      fonds synthétiques « blocs alignés » et « blocs décalés » publiés en
      annexe comme bornes. Cela tranche aussi § 6.3 : les coupures du clip sont
      réelles, les pertes en sortie (B12) sont comptées et publiées à part.
    - **K-AMF** rejoint la campagne à côté de K (x264 livré) : `encoder =
      "amf"` a donné 4,0 ms p50 Spout → Spout en exploration contre 25,8 ms.
      K reste le chiffre du produit actuel, K-AMF celui de la release 0.6.0
      (levier 1 de [plan-latency.md](plan-latency.md), priorité du projet).

**Deux comparaisons publiées, jamais mélangées** (texte d'origine ; depuis le
2026-09-15, **K vs NN** est la tête et **K vs N** est reportée) :

| Comparaison | Question | Frontière |
|---|---|---|
| **K vs N** (tête) | « Dans un workflow Spout, qu'est-ce qui est le plus rapide ? » | Spout → Spout des deux côtés |
| **K vs NN** | « Chaque système dans son écosystème natif » : Spout → Kyber → Spout contre NDI → NDI | K : texture GPU publiée → texture GPU publiée ; NN : buffer CPU remis au SDK → buffer CPU rendu par le SDK |

NN est le **cas le plus favorable à NDI** : pas de lecture GPU → CPU à l'entrée,
pas d'upload à la sortie, travail que K, lui, fait. Lecture honnête : si K ≤ NN,
la conclusion est forte ; si K > NN, l'écart NN → N chiffre ce que coûte le
passage par Spout côté NDI, et la décomposition de K (§ 4.3) dit ce qu'il coûte
côté Kyber. NN sert aussi de décomposition pour N (cœur NDI sans ponts).

### 4.2 Identification des frames, robuste à la compression

Un code binaire en **luminance seule**, dimensionné pour survivre à x264 5 Mbps
et à SpeedHQ :

- **Cellules 32 × 32 px** alignées sur la grille des macroblocs 16 × 16 ; seule
  la zone centrale 16 × 16 de chaque cellule est échantillonnée (bords rongés par
  le déblocage ignorés).
- **Charge utile** : compteur 24 bits + CRC-8 = 32 cellules (2 × 16). Deux
  cellules de référence en tête de chaque ligne (noir 16 / blanc 235, inversées
  sur la seconde) → seuil adaptatif, insensible à la plage limitée/pleine et à la
  gamme. Bande totale **576 × 64 px** (18 × 2 cellules ; implémenté à l'étape 1).
- **Deux copies** (coin haut-gauche et bas-droit) : une frame n'est acceptée que
  si les deux décodent au même ID avec CRC valide — détecte déchirement et
  mélange de frames.
- **Fond non trivial et figé** : motif pseudo-aléatoire en mouvement à graine
  fixe, pour que l'encodeur travaille comme sur un contenu réel. Le même fichier
  de graines pour tous les runs.
- Pas de chroma dans le code : le 4:2:0 ne le touche pas.

Une frame dont le code ne décode pas est **comptée**, jamais devinée.

### 4.3 Asymétrie NDI — ne pas fausser la comparaison

Les deux systèmes font le même travail aux deux bouts (lire un sender Spout,
publier un sender Spout). KyberFrog le fait **dans ses processus**
(`iosys_spout.c`, kyspout), NDI par **deux applications-ponts**. Ce n'est pas un
biais de mesure : c'est le coût réel qu'un opérateur Spout paie avec NDI. D'où :

1. **Chiffre de tête = Spout → Spout**, même frontière, ponts inclus. C'est la
   question opérationnelle.
2. **Décomposition symétrique en annexe**, jamais soustraite d'un seul côté :

   | Segment | K (métriques internes) | N |
   |---|---|---|
   | Entrée | `acquired − t_pub`, puis `encoding − acquired` | `N − NN` réparti (non séparable finement) |
   | Cœur transport + codec | `encoding → decoded` | `NN` (envoi SDK → trame reçue) |
   | Sortie | `displayed − decoded`, puis `t_out − displayed` | idem entrée |

3. On n'écrit **jamais** « NDI sans ses ponts » face à « KyberFrog complet ».
4. Réglages NDI figés et consignés : NDI High Bandwidth (pas HX), versions du
   runtime et des ponts, format de couleur de réception le plus rapide, pas
   d'audio, pas de verrouillage de cadence dans les ponts.
5. **À confirmer :** le transport NDI entre deux processus d'une même machine
   peut différer de son transport réseau — raison de plus pour que la Phase B
   (deux machines) refasse la comparaison.

### 4.4 Frontière Rust / Python

| Rust — `kybench` (chemin critique temporel) | Python — orchestration et analyse (jamais dans le chemin temporel) |
|---|---|
| générateur Spout 1080p60 cadencé (timer haute résolution), écrit l'ID, horodate `t_pub` | lance/arrête les processus (kycontroller, kyclient `--metrics`, ponts NDI, kybench) |
| sonde Spout : scrutation du compteur ≤ 0,25 ms, copie d'une petite ROI vers une texture *staging*, `Map`, décodage, horodatage `t_out` | fige et snapshotte l'environnement (versions, SHAs, hash TOML, plan d'alimentation, HAGS, processus actifs) |
| relais-étalon : relit un sender et republie avec un retard programmé (k frames ou x ms) | ordre randomisé par blocs, préchauffage, durée |
| émetteur/récepteur NDI SDK (config NN) | ingestion `kybench.csv` + `metrics.json`, jointure, rejet automatique |
| sortie : CSV d'événements `(config, run, id, t_pub, t_out, décodage_ok)` en µs QPC | statistiques, bootstrap, ECDF, rapport |

Emplacement : `kyberfrog/bench/kybench/` (crate autonome, **hors workspace**
principal) et les scripts Python à plat dans `kyberfrog/bench/`. `kybench` réimplémente le
registre Spout (~600 lignes, modèle `kyspout` + `iosys_spout.c`) plutôt que de
dépendre de la chaîne de forks : 1 h 30 de build évitée.

---

## 5. Plan séquentiel

Chaque étape a **une porte observable** ; aucune étape ne démarre avant que la
précédente soit franchie. Le coût modèle est estimé en fin de document (§ 7).

**Qui exécute les vérifications.** Règle : une porte qui ne demande que la
machine est exécutée **automatiquement par l'agent**, dès qu'elle est
exécutable — pas en fin de chantier. L'**opérateur** n'intervient que pour ce
qu'un agent ne peut ou ne doit pas faire : accepter une licence ou remplir un
formulaire de téléchargement, régler une application graphique, redémarrer,
fermer ses propres applications, laisser la machine au repos pendant une
mesure, et un seul contrôle visuel par instrument. Chaque étape le précise
(**Exécution**).

### Étape 0 — Gel et inventaire

- **Construire un bundle cohérent** aux SHAs `kyberfrog-dev` (le poste a un
  installeur v0.5.0 sans zero-copy et un `KyberFrog-test` mixte, § 0) ; vérifier
  la présence de la chaîne `D3D11 zero-copy` dans `kyclient.dll` ; relever les
  révisions affichées au démarrage, la version NDI (6.2.1 installée), les
  versions des ponts.
- Script d'inventaire (Python) produisant `env.json` : Windows build, pilote
  GPU, plan d'alimentation, HAGS, Game Mode, résolution d'horloge, processus en
  cours.
- **Porte :** `env.json` généré deux fois de suite est identique (hors horodatage),
  et un `kyclient --spout-out bench-out` sur un kycontroller local publie un
  sender visible par un récepteur Spout quelconque.

**Exécution :** *opérateur* pour fermer KyberFrog, Resolume et TouchDesigner et figer les réglages qui demandent un redémarrage (HAGS) ; *agent Opus 5* pour l'inventaire, le build du bundle et le smoke test (déjà rodé, § 0).

**Modèle recommandé : Opus 5** — inventaire et script à spec connue, pas de
boucle de mise au point.

!!! success "Franchie le 2026-09-13 (agent Opus 5)"
    - **Bundle** : `kyber-desktop` `643ee0e` (le SHA pinné par
      `packaging/versions.sh`), buildé en local dans `kyber/debian-win64:local-0.27`
      en reproduisant le job CI `build-fork` (clone au SHA + submodules +
      `build-win32.sh -p`) — **~17 min** sur le Ryzen 5 5600X, pas 1 h 30. La CI
      n'avait aucun bundle Windows en cache pour ce SHA. Révisions embarquées,
      cohérentes : kyctl `0.20.0-66-gb29f8f4`, kyber-desktop `0.26.0-42-g643ee0e`,
      kymedia `0.17.0-33-g7f0360f` ; chaîne `D3D11 zero-copy` présente dans
      `kyclient.dll`. Déposé hors dépôt : `C:\Users\trist\KyberFrog-bench\bundle-643ee0e`.
    - **Inventaire** : [`bench/inventory.py`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/feat/bench-latency-phase-a/bench/inventory.py)
      → `env.json`, deux passes identiques hors horodatage (`--diff`). Copie :
      `bench/runs/2026-09-13-step0/env.json`.
    - **Smoke test** : [`bench/smoke.py`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/feat/bench-latency-phase-a/bench/smoke.py)
      lance kycontroller + `kyclient --spout-out bench-out` ; la sonde
      `bench/spout_probe.py` (lecture de `SpoutSenderNames`, du bloc d'info et du
      compteur `_Count_Semaphore`, sans SDK) voit le sender en 2560 × 1440
      BGRA à **60,10 fps** sur 10 s.
    - **Constats** :
        - la capture d'écran (DXGI Desktop Duplication) ne produit de frame **que
          si l'écran change** — un bureau immobile a débité **0 fps** au premier
          essai. Le smoke test anime une fenêtre témoin ; le banc, lui, part d'une
          source Spout (`kybench`), non concernée ;
        - un `kill` de kycontroller laisse **kyavserver vivant plusieurs secondes**
          (le Job Object ne le tue pas immédiatement) : tuer l'arbre
          (`taskkill /T /F`), sans quoi deux runs consécutifs se chevauchent ;
        - `metrics.json` reste **vide** si kyclient est tué en `/F` : la collecte
          des métriques (étape 4) devra arrêter kyclient proprement ;
        - poste relevé : Windows 11 25H2 build 26200.9445, **HAGS désactivé**,
          Game Mode par défaut (actif), plan Haute performance, trois écrans à
          60 Hz (principal 2560 × 1440), résolution du timer à **1,0 ms**
          imposée par une application tierce (Chrome/Discord ouverts) ;
        - ponts leadedge Spout ↔ NDI **non installés** — à consigner à l'étape 5.
    - **Reste opérateur** avant les runs de mesure (étape 2) : fermer navigateurs
      et Discord (ils fixent la résolution du timer), décider de l'état HAGS à
      figer (redémarrage si changé).

### Étape 1 — Construire `kybench`

- Générateur, sonde, codec d'ID (§ 4.2), relais-étalon, sortie CSV. Build via
  l'image MinGW habituelle.
- **Porte :** générateur → sonde sur une ROI décode **100 %** des IDs pendant
  60 s, sans trou ni doublon ; le relais-étalon republie.

**Exécution :** *agent Opus 5*, automatique — build, lancement générateur → sonde, lecture du CSV.

**Modèle recommandé : Opus 5** — implémentation dont la spec est écrite ici ;
la mise au point physique fine est l'objet de l'étape 2, pas de celle-ci.

!!! success "Franchie le 2026-09-13 (agent Opus 5)"
    - **`bench/kybench/`** : crate Rust autonome (hors workspace, `windows` 0.52,
      aucune dépendance à la chaîne de forks), buildée par `bench/kybench/build.sh`
      dans `kyber/debian-win64:local-0.27` en quelques secondes après le premier
      build. Commandes `gen`, `probe`, `relay`, `list` ; CSV en µs QPC.
      Codec d'ID (`idcode.rs`) couvert par 4 tests unitaires (aller-retour,
      bruit + plage limitée, rejet sans contraste / CRC faux, alignement).
      La sortie NDI (config NN) viendra à l'étape 5.
    - **Porte** (`bench/step1_gate.py`, résultats dans
      `bench/runs/2026-09-13-step1/`) : générateur → sonde **3 600 / 3 600**
      frames décodées sur 60 s, 0 trou, 0 doublon, 0 saut de compteur ;
      générateur → relais 50 ms → sonde : **1 200 / 1 200** décodées.
    - **Constat qui a changé le code** : après un simple `Flush`, un récepteur sur
      un autre device D3D11 lit encore **la frame précédente** (premier essai :
      ID 0 lu deux fois, ID 1 perdu). Le générateur et le relais attendent donc
      l'exécution GPU de la copie (requête `D3D11_QUERY_EVENT`) avant de signaler
      la frame, et `t_pub` est pris juste avant `ReleaseSemaphore` — sinon la
      sonde voyait parfois le compteur avant l'horodatage (latence −0,035 ms).
      **Conséquence pour K** : kyspout signale après un `Flush` seul ; une sonde
      peut donc y lire une frame en retard — c'est précisément ce que la double
      copie + le contrôle de monotonie des IDs détecteront à l'étape 3.
    - **Chiffres indicatifs** (poste non figé, Chrome ouvert — l'étape 2 les
      mesurera pour de vrai) : F0 p50 **0,13 ms**, p99 0,25 ms, max 1,76 ms ;
      relais consigne 50 ms → p50 **50,61 ms**, p99 51,26, max 58,71 ;
      coût de publication (`t_pub − échéance`) p50 1,29 ms ; intervalle entre
      frames p1 16,31 / p50 16,67 / p99 17,05 ms ; lecture de la ROI (copie +
      `Map`) p50 0,18 ms.

### Étape 2 — **PORTE CRITIQUE : plancher de bruit du banc**

Rien d'autre ne démarre tant qu'elle n'est pas franchie.

1. **F0 à vide** : générateur → sonde, 3 runs × 5 min.
2. **F0 sous charge** : même chose pendant qu'un pipeline K tourne sur un
   *autre* sender (même charge CPU/GPU qu'en campagne).
3. **Lecteur concurrent** : F0 pendant qu'un second récepteur lit le même sender
   (le cas de kyavserver en config K).
4. **Étalon d'exactitude** : relais à retard programmé 3 frames (50,0 ms) puis
   7 ms ; la sonde doit retrouver la consigne.
5. **Pacing du générateur** : jitter de `t_pub` par rapport à la grille 60 Hz.

**Porte (toutes requises) :**

| Critère | Seuil |
|---|---|
| IDs perdus / erreurs de décodage | 0 sur 3 × 18 000 frames |
| F0 p50 (à vide et sous charge) | ≤ 1,5 ms |
| F0 p99 sous charge | ≤ 3 ms |
| Écart p50 entre runs | ≤ 0,3 ms |
| Étalon 50 ms | médiane dans 50,0 ± 0,5 ms, p99 − p50 ≤ 1 ms |
| Jitter `t_pub` p99 | ≤ 1 ms |
| Lecteur concurrent | F0 p50 dégradé de ≤ 0,3 ms |

Pièges prévisibles, à traquer avant d'en conclure quoi que ce soit : résolution
du timer Windows (un `Sleep` à 15,6 ms donne un plancher en marches d'escalier),
`Map` bloquant sur un GPU chargé, contention du mutex d'accès Spout.

**Exécution :** *agent*, automatique, en boucle ; *opérateur* seulement pour laisser la machine au repos pendant les runs et, une fois, regarder le sender étalon dans un récepteur Spout (TouchDesigner) pour valider à l'œil que l'ID affiché est le bon.

**Modèle recommandé : Fable 5.1** — c'est exactement la boucle longue et
auto-corrective (lancer, voir une distribution bimodale, remonter au timer ou au
`Map`, corriger, relancer) sous contrainte physique ; et tout le reste du plan
hérite de son résultat.

!!! success "Franchie le 2026-09-14 (agent Fable 5.1), deux passes complètes"
    Porte : [`bench/step2_gate.py`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/feat/bench-latency-phase-a/bench/step2_gate.py)
    (cinq sous-tests, un JSON chacun, porte agrégée `step2.json`) ; résultats
    dans `bench/runs/2026-09-13-step2/` (passe finale, CSV compressés) et
    `bench/runs/2026-09-13-step2-pass1/` (première passe, JSON et logs).
    Poste figé : Chrome/Discord/Beeper fermés, HAGS off, timer 1 ms (VS Code),
    `env.json` **identique** avant et après chaque passe.

    **Résultat de la passe finale** (sonde à 50 µs de scrutation, thread
    `TIME_CRITICAL`) — 7 runs × 18 000 frames = **126 000 frames, 0 perte, 0
    erreur de décodage, 0 doublon** :

    | Critère | Seuil | Mesuré |
    |---|---|---|
    | F0 à vide, p50 (3 runs) | ≤ 1,5 ms | **0,026 / 0,025 / 0,026 ms** ; p99 0,050 ; max 0,31 |
    | F0 sous charge K, p50 (3 runs) | ≤ 1,5 ms | **0,027 / 0,027 / 0,024 ms** (K à 60,0 fps pendant les runs) |
    | F0 sous charge, p99 | ≤ 3 ms | **0,051 ms** ; max 0,16 |
    | Écart p50 entre runs | ≤ 0,3 ms | 0,001 (vide), 0,003 (charge) |
    | Étalon 3 frames (50,0 ms) | 50,0 ± 0,5 ; p99 − p50 ≤ 1 | **p50 50,093** ; p99 50,162 (Δ 0,069) ; 1 frame à 59,0 |
    | Étalon 7 ms | idem | **p50 7,049** ; p99 7,093 (Δ 0,044) ; max 7,14 |
    | Jitter `t_pub` p99 (110 880 frames) | ≤ 1 ms | **0,000 ms** ; intervalle p1–p99 16,667 ms |
    | Lecteur concurrent (kyavserver sur le même sender) | p50 dégradé ≤ 0,3 ms | **−0,002 ms** (p50 0,024, p99 0,051) |

    Lecture : la résolution du banc est **~0,05 ms au p99**, soit 0,3 % d'une
    période à 60 Hz ; l'étalon retrouve la consigne à **+0,05 / +0,09 ms**
    (deux détections de sonde). Le coût du `Map` de la ROI, hors chemin de
    mesure, monte de 0,24 ms à vide à 0,44 ms sous charge et 0,97 ms quand
    kyavserver copie la même texture (attente du mutex Spout).

    **Ce que la boucle a corrigé dans `kybench`** (chaque point vu en mesure,
    puis re-mesuré) :

    1. **Cadence du générateur** : publier *après* l'échéance coûtait 1,29 ms
       (copie GPU + attente). `Sender::publish_at` fait la copie et l'attente GPU
       pendant un *lead* de 2 ms **avant** l'échéance et ne signale qu'à
       l'échéance → `t_pub` sur la grille (écart p99 0,000 ms).
    2. **Course sur le compteur Spout** (constat le plus lourd, → piste **B11** de
       [plan-latency.md](plan-latency.md)) : la lecture « wait puis release » du
       SDK n'est pas atomique entre deux lecteurs. Avec la sonde d'origine à
       côté de kyavserver, **les deux** voyaient le compteur osciller
       (962/963/962…) : kyavserver a capturé ~900 frames fantômes/s et sorti
       259 fps. La sonde lit désormais le compteur **sans le toucher**
       (`NtQuerySemaphore`), n'accepte que `count > last`, et prend sa
       référence de départ sur le maximum d'une rafale de lectures. kyavserver,
       lui, reste vulnérable à tout voisin SDK (voir B11).
    3. **Étalon à 3 frames** : la fenêtre de publication (échéance − 2 ms)
       tombait exactement sur l'arrivée de la frame k + 3 → détection d'entrée
       à 1,18 ms au lieu de 0,12, étalon à 51,3 ms. Le relais arme (copie +
       mutex) puis continue à scruter l'entrée et signale à l'échéance.
    4. **`yield_now` dans le pacer** : sous charge K, céder le cœur retardait la
       détection de 0,6–0,9 ms sur ~1/4 des frames → attente active
       (`spin_loop`) sur le dernier 1,5 ms.
    5. **Queue résiduelle avec kyavserver comme voisin** (première passe, sonde à
       250 µs) : p95 0,68 / p99 1,15 ms, 9 % des frames > 0,5 ms, sur le seul
       sous-test 3. Éliminé un à un : priorité `TIME_CRITICAL` (vérifiée
       effective, sans effet), lead 200 µs (sans effet), sémaphore (un voisin
       kybench en lecture *destructive* style SDK : aucune queue), second lecteur
       kybench (aucune queue). Seule la **scrutation à 50 µs** la fait
       disparaître (p99 0,051) — retenue par défaut ; le mécanisme exact,
       propre à kyavserver, n'est **pas identifié** et est consigné tel quel.
    6. Chemin `--out` relatif : kycontroller (cwd = bundle) ne trouvait pas
       `KYBER_CONFIG_PATH` → résolu en absolu.
    7. Timecode `MM:SS:FF` + compteur décimal dessinés au centre de la frame
       (sept segments, hors des bandes d'ID) pour le contrôle visuel.

    **Contrôle visuel opérateur (2026-09-14, TouchDesigner, deux Syphon Spout In
    TOP)** : `kybench-src` et `kybench-relay` (relais 60 frames) lisibles,
    timecode et compteur du relais **exactement 1 s / 60 frames** derrière la
    source — **conforme**. Incident sans suite : un `kybench gen` relancé
    pendant que TD attendait déjà le sender a reçu une fois « Accès refusé
    (0x80070005) » sur un objet Spout ; la relance suivante a réussi.

    **Non couvert par la porte, consigné** : 0 à 20 frames par run de 18 480
    (≤ 0,11 %) publiées avec 0,5 à 11 ms de retard, par rafales de 1 à 3
    (intervalle min 5,8 / max 27,5 ms) — décrochage OS/GPU du générateur, sans
    effet sur `t_out − t_pub` ni sur le p99 ; en config K ces frames arriveront
    groupées à l'encodeur. Une frame de l'étalon 50 ms signalée 8,8 ms après son
    échéance (même classe). La première passe (sonde 250 µs, priorité normale)
    avait déjà franchi la porte (p50 0,12–0,14, p99 0,25, étalons 50,31 / 7,25)
    : les chiffres finaux ne dépendent pas du réglage retenu, seule la queue
    du sous-test 3 en dépendait.

### Étape 3 — Codec d'ID sous compression

- Rejouer le générateur à travers K et N sur 10 min chacun, sans mesure de
  latence : seul le taux de décodage compte. Réduire volontairement le débit
  x264 au minimum configurable (`minimum_bitrate`, 5 Mbps par défaut) pour
  trouver la marge.
- **Porte :** ≥ 99,99 % des frames reçues décodées avec CRC valide dans les deux
  configs ; aucune frame acceptée à tort (CRC + double copie).

**Exécution :** *agent Opus 5*, automatique.

**Modèle recommandé : Opus 5** — test à critère binaire, spec du codec déjà
fixée.

!!! success "Volet K franchi le 2026-09-15 (agent Opus 5) — volet NN intégré à la porte de l'étape 5"
    Porte : [`bench/step3_gate.py`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/feat/bench-latency-phase-a/bench/step3_gate.py)
    (générateur → chaîne K → sonde sur `bench-out`, un JSON par cas, porte
    agrégée `step3.json`) ; résultats dans `bench/runs/2026-09-14-step3/`.
    Aucun lecteur Spout sur le sender source à part kyavserver (TouchDesigner
    fermé, le script refuse de démarrer si une application Spout connue
    tourne) ; la sonde lit `bench-out` sans toucher au compteur. Discord ouvert
    pendant k-min et la marge (sans effet sur un taux de décodage).

    | Cas | Débit x264 effectif (log kyavserver) | Frames vues | Décodées CRC OK | Faux positifs | Sortie | IDs distincts/s | Frames manquantes |
    |---|---|---:|---:|---:|---:|---:|---:|
    | **k-default** (10 min) | `b = 20000000` (demande kyclient par défaut) | 36 000 | **36 000 (100 %)** | **0** | 60,0 fps | 59,555 | 267 (0,74 %), dont 264 sur coupure |
    | **k-min** (10 min) | `b = 5000000` (`--bitrate 5M` = `minimum_bitrate` par défaut) | 36 000 | **36 000 (100 %)** | **0** | 60,0 fps | 59,983 | 10 (0,028 %), dont 6 sur coupure |
    | marge 2M (2 min, hors porte) | `b = 2000000` (`minimum_bitrate` abaissé) | 7 199 | **7 199 (100 %)** | 0 | 60,0 fps | 59,901 | 12, dont 8 sur coupure |
    | marge 1M (2 min, hors porte) | `b = 1000000` | 7 200 | 1 324 (18,4 %) | **0** | 60,0 fps | — | — |
    | marge 500K (2 min, hors porte) | `b = 500000` | 7 200 | 665 (9,2 %) | **0** | 60,0 fps | — | — |

    **Marge** : le codec décode encore 100 % à **2 Mbps**, soit 2,5× sous le
    plancher configurable ; la falaise est entre 2 et 1 Mbps. Même effondré
    (1M : 3 851 frames aux deux copies rejetées, 2 024 à une seule copie valide),
    il n'accepte **aucune** frame fausse. **La double copie a servi** : à 1M comme
    à 500K, une frame avait deux bandes à CRC-8 valide mais des IDs différents
    (5 008 / 5 022 et 3 645 / 3 647) — une bande seule aurait été acceptée à tort
    (probabilité attendue ~1/256 par bande détruite) ; la frame est rejetée.

    **Porte** : ≥ 99,99 % décodées et 0 faux positif dans les deux cas K —
    **franchie** ; conditions de validité vérifiées : débit x264 égal à la
    consigne, aucun changement de débit en cours de run, 0 frame jetée par
    kyavserver pendant les runs (37 au démarrage de k-min, backlog B5), sortie à
    60,0 fps. Faux positif = ID à CRC valide mais incohérent : absent du
    générateur, retour en arrière, ou latence à plus de 250 ms de la médiane du
    run. Latence indicative (hors porte) : p50 25,3 ms à 20 Mbps, 25,9 ms à 5 Mbps.

    **Débit réel de kyavserver** : 60 captures/s — aucune frame jetée, IDs
    consécutifs en sortie hors pertes ci-dessous, aucun doublon « fantôme »
    (B11 non déclenché).

    **Constat : la sortie K perd une frame à chaque à-coup** (→ piste **B12** de
    [plan-latency.md](plan-latency.md)). Le fond du générateur saute toutes les
    128 frames (`WRAP_PX / SCROLL_PX`) ; à 20 Mbps cette frame encode ~5 ms plus
    lentement et arrive ~17 ms en retard au rendu, au moment où la suivante est
    déjà décodée. La sonde lit alors **n + 1 sous le compteur de n** : n n'est
    jamais visible, n + 1 est vue deux fois (écart 2,5 ms). Diagnostic
    (`diag-cut/`) : run de 40 s avec `kyclient --metrics true`, métriques jointes
    aux IDs (`join.txt`) ; puis A/B de la sonde avec `--finish-in-lock` (copie de
    la ROI exécutée par le GPU **avant** de relâcher le mutex Spout) : perte
    **inchangée** (26 sur coupure en 60 s) — ce n'est pas un artefact de sonde,
    tout récepteur Spout la subit. À 5 Mbps la frame de coupure est plafonnée
    par le contrôle de débit, l'à-coup disparaît presque (6 pertes sur coupure).

    **Conséquences pour la suite** :

    - le codec d'ID est validé à **5 Mbps sur un fond aléatoire en mouvement** —
      le cas le plus dur prévu par le plan ;
    - **§ 6.3 en conflit** : à débit par défaut, chaque run K dépasse le rejet
      « frames perdues > 0,1 % » à cause de B12, pas du banc. **Décision
      opérateur à prendre avant l'étape 6** : (a) garder le générateur (des
      coupures sont réalistes en VJ) et compter à part les pertes « en sortie »
      de K, en les publiant comme un résultat ; ou (b) rendre le fond cyclique
      sans coupure pour isoler la latence, B12 étant mesuré dans un run dédié ;
    - l'étape 4 dispose d'une recette de jointure ID ↔ PTS qui marche
      (`acquired` le plus proche après `t_pub` : chaque frame du run de 40 s
      appariée ; l'ambiguïté reste à vérifier à l'étape 4) et
      d'un premier cas d'étude pour sa décomposition.

### Étape 4 — Chaîne K instrumentée : horloges, jointure, overhead

1. Lancer K avec `kyclient --metrics` ; vérifier `network_ping.offset_micros`
   ≈ 0 et que `acquired − t_pub` est positif et de l'ordre de la milliseconde
   (sinon txproto n'est pas en QPC, § 3.3).
2. **Jointure** ID pixel ↔ PTS : par appariement monotone `t_pub(n) <
   acquired(pts)` au plus proche ; valider sur 18 000 frames qu'aucun
   appariement n'est ambigu.
3. **Recoupement** : `t_out(n) − displayed(pts)` doit avoir la même distribution
   que le plancher F0 (± 1 ms).
4. **Overhead** : 3 runs K avec `--metrics`, 3 sans ; différence des médianes
   Spout → Spout ≤ 0,5 ms, sinon la décomposition se fait sur des runs dédiés.
5. **Complétude** : fraction de frames ayant les dix clés, et décompte des
   `message dropped` / `Metric discarded` dans les logs.
6. **Queue de latence** : corréler dans le temps `network_remote.packets_lost`
   (~30/s en boucle locale au smoke test) avec les pics de `sent → received`
   et `received → decoding`.
7. **CPU de kyavserver en source Spout** : tranche la question `Sleep(0)`
   (§ 3.3) ; un cœur plein à vide = piste d'amélioration, pas un rejet.

**Porte :** |offset| ≤ 1 ms, jointure non ambiguë, recoupement ± 1 ms, overhead
≤ 0,5 ms, complétude ≥ 95 % (en dessous : la décomposition est publiée avec ce
chiffre, le chiffre de tête n'est pas affecté), et une explication écrite de la
queue p99 (corrélée ou non aux pertes).

**Exécution :** *agent Opus 5*, automatique (même recette que le smoke test du § 0, source Spout `kybench` au lieu de l'écran).

**Modèle recommandé : Opus 5** — les horloges sont confirmées (§ 0), les
critères sont écrits. **Escalade Fable 5.1 seulement si la queue p99 reste
inexpliquée** : remonter des pertes QUIC en boucle locale jusqu'à la config
quinn / buffers UDP de kyproto, à travers kymux, kyctl et le runtime Windows —
surface large et boucle expérimentale.

!!! success "Franchie le 2026-09-15 (agent Opus 5) — sans escalade Fable"
    Porte : [`bench/step4_gate.py`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/feat/bench-latency-phase-a/bench/step4_gate.py) ;
    résultats dans `bench/runs/2026-09-15-step4/` (un dossier par run : CSV
    générateur et sonde, `metrics.json`, logs, `run.json` ; un JSON d'analyse
    par run ; porte agrégée `step4.json`). Six runs frais de 5 min après 60 s
    de préchauffage, entrelacés avec / sans `--metrics` (m, p, p, m, m, p),
    bundle `643ee0e`, x264 20 Mbps (débit effectif relu dans le log), source
    `kybench-src` 1080p60, sonde sur `bench-out`, aucune autre application
    Spout. Chaque run : 18 000 frames dans la fenêtre, 60,0 fps en sortie,
    0 frame jetée par kyavserver pendant la mesure.

    **Arrêt propre de kyclient trouvé** : binaire console dont le handler
    `ctrlc` déclenche la déconnexion. `KPipeline` le lance dans son propre
    groupe de processus et lui envoie `CTRL_BREAK_EVENT` : sortie rc=0 en
    ~0,1 s, `metrics.json` complet (dernier `displayed` écrit 1,4 ms après le
    signal). Les six runs se sont arrêtés ainsi, aucun kill forcé. Effet de
    bord révélé : au logout, kycontroller boucle sur `accept` et écrit ~6 000
    WARN en 60 ms (piste B13), d'où des `kycontroller.log` de ~800 Ko.

    | Critère | Seuil | Mesuré | |
    |---|---|---|---|
    | \|`offset_micros`\| | ≤ 1 ms | max **44 µs** (117 échantillons) | ✅ |
    | `acquired − t_pub` | positif, ~ms | min 0,063 ms, p50 0,085–0,093, p99 0,14–0,17 | ✅ |
    | Jointure ID ↔ PTS | non ambiguë | 53 999 / 54 000 frames appariées, **0 ambiguë** (voir écart ci-dessous) | ✅ |
    | Recoupement `t_out − displayed` | F0 ± 1 ms (p50 0,027 / p99 0,051) | p50 **0,026–0,027**, p99 **0,049** | ✅ |
    | Overhead `--metrics` | \|Δ médiane des p50\| ≤ 0,5 ms | **−0,12 ms** (25,78 avec, 25,89 sans) | ✅ |
    | Complétude (9 clés) | ≥ 95 % | 99,994 / 100 / 100 % ; 0 `skipped` ; 0 `message dropped`, `Metric discarded`, `Failed to send Metrics` | ✅ |
    | Queue p99 | explication écrite | oui, ci-dessous : coupures de contenu, pas les pertes QUIC | ✅ |

    **Écart au pré-enregistrement (jointure), visible dans l'historique.** Le
    critère écrit avant le premier run jugeait un appariement ambigu si la
    capture tombait à plus d'une demi-période de la publication ou si le pas
    des PTS s'écartait de celui des `t_pub`. Au run 1 il a signalé 2 frames
    (5806 capturée 12,2 ms après publication pendant un à-coup système où le
    générateur publiait lui-même en retard, puis 5807 par ricochet). Un
    **contrôle pixel**, indépendant de toute horloge, a été ajouté : l'ID lu
    par la sonde au plus près de `displayed(pts)` doit être l'ID apparié. Il
    confirme les 2 frames, et toutes les autres : 53 600 confirmées, 399
    lectures de n + 1 déjà décodée (B12, voir plus bas), **0 contradiction**.
    La porte exige désormais 0 signalement non confirmé et 0 contradiction ;
    le verdict de l'ancien critère reste dans `step4_gate.log` (NOT PASSED),
    l'analyse actuelle dans `step4_gate-analyse.log`. La borne physique d'une
    mauvaise capture est la période moins l'avance du générateur (14,7 ms),
    pas une demi-période.

    **Décomposition K, Spout → Spout** (médiane des trois runs avec métriques ;
    1080p60, x264 20 Mbps, boucle locale) :

    | Segment | p50 ms | p99 ms | part de la médiane |
    |---|---:|---:|---:|
    | `t_pub` → `acquired` (détection et copie Spout) | 0,09 | 0,14 | 0,4 % |
    | `acquired` → `encoding` (hwdownload + conversion) | 2,91 | 3,47 | 11 % |
    | `encoding` → `encoded` (**x264**) | **19,92** | 21,28 | **77 %** |
    | `encoded` → `sent` | 0,05 | 0,06 | 0,2 % |
    | `sent` → `received` (kycom + kydup + QUIC + kycom) | 0,26 | 0,42 | 1,0 % |
    | `received` → `decoding` | 0,01 | 0,02 | — |
    | `decoding` → `decoded` | 0,16 | 1,00 | 0,6 % |
    | `decoded` → `prepared` (rendu dans la texture Spout) | 2,37 | 2,44 | 9 % |
    | `prepared` → `displayed` | 0,02 | 0,03 | — |
    | `displayed` → `t_out` (sonde) | 0,03 | 0,05 | — |
    | **`t_pub` → `t_out`** | **25,78** | **29,15** | |

    Chiffre de tête par run (première apparition de chaque ID) :

    | Run | p50 ms | p95 | p99 | max | IDs distincts / 18 000 | perdus en sortie (dont frame de coupure) |
    |---|---:|---:|---:|---:|---:|---:|
    | k-metrics-1 | 25,19 | 26,78 | 29,15 | 110,1 | 17 859 | 141 (130) |
    | k-plain-1 | 26,12 | 27,49 | 30,12 | 51,7 | 17 865 | 135 (132) |
    | k-plain-2 | 25,89 | 26,95 | 28,15 | 49,7 | 17 866 | 134 (131) |
    | k-metrics-2 | 25,78 | 26,73 | 28,69 | 48,5 | 17 877 | 123 (121) |
    | k-metrics-3 | 25,85 | 26,82 | 29,49 | 51,1 | 17 866 | 134 (128) |
    | k-plain-3 | 25,84 | 26,65 | 27,72 | 46,3 | 17 865 | 135 (135) |

    L'overhead de `--metrics` n'est **pas détectable** : −0,12 ms, alors que les
    p50 varient de 0,93 ms d'un run à l'autre (25,19 à 26,12). À retenir pour
    le pilote (étape 7) : la variance inter-runs est du même ordre que l'écart
    qu'on voudra résoudre. Le run k-metrics-1 a subi un à-coup système (16
    publications du générateur en retard, x264 jusqu'à 103 ms, max 110 ms).

    **Queue p99 — expliquée, sans lien avec les pertes QUIC.** Le p99 de tête
    (27,7–30,1 ms) n'est qu'à ~3,5 ms du p50 : la queue de 80 ms du smoke test
    (source écran 1440p) n'existe pas en Spout 1080p60. Elle vient des coupures
    du générateur (fond qui saute toutes les 128 frames) :

    | Frames (médianes, 3 runs) | x264 ms | `sent → received` | `decoding → decoded` | `decoded → prepared` | tête |
    |---|---:|---:|---:|---:|---:|
    | frame de coupure (n % 128 = 0), vue en sortie : 11 à 19 par run | 25,0–27,0 | **7,9–10,6** | 1,3–1,6 | 5,8 | 43,9–47,4 |
    | frame suivante | 20,1–20,9 | 0,29–0,33 | **1,6–5,7** | 2,2 | 25,0–30,2 |
    | toutes les autres | 19,5–19,9 | 0,24–0,27 | 0,16 | 2,37 | 25,2–25,8 |

    - La frame de coupure est plus grosse : x264 +5 à 7 ms **et** transport
      8 à 11 ms au lieu de 0,25 — un coût qui dépend de la taille de la frame,
      pas d'une perte. La plupart de ces frames ne sont jamais vues (B12).
    - La frame suivante attend le décodeur 1,6 à 5,7 ms au lieu de 0,16 :
      c'est la contention qui produit B12.
    - Parmi les frames au-dessus du p99, 29 %, 59 % et 63 % sont à ± 2 d'une
      coupure (3 % attendus au hasard). **Hors coupures, le p99 tombe à
      27,3–28,3 ms** : ~2 ms de queue résiduelle.
    - Pertes QUIC vues du serveur : **~70/s**, régulières (216 à 565 par
      fenêtre de 5 s), 0 côté client. Corrélation, sur 60 fenêtres de 5 s par
      run, entre pertes et pics de `sent → received`, de `received → decoding`
      ou nombre de frames de queue : |r| ≤ 0,35 (Pearson et Spearman), de signe
      instable d'un run à l'autre. Leur cause reste inconnue (B4), mais elles ne
      font pas la queue.
    - Étape 10 **non déclenchée** : `sent → received` = 1 % de la médiane.

    **CPU de kyavserver** (processus de session ; un second kyavserver
    d'énumération des écrans reste à 0) : 1,8–2,1 cœurs pendant les runs,
    dont **un thread à 0,94–0,98 cœur**. Générateur à **1 fps** : ce thread
    reste à **0,999 cœur**, seul actif. Contre-épreuve en **source écran**
    (duplication DXGI 2560 × 1440, bureau quasi statique → ~7,7 fps en sortie,
    `cpu-screen/`) : processus à 0,25 cœur, aucun thread au-dessus de 0,04. La
    scrutation Spout occupe donc un cœur quel que soit le débit → **B6 constaté** (pas un rejet, une piste). kyclient 0,04–0,08 cœur,
    kycontroller 0,02–0,08.

    **B12 précisé par les métriques** (399 frames perdues en sortie, 3 runs) :
    `prepared(n)` est terminé 0,10 ms **avant** `decoded(n + 1)`, puis
    `displayed(n)` tombe **0,003–0,004 ms après** `decoded(n + 1)` (max
    0,048 ms), et la sonde lit n + 1 sous le compteur de n 0,03 ms plus tard.
    L'affichage de n est donc bloqué jusqu'à la fin du décodage de n + 1, et la
    texture publiée contient alors n + 1. Cela corrige la note B12 écrite à
    l'étape 3 (`prepared`/`displayed` de n datés avant `decoded` de n + 1),
    tirée de `join.txt` aux valeurs arrondies à 0,1 ms. Le verrou en cause
    n'est pas identifié.

    **Conséquences** :

    - la décomposition K est fiable (horloges, jointure, complétude) et
      gratuite : `--metrics` peut rester actif dans tous les runs K de la
      campagne ;
    - **§ 6.3 toujours en conflit** : 0,68 à 0,78 % de frames perdues en
      sortie par run (B12), au-dessus du rejet à 0,1 %. Décision opérateur
      (a) / (b) toujours attendue avant l'étape 6 ;
    - pour une comparaison K vs N sans biais de contenu, la queue de K est
      presque entièrement faite des coupures : l'option (b) de § 6.3
      isolerait la latence, l'option (a) mesure un contenu VJ réaliste.

### Étape 5 — Chaîne NN : NDI → NDI *(N reportée, décision du 2026-09-15)*

- `kybench ndi-gen` : même générateur que `gen` (fond, ID, HUD), frame rendue
  sur le GPU avant l'échéance ; `t_pub` = échéance, puis relecture GPU → CPU
  (staging + `Map`) et `NDIlib_send_send_video_v2` (BGRX, `clock_video`
  désactivé : le cadencement reste celui du générateur). Ces deux coûts sont
  ceux de l'application, ils comptent dans NN.
- `kybench ndi-probe` : découverte (`NDIlib_find`), réception SDK
  (`NDIlib_recv_capture_v2`), décodage de l'ID, puis upload de la frame dans
  une texture GPU — ce que paie tout récepteur réel. Latences publiées à la
  réception (`t_out`) et après upload (`t_up`) ; pertes relues par
  `NDIlib_recv_get_performance`.
- Symétrie avec K : dans `gen`, la copie dans la texture Spout partagée est
  faite **avant** `t_pub` ; sa durée est désormais consignée (colonne
  `copy_us`) pour publier ce biais plutôt que le supposer négligeable.
- Second récepteur : libNoIDea + décodeur `speedhq` de FFmpeg, une fois la
  réception SDK validée.
- **Porte :** NN tourne 10 min, IDs décodés ≥ 99,99 %, 0 faux positif (le
  volet NN de l'étape 3), versions du SDK / runtime et réglages (FourCC,
  format de réception, bande passante) consignés dans `env.json`.

**Exécution :** *opérateur* : NDI SDK 6.3.2 installé le 2026-09-15 ; *agent Opus 5* pour l'intégration dans `kybench` et les runs de validation.

**Modèle recommandé : Opus 5** — intégration d'un SDK documenté et d'outils
existants.

### Étape 6 — Orchestrateur et pré-enregistrement du protocole

- Python : lanceur de runs (processus frais à chaque run), ordre randomisé par
  blocs, préchauffage, arrêt propre, collecte, rejet automatique (§ 6).
- Le protocole (§ 6) est **committé avant la campagne** ; toute modification
  ultérieure est un commit visible.
- **Porte :** un « run à blanc » de chaque config produit un dossier complet
  (`env.json`, CSV, `metrics.json`, logs, verdict de rejet) sans intervention.

**Exécution :** *agent Opus 5*, automatique.

**Modèle recommandé : Opus 5** — code d'orchestration classique à spec écrite.

### Étape 7 — Pilote

- 3 runs par config (F0, K, N, NN). Estimer la variance inter-runs,
  vérifier que la durée de 5 min stabilise le p99, ajuster N (nombre de runs) si
  la variance l'exige — **avant** la campagne, jamais après.
- **Porte :** N et durée finaux committés dans le protocole.

**Exécution :** *agent Opus 5*, automatique ; *opérateur* : machine au repos (~1 h).

**Modèle recommandé : Opus 5** — exécution et calcul simples.

### Étape 8 — Campagne

- Session unique, configs entrelacées, plancher F0 mesuré en début et en fin de
  session.
- **Porte :** ≤ 20 % de runs rejetés par config, plancher début/fin stable.

**Exécution :** *agent* pour lancer et surveiller ; *opérateur* : créneau de ~4 h machine intouchée, idéalement de nuit, et relecture du verdict de rejet au matin.

**Modèle recommandé : Opus 5** — exécution du protocole, aucune décision de
conception.

### Étape 9 — Analyse et rapport

- Statistiques § 6.4, ECDF par config, décomposition symétrique § 4.3,
  rapport versionné dans `docs/dev/`.
- **Porte :** le rapport régénère tous ses chiffres depuis les dossiers de runs
  par une seule commande.

**Exécution :** *agent Opus 5* ; *opérateur* pour relire le rapport.

**Modèle recommandé : Opus 5** — analyse statistique standard sur données
propres.

### Étape 10 — *Conditionnelle* : découper `sent → received`

Déclenchée **seulement** si ce segment dépasse 25 % de la latence K ou 5 ms en
médiane. Ajouter deux horodatages dans le fork : à la sortie de kydup/kycom côté
kycontroller et à la réception kyproto côté kyclient (lecture de la PTS dans
l'en-tête kymux), acheminés par le canal métriques existant.

- Contrainte : kymux/kyproto sont **upstream non forkés** ; instrumenter dans
  kyctl (kydup, wrappers kycom) plutôt que forker kymux. En mode Mono il n'y a
  pas de kydup — le point d'accroche est à trouver.
- **Porte :** la somme des sous-segments égale `received − sent` à ± 0,2 ms.

**Exécution :** *agent Fable 5.1* (builds fork locaux via Docker) ; *opérateur* pour valider l'E2E matériel du bundle instrumenté.

**Modèle recommandé : Fable 5.1** — surface large en une passe (txproto,
kycom, kydup, kyproto, VLC), jointures entre quatre repos, et boucle de build
fork de 1 h 30 où chaque itération ratée coûte cher.

---

## 6. Protocole

### 6.1 Conditions figées

Machine de dev (RX 7800 XT, pilote 32.0.31041.1004 au 2026-09-13) ; Windows,
NDI 6 Tools 6.2.1 et ponts à versions consignées ; **KyberFrog arrêté** (ses
viewers écrivent dans le même log kyclient et redémarrent en boucle quand leur
émetteur est absent) ; plan d'alimentation haute performance ; HAGS et Game Mode
dans un état consigné ; aucune autre application Spout/NDI ; Resolume et
TouchDesigner **fermés** (le générateur est `kybench`) ; aucune fenêtre de
rendu (sortie Spout seule) ; antivirus avec exclusion du dossier de runs ;
priorité normale (pas de temps réel) ; boucle locale, NIC non sollicitée.

Le premier lancement d'un kycontroller depuis un nouveau chemin ouvre le prompt
du **pare-feu Windows** : sans effet en boucle locale (le loopback n'est pas
filtré), mais la règle entrante créée est requise en Phase B ; elle est liée au
chemin de l'exécutable, donc à redonner pour tout nouveau bundle.

Config K : `kyber_config.toml` généré par KyberFrog (x264, `multi_client`
absent ⇒ Multi, protocole Reliable, débit par défaut), hashé dans `env.json`.

### 6.2 Runs

| Paramètre | Valeur de départ (révisable **au pilote seulement**) |
|---|---|
| Configurations | F0, K (x264 livré), K-AMF, NN (N reportée) — décisions du 2026-09-15 |
| Contenu | clip VJ réel (tête) ; fonds synthétiques aligné / décalé (annexe) |
| Runs par config | 10 |
| Durée mesurée | 5 min (18 000 frames) |
| Préchauffage écarté | 60 s |
| Ordre | blocs randomisés (chaque bloc = une permutation des configs) |
| Processus | relancés à chaque run |
| Plancher F0 | en début et fin de session + canal témoin continu (voir 6.3). Le canal témoin est une sonde `kybench` sur le sender **source** pendant que kyavserver le capture : compatible **uniquement** parce que la sonde lit le compteur sans le toucher (`NtQuerySemaphore`) — une lecture style SDK ferait capturer des frames fantômes à kyavserver (étape 2, piste B11 de [plan-latency.md](plan-latency.md)) |

### 6.3 Critères de rejet (automatiques, pré-enregistrés)

Un run est rejeté — **consigné, jamais effacé** — si :

- frames perdues > 0,1 % ou une erreur de CRC acceptée ;
- redémarrage d'un processus de la chaîne (log) ;
- résolution ≠ 1920 × 1080 ou format ≠ BGRA en sortie ;
- jitter `t_pub` p99 > 1 ms ;
- |`offset_micros`| > 1 ms (config K) ;
- processus étranger > 10 % CPU pendant le run ;
- throttling thermique GPU/CPU relevé.

Une session est rejetée si le plancher F0 début/fin diffère de > 0,3 ms en
médiane, ou si > 20 % des runs d'une config sont rejetés.

### 6.4 Traitement statistique

- **Unité statistique = le run**, pas la frame : les latences successives sont
  autocorrélées, un bootstrap au niveau frame mentirait sur l'incertitude.
- Par run : p50, p95, p99, max, taux de perte, taux de `skipped` (K).
- Par config : médiane des p50 de runs et médiane des p99, **IC 95 % bootstrap
  sur les runs** (10 000 rééchantillonnages).
- Comparaison K vs N : IC bootstrap de la différence des médianes. Conclusion
  « plus rapide » seulement si l'IC exclut 0 **et** la différence dépasse le p99
  du plancher.
- Graphiques : ECDF superposées, jamais de moyenne seule.
- Le plancher n'est **pas soustrait** : il est publié à côté, comme borne de
  résolution.

---

## 7. Choix de modèle et budget

Tarifs (table en cache du 2026-06-24, USD/M tokens) : Opus 5 = 5 entrée /
25 sortie ; Fable 5.1 = 10 / 50, lecture de cache Fable 0,25. Sur une longue
boucle agentique très cachée, Fable coûte **≈ 1,5×** Opus (la lecture de cache
relativement bon marché compense une partie du ×2 en sortie).

Hypothèse d'ordre de grandeur, pour une étape de type boucle instrument : ~150
tours, contexte moyen ~100 k tokens caché, ~4 k tokens de sortie (réflexion
comprise) par tour ⇒ **Fable ≈ 40 $, Opus ≈ 26 $**. À prendre comme une
fourchette (± 50 %), pas un devis.

**Classement des étapes taggées Fable, par rapport bénéfice/coût :**

| Rang | Étape | Coût Fable estimé | Bénéfice | Pourquoi ce rang |
|---|---|---|---|---|
| **1** | **Étape 2 — plancher de bruit** | ~20–40 € | tout le plan en dépend ; un plancher faux invalide chaque chiffre publié | c'est la boucle physique la plus dure et la seule qui **conditionne** toutes les autres |
| 2 | Étape 4 — escalade *si* la queue p99 reste inexpliquée (pertes QUIC en boucle locale) | ~15–30 € | explique la queue, ouvre une piste d'amélioration concrète ; ne touche pas le chiffre de tête | conditionnelle ; Opus d'abord. **Non déclenchée** (2026-09-15) : queue expliquée par les coupures de contenu, pas par les pertes |
| 3 | Étape 10 — découpe du transport dans le fork | ~30–50 € | n'intéresse que si le transport pèse lourd | conditionnelle, la plus chère (builds 1 h 30), bénéfice incertain avant la campagne |

**Si une seule étape est financée en Fable : l'étape 2.** Avec ~50 € au total,
elle laisse de quoi faire le reste en Opus ; les deux autres ne se déclenchent
que sur un résultat qui n'existe pas encore.

---

## 8. Hors Phase A

- Deux machines, NIC réelle, switch (Phase B) — la boucle locale ne sollicite
  pas le réseau, « 1 Gbps propre » y est une condition nominale, pas mesurée.
- Plusieurs récepteurs, effet de la backpressure kydup.
- Encodeurs GPU (AMF/NVENC, #28-1), mode Mono et protocoles `Unreliable*`
  (#28-3), réglages de `video_buffer` — et toutes les pistes d'amélioration
  relevées par l'étude, notées dans [plan-latency.md](plan-latency.md#pistes-relevees-par-le-banc-2026-09-13)
  pour une Phase A+ sur le banc validé.
- Glass-to-glass (écran, photodiode), latence de présentation DWM.
- Audio et synchronisation A/V.
- Autres résolutions et cadences (4K, 30/120 fps), HDR.
- Pertes et dégradations réseau simulées.
- Resolume/TouchDesigner comme source réelle (validation « Arena dans la
  boucle » envisagée après la campagne).
- Config N (flux Spout transporté par des ponts NDI) — reportée le 2026-09-15.
- NDI HX, SRT/RTSP (#18).
- Linux.
- Toute **optimisation** : Phase A mesure, elle ne corrige rien.

## 9. Ce qui n'a pas pu être vérifié

- **Pas de source Spout ni de 1080p au smoke test** : les chiffres du § 0 sont
  indicatifs (écran 2560 × 1440).
- ~~Coût CPU de la scrutation Spout~~ — constaté à l'étape 4 (un cœur) ; que
  ce soit bien `Sleep(0)` dans `av_usleep` reste déduit (thread sans nom, pas
  de pile échantillonnée).
- Cause des pertes QUIC en boucle locale côté serveur (~70/s à 20 Mbps) — non
  identifiée ; l'étape 4 a montré qu'elles **n'expliquent pas** la queue p99.
- Contenu du bundle de l'installeur v0.5.1 (NSIS non déballé).
- Transport NDI entre processus locaux, réglages exacts exposés par les ponts
  leadedge ; le NDI SDK (en-têtes) n'est pas installé — seul le runtime
  `Processing.NDI.Lib.x64.dll` de NDI 6 Tools l'est.
- MR : `kyber-frog/kyberfrog` n'a **aucune MR ouverte** ; les MR des repos du
  fork (kyctl, kymedia, txproto, kyber-desktop, kysdk) renvoient **403** via
  l'API avec le compte `tritriper` — fonctionnalité MR désactivée ou droits
  insuffisants, non tranché.
