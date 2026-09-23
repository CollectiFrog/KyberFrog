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
#   -a <arch>   Architecture du bundle : amd64 (défaut) ou arm64
#   -s <path>   Checkout kyber-desktop à utiliser (défaut : ../kyber-desktop)
#   -o <path>   Où déposer le bundle produit (défaut : <kyberfrog>/dist)
#   -i <image>  Image de build (défaut : kyber/debian-linux:local[-arm64])
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
#
#   # Bundle arm64 (émulé, une nuit)
#   packaging/linux/build-fork-local.sh -a arm64 -b
#
# arm64 : le build tourne dans un conteneur linux/arm64 **émulé** par qemu
# (binfmt de Docker Desktop), faute de machine ARM à la CI comme sur le poste.
# C'est lent — plusieurs fois la durée d'un build amd64 — mais sans limite de
# temps, là où un runner SaaS coupe à 3 h. Le bundle produit est ensuite poussé
# **une fois** dans le Generic Package Registry à la clé du SHA kyber-desktop ;
# la CI ne fait plus que le cache hit (docs/dev/releasing.md § Bundle fork
# arm64). La commande d'upload est imprimée en fin de build.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # packaging/linux
KYBERFROG_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"        # racine kyberfrog
WORKSPACE_DIR="$(dirname "$KYBERFROG_DIR")"

SOURCE_DIR="$WORKSPACE_DIR/kyber-desktop"
OUTPUT_DIR="$KYBERFROG_DIR/dist"
ARCH="amd64"
IMAGE=""
BUILD_IMAGE=false
FRESH=false
CHECK_ONLY=false

usage() { sed -n '2,52p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

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

while getopts "a:s:o:i:bfch" opt; do
    case $opt in
        a) ARCH="$OPTARG" ;;
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

# --- architecture -----------------------------------------------------------
# Chaque arch a son image et son volume : les deux builds coexistent sur le
# poste sans se marcher dessus, et sans réinvalider le cache contrib de l'autre.
case "$ARCH" in
    amd64) PLATFORM="linux/amd64"; FORK_ARCH="x86_64"  ;;
    arm64) PLATFORM="linux/arm64"; FORK_ARCH="aarch64" ;;
    *) echo "ERROR: arch '$ARCH' inconnue (amd64 ou arm64)." >&2; exit 1 ;;
esac
if [ "$ARCH" = "amd64" ]; then
    IMAGE="${IMAGE:-kyber/debian-linux:local}"
    VOLUME="kyberfrog-forkbuild"
else
    IMAGE="${IMAGE:-kyber/debian-linux:local-$ARCH}"
    VOLUME="kyberfrog-forkbuild-$ARCH"
fi

if [ ! -d "$SOURCE_DIR/.git" ]; then
    echo "ERROR: pas de checkout kyber-desktop en $SOURCE_DIR (passer -s)." >&2
    exit 1
fi

KD_SHA="$(git -C "$SOURCE_DIR" rev-parse HEAD)"

echo "==> Build fork Linux (local)"
echo "    sources : $SOURCE_DIR ($(git -C "$SOURCE_DIR" rev-parse --short HEAD))"
echo "    arch    : $ARCH ($PLATFORM) -> kyber-linux-$FORK_ARCH.tar.bz2"
echo "    image   : $IMAGE"
echo "    volume  : $VOLUME"
echo "    sortie  : $OUTPUT_DIR"
if [ "$ARCH" != "amd64" ]; then
    echo "    NOTE    : conteneur $PLATFORM émulé (qemu) — compter plusieurs heures."
fi

# --- image ------------------------------------------------------------------
if [ "$BUILD_IMAGE" = true ] || ! docker_run image inspect "$IMAGE" >/dev/null 2>&1; then
    echo "==> docker build $IMAGE"
    docker_run build --platform "$PLATFORM" -t "$IMAGE" \
        "$(host_path "$KYBERFROG_DIR/ops/docker-images/debian-linux")"
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
docker_run run --rm --platform "$PLATFORM" \
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
    docker_run run --rm --platform "$PLATFORM" \
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

echo "==> build-linux.sh -p (long : ~1h30 à froid en amd64, logs en direct)"
docker_run run --rm --platform "$PLATFORM" \
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

BUNDLE="$OUTPUT_DIR/kyber-linux-$FORK_ARCH.tar.bz2"

echo ""
echo "==> Bundle déposé dans $OUTPUT_DIR :"
ls -lh "$BUNDLE"

# Le bundle arm64 n'est jamais construit par la CI (voir l'en-tête) : c'est CET
# artefact que le job `build-fork-linux-arm64` télécharge. Tant qu'il n'est pas
# dans le registre à la clé du SHA, la chaîne arm64 échoue en disant quoi faire
# — autant imprimer la commande ici, SHA déjà résolu.
if [ "$ARCH" != "amd64" ]; then
    cat <<EOF

==> Publier ce bundle pour la CI (une fois par SHA kyber-desktop) :

    PROJECT_ID=<id du projet kyberfrog sur gitlab.com>
    TOKEN=<personal access token, scope api>
    curl --fail --header "PRIVATE-TOKEN: \$TOKEN" \\
      --upload-file "$BUNDLE" \\
      "https://gitlab.com/api/v4/projects/\$PROJECT_ID/packages/generic/kyberfrog-fork-bundle-linux-arm64/$KD_SHA/kyber-linux-$FORK_ARCH.tar.bz2"

    SHA kyber-desktop : $KD_SHA
    (c'est la valeur à mettre dans packaging/versions.sh)
EOF
fi
