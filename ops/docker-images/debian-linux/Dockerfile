# KyberFrog — image de build Linux amd64 (P1 du plan docs/dev/plan-linux-amd64.md).
#
# Pendant Linux de l'image `debian-win64` utilisée par le job `build-fork` :
# elle contient tout ce qu'il faut pour compiler la chaîne de forks Kyber
# (contrib ffmpeg/VLC/txproto + les crates Rust) et pour empaqueter le `.deb`.
#
# L'upstream kyber.stream a ses propres images (`ops/docker-images/debian-trixie`
# pinnée par SHA dans le .gitlab-ci.yml de kyber-desktop) mais elles vivent dans
# un registry privé qu'un token CI kyber-frog ne peut pas lire — d'où celle-ci,
# construite à partir de la liste de paquets documentée dans le README de
# kyber-desktop (§ Linux / Debian), qui est la seule source de vérité à jour.
#
# Construite et poussée **par la CI** (job `image-debian-linux`), pas à la main :
# voir README.md à côté.

FROM debian:trixie-slim

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8

# `apt-get build-dep vlc` (plus bas) exige les dépôts source. Trixie déclare ses
# sources au format deb822 dans debian.sources ; on couvre aussi le vieux format
# au cas où l'image de base changerait.
RUN set -eux; \
    if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
        sed -i 's/^Types: deb$/Types: deb deb-src/' /etc/apt/sources.list.d/debian.sources; \
    fi; \
    if [ -f /etc/apt/sources.list ]; then \
        sed -i 's/^deb \(.*\)$/deb \1\ndeb-src \1/' /etc/apt/sources.list; \
    fi

# Liste reprise telle quelle du README de kyber-desktop (§ Debian: Install
# required system packages), plus :
#   - ca-certificates / xz-utils / bzip2 / patch / file : décompression et
#     patching du contrib (tarballs ffmpeg, patches contrib/ffmpeg/000N-*.patch),
#   - dpkg-dev / fakeroot / lintian : construction et contrôle du .deb (P3),
#   - python3-pip : installe le meson récent exigé par kymedia (voir plus bas).
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        build-essential cmake meson ninja-build pkg-config \
        llvm-dev libclang-dev clang libssl-dev \
        autoconf libtool curl git wget unzip \
        python3 python3-setuptools python3-mako python3-venv \
        python3-jinja2 python3-pip \
        lua5.4 liblua5.4-dev \
        autopoint gettext bison flex nasm ant ragel help2man \
        libdrm-dev libgbm-dev glslang-dev liblcms2-dev libpulse-dev \
        libxcb-xkb-dev libxkbcommon-dev libxcb-xtest0-dev \
        libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
        libwayland-dev wayland-protocols libvulkan-dev \
        libevdev-dev libudev-dev libinput-dev \
        ca-certificates xz-utils bzip2 patch file \
        dpkg-dev fakeroot lintian; \
    apt-get build-dep -y vlc; \
    rm -rf /var/lib/apt/lists/*

# kymedia ≥ 0.27 exige meson >= 1.10 ; Debian (Trixie comprise) en livre un plus
# ancien. Le meson pip s'installe dans /usr/local/bin et masque celui d'apt —
# exactement le contournement que le job CI Windows applique à la volée, figé
# ici une fois pour toutes.
RUN python3 -m pip install --break-system-packages --no-cache-dir 'meson>=1.10'

# Rust : version et cargo-c pinnés par le README de kyber-desktop. cargo-c est
# requis par le contrib (libs C construites depuis des crates Rust).
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.89.0
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION"; \
    rustup component add clippy rustfmt; \
    cargo install --locked cargo-c@0.10.15; \
    rm -rf "$CARGO_HOME/registry" "$CARGO_HOME/git"; \
    chmod -R a+w "$CARGO_HOME" "$RUSTUP_HOME"

# Le node du job `build-ui` reste séparé (image node:22-alpine) : cette image ne
# construit que le natif.

RUN set -eux; \
    rustc --version; \
    cargo --version; \
    meson --version; \
    cmake --version | head -1

WORKDIR /work
