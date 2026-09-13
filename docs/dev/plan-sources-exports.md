# Sources et exports étendus (#18) — et l'écran figé côté émetteur (#26)

## #18 — protocoles d'entrée et de sortie

KyberFrog gère aujourd'hui en entrée Spout, écran, webcam et « Tout envoyer »,
et en sortie la fenêtre et Spout. Les quatre sous-items ci-dessous sont
indépendants et livrables séparément.

**Ordre conseillé :** D/F (SRT/RTSP, peu de code fork) → C/E (NDI, dépendance
propriétaire).

### Sources (Émission)

- **D — SRT / RTSP input** : ingérer un flux SRT ou RTSP (caméras IP…). FFmpeg
  accepte `rtsp://` et `srt://` comme URL d'entrée, et txproto s'appuie déjà sur
  FFmpeg : peu de code fork attendu. Nouveau variant `Source::Url { url }`.
  *Complexité : faible à moyenne — à valider côté txproto.*
- **C — NDI input** : ingérer un flux NDI et le retransmettre en Kyber. Voie
  d'intégration : le **plugin NDI de libVLC**
  ([tobiasebsen/libndi](https://github.com/tobiasebsen/libndi)) côté
  kyvlcplayer, sur le même chemin que la sortie Spout. Nouveau variant
  `Source::Ndi { name }`. Interop caméras réseau, switchers Tricaster/ATEM, OBS.
  *Complexité : moyenne.*

### Exports (Réception)

- **F — SRT / RTSP output** : sortie réseau d'un flux reçu (enregistrement,
  re-streaming), via FFmpeg côté kyvlcplayer ou txproto.
  *Complexité : faible à moyenne.*
- **E — NDI output** : republier un flux Kyber reçu en NDI (switchers, OBS,
  écrans NDI), par le même plugin VLC que C, sur le modèle de la sortie Spout.
  *Complexité : moyenne.*

**Pourquoi le plugin VLC pour NDI** : il se branche là où la sortie Spout est
déjà câblée, sans introduire libndi dans la chaîne txproto/FFmpeg ; entrée et
sortie partagent la même dépendance.

## #26 — écran source figé côté émetteur *(icebox)*

La sélection d'écran (#18-B) est côté **réception** : chaque viewer demande
l'écran voulu à la connexion, ce qui couvre le besoin actuel. La variante : un
**transmetteur qui impose son écran** à tout client (« le transmetteur possède
l'écran »), utile pour un mapping émetteur → écran documenté côté régie.

Recette, côté **fork** :

1. ajouter `display_id: Option<u32>` au `Config` de kyavserver
   (`kyavservice/src/config.rs`) ;
2. le faire primer sur le `display_id` demandé par le client dans
   `video_config`, comme `spout_sender` ;
3. le remonter via kycontroller ;
4. rétablir un champ sur `Source::Screen` + `gen.rs` + un picker côté émission.

Chaîne de build ~1 h et validation visuelle obligatoire : **complexité
moyenne**.
