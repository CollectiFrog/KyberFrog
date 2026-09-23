#!/usr/bin/env bash
# dev.sh — one entry point for the KyberFrog dev environment.
#
# Every build runs in Docker. This script owns the `docker run` lines: which
# image, which directory to mount, and the Git Bash path rewriting that breaks
# docker when typed by hand. It pulls a missing build image on first use.
#
# Usage: ./dev.sh <command> [args]        (PowerShell / cmd: .\dev <command>)
#
#   setup [--fork]    Prepare this machine: check Docker, pull the Windows build
#                     image, fetch the pinned fork bundle, build the dashboard.
#                     --fork also checks out the Kyber fork chain into
#                     vendor/kyber-desktop, for working on the fork itself.
#   test [args]       cargo test --workspace --locked (Linux target: Win32 code
#                     is stubbed, so this is the fast loop)
#   check             cargo check for the Windows target (compiles Win32 code)
#   ui                Build the dashboard into ui/dist
#   exe               Build target/x86_64-pc-windows-gnu/release/kyberfrog.exe
#   bundle [arch]     Fetch the pinned fork bundle: win64 (default),
#                     linux-amd64 or linux-arm64
#   installer [args]  Build dist/KyberFrog-Setup-<version>.exe
#                     (args go to packaging/build-installer.sh)
#   deb [args]        Build dist/kyberfrog_<version>_amd64.deb
#                     (args go to packaging/linux/build-deb.sh)
#   docs              Serve the documentation on http://localhost:8000
#
# See docs/dev/setup.md.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# Build images, published in this project's registry and pullable without a
# login. The packaging scripts and docs use the local names on the left.
WIN64_IMAGE="kyber/debian-win64:local"
WIN64_REMOTE="registry.gitlab.com/kyber-frog/kyberfrog/debian-win64:latest"
LINUX_IMAGE="kyber/debian-linux:local"
LINUX_REMOTE="registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-amd64"
# Fork builds only: kymedia >= 0.27 needs meson >= 1.10, the win64 image ships
# 1.7. Derived locally until that image is rebuilt.
FORK_IMAGE="kyber/debian-win64:local-0.27"
NODE_IMAGE="node:22-alpine"
DOCS_IMAGE="squidfunk/mkdocs-material"

if [ -t 2 ]; then C_OK=$'\033[1;32m' C_ERR=$'\033[1;31m' C_END=$'\033[0m'; else C_OK="" C_ERR="" C_END=""; fi
log() { printf '%s==>%s %s\n' "$C_OK" "$C_END" "$*" >&2; }
die() { printf '%sERROR:%s %s\n' "$C_ERR" "$C_END" "$*" >&2; exit 1; }

# Git Bash rewrites paths behind docker's back: host paths must become
# C:/… (cygpath -m) and container paths (/work) must not be touched at all
# (MSYS_NO_PATHCONV). Both are no-ops on Linux.
host_path() {
    if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi
}
docker_cmd() { MSYS_NO_PATHCONV=1 docker "$@"; }

# Run in <image> with the repo mounted on /work.
in_image() {
    local image="$1"; shift
    docker_cmd run --rm -v "$(host_path "$ROOT"):/work" -w /work "$image" "$@"
}

require_docker() {
    command -v docker >/dev/null 2>&1 ||
        die "docker not found. Install Docker Desktop (WSL2 backend on Windows)."
    docker info >/dev/null 2>&1 ||
        die "Docker is installed but not running. Start Docker Desktop and retry."
}

# Pull <remote> and tag it <local>, unless <local> already exists — a locally
# built image is never replaced.
ensure_image() {
    local local_tag="$1" remote="$2"
    docker image inspect "$local_tag" >/dev/null 2>&1 && return 0
    log "Pulling $remote (first time only — several GB)"
    docker pull "$remote"
    docker tag "$remote" "$local_tag"
}

ensure_fork_image() {
    docker image inspect "$FORK_IMAGE" >/dev/null 2>&1 && return 0
    ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"
    log "Deriving $FORK_IMAGE (meson >= 1.10 for the fork build)"
    printf '%s\n' \
        "FROM $WIN64_IMAGE" \
        "RUN apt-get update && apt-get install -y python3-pip && \\" \
        "    python3 -m pip install --break-system-packages 'meson>=1.10'" |
        docker build -t "$FORK_IMAGE" -
}

ensure_ui() {
    [ -f ui/dist/index.html ] || cmd_ui
}

# ---------------------------------------------------------------- commands

cmd_setup() {
    local fork=false
    case "${1:-}" in
        --fork) fork=true ;;
        "") ;;
        *) die "setup: unknown option '$1' (only --fork)" ;;
    esac

    [ "$fork" = true ] && check_fork_path
    require_docker
    log "Docker OK ($(docker version --format '{{.Server.Version}}' 2>/dev/null))"
    ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"
    log "Fetching the fork bundle pinned by vendor/kyber-desktop"
    packaging/fork-bundle.sh -a win64 >/dev/null
    cmd_ui

    if [ "$fork" = true ]; then
        setup_fork
    fi

    cat >&2 <<'EOF'

Ready. Next:
  ./dev.sh test         run the test suite (~3-4 min the first time)
  ./dev.sh installer    build dist/KyberFrog-Setup-<version>.exe
  ./dev.sh help         every command
EOF
}

# The chain nests deep: .git/modules/vendor/kyber-desktop/modules/… reaches
# ~175 characters below the repo root. core.longpaths lifts Windows' 260 limit
# for most of git, but not for its HTTPS transport, which rejects a longer
# $GIT_DIR outright — so the repo root itself must stay short. Checked before
# anything is downloaded.
check_fork_path() {
    case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) ;; *) return 0 ;; esac
    local root; root="$(host_path "$ROOT")"
    if [ "${#root}" -gt 80 ]; then
        die "the repo root is ${#root} characters long ($root). The fork chain nests ~175 more, and git's HTTPS transport stops at 260. Clone into a directory of 80 characters at most, e.g. C:/dev/kyberfrog."
    fi
}

setup_fork() {
    # `-c` options reach every nested clone of the submodule update.
    local opts=() windows=false https=false
    case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) windows=true ;; esac

    # Windows' 260-character limit, for the rest of git (see check_fork_path).
    if [ "$windows" = true ]; then
        opts+=(-c core.longpaths=true)
        git config core.longpaths true
    fi

    # The fork's nested submodules use SSH URLs. Without a GitLab SSH key,
    # clone them over HTTPS instead. Pushing to the fork still needs a key.
    if ! GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=10" \
            git ls-remote git@gitlab.com:kyber-frog/kyber-desktop.git HEAD >/dev/null 2>&1; then
        log "No SSH access to gitlab.com: cloning the fork chain over HTTPS"
        opts+=(-c "url.https://gitlab.com/.insteadOf=git@gitlab.com:")
        https=true
    fi

    log "Checking out the fork chain into vendor/kyber-desktop (~1 min, ~740 MB)"
    git "${opts[@]}" submodule update --init --recursive --jobs 4 vendor/kyber-desktop
    if [ "$windows" = true ]; then
        git submodule foreach --recursive --quiet 'git config core.longpaths true'
    fi
    ensure_fork_image

    cat >&2 <<'EOF'

Fork chain checked out at the pin. Every sub-repo sits on a detached HEAD:
put the one you edit on its fork branch first, e.g.
  git -C vendor/kyber-desktop/kysdk/kyctl switch kyberfrog-dev
EOF
    if [ "$https" = true ]; then
        cat >&2 <<'EOF'

The chain was cloned over HTTPS, but its remotes stay git@gitlab.com: URLs.
For later fetches, either add an SSH key to GitLab, or make the rewrite
permanent for your account:
  git config --global url."https://gitlab.com/".insteadOf "git@gitlab.com:"
EOF
    fi
}

cmd_test() {
    require_docker; ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"
    in_image "$WIN64_IMAGE" cargo test --workspace --locked "$@"
}

cmd_check() {
    require_docker; ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"
    in_image "$WIN64_IMAGE" cargo check --workspace --locked --target x86_64-pc-windows-gnu "$@"
}

cmd_ui() {
    require_docker
    log "Building the dashboard (ui/dist)"
    docker_cmd run --rm -v "$(host_path "$ROOT/ui"):/ui" -w /ui "$NODE_IMAGE" \
        sh -c "npm ci --no-audit --no-fund && npm run build"
}

cmd_exe() {
    require_docker; ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"
    in_image "$WIN64_IMAGE" cargo build --release --locked --target x86_64-pc-windows-gnu "$@"
}

cmd_bundle() {
    local path
    path="$(packaging/fork-bundle.sh -a "${1:-win64}")"
    printf '%s\n' "$(host_path "$path")"
}

cmd_installer() {
    require_docker; ensure_image "$WIN64_IMAGE" "$WIN64_REMOTE"; ensure_ui
    in_image "$WIN64_IMAGE" bash packaging/build-installer.sh "$@"
}

cmd_deb() {
    require_docker; ensure_image "$LINUX_IMAGE" "$LINUX_REMOTE"; ensure_ui
    in_image "$LINUX_IMAGE" bash packaging/linux/build-deb.sh "$@"
}

cmd_docs() {
    require_docker
    log "Docs on http://localhost:8000 (Ctrl-C to stop)"
    docker_cmd run --rm --init -p 8000:8000 -v "$(host_path "$ROOT"):/docs" "$DOCS_IMAGE" \
        serve -a 0.0.0.0:8000
}

cmd_help() { sed -n '2,27p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

# ---------------------------------------------------------------- dispatch

command="${1:-help}"
[ $# -gt 0 ] && shift
case "$command" in
    setup|test|check|ui|exe|bundle|installer|deb|docs|help) "cmd_$command" "$@" ;;
    -h|--help) cmd_help ;;
    *) cmd_help >&2; die "unknown command '$command'" ;;
esac
