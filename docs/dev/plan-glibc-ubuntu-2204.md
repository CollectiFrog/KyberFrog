# Abaisser le plancher glibc — cible Ubuntu 22.04

**État : étude, non engagée.** Ce document chiffre ce que coûterait le support
d'Ubuntu 22.04 (glibc 2.35) et de Debian 12 (2.36), aujourd'hui exclus par le
plancher **glibc ≥ 2.39**. Rien n'a été modifié ni testé : Docker n'était pas
disponible sur le poste d'analyse.

Périmètre actuel et raisons du plancher :
[Installation](../user/installation.md#linux-debian-ubuntu-raspberry-pi-os),
[plan-linux-amd64.md](plan-linux-amd64.md).

## Le constat

Le plancher n'est pas un choix de code mais un **héritage de l'image de build**
(`FROM debian:trixie-slim`, `ops/docker-images/debian-linux/Dockerfile`). Un
binaire ELF fige à la compilation la version glibc la plus haute qu'il référence ;
aucun réglage a posteriori (`RUSTFLAGS`, option de `build-deb.sh`, variable
d'environnement) ne peut l'abaisser.

Mesuré sur un build local (`objdump -T`) :

| Binaire | glibc max requis |
| --- | --- |
| `kyberfrog` | **2.39** |
| `kycontroller` | **2.39** |
| `kyclient` | **2.39** |
| `kyavserver` | 2.34 |

**Deux symboles seulement** portent l'exigence 2.39, dans `kyberfrog` comme dans
`kyclient` :

```
pidfd_spawnp@GLIBC_2.39
pidfd_getpid@GLIBC_2.39
```

Ils viennent de `std::process` : depuis **Rust 1.89** — précisément la version
pinnée par l'image (`RUST_VERSION=1.89.0`) — la bibliothèque standard utilise
`posix_spawn` avec pidfd dès que la glibc de *build* expose ces symboles.

### Pourquoi « symbole faible » ne sauve pas

Les deux symboles sont `WEAK` dans `.dynsym`, ce qui laisse croire qu'un
chargement sur glibc ancienne dégraderait proprement. Ce n'est pas le cas :
l'entrée correspondante de `.gnu.version_r` porte `Flags: none`, pas `WEAK`.
Le *loader* exige donc `GLIBC_2.39` au chargement, **même si le code n'est jamais
exécuté**. Le binaire ne démarre pas sur Ubuntu 22.04.

Conséquence directe : ce n'est pas un chantier de ce dépôt. C'est un chantier
**image de build + recompilation de la chaîne de forks Kyber**.

## Les deux voies

### Voie A — rebaser l'image de build (complète)

Changer la base et **tout recompiler**, KyberFrog et le fork.

| Base candidate | glibc | Couvre |
| --- | --- | --- |
| `debian:bookworm-slim` | 2.36 | Debian 12+, Ubuntu 22.10+ — **pas** 22.04 |
| `ubuntu:22.04` | 2.35 | Ubuntu 22.04+, Debian 12+ |

Seule `ubuntu:22.04` atteint la cible demandée. À noter : un binaire construit
sur base Ubuntu reste installable sur Debian, la glibc étant
forward-compatible — le `.deb` recalcule de toute façon ses dépendances via
`dpkg-shlibdeps`, il n'y a donc pas de dépendance Debian figée à corriger.

**Le risque n'est pas la glibc, c'est la liste de paquets.** L'image actuelle
dépend de Trixie pour des versions récentes ; sur une base 2 à 3 ans plus
ancienne, chaque ligne est à revalider. Points chauds relevés dans le
Dockerfile :

- `meson >= 1.10` exigé par kymedia ≥ 0.27 — **déjà contourné** par
  `pip install 'meson>=1.10'`, donc probablement sans douleur.
- `apt-get build-dep -y vlc` — tire les dépendances VLC **de la distribution** :
  c'est la ligne la plus exposée, le contenu diffère fortement entre Jammy et
  Trixie.
- `glslang-dev`, `libvulkan-dev`, `wayland-protocols`, `libinput-dev`,
  `liblcms2-dev` — versions nettement plus anciennes sur Jammy ; à vérifier
  contre les exigences du contrib ffmpeg/VLC.
- `lua5.4` / `liblua5.4-dev` — présents sur Jammy, à confirmer.

Ces versions n'ont **pas** pu être vérifiées (pas de Docker sur le poste) :
c'est l'inconnue principale du chantier.

**Rust — piste testée, et écartée.** L'hypothèse initiale était que pinner
`RUST_VERSION` à 1.88 (avant l'introduction de pidfd dans `std`) suffirait.
**C'est faux, vérifié le 2026-09-20** : `cargo +1.88.0 build --release`
compile proprement mais produit exactement les mêmes symboles —

```
objdump -T target/release/kyberfrog | grep pidfd
  w DF *UND* (GLIBC_2.39) pidfd_spawnp
  w DF *UND* (GLIBC_2.39) pidfd_getpid
```

— et le `.gnu.version_r` garde `Name: GLIBC_2.39  Flags: none`. Les symboles ne
viennent pas de la version de rustc mais de la **glibc de la machine de build** :
`std` détecte à l'édition de liens que `libc.so.6` les expose et s'y lie. Seul
un changement de base (glibc plus ancienne) les supprime.

Conséquence : il n'existe **aucun raccourci**. Le rebase de l'image est la seule
voie, et la base doit descendre à 2.35 (`ubuntu:22.04`) — Bookworm (2.36) ne
couvre pas 22.04.

**Coût.** Un build fork complet est documenté à **~1h30 par essai**
(`packaging/linux/build-fork-local.sh`). Compter plusieurs itérations avant que
le contrib passe sur une base ancienne. La CI (`image-debian-linux`, job Kaniko)
reconstruit l'image ; la boucle de mise au point se fait en local.

**Étapes.**

1. ~~Tester `RUST_VERSION=1.88`~~ — **fait, sans effet** (voir ci-dessus).
   Commencer directement à l'étape 2.
2. Construire une image `ubuntu:22.04` avec la même liste de paquets ; relever
   tout ce qui ne s'installe pas ou régresse en version.
3. Compiler la chaîne fork dessus ; traiter les échecs du contrib
   (ffmpeg/VLC/txproto) un à un.
4. Vérifier le plancher obtenu sur **les quatre** binaires, pas seulement
   `kyberfrog`.
5. Produire le `.deb`, l'installer sur une VM Ubuntu 22.04 vierge, valider E2E
   (transmetteur écran + viewer), comme il a été fait pour Debian 13.
6. Décider : le plancher bas devient-il **la** cible unique, ou faut-il **deux**
   chaînes (une 22.04, une Trixie) ? Deux chaînes doublent le temps CI et la
   surface de test — à éviter sans raison forte.
7. Mettre à jour `installation.md`, `README.md` et `plan-linux-amd64.md`.

### Voie B — build natif sur 22.04 (demi-solution, non recommandée)

Compiler KyberFrog directement sur une machine Ubuntu 22.04 (`cargo` + `npm`
suffisent, cf. la boucle de dev validée). Cela résout le binaire `kyberfrog`,
mais **pas** `kycontroller` ni `kyclient`, qui restent en 2.39.

Résultat : l'application démarre, le dashboard répond, et **aucun transmetteur
ni viewer ne peut se lancer**. Sans intérêt opérationnel, sauf pour travailler
sur l'orchestrateur seul.

## Recommandation

Avant d'engager la voie A, **vérifier que 22.04 est vraiment nécessaire**.
Ubuntu 24.04 LTS est supportée jusqu'en 2029 et fonctionne aujourd'hui sans
aucune modification ; une mise à niveau du poste cible coûte très probablement
moins qu'une seconde chaîne de build à maintenir.

Si 22.04 est imposée (parc figé, matériel contraint), il faut assumer le rebase
complet sur `ubuntu:22.04` : le raccourci par la version de Rust a été testé et
ne fonctionne pas. Le coût réel est la recompilation de la chaîne de forks sur
une base plus ancienne (~1h30 par itération), avec `apt-get build-dep vlc` comme
principal point d'incertitude.

**À ne pas confondre avec le chantier arm64** (`feat/arm64-triplet` côté fork) :
celui-ci dé-code en dur le triplet d'architecture
(`x86_64-linux-gnu` → `$(uname -m)-linux-gnu`) et ne touche pas au plancher
glibc. Les deux problèmes sont orthogonaux — le travail arm64 n'est d'aucun
secours pour 22.04 en x86_64, et réciproquement.
