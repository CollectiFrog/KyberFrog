#!/bin/bash
# Fetch the Kyber fork bundle that CI built for the pinned kyber-desktop SHA.
#
# KyberFrog depends on the fork as *binaries* (kycontroller / kyavserver /
# kyclient + their libraries), never as source. The CI fork jobs build that
# bundle once per kyber-desktop SHA and store it in this project's public
# Generic Package Registry; this script downloads the one matching the gitlink
# `vendor/kyber-desktop` — no credentials, no fork checkout, no 1 h 30 build.
#
# Usage:
#   packaging/fork-bundle.sh [-a <arch>] [-o <dir>]
#
#   -a <arch>   win64 (default), linux-amd64 or linux-arm64
#   -o <dir>    Cache root (default: <kyberfrog>/dist/fork-bundle). The bundle
#               lands in <dir>/<sha>/, so a pin bump never reuses a stale one.
#   -h          Help.
#
# Prints the path of the downloaded archive on stdout, logs on stderr — so a
# script can do `FORK_BUNDLE="$(packaging/fork-bundle.sh -a win64)"`.
#
# A bundle built from a local fork checkout is not fetched: pass it to the
# packaging scripts with -f.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # packaging
KYBERFROG_DIR="$(dirname "$SCRIPT_DIR")"

ARCH="win64"
CACHE_DIR="$KYBERFROG_DIR/dist/fork-bundle"

usage() { sed -n '2,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

while getopts "a:o:h" opt; do
    case $opt in
        a) ARCH="$OPTARG" ;;
        o) CACHE_DIR="$OPTARG" ;;
        h) usage 0 ;;
        *) usage 1 >&2 ;;
    esac
done

# Package and file names: keep in sync with the FORK_PACKAGE* / FORK_BUNDLE*
# variables of .gitlab-ci.yml, which upload under exactly these keys.
case "$ARCH" in
    win64)       PACKAGE="kyberfrog-fork-bundle";             FILE="kyberfrog-fork-bundle.zip" ;;
    linux-amd64) PACKAGE="kyberfrog-fork-bundle-linux-amd64"; FILE="kyber-linux-x86_64.tar.bz2" ;;
    linux-arm64) PACKAGE="kyberfrog-fork-bundle-linux-arm64"; FILE="kyber-linux-aarch64.tar.bz2" ;;
    *) echo "fork-bundle: unknown arch '$ARCH' (win64, linux-amd64 or linux-arm64)." >&2; exit 1 ;;
esac

# shellcheck source=versions.sh
source "$SCRIPT_DIR/versions.sh"

# URL-encoded project path; override to fetch from a fork of this project.
PROJECT="${KYBERFROG_PROJECT:-kyber-frog%2Fkyberfrog}"
URL="https://gitlab.com/api/v4/projects/$PROJECT/packages/generic/$PACKAGE/$KYBER_DESKTOP_REF/$FILE"
DEST="$CACHE_DIR/$KYBER_DESKTOP_REF/$FILE"

if [ -s "$DEST" ]; then
    echo "fork-bundle: $ARCH @ ${KYBER_DESKTOP_REF:0:8} already in $DEST" >&2
else
    echo "fork-bundle: downloading $ARCH @ ${KYBER_DESKTOP_REF:0:8}..." >&2
    mkdir -p "$(dirname "$DEST")"
    # Download beside the target, then rename: an interrupted run never leaves
    # a truncated archive that the cache check above would take as complete.
    if ! curl --fail --location --silent --show-error -o "$DEST.part" "$URL"; then
        rm -f "$DEST.part"
        rmdir "$(dirname "$DEST")" 2>/dev/null || true
        cat >&2 <<EOF
fork-bundle: no $ARCH bundle in the registry for kyber-desktop $KYBER_DESKTOP_REF.

  The pin (gitlink vendor/kyber-desktop) points at a SHA the CI fork job has not
  built yet — it publishes the bundle on its first pipeline after the bump.
  Until then, build the fork locally and pass the result with -f:
    win64:        vendor/kyber-desktop/build-win32.sh -p   (see docs/dev/building.md)
    linux-*:      packaging/linux/build-fork-local.sh [-a arm64]

  Tried: $URL
EOF
        exit 1
    fi
    mv "$DEST.part" "$DEST"
fi

printf '%s\n' "$DEST"
