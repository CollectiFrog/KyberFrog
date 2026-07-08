#!/usr/bin/env bash
# fork-lint.sh — pre-flight checks for the Kyber fork chain.
#
# Verifies, for every repo of the chain, the invariants whose violation has
# already broken release pipelines (see docs/dev/audit-fork-chain.md §4):
#
#   1. the COMMITTED .gitmodules URL matches the submodule's local origin —
#      a fresh CI clone fetches the committed URL, local remotes lie;
#   2. every gitlink pin is reachable from a branch (or tag) on origin,
#      checked after a fresh `fetch --prune` — stale tracking refs lie too;
#   3. working trees are clean (WARN only: local build dirt is common).
#
# Usage: fork-lint.sh [fork-root] [--offline]
#   fork-root  path to the kyber-desktop checkout
#              (default: ../../kyber-desktop relative to this script)
#   --offline  skip fetches: faster, but reachability results may be stale
#
# Exit code: 0 all OK (warnings allowed), 1 at least one FAIL.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/../../kyber-desktop"
OFFLINE=0
for arg in "$@"; do
    case "$arg" in
        --offline) OFFLINE=1 ;;
        *) ROOT="$arg" ;;
    esac
done
ROOT="$(cd "$ROOT" && pwd)" || { echo "FAIL: fork root not found"; exit 1; }

FAILS=0
WARNS=0
ok()   { echo "  OK    $1"; }
warn() { echo "  WARN  $1"; WARNS=$((WARNS+1)); }
fail() { echo "  FAIL  $1"; FAILS=$((FAILS+1)); }

# Normalize a git URL to host/path for comparison: ssh scp-like and https
# forms of the same repo must compare equal, trailing .git ignored.
normalize_url() {
    local u="$1"
    u="${u#https://}"; u="${u#http://}"
    u="${u#ssh://git@}"; u="${u#git@}"
    u="${u/:/\/}"
    u="${u%.git}"
    # collapse a possible double slash from ssh://host:port forms
    echo "$u"
}

# Resolve a relative submodule URL (../x.git) against the parent's URL,
# the same way git does (strip one path component per "../").
resolve_url() {
    local base="$1" rel="$2"
    case "$rel" in
        ../*) ;;
        *) echo "$rel"; return ;;
    esac
    base="${base%.git}"
    while [ "${rel#../}" != "$rel" ]; do
        rel="${rel#../}"
        # strip last path component of base ("/" or ":" boundary)
        if [ "${base##*/}" != "$base" ]; then base="${base%/*}";
        else base="${base%:*}:"; fi
    done
    case "$base" in
        *:) echo "${base}${rel}" ;;
        *)  echo "${base}/${rel}" ;;
    esac
}

lint_repo() {
    local repo="$1" label="$2"
    echo "== $label"

    if [ -n "$(git -C "$repo" status --porcelain 2>/dev/null | head -1)" ]; then
        warn "working tree not clean (local dirt — fine for dev, not for a release tag)"
    else
        ok "working tree clean"
    fi

    # No committed .gitmodules → leaf repo, nothing more to check.
    git -C "$repo" cat-file -e HEAD:.gitmodules 2>/dev/null || return 0

    local parent_url
    parent_url="$(git -C "$repo" remote get-url origin 2>/dev/null)"

    git -C "$repo" config --blob HEAD:.gitmodules --get-regexp '^submodule\..*\.path$' |
    while read -r key path; do
        local name="${key#submodule.}"; name="${name%.path}"
        local url_committed url_resolved sub pin
        url_committed="$(git -C "$repo" config --blob HEAD:.gitmodules --get "submodule.$name.url")"
        url_resolved="$(resolve_url "$parent_url" "$url_committed")"
        sub="$repo/$path"
        pin="$(git -C "$repo" ls-tree -d HEAD "$path" | awk '{print $3}')"

        if [ ! -e "$sub/.git" ]; then
            fail "$name: submodule not initialized at $path"
            continue
        fi

        local url_local
        url_local="$(git -C "$sub" remote get-url origin 2>/dev/null)"
        if [ "$(normalize_url "$url_resolved")" != "$(normalize_url "$url_local")" ]; then
            fail "$name: committed URL ($url_committed -> $url_resolved) != local origin ($url_local) — a fresh CI clone WILL diverge"
        else
            ok "$name: committed URL matches origin"
        fi

        if [ "$OFFLINE" -eq 0 ]; then
            git -C "$sub" fetch --quiet --prune origin 2>/dev/null || warn "$name: fetch failed (network?)"
        fi
        if git -C "$sub" branch -r --contains "$pin" 2>/dev/null | grep -q . ||
           git -C "$sub" tag --contains "$pin" 2>/dev/null | grep -q .; then
            ok "$name: pin ${pin:0:9} reachable on origin"
        else
            fail "$name: pin ${pin:0:9} NOT reachable from any origin branch/tag — CI will die with 'not our ref'"
        fi
    done

    # The while loop above runs in a subshell (pipe): re-collect its verdict.
    # Workaround: recount by re-running checks is overkill; instead the
    # subshell writes markers to a temp file.
}

# Because the per-submodule loop runs in a pipe subshell, counters set there
# are lost. Run everything with counters in a temp file instead.
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT
{
    lint_repo "$ROOT"                                   "kyber-desktop"
    lint_repo "$ROOT/kysdk"                             "kysdk"
    lint_repo "$ROOT/kysdk/kyctl"                       "kyctl"
    lint_repo "$ROOT/kysdk/kymedia"                     "kymedia"
    lint_repo "$ROOT/kysdk/kynput"                      "kynput"
    lint_repo "$ROOT/kysdk/kymedia/external/txproto"    "txproto"
    lint_repo "$ROOT/kysdk/kymedia/external/vlc-rs"     "vlc-rs"
} | tee "$TMP"

echo
FAILS="$(grep -c '^  FAIL' "$TMP")"
WARNS="$(grep -c '^  WARN' "$TMP")"
echo "fork-lint: $FAILS fail(s), $WARNS warning(s)"
[ "$FAILS" -eq 0 ]
