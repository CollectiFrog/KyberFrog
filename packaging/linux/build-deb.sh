#!/bin/bash
# Build a self-contained KyberFrog .deb (Debian/Ubuntu).
#
# Stages the kyberfrog binary + the built web UI (ui/dist) + the Kyber fork
# Linux bundle (kycontroller / kyavserver / kyclient + their .so) into a package
# tree, then runs dpkg-deb. The result is one .deb that drops everything under
# /usr/lib/kyberfrog, symlinks /usr/bin/kyberfrog, and installs a systemd *user*
# service enabled for all users (autostart at graphical login).
#
# Mirror of packaging/build-installer.sh (the Windows installer), targeting
# Linux instead of MinGW + NSIS. Mount the WORKSPACE ROOT so the sibling fork
# bundle under apps/kyber-desktop is reachable:
#
#   docker run --rm -v "${PWD}:/work" -w /work <linux-build-image> \
#     bash apps/KyberFrog/packaging/linux/build-deb.sh -f <fork-bundle>
#
# Options:
#   -f <path>   Fork Linux bundle: a directory, or a .tar.bz2/.tar.gz/.zip
#               (produced by apps/kyber-desktop/build-linux.sh -p; extracted for
#               you). Default: apps/kyber-desktop/kyber-linux-<arch>[.tar.bz2].
#   -v <ver>    Version string (default: git describe, else 0.0.0-dev).
#   -a <arch>   Debian arch: amd64 (default) or arm64.
#   -t <triple> Rust target triple (default derived from -a).
#   -o <path>   Output dir for the .deb (default: <KyberFrog>/dist).
#   -s          Skip the cargo build; reuse an already-built binary.
#   -h          Help.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # packaging/linux
PACKAGING_DIR="$(dirname "$SCRIPT_DIR")"                     # packaging
KYBERFROG_DIR="$(dirname "$PACKAGING_DIR")"                  # apps/KyberFrog
APPS_DIR="$(dirname "$KYBERFROG_DIR")"                       # apps

FORK_BUNDLE=""
VERSION=""
ARCH="amd64"
TARGET=""
OUTPUT_DIR="$KYBERFROG_DIR/dist"
SKIP_CARGO=false

usage() { sed -n '2,33p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

while getopts "f:v:a:t:o:sh" opt; do
    case $opt in
        f) FORK_BUNDLE="$OPTARG" ;;
        v) VERSION="$OPTARG" ;;
        a) ARCH="$OPTARG" ;;
        t) TARGET="$OPTARG" ;;
        o) OUTPUT_DIR="$OPTARG" ;;
        s) SKIP_CARGO=true ;;
        h) usage 0 ;;
        *) usage 1 ;;
    esac
done

# --- arch / target ----------------------------------------------------------
case "$ARCH" in
    amd64) FORK_ARCH="x86_64";  DEFAULT_TARGET="x86_64-unknown-linux-gnu" ;;
    arm64) FORK_ARCH="aarch64"; DEFAULT_TARGET="aarch64-unknown-linux-gnu" ;;
    *) echo "ERROR: unsupported arch '$ARCH' (use amd64 or arm64)." >&2; exit 1 ;;
esac
TARGET="${TARGET:-$DEFAULT_TARGET}"
EXE_REL="target/$TARGET/release/kyberfrog"

# --- locate the fork bundle -------------------------------------------------
if [ -z "$FORK_BUNDLE" ]; then
    for cand in \
        "$APPS_DIR/kyber-desktop/kyber-linux-$FORK_ARCH" \
        "$APPS_DIR/kyber-desktop/kyber-linux-$FORK_ARCH.tar.bz2"; do
        if [ -e "$cand" ]; then FORK_BUNDLE="$cand"; break; fi
    done
fi
if [ -z "$FORK_BUNDLE" ] || [ ! -e "$FORK_BUNDLE" ]; then
    echo "ERROR: no fork bundle found. Pass one with -f <dir|archive>." >&2
    echo "       Build it via apps/kyber-desktop/build-linux.sh -p." >&2
    exit 1
fi

# --- version ----------------------------------------------------------------
# Same SSOT as the Windows installer: the git tag. Cargo.toml version is only
# the offline fallback.
cargo_version() {
    grep -E '^version = ' "$KYBERFROG_DIR/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
}
if [ -z "$VERSION" ]; then
    if ! VERSION="$(git -C "$KYBERFROG_DIR" describe --tags --exact-match 2>/dev/null)"; then
        SHA="$(git -C "$KYBERFROG_DIR" rev-parse --short HEAD 2>/dev/null || echo nogit)"
        VERSION="$(cargo_version)-$SHA"
    fi
fi
# Debian version field: no leading 'v'; '-' separates the (optional) revision,
# so a dev string like 0.2.2-3-gabc must use a different separator. '~' sorts
# before everything, which is correct for a pre-release of the next tag.
DEB_VERSION="$(echo "${VERSION#v}" | tr '-' '~')"

echo "==> KyberFrog .deb build"
echo "    version:     $VERSION (deb: $DEB_VERSION)"
echo "    arch/target: $ARCH / $TARGET"
echo "    fork bundle: $FORK_BUNDLE"
echo "    output:      $OUTPUT_DIR"

# --- resolve the fork bundle to a directory ---------------------------------
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [ -d "$FORK_BUNDLE" ]; then
    FORK_DIR="$FORK_BUNDLE"
else
    mkdir -p "$WORK/fork"
    case "$FORK_BUNDLE" in
        *.tar.bz2) tar xjf "$FORK_BUNDLE" -C "$WORK/fork" ;;
        *.tar.gz)  tar xzf "$FORK_BUNDLE" -C "$WORK/fork" ;;
        *.zip)     unzip -q "$FORK_BUNDLE" -d "$WORK/fork" ;;
        *) echo "ERROR: unknown bundle type: $FORK_BUNDLE" >&2; exit 1 ;;
    esac
    # The archive wraps everything in one top-level folder; descend into it.
    FORK_DIR="$(find "$WORK/fork" -mindepth 1 -maxdepth 1 -type d | head -1)"
    FORK_DIR="${FORK_DIR:-$WORK/fork}"
fi
if [ ! -d "$FORK_DIR/bin" ] || [ ! -d "$FORK_DIR/lib" ]; then
    echo "ERROR: fork bundle has no bin/ or lib/ (got $FORK_DIR)." >&2
    exit 1
fi

# --- build kyberfrog --------------------------------------------------------
if [ "$SKIP_CARGO" = false ]; then
    echo "==> Building kyberfrog (cargo --release --target $TARGET)..."
    export KYBERFROG_VERSION="$VERSION"
    ( cd "$KYBERFROG_DIR" && cargo build --release --target "$TARGET" )
fi
if [ ! -f "$KYBERFROG_DIR/$EXE_REL" ]; then
    echo "ERROR: $KYBERFROG_DIR/$EXE_REL not found (build it, or drop -s)." >&2
    exit 1
fi

# --- web UI -----------------------------------------------------------------
UI_DIST="$KYBERFROG_DIR/ui/dist"
if [ ! -f "$UI_DIST/index.html" ]; then
    if command -v npm >/dev/null 2>&1; then
        echo "==> Building web UI (npm ci && npm run build)..."
        ( cd "$KYBERFROG_DIR/ui" && npm ci && npm run build )
    else
        echo "ERROR: web UI not built ($UI_DIST/index.html missing) and npm not found." >&2
        exit 1
    fi
fi

# --- assemble the package tree ----------------------------------------------
# /usr/lib/kyberfrog/bin  -> kyberfrog + fork binaries (+ ui/dist next to it,
#                            matching web.rs::ui_dist = <exe_dir>/ui/dist)
# /usr/lib/kyberfrog/lib  -> fork shared libraries
# /usr/bin/kyberfrog      -> symlink into bin (current_exe() resolves it, so
#                            default_install_dir lands next to kycontroller)
echo "==> Assembling package tree..."
TREE="$WORK/pkg"
BINDIR="$TREE/usr/lib/kyberfrog/bin"
LIBDIR="$TREE/usr/lib/kyberfrog/lib"
mkdir -p "$BINDIR" "$LIBDIR" \
    "$TREE/usr/bin" \
    "$TREE/usr/lib/systemd/user" \
    "$TREE/usr/lib/udev/rules.d" \
    "$TREE/usr/lib/modules-load.d" \
    "$TREE/usr/share/doc/kyberfrog" \
    "$TREE/DEBIAN"

cp -a "$FORK_DIR/bin/." "$BINDIR/"
cp -a "$FORK_DIR/lib/." "$LIBDIR/"
for b in kyclient kycontroller kyavserver; do
    if [ ! -f "$BINDIR/$b" ]; then
        echo "ERROR: fork bundle is missing bin/$b." >&2
        exit 1
    fi
done

cp "$KYBERFROG_DIR/$EXE_REL" "$BINDIR/kyberfrog"
mkdir -p "$BINDIR/ui/dist"
cp -a "$UI_DIST/." "$BINDIR/ui/dist/"

ln -sf ../lib/kyberfrog/bin/kyberfrog "$TREE/usr/bin/kyberfrog"
cp "$SCRIPT_DIR/kyberfrog.service" "$TREE/usr/lib/systemd/user/kyberfrog.service"

# Remote control injects clicks and keystrokes through /dev/uinput, which Debian
# ships root-only: without these two the pointer moves on the remote machine and
# nothing else ever happens — no click, no keystroke, no error either.
cp "$SCRIPT_DIR/99-kyberfrog-uinput.rules" "$TREE/usr/lib/udev/rules.d/99-kyberfrog-uinput.rules"
cp "$SCRIPT_DIR/uinput.conf" "$TREE/usr/lib/modules-load.d/kyberfrog-uinput.conf"

# docs
[ -f "$KYBERFROG_DIR/README.md" ] && cp "$KYBERFROG_DIR/README.md" "$TREE/usr/share/doc/kyberfrog/"
[ -f "$KYBERFROG_DIR/COPYING.AGPLv3" ] && cp "$KYBERFROG_DIR/COPYING.AGPLv3" "$TREE/usr/share/doc/kyberfrog/copyright"

# --- dependencies -----------------------------------------------------------
# Computed, never hand-written: the binaries pull in EGL/GL, VA-API, ALSA,
# libinput, several xcb-* … and a hand-maintained list drifts silently. The
# symptom is nasty — the package installs cleanly, then kyavserver and kyclient
# refuse to start on a missing .so.
#
# -l puts the *bundled* library dirs on the search path so dpkg-shlibdeps
# resolves our own .so locally instead of hunting for a package that owns them;
# --ignore-missing-info keeps it from failing over those same unpackaged libs.
LIBDIRS=("$LIBDIR")
while IFS= read -r d; do LIBDIRS+=("$d"); done < <(find "$LIBDIR" -mindepth 1 -maxdepth 1 -type d)

# The bundled .so are analysed too, not just the executables: the system
# dependencies (GL/EGL, ALSA, VA-API, libinput…) are pulled in by libtxproto and
# the FFmpeg/VLC libraries, not by the binaries directly. Analysing only bin/
# yields a control file that lists libc and little else.
BINARIES=()
while IFS= read -r b; do BINARIES+=("$b"); done < <(
    find "$BINDIR" -maxdepth 1 -type f -executable
    find "$LIBDIR" -type f -name '*.so*'
)

DEPENDS=""
if command -v dpkg-shlibdeps >/dev/null 2>&1; then
    echo "==> Computing dependencies with dpkg-shlibdeps..."
    SHLIB_ARGS=()
    for d in "${LIBDIRS[@]}"; do SHLIB_ARGS+=("-l$d"); done
    # dpkg-shlibdeps insists on a debian/control next to it; give it a stub.
    mkdir -p "$WORK/shlib/debian"
    printf 'Source: kyberfrog\n\nPackage: kyberfrog\nArchitecture: %s\n' "$ARCH" \
        > "$WORK/shlib/debian/control"
    if DEPENDS=$( cd "$WORK/shlib" && dpkg-shlibdeps -O --ignore-missing-info \
                    "${SHLIB_ARGS[@]}" "${BINARIES[@]}" 2>/dev/null \
                  | sed -n 's/^shlibs:Depends=//p' ); then
        echo "    -> $DEPENDS"
    fi
fi
if [ -z "$DEPENDS" ]; then
    echo "WARNING: dpkg-shlibdeps unavailable or failed; falling back to a minimal list." >&2
    DEPENDS="libc6"
fi

# control + maintainer scripts
INSTALLED_SIZE="$(du -ks "$TREE/usr" | cut -f1)"
# @DEPENDS@ is substituted line-wise, not with sed: the generated list contains
# `|` alternatives (libjack-jackd2-0 | libjack-0.125) and parentheses, which
# collide with every sed delimiter worth using.
sed -e "s/@VERSION@/$DEB_VERSION/" -e "s/@ARCH@/$ARCH/" "$SCRIPT_DIR/control" \
| while IFS= read -r line; do
    if [ "$line" = "Depends: @DEPENDS@" ]; then
        printf 'Depends: %s\n' "$DEPENDS"
    else
        printf '%s\n' "$line"
    fi
done > "$TREE/DEBIAN/control"
echo "Installed-Size: $INSTALLED_SIZE" >> "$TREE/DEBIAN/control"
for script in postinst prerm postrm; do
    cp "$SCRIPT_DIR/$script" "$TREE/DEBIAN/$script"
    chmod 0755 "$TREE/DEBIAN/$script"
done

# --- build the .deb ---------------------------------------------------------
mkdir -p "$OUTPUT_DIR"
OUTPUT_NAME="kyberfrog_${DEB_VERSION}_${ARCH}.deb"
echo "==> Running dpkg-deb -> $OUTPUT_NAME"
dpkg-deb --build --root-owner-group "$TREE" "$OUTPUT_DIR/$OUTPUT_NAME"

echo ""
echo "==> Done: $OUTPUT_DIR/$OUTPUT_NAME"
ls -lh "$OUTPUT_DIR/$OUTPUT_NAME"
