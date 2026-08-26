# Sources et exports étendus (#18) — et la variante écran figé (#26)

> Extrait d'`IMPROVEMENTS.md` le 2026-08-25 sans réécriture. A (webcam) et B
> (sélection d'écran) sont livrés — voir le CHANGELOG. Le suivi des items vit
> dans le board (`docs/dev/backlog.md`).

## #18 — les quatre protocoles restants


> Chaque sous-item est indépendant et peut être livré séparément. **A (webcam)
> et B (sélection d'écran) livrés** (voir **Shipped** ci-dessous) ; restent
> C/D/E/F. Les items "FFmpeg natif" (D/F) sont probablement peu coûteux ;
> l'item "NDI" (C/E) demande plus de travail (dépendance libndi propriétaire).
> Priorité à décider selon les besoins terrain.

**Sources (Émission)**

- **C — NDI input** : ingérer un flux NDI et le re-transmettre en Kyber.
  Côté réception, **libVLC dispose déjà d'un plugin NDI** (
  [tobiasebsen/libndi](https://github.com/tobiasebsen/libndi)) — à explorer
  comme voie d'intégration (kyvlcplayer, même chemin que le Spout output #8)
  plutôt que d'ajouter NDI dans la chaîne txproto/FFmpeg. Nouveau variant source
  `Source::Ndi { name }` dans KyberFrog. Interop avec caméras réseau, switchers
  Tricaster/ATEM, OBS.
  *Complexité : moyenne (si via VLC plugin) à haute (si via FFmpeg txproto).*

- **D — SRT / RTSP input** : ingérer un flux SRT ou RTSP (caméras IP, etc.).
  FFmpeg le supporte nativement (`rtsp://`, `srt://` comme URL d'entrée) ;
  txproto utilise déjà FFmpeg → probablement peu de code fork nécessaire.
  Nouveau variant `Source::Url { url }` dans KyberFrog.
  *Complexité : faible à moyenne — à valider côté txproto.*

**Exports (Réception)**

- **E — NDI output** : re-publier un flux Kyber reçu en NDI (sortie vers
  switchers, OBS, écrans NDI). Même voie que C : plugin VLC NDI
  ([tobiasebsen/libndi](https://github.com/tobiasebsen/libndi)) côté
  kyvlcplayer, similaire au Spout output (#8) mais protocole NDI.
  *Complexité : moyenne (si via VLC plugin).*

- **F — SRT / RTSP output** : sortie réseau d'un flux reçu vers d'autres
  systèmes (enregistrement, re-streaming). Via FFmpeg côté kyvlcplayer ou
  txproto.
  *Complexité : faible à moyenne.*

**Ordre conseillé (restant) :** D/F (SRT/RTSP, peu de fork) → C/E (NDI,
dépendance lourde).

## #26 — variante : écran source figé côté émetteur

- **What:** aujourd'hui la sélection d'écran (#18-B) est côté *réception* —
  chaque viewer demande l'écran voulu à la connexion. Piste : un
  **transmetteur** qui impose son écran à tout client qui s'y connecte
  (sémantique « le transmetteur possède l'écran », utile pour un mapping
  émetteur→écran unique documenté côté régie).
- **Why:** pure réflexion — **l'implémentation actuelle (#18-B) fonctionne
  bien et remplit le use case** ; ceci n'est pas un besoin exprimé, juste une
  variante à garder en tête si le terrain en montre le besoin.
- **How (si un jour utile) :** changement **fork** : (1) ajouter une clé
  `display_id: Option<u32>` au `Config` de kyavserver
  (`kyavservice/src/config.rs`), (2) la faire primer sur le `display_id`
  demandé par le client dans `video_config` (comme `spout_sender`
  aujourd'hui), (3) la remonter via kycontroller, (4) *puis* rétablir un champ
  côté `Source::Screen` + `gen.rs` + un picker émission. Chaîne de build ~1h +
  validation visuelle obligatoire → complexité moyenne, à mettre au niveau de
  #8/#17, **pas** en « faible ».
