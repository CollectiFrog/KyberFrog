# Banc de latence KyberFrog vs NDI (#28-4)

Méthode et conclusions. Comment lancer le banc :
[`bench/README.md`](../../bench/README.md). Données :
[`bench/runs/comparaison-k-ndi/`](../../bench/runs/comparaison-k-ndi/README.md).

## Résultat

Ryzen 5 5600X, Radeon RX 7800 XT, 1080p60, 20 Mbps, boucle locale, contenu
synthétique par défaut.

| Configuration | Médiane | 99 sur 100 sous | Images perdues |
|---|---:|---:|---:|
| KyberFrog, encodeur GPU AMF | **3,9 ms** | 5,9 ms | 0,044 % |
| KyberFrog, x264 (livré avant 0.6.0) | 25,8 ms | 31,6 ms | 0,69 % ⚠ |
| NDI SDK 6.3.2 | 15 à 26 ms selon le contenu | 29,4 ms au pire contenu | 0 % |
| Plancher de l'instrument | 0,03 ms | 0,05 ms | 0 % |

⚠ Ce taux de perte est un artefact du fond synthétique, pas une propriété de
KyberFrog — voir « Trouvé au passage » et l'annexe sur clip réel.

Deux conclusions :

1. **L'encodeur décide de tout.** Passer de x264 à AMF fait tomber la chaîne de
   25,8 à 3,9 ms, soit d'une image et demie à un quart d'image à 60 Hz.
   L'encodage passe de 19,9 ms (77 % du total) à 1,9 ms, et le download GPU→CPU
   disparaît : kyavservice branche les frames D3D11 de la capture directement
   sur `h264_amf`. C'est ce qui a motivé la release 0.6.0 (#28-1).
2. **KyberFrog en AMF est stable, NDI dépend du contenu.** NDI compresse chaque
   image séparément : selon le mouvement à l'écran il livre entre 15,0 et
   26,0 ms, et sur le cas alterné une image sur deux en 13,5 ms et l'autre en
   27,2 ms. KyberFrog ne bouge pas avec le contenu.

## Frontière de mesure

L'instrument publie une image numérotée et note l'heure d'émission ; à l'autre
bout il relit le numéro et note l'heure d'arrivée.

- **KyberFrog** : Spout → kyavserver → kycontroller → kyclient → Spout. Tout le
  trajet.
- **NDI** : mêmes images, envoi SDK → réception SDK → texture GPU.
  **Pas de pont Spout en sortie**, donc NDI est mesuré sur un trajet plus court
  que KyberFrog. L'asymétrie va contre KyberFrog : le chiffre annoncé ne peut
  pas être accusé de charger NDI. Le transfert vers le GPU, 1,2 ms, est compté
  du côté NDI.
- On n'écrit jamais « NDI sans ses ponts » face à « KyberFrog complet ».

Le numéro d'image est encodé en gros blocs redondants, lisibles après
compression, et vérifié en double (deux zones, `id_a` / `id_b`) : une image dont
le numéro est douteux est jetée plutôt que comptée de travers.

**Les horloges sont comparables** : QPC en microsecondes côté kyproto, VLC et
txproto, sur une seule machine, offset de synchronisation mesuré ≤ 19 µs.

## L'instrument

`bench/kybench/`, en Rust, hors du workspace principal : il réimplémente le
registre Spout plutôt que de dépendre de la chaîne de forks (1 h 30 de build
évitée). Il tient le chemin critique temporel — générateur cadencé, sonde,
codec de numéro, émetteur et récepteur NDI. Python reste hors du chemin
temporel : lancement des processus, inventaire, analyse.

La sonde Spout lit le compteur de frames sans le toucher (`NtQuerySemaphore`).
Une lecture façon SDK ferait capturer des frames fantômes à kyavserver.

Le plancher de l'instrument est à 0,03 ms, soit trois millièmes du plus petit
chiffre publié. Il n'est jamais retranché des résultats.

## Trouvé au passage

- **Un cœur brûlé en permanence par la scrutation Spout.** kyavserver tient
  1,8–2,1 cœurs pendant un run, dont un thread à 0,94–0,98 cœur. Générateur
  ralenti à 1 fps : ce thread reste à 0,999 cœur. En source écran (duplication
  DXGI) le processus tombe à 0,25 cœur, aucun thread au-dessus de 0,04. La
  scrutation Spout occupe donc un cœur quel que soit le débit.
- **Des images perdues en sortie sous x264, sur les à-coups de contenu.**
  L'affichage de l'image *n* est bloqué jusqu'à la fin du décodage de *n+1*, et
  la texture publiée contient alors *n+1*. Le verrou en cause n'est pas
  identifié. Le taux dépend entièrement de ce qui est joué : 0,69 % sur le fond
  synthétique, dont la boucle fait une coupure franche à chaque tour, mais
  **0,028 % sur un clip VJ réel**, qui n'en a pas. Ce n'est donc pas un taux de
  perte de KyberFrog, c'est la réaction de la chaîne à une rupture d'image. En
  AMF, 0,044 % sur le fond synthétique et 0 % sur le clip.
- **x264 est bridé à 2 threads** (`txproto/src/encode.c:89`). Le passer à 6 ne
  gagne que 0,8 ms — sans commune mesure avec le passage à AMF.
- **QUIC signale ~30 paquets perdus par seconde en boucle locale**, côté serveur.
- **Les ~6 premières secondes d'une session sortent avec 4 à 6 s de retard**
  (backlog de démarrage). D'où la chauffe avant toute mesure.
- **Le crash AMF « en boucle silencieuse » n'est pas reproduit** avec le bundle
  actuel : 10 min à 60 fps, aucun redémarrage, arrêt propre. C'est ce qui a
  débloqué #28-1.
- **Le transport NDI n'explique pas ses variations.** Forcer TCP, UDP, RUDP ou
  l'adaptateur loopback donne le même motif à 0,3 ms près ; les modes d'envoi
  sync, async et clocké aussi. C'est bien le contenu.

## Annexe : contenu VJ réel

Les chiffres de tête sont sur fond synthétique. Pour vérifier qu'ils ne sont pas
un artefact de ce fond, les trois configurations ont été rejouées sur un clip VJ
(`Tunel_01_movie1.mov`, boucle de 2 s, mise à l'échelle en 1080p), un run de
60 s chacune, `bench/runs/comparaison-k-ndi/clip-2026-09-16/`.

| Configuration | Clip réel | Fond synthétique | Images perdues, clip |
|---|---:|---:|---:|
| KyberFrog AMF | **4,07 ms** | 3,9 ms | 0 % |
| NDI | 21,73 ms | 15 à 26 ms | 0 % |
| KyberFrog x264 | 30,23 ms | 25,8 ms | 0,028 % |

Trois enseignements :

1. **AMF ne bouge pas** : 3,9 → 4,07 ms. C'est la conclusion centrale, et elle
   tient sur du contenu réel.
2. **NDI tombe à 21,7 ms**, à l'intérieur de la fourchette 15–26 annoncée : la
   fourchette encadre bien le réel.
3. **x264 se dégrade** : 30,2 ms au lieu de 25,8, p99 42,7 au lieu de 31,6. Sur
   le fond synthétique il était à égalité avec NDI ; sur un vrai clip il passe
   derrière (30,2 contre 21,7). L'écart AMF / x264 monte à 7,4×.

Réserves : le clip est du 720p remonté en 1080p et rejoué en boucle, donc moins
de détail fin qu'un 1080p natif — NDI et x264 y sont sans doute un peu flattés.
Un run par configuration, indicatif.

## Ce qui n'est pas mesuré

- **Une seule machine, boucle locale.** Pas de réseau réel, pas de seconde
  machine. Le transport NDI entre deux processus d'une même machine peut
  différer de son transport réseau.
- **Un seul GPU.** Rien n'est mesuré sur carte NVIDIA (NVENC).
- **Qualité visuelle non contrôlée.** Les 20 Mbps sont tenus des deux côtés,
  aucune comparaison d'image n'a été faite.
- **Contenu synthétique pour les chiffres de tête** ; un clip VJ réel est en
  annexe, mais sur un seul run par configuration.
- **Un seul transmetteur**, une seule source Spout.

## Hors périmètre

Décidé le 2026-09-16, après le résultat ci-dessus : le banc s'arrête au
lancement manuel. Ne sont pas faits, et ne sont pas prévus —

- orchestrateur, campagne automatique, runs de nuit, rapport généré ;
- comparaison par ponts Spout ↔ NDI (config « N »), qui mesurerait NDI sur
  exactement la même frontière que KyberFrog ;
- second récepteur NDI par libNoIDea, prévu pour le NDI natif de KyberFrog ;
- Phase B sur deux machines.

Le banc a répondu à la question qui le motivait — combien coûte la chaîne
KyberFrog, et comment elle se situe face à NDI. Le reste relève d'une
optimisation qui n'est pas à l'ordre du jour.
