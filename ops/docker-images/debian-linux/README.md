# Image de build Linux amd64

Pendant Linux de l'image `debian-win64` du job `build-fork`. Elle contient la
toolchain complète pour compiler la chaîne de forks Kyber sur Linux (contrib
ffmpeg / VLC / txproto + crates Rust) **et** pour empaqueter le `.deb` (P3).

Publiée dans le registry du projet sous
`registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-amd64`.

## Construire et pousser — depuis la CI

Le job **`image-debian-linux`** (stage `build`) fait tout, avec **Kaniko** plutôt
que `docker build` : le runner de ce projet est l'executor Kubernetes, où
`docker:dind` a échoué (`Cannot connect to the Docker daemon` — il faudrait un
pod privilégié que le runner n'accorde pas). Kaniko construit depuis un
`Dockerfile` sans démon Docker, dans un pod non privilégié ; le `Dockerfile`
lui-même n'a pas changé. Le job pousse deux tags : `latest-amd64` et le SHA
court du commit.

Il est **manuel**, `allow_failure: true`, et ne tourne que sur `dev`, sur MR ou
sur tag : il ne peut ni partir tout seul ni bloquer un pipeline. Le lancer
depuis l'UI GitLab (pipeline → bouton ▶ sur le job) après toute modification du
`Dockerfile`. (Sur `feat/linux-support`, le temps du rodage de cette chaîne, il
part automatiquement à chaque push — voir le bloc marqué dans `.gitlab-ci.yml`.)

Une fois l'image poussée, le job `build-fork-linux` peut tourner.

## Construire à la main (optionnel)

Utile seulement pour itérer vite sur le `Dockerfile` ; la CI reste la référence.

```bash
docker build -t kyber/debian-linux:local ops/docker-images/debian-linux
```

## Ce qui est figé dedans, et pourquoi

| Choix | Raison |
|---|---|
| Base `debian:trixie-slim` | Même famille que les images upstream (`ops/docker-images/debian-trixie`), inaccessibles depuis un token CI kyber-frog. |
| Liste apt | Copiée du README de kyber-desktop (§ *Debian: Install required system packages*) — la seule liste tenue à jour. Complétée par `dpkg-dev`/`fakeroot`/`lintian` pour le `.deb`, et les outils de décompression/patch du contrib. |
| `apt-get build-dep vlc` | Exigé par le README. Impose d'activer les dépôts `deb-src`, fait en tête de Dockerfile (format deb822 de Trixie). |
| `meson>=1.10` par pip | kymedia ≥ 0.27 l'exige et Debian livre plus ancien. Le binaire pip masque celui d'apt — c'est le contournement que le job CI Windows appliquait à la volée, figé ici. |
| Rust `1.89.0` + `cargo-c@0.10.15` | Versions pinnées par le README de kyber-desktop. |
| Pas de Node | Le dashboard est construit par le job `build-ui` (image `node:22-alpine`). |

## Pourquoi Kaniko et pas `docker:dind`

`docker:dind` est plus simple à lire mais demande un pod privilégié — normal
sur les runners partagés GitLab.com, pas garanti sur un runner Kubernetes
maison comme celui de ce projet (constaté : le service ne démarre pas, le job
échoue avant même `docker build`). [Kaniko](https://docs.gitlab.com/ci/docker/using_kaniko/)
construit l'image depuis le `Dockerfile` sans démon Docker du tout, donc sans
rien demander au runner. Les identifiants passent par
`/kaniko/.docker/config.json`, généré dans `before_script` à partir du couple
`$CI_REGISTRY_USER`/`$CI_REGISTRY_PASSWORD` que GitLab injecte — pas de
`docker login`, Kaniko n'a pas de CLI Docker.

## arm64

Hors périmètre pour l'instant (voir § 7 du plan). Le jour venu, cette image se
décline en `:latest-arm64` en la construisant sur un runner arm64 natif — le
`Dockerfile` n'a pas besoin de changer.
