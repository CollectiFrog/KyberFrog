# Banc de latence (#28-4)

Mesure le retard entre l'image émise et l'image reçue, pour KyberFrog et pour
NDI, sur la même machine et avec les mêmes images. Lancement **manuel**, une
configuration à la fois.

Résultat publié : [`runs/comparaison-k-ndi/`](runs/comparaison-k-ndi/README.md).
Méthode et conclusions : [`docs/dev/bench-latency.md`](../docs/dev/bench-latency.md).

## Ce qu'il faut avant

- Windows, Python 3.12 (bibliothèque standard seule), aucune dépendance à installer.
- **Aucune autre application Spout ouverte** (Resolume, OBS, TouchDesigner…) :
  elles perturbent la mesure et font échouer la sonde.
- Un bundle KyberFrog décompressé, pour les configurations `k-*`
  (défaut : `C:\Users\trist\KyberFrog-bench\bundle-643ee0e`, sinon `--bundle`).
- NDI 6 Runtime installé, pour la configuration `ndi` (trouvé par
  `NDI_RUNTIME_DIR_V6`, sinon `--ndi-dll`).
- `kybench.exe` construit (voir plus bas).
- Machine au repos : pas de navigateur qui joue une vidéo, pas de build en cours.

## Lancer une mesure

```powershell
# le plancher de l'instrument : à faire une fois, pour vérifier que le banc est sain
python bench\latency_bench.py f0     --out bench\runs\<date>-essai\f0     --seconds 60

# KyberFrog, encodeur GPU (le défaut depuis 0.6.0)
python bench\latency_bench.py k-amf  --out bench\runs\<date>-essai\k-amf  --seconds 180

# KyberFrog, encodeur logiciel (ce qui était livré avant 0.6.0)
python bench\latency_bench.py k-x264 --out bench\runs\<date>-essai\k-x264 --seconds 180

# NDI, pour comparaison
python bench\latency_bench.py ndi    --out bench\runs\<date>-essai\ndi    --seconds 180
```

Une configuration `k-*` prend environ `--seconds` + 60 s (démarrage de la chaîne
puis chauffe). `f0` et `ndi` démarrent en quelques secondes.

Le contenu de l'image change le résultat en NDI. Pour le montrer :

```powershell
python bench\latency_bench.py ndi --out ...\ndi-aligne --scroll 16 --scroll-offset 0
python bench\latency_bench.py ndi --out ...\ndi-decale --scroll 16 --scroll-offset 8
```

Pour mesurer sur un vrai clip plutôt que sur le fond synthétique, décode-le
d'abord en images brutes — quelques secondes suffisent, elles sont rejouées en
boucle et tiennent en mémoire (2 s de 1080p = 1 Go) :

```powershell
# ffmpeg du bundle ; -t 2 = deux secondes, mises à l'échelle en 1080p
C:\Users\trist\KyberFrog-bench\bundle-643ee0e\ffmpeg.exe -i mon-clip.mov `
  -t 2 -vf "scale=1920:1080:flags=bicubic" -pix_fmt bgra -f rawvideo clip.bgra

python bench\latency_bench.py k-amf --out ...\k-amf --seconds 60 --clip clip.bgra
```

Le numéro d'image et le timecode restent dessinés par-dessus le clip, donc la
mesure fonctionne à l'identique. Les deux côtés, KyberFrog et NDI, rejouent la
même suite d'images.

Inventaire du poste et du bundle, à joindre à toute campagne :

```powershell
python bench\inventory.py --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e -o env.json
```

## Récupérer les résultats

Chaque dossier `--out` contient :

| Fichier | Contenu |
|---|---|
| `summary.txt` | le résultat en clair, lisible tel quel |
| `result.json` | les mêmes chiffres en détail : p50 / p95 / p99 / min / max, images perdues, régularité du générateur |
| `gen.csv`, `probe.csv` | une ligne par image, pour tout recalculer |
| `*.log` | sorties des processus ; pour les configs `k-*`, aussi `kyber_config.toml`, `kycontroller.log`, `kyclient.stdout.log` |

`summary.txt` ressemble à ceci :

```
=== k-amf — retard entre l'image émise et l'image reçue
  une image sur deux sous :    4.0 ms   (0.24 image à 60 Hz)
  19 images sur 20 sous   :    4.5 ms
  99 images sur 100 sous  :    5.2 ms
  pire cas vu             :   17.6 ms
  images arrivées         : 1800 sur 1800  (0.000 % perdues)
```

**Le chiffre à retenir est la première ligne** (la médiane). La dernière ligne
dit si la mesure est fiable : si des images manquent en `f0`, la mesure ne vaut
rien, quelque chose d'autre tourne sur la machine.

Avant de publier un chiffre : le refaire au moins deux fois et vérifier que les
médianes concordent à moins d'une milliseconde.

## Formats et tailles de source (audit des résolutions)

Même chaîne que les configurations `k-*`, mais on vérifie l'**image**, pas le
temps : `kybench pattern` publie quatre barres de couleur (rouge, vert, bleu,
gris 50 % ; quatre gris pour un format mono) dans le format DXGI demandé —
celui d'un TouchDesigner en 16 bits flottant, d'un Unreal en 10 bits —,
KyberFrog les transporte jusqu'à `kyclient --spout-out`, et `kybench check`
compare le centre de chaque barre à ce qu'il devrait être en 8 bits.

```powershell
python bench\format_check.py --bundle <bundle> --out bench\runs\<date>-formats `
    --encoder amf --formats bgra8,rgb10a2,rgba16f,rgba32f,r8 --size 1280x720
```

`results.md` donne un verdict par format : `pass`, `wrong colours` ou
`no picture`, avec les lignes du `kycontroller.log` qui l'expliquent.
Formats acceptés par `pattern` : `bgra8 rgba8 bgrx8 rgb10a2 rgba16f rgba16
rgba32f r8 r16 r16f r32f`. Une application Spout ouverte ne gêne pas ce test.

## Construire `kybench.exe`

Le générateur et la sonde sont en Rust, hors du workspace principal, et
réimplémentent le registre Spout — pas de dépendance à la chaîne de forks,
1 h 30 de build évitée.

```powershell
cd bench\kybench
docker run --rm -v "${PWD}:/src" -v kybench-cargo-registry:/cargo/registry `
  -v kybench-target:/target -w /src kyber/debian-win64:local-0.27 bash build.sh
```

## Les fichiers

| Fichier | Rôle |
|---|---|
| `latency_bench.py` | lance une configuration (`f0`, `k-x264`, `k-amf`, `ndi`) et écrit `summary.txt` + `result.json` |
| `inventory.py` | `env.json` du poste et du bundle ; `--diff A B` compare deux inventaires |
| `spout_probe.py` | liste les senders Spout et lit leur débit, sans SDK ; sert à vérifier qu'aucune application Spout ne traîne |
| `kybench/` | l'instrument (Rust) : générateur Spout cadencé, sonde, codec d'ID ; barres de couleur dans tout format DXGI (`pattern`) et leur vérification (`check`) ; côté NDI `ndi-gen`, `ndi-probe`, `ndi-list` (runtime chargé à l'exécution) |
| `format_check.py` | passe chaque format de source Spout dans toute la chaîne et vérifie les couleurs reçues (`results.md`) |

## Pièges

- `KYBER_CONFIG_PATH` doit être **absolu** — `latency_bench.py` s'en charge.
- kyclient s'arrête par `CTRL_BREAK` dans son propre groupe de processus. Un
  `taskkill` sans `/F` n'a pas d'effet (pas de fenêtre en `--spout-out`) ; avec
  `/F` le dernier tampon est perdu. Le kill forcé n'est qu'un repli, consigné
  dans `result.json` (`client_stop`).
- Docker Desktop est à relancer à la main après un redémarrage de la machine.
- Le registre Spout a la taille choisie par la **première** application qui
  l'a créé (TouchDesigner : 112 places). `kybench` la lit au lieu de supposer
  256 places : une vue plus grande que le registre est refusée (« accès
  refusé »), ce qui empêchait tout `gen` quand TD était ouvert.
- Si la chaîne K refuse de démarrer (« sender absent après 60 s »), vérifier
  qu'aucun `kycontroller.exe` ou `kyavserver.exe` ne traîne d'un run précédent.
