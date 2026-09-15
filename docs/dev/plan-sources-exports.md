# Sources et exports étendus (#18) — et l'écran figé côté émetteur (#26)

## #18 — protocoles d'entrée et de sortie

KyberFrog gère aujourd'hui en entrée Spout, écran, webcam et « Tout envoyer »,
et en sortie la fenêtre et Spout. Les quatre sous-items ci-dessous sont
indépendants et livrables séparément.

**Ordre conseillé :** E (NDI output, réutilise la sortie Spout) et D/F
(SRT/RTSP, peu de code fork) → C (NDI input, nouveau chemin de capture).

### Sources (Émission)

- **D — SRT / RTSP input** : ingérer un flux SRT ou RTSP (caméras IP…). FFmpeg
  accepte `rtsp://` et `srt://` comme URL d'entrée, et txproto s'appuie déjà sur
  FFmpeg : peu de code fork attendu. Nouveau variant `Source::Url { url }`.
  *Complexité : faible à moyenne — à valider côté txproto.*
- **C — NDI input** : ingérer un flux NDI (caméras réseau, switchers
  Tricaster/ATEM, OBS) et le retransmettre en Kyber. FFmpeg n'a plus d'entrée
  NDI : il faut un **iosys txproto** construit sur le SDK NDI, qui livre des
  frames au graphe d'encodage de kyavserver comme le fait la capture Spout.
  Nouveau variant `Source::Ndi { name }`. *Complexité : moyenne à haute — à
  engager après E, qui tranche la question de licence.*

### Exports (Réception)

- **F — SRT / RTSP output** : sortie réseau d'un flux reçu (enregistrement,
  re-streaming), via FFmpeg côté kyvlcplayer ou txproto.
  *Complexité : faible à moyenne.*
- **E — NDI output** : republier un flux Kyber reçu en source NDI (switchers,
  OBS, écrans NDI). *Complexité : faible à moyenne.*

## #18-E — NDI output, sur le chemin de la sortie Spout

### Principe

La redirection Spout a déjà un chemin CPU (`setup_spout_output_smem`,
`kyvlcplayer/src/player.rs`) : libVLC livre chaque frame décodée en **BGRA
mémoire** et `SpoutSender::send_bgra(buffer, width, height, pitch)` la publie.
Un émetteur NDI consomme exactement ce format
(`NDIlib_send_send_video_v2`, FourCC BGRA). La sortie NDI **branche un émetteur
NDI à cet endroit**, sans plugin VLC ni changement de VLC.

| Couche | Travail |
|---|---|
| `kyctl` — nouveau crate `kyndi` | `NdiSender { open(name), send_bgra(…) }` sur le SDK NDI, même surface que `kyspout` |
| `kyctl/kyvlcplayer` | `setup_ndi_output` : callbacks smem → `NdiSender` ; `set_ndi_out` dans la C-API et `kyclient-rs` |
| `kyber-desktop/kyclient` | flag `--ndi-out <name>`, windowless comme `--spout-out` (même display id hôte), exclusif avec `--fullscreen` |
| KyberFrog `shared` | `Viewer.ndi_out: Option<String>` + argument dans `Globals::kyclient_args()` + tests round-trip |
| KyberFrog UI | décommenter le type de réception « Redirection NDI » (`ViewerFormDrawer.tsx`), sender `KyberFrog-<nom>` |

**Chargement du runtime NDI à l'exécution** : le binaire ne linke pas le SDK,
il charge la bibliothèque du **runtime NDI installé sur la machine**
(`Processing.NDI.Lib.x64.dll` sous Windows, `libndi.so` sous Linux). Sans
runtime, `--ndi-out` échoue avec un message explicite et l'UI masque le type.

### Bilan

**Pour**

- **Chemin déjà éprouvé** : callbacks smem, mode windowless, gestion du display
  id et câblage KyberFrog existent pour Spout ; seul l'émetteur change.
- **Interop large** : OBS, vMix, switchers et écrans NDI reçoivent un flux
  Kyber sans rien installer de spécifique à KyberFrog.
- **Cross-platform** : NDI existe sous Windows et Linux, contrairement à Spout.
- **Aucun binaire propriétaire embarqué** : le runtime reste celui de
  l'utilisateur.

**Contre**

- **Une copie CPU par frame** (download GPU→CPU dans libVLC) : pas de
  zero-copy, contrairement à Spout.
- **Encodage NDI en plus** : l'émetteur NDI recompresse (SpeedHQ) ce que Kyber
  vient de décoder — coût CPU et latence à mesurer (#28-4).
- **Runtime NDI requis** sur la machine réceptrice.
- **SDK sous licence propriétaire** : les conditions d'usage et de mention de
  la marque NDI s'appliquent, à valider avant la release.

### Prérequis

- [ ] **Décision opérateur — licence** : SDK NDI propriétaire face à un fork
  AGPL. Le chargement dynamique du runtime utilisateur évite de le
  redistribuer ; les conditions de marque et d'attribution restent à lire.

### Validation

Viewer « Redirection NDI » sur un émetteur Kyber → la source `KyberFrog-<nom>`
apparaît dans **NDI Studio Monitor** ou dans OBS (source NDI), image et couleurs
correctes, taille native ; arrêt du viewer → la source disparaît.

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
