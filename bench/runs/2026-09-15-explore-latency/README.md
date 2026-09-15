# Exploration hors protocole — contenu, transport NDI, leviers K (2026-09-15)

Runs courts (15 à 60 s), **indicatifs**, faits pour répondre à deux questions
de l'opérateur ; ils ne franchissent aucune porte et ne remplacent pas la
campagne. Scripts copiés tels qu'exécutés (chemins de scratchpad en dur) ;
bundle `643ee0e`, boucle locale, RX 7800 XT, Ryzen 5 5600X.

## Le contenu du générateur décide de K vs NN

Même fond pseudo-aléatoire en blocs de 16 px, trois mouvements (`--scroll`,
`--scroll-offset` de `gen` / `ndi-gen`). NN mesuré jusqu'à la texture GPU du
récepteur (`t_up`), K jusqu'à la sonde Spout (`t_out`).

| Fond | K x264 p50 | NN p50 | NN frames paires / impaires |
|---|---:|---:|---:|
| blocs alignés sur la grille 16×16 (scroll 16) | 26,2 ms | 15,0 ms | 15,0 / 14,9 |
| blocs décalés d'un demi-bloc (scroll 16, offset 8) | 25,9 ms | 19,6 ms | 19,7 / 19,6 |
| défaut : alterne à chaque frame (scroll 8) | 25,8 ms | 25,7 ms | 13,5 / 27,2 |

`nn-scroll/`, `k-scroll/`. Le transport NDI n'y est pour rien : réception
forcée en TCP, UDP, RUDP ou adaptateur loopback (`NDI_CONFIG_DIR` par
processus, `nn_transport.py`) donne le même motif à 0,3 ms près ; les modes
d'envoi sync / async / clocké non plus (`nn_modes.py`).

## Leviers K sans code (`k_levers.py`, 60 s, `--metrics`)

| Config | tête p50 | p99 | x264 / AMF | `acquired → encoding` | `decoded → prepared` | CPU kyavserver |
|---|---:|---:|---:|---:|---:|---:|
| x264 livré (étape 4, 3 runs × 5 min) | 25,8 | 29,2 | 19,9 | 2,9 | 2,37 | 1,8–2,1 cœurs |
| x264 `threads = 6` (au lieu de 2 imposés par `txproto/src/encode.c:89`) | 25,0 | 30,8 | 18,7 | 3,2 | 2,41 | 2,0 |
| **`encoder = "amf"`** (h264_amf `ultralowlatency`, D3D11 zero-copy, 20 Mbps) | **4,0** | **10,9** | **1,9** | **0,02** | 1,08 | 1,0 (dont le thread de scrutation Spout) |

AMF : 3 594 / 3 600 IDs, jointure 0 ambiguë, recoupement = F0, 0 frame
jetée. Qualité visuelle et stabilité sur 10 min **non vérifiées**.
