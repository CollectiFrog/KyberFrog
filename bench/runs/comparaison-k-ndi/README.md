# Comparaison KyberFrog vs NDI — résultat publié

Le résultat unique qui alimente le tableau du [README du projet](../../../README.md#latency)
et de [docs/dev/bench-latency.md](../../docs/dev/bench-latency.md).

Poste : Ryzen 5 5600X, Radeon RX 7800 XT, Windows 11 25H2 (`env.json`).
Bundle KyberFrog `643ee0e`, source Spout 1080p60, 20 Mbps, boucle locale.
NDI SDK 6.3.2, réception UYVY, High Bandwidth, sans audio.

## Ce qui est mesuré

L'instrument publie une image numérotée sur un sender Spout et note l'heure
(`t_pub`). À l'autre bout il relit le numéro et note l'heure d'arrivée. La
latence publiée est l'écart entre les deux, image par image.

- **KyberFrog** : Spout → chaîne complète (kyavserver, kycontroller, kyclient)
  → Spout. Tout le trajet, ponts compris.
- **NDI** : mêmes images envoyées par le SDK NDI, reçues par le SDK NDI et
  montées dans une texture GPU. **Il n'y a pas de pont Spout en sortie** : NDI
  est mesuré plus court que KyberFrog, donc avantagé. C'est délibéré — le
  chiffre annoncé ne peut pas être accusé de charger NDI.
- **Plancher (`f0`)** : l'instrument seul, sans rien entre les deux bouts.
  0,03 ms — négligeable devant tout le reste.

## Chiffres publiés (contenu par défaut)

| Configuration | Moitié des images sous | 99 sur 100 sous | Images perdues | Source |
|---|---:|---:|---:|---|
| KyberFrog, encodeur GPU AMF | **3,9 ms** | 5,9 ms | 0,044 % | `k-amf/`, 10 min, 35 984 images |
| KyberFrog, x264 (avant 0.6.0) | 25,8 ms | 31,6 ms | 0,69 % | `k-x264/k-default-*`, 60 s |
| NDI (SDK 6.3.2) | 26,0 ms | 29,4 ms | 0 % | `verif-2026-09-16/ndi/`, 20 s |
| Plancher de l'instrument | 0,03 ms | 0,05 ms | 0 % | `verif-2026-09-16/f0/` |

## Le contenu de l'image change NDI, pas KyberFrog

NDI compresse chaque image séparément ; son coût dépend de ce qu'il y a à
l'écran. Même fond, trois mouvements :

| Fond | KyberFrog x264 | NDI |
|---|---:|---:|
| blocs alignés sur la grille de compression 16×16 | 26,2 ms | 15,0 ms |
| blocs décalés d'un demi-bloc | 25,9 ms | 19,6 ms |
| défaut, alterne à chaque image | 25,8 ms | 26,0 ms |

Sur le cas alterné, NDI livre une image sur deux en 13,5 ms et l'autre en
27,2 ms — d'où une médiane à 26 ms. Le transport n'y est pour rien : forcer
TCP, UDP, RUDP ou l'adaptateur loopback donne le même motif à 0,3 ms près, et
les modes d'envoi sync / async / clocké aussi.

**Conséquence** : NDI est annoncé entre 15 et 26 ms selon le contenu, KyberFrog
en AMF à 3,9 ms quel que soit le contenu.

## Reprise du 2026-09-16 (`verif-2026-09-16/`)

Les quatre configurations relancées avec [`latency_bench.py`](../../latency_bench.py),
runs courts (20 à 30 s), contenu par défaut, pour vérifier que l'outil publié
reproduit les chiffres ci-dessus :

| Configuration | p50 | p95 | p99 | pire | perte |
|---|---:|---:|---:|---:|---:|
| `f0` | 0,03 ms | 0,05 | 0,05 | 0,28 | 0 % |
| `k-amf` | 3,97 ms | 4,49 | 5,20 | 17,6 | 0 % |
| `k-x264` | 26,8 ms | 28,1 | 29,0 | 38,5 | 0,78 % |
| `ndi` | 25,97 ms | 28,4 | 29,4 | 31,2 | 0 % |

Écart au run long : ≤ 1 ms. Les runs courts n'ont pas la durée nécessaire pour
un p99 ou un taux de perte stable — ils valident l'outil, pas les chiffres.

En NDI, l'image est reçue à 24,76 ms et montée en texture GPU à 25,97 ms :
le transfert vers le GPU coûte 1,2 ms, compté dans le chiffre NDI publié.

## Ce que ces chiffres ne disent pas

- **Une seule machine, une seule boucle locale.** Pas de réseau réel, pas de
  seconde machine, pas d'autre GPU. Sur carte NVIDIA (NVENC) rien n'est mesuré.
- **Qualité visuelle non contrôlée.** Les 20 Mbps sont tenus des deux côtés,
  mais aucune comparaison d'image n'a été faite.
- **Contenu synthétique.** Le fond est un damier pseudo-aléatoire, pas un clip
  VJ réel.
- Les runs `k-amf`, `k-x264` et `ndi` ont été produits par des scripts
  d'exploration depuis remplacés par `latency_bench.py`. Les CSV bruts sont
  conservés (`*.csv.gz`), les chiffres restent recalculables.
