#!/bin/bash
# Build a single self-contained KyberFrog-Setup.exe.
#
# Stages kyberfrog.exe + the Kyber fork binaries (kycontroller / kyavserver /
# kyclient + their DLLs + the libVLC plugins\) into one folder, then runs NSIS
# to produce a double-click installer that adds itself to PATH. No file is left
# scattered: every output lands under dist/.
#
# Runs entirely inside kyber/debian-win64:local (cargo + makensis both present).
# Mount the WORKSPACE ROOT (not just apps/KyberFrog) so the sibling fork bundle
# is reachable:
#
#   docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
#     bash apps/KyberFrog/packaging/build-installer.sh
#
# Options:
#   -f <path>   Fork binaries bundle: a folder, or a .zip (extracted for you).
#               Default: apps/kyber-desktop/kyberfrog-spout-e2e[.zip].
#   -v <ver>    Version string (default: git describe, else 0.0.0-dev).
#   -o <path>   Output dir for the setup .exe (default: <KyberFrog>/dist).
#   -s          Skip the cargo build; reuse an already-built kyberfrog.exe.
#   -h          Help.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KYBERFROG_DIR="$(dirname "$SCRIPT_DIR")"            # apps/KyberFrog
APPS_DIR="$(dirname "$KYBERFROG_DIR")"              # apps
TARGET="x86_64-pc-windows-gnu"
EXE_REL="target/$TARGET/release/kyberfrog.exe"

FORK_BUNDLE=""
VERSION=""
OUTPUT_DIR="$KYBERFROG_DIR/dist"
SKIP_CARGO=false

usage() { sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

while getopts "f:v:o:sh" opt; do
    case $opt in
        f) FORK_BUNDLE="$OPTARG" ;;
        v) VERSION="$OPTARG" ;;
        o) OUTPUT_DIR="$OPTARG" ;;
        s) SKIP_CARGO=true ;;
        h) usage 0 ;;
        *) usage 1 ;;
    esac
done

# --- locate the fork binaries bundle ---------------------------------------
if [ -z "$FORK_BUNDLE" ]; then
    if [ -d "$APPS_DIR/kyber-desktop/kyberfrog-spout-e2e" ]; then
        FORK_BUNDLE="$APPS_DIR/kyber-desktop/kyberfrog-spout-e2e"
    elif [ -f "$APPS_DIR/kyber-desktop/kyberfrog-spout-e2e.zip" ]; then
        FORK_BUNDLE="$APPS_DIR/kyber-desktop/kyberfrog-spout-e2e.zip"
    else
        echo "ERROR: no fork bundle found. Pass one with -f <dir|zip>." >&2
        echo "       Build it via apps/kyber-desktop/build-win32.sh -p." >&2
        exit 1
    fi
fi

# --- version ----------------------------------------------------------------
# SSOT is Cargo.toml's [workspace.package] version; a v<version> git tag cuts a
# release. On an exact tag, use it (e.g. v0.1.0); otherwise a dev build is
# <cargo-version>-<short-sha> so it's tied to the declared version.
cargo_version() {
    grep -E '^version = ' "$KYBERFROG_DIR/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
}
if [ -z "$VERSION" ]; then
    if ! VERSION="$(git -C "$KYBERFROG_DIR" describe --tags --exact-match 2>/dev/null)"; then
        SHA="$(git -C "$KYBERFROG_DIR" rev-parse --short HEAD 2>/dev/null || echo nogit)"
        VERSION="$(cargo_version)-$SHA"
    fi
fi

echo "==> KyberFrog installer build"
echo "    version:     $VERSION"
echo "    fork bundle: $FORK_BUNDLE"
echo "    output:      $OUTPUT_DIR"

# --- staging area -----------------------------------------------------------
STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

# 1) the whole fork bundle (binaries + DLLs + plugins\ + certs) — the exact set
#    proven to run end-to-end. Trimming unused tools is a later optimisation.
echo "==> Staging fork binaries..."
if [ -d "$FORK_BUNDLE" ]; then
    cp -a "$FORK_BUNDLE/." "$STAGING/"
else
    unzip -q "$FORK_BUNDLE" -d "$STAGING/_zip"
    # The zip wraps everything in one top-level folder; flatten it.
    inner="$(find "$STAGING/_zip" -mindepth 1 -maxdepth 1 -type d | head -1)"
    cp -a "${inner:-$STAGING/_zip}/." "$STAGING/"
    rm -rf "$STAGING/_zip"
fi
if [ ! -f "$STAGING/kyclient.exe" ] || [ ! -d "$STAGING/plugins" ]; then
    echo "ERROR: fork bundle is missing kyclient.exe or plugins/." >&2
    exit 1
fi

# 2) build kyberfrog.exe (unless reusing a prior build)
if [ "$SKIP_CARGO" = false ]; then
    echo "==> Building kyberfrog.exe (cargo --release --target $TARGET)..."
    ( cd "$KYBERFROG_DIR" && cargo build --release --target "$TARGET" )
fi
if [ ! -f "$KYBERFROG_DIR/$EXE_REL" ]; then
    echo "ERROR: $KYBERFROG_DIR/$EXE_REL not found (build it, or drop -s)." >&2
    exit 1
fi
cp "$KYBERFROG_DIR/$EXE_REL" "$STAGING/kyberfrog.exe"

# 3) installer-side assets
echo "==> Staging installer assets..."
cp "$KYBERFROG_DIR/kyberfrog/assets/kyberfrog.ico"      "$STAGING/kyberfrog.ico"
cp "$KYBERFROG_DIR/kyberfrog/install/install-kyberfrog.ps1" "$STAGING/install-kyberfrog.ps1"
cp "$KYBERFROG_DIR/COPYING.AGPLv3"                      "$STAGING/license.txt"
cp "$SCRIPT_DIR/windows/INSTALL.md"                     "$STAGING/INSTALL.md"

# Strip the bundled fork's own icon so only kyberfrog.ico ships (cosmetic).
rm -f "$STAGING/kyber.ico"

# --- run NSIS ---------------------------------------------------------------
mkdir -p "$OUTPUT_DIR"
OUTPUT_NAME="KyberFrog-Setup-$VERSION.exe"
echo "==> Running makensis -> $OUTPUT_NAME"
makensis -V3 \
    -DPRODUCT_VERSION="$VERSION" \
    -DSTAGING_DIR="$STAGING" \
    -DOUTPUT_DIR="$OUTPUT_DIR" \
    -DOUTPUT_NAME="$OUTPUT_NAME" \
    "$SCRIPT_DIR/windows/kyberfrog.nsi"

echo ""
echo "==> Done: $OUTPUT_DIR/$OUTPUT_NAME"
ls -lh "$OUTPUT_DIR/$OUTPUT_NAME"
