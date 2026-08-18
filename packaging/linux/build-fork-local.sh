#!/bin/bash
# Build the Kyber fork Linux bundle **on this machine**, in the debian-linux
# image, instead of burning a CI runner for it.
#
# Pourquoi local : un build fork complet coûte ~1h30, et le cache CI est keyé
# sur le SHA entier de kyber-desktop — le moindre commit fork invalide tout.
# Itérer via la CI, c'est 1h30 par essai, sans accès aux logs intermédiaires.
# En local on a les logs docker en direct, le cache cargo/contrib qui persiste
# entre les runs, et une machine généralement plus rapide que le runner.
#
# La CI reste la référence pour l'artefact *officiel* (SHA pinné dans
# packaging/versions.sh) ; ce script est l'outil de la boucle de dev.
#
# Le build tourne dans un **volume docker**, pas dans le bind mount : sur
# Windows/macOS un bind mount est catastrophique en I/O pour un arbre de build
# de cette taille (contrib ffmpeg/VLC = des centaines de milliers de fichiers).
# Les sources y sont copiées une fois, puis réutilisées d'un run à l'autre.
#
# Usage:
#   packaging/linux/build-fork-local.sh [options]
#
#   -s <path>   Checkout kyber-desktop à utiliser (défaut : ../kyber-desktop)
#   -o <path>   Où déposer le bundle produit (défaut : <kyberfrog>/dist)
#   -i <image>  Image de build (défaut : kyber/debian-linux:local)
#   -b          (Re)construire l'image avant le build
#   -f          Repartir de zéro : efface le volume et recopie les sources
#   -c          Vérifier seulement (cargo check du workspace, pas de build complet)
#   -h          Aide
#
# Exemples:
#   # Première fois : construire l'image puis tout builder
#   packaging/linux/build-fork-local.sh -b
#
#   # Boucle de dev : juste re-vérifier que ça compile (quelques minutes)
#   packaging/linux/build-fork-local.sh -c
#
#   # Repartir propre après un rebase de la chaîne de forks
#   packaging/linux/build-fork-local.sh -f

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # packaging/linux
KYBERFROG_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"        # racine kyberfrog
WORKSPACE_DIR="$(dirname "$KYBERFROG_DIR")"

SOURCE_DIR="$WORKSPACE_DIR/kyber-desktop"
OUTPUT_DIR="$KYBERFROG_DIR/dist"
IMAGE="kyber/debian-linux:local"
BUILD_IMAGE=false
FRESH=false
CHECK_ONLY=false
VOLUME="kyberfrog-forkbuild"

usage() { sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

# --- Git Bash / MSYS ---------------------------------------------------------
# Sous Git Bash, deux réécritures cassent docker silencieusement :
#   * les chemins *hôte* `/c/Users/...` doivent devenir `C:/Users/...`,
#   * les chemins *conteneur* (`/src`, `/build`, `-w /build/...`) sont convertis
#     en chemins Windows alors qu'ils doivent rester tels quels.
# `cygpath -m` règle le premier, `MSYS_NO_PATHCONV=1` le second.
host_path() {
    if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi
}
docker_run() { MSYS_NO_PATHCONV=1 docker "$@"; }

while getopts "s:o:i:bfch" opt; do
    case $opt in
        s) SOURCE_DIR="$OPTARG" ;;
        o) OUTPUT_DIR="$OPTARG" ;;
        i) IMAGE="$OPTARG" ;;
        b) BUILD_IMAGE=true ;;
        f) FRESH=true ;;
        c) CHECK_ONLY=true ;;
        h) usage 0 ;;
        *) usage 1 ;;
    esac
done

if [ ! -d "$SOURCE_DIR/.git" ]; then
    echo "ERROR: pas de checkout kyber-desktop en $SOURCE_DIR (passer -s)." >&2
    exit 1
fi

echo "==> Build fork Linux (local)"
echo "    sources : $SOURCE_DIR ($(git -C "$SOURCE_DIR" rev-parse --short HEAD))"
echo "    image   : $IMAGE"
echo "    volume  : $VOLUME"
echo "    sortie  : $OUTPUT_DIR"

# --- image ------------------------------------------------------------------
if [ "$BUILD_IMAGE" = true ] || ! docker_run image inspect "$IMAGE" >/dev/null 2>&1; then
    echo "==> docker build $IMAGE"
    docker_run build -t "$IMAGE" "$(host_path "$KYBERFROG_DIR/ops/docker-images/debian-linux")"
fi

# --- volume -----------------------------------------------------------------
if [ "$FRESH" = true ]; then
    echo "==> Volume $VOLUME effacé (-f)"
    docker_run volume rm "$VOLUME" >/dev/null 2>&1 || true
fi
docker_run volume create "$VOLUME" >/dev/null

# Les sources (working trees des submodules compris, .git inclus) sont copiées
# dans le volume : aucune plomberie git, pas de clé SSH dans le conteneur, pas
# de re-clone réseau des repos privés.
#
# Les artefacts régénérables du checkout Windows sont exclus — sans ça on
# recopierait plusieurs Go de rootfs mingw et de target/ à travers le bind
# mount, pour rien. Ils n'ont aucune influence sur un build Linux.
echo "==> Synchronisation des sources vers le volume"
docker_run run --rm \
    -v "$(host_path "$SOURCE_DIR"):/src:ro" \
    -v "$VOLUME:/build" \
    "$IMAGE" \
    bash -c '
        set -e
        mkdir -p /build/kyber-desktop
        echo "    copie (artefacts de build exclus)..."
        tar -C /src -cf - \
            --exclude="./target" \
            --exclude="./rootfs-*" \
            --exclude="./kyberfrog-fork-bundle*" \
            --exclude="./contrib/work" \
            --exclude="*/contrib/work" \
            --exclude="*/target" \
            --exclude="*.log" \
            . | tar -C /build/kyber-desktop -xf -
        git config --global --add safe.directory "*"
        echo "    HEAD dans le volume : $(git -C /build/kyber-desktop rev-parse --short HEAD)"
    '

# --- build ------------------------------------------------------------------
mkdir -p "$OUTPUT_DIR"

if [ "$CHECK_ONLY" = true ]; then
    echo "==> cargo check (kyavservice — la crate qui cassait en Linux)"
    docker_run run --rm \
        -v "$VOLUME:/build" \
        -w /build/kyber-desktop \
        "$IMAGE" \
        bash -c '
            set -e
            git config --global --add safe.directory "*"
            cd kysdk/kymedia
            cargo check -p kyavservice --all-targets 2>&1 | tail -40
        '
    echo "==> Check terminé."
    exit 0
fi

echo "==> build-linux.sh -p (long : ~1h30 à froid, logs en direct)"
docker_run run --rm \
    -v "$VOLUME:/build" \
    -v "$(host_path "$OUTPUT_DIR"):/out" \
    -w /build/kyber-desktop \
    "$IMAGE" \
    bash -c '
        set -e
        git config --global --add safe.directory "*"
        ./build-linux.sh -p
        cp -v kyber-linux-*.tar.bz2 /out/
    '

echo ""
echo "==> Bundle déposé dans $OUTPUT_DIR :"
ls -lh "$OUTPUT_DIR"/kyber-linux-*.tar.bz2
