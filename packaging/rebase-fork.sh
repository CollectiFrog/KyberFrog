#!/usr/bin/env bash
# rebase-fork.sh — rebase the whole Kyber fork chain onto a new upstream
# version, in one supervised cascade.
#
# What it does (see docs/dev/plans-fork-restructure.md, "statu quo outillé"):
#   1. fetches origin + upstream in every fork repo (adds the upstream
#      remotes if missing);
#   2. resolves the rebase TARGET of every repo up front: the root target is
#      the upstream tag, each child's target is the gitlink recorded by its
#      parent's target tree (ls-tree walk — no guessing);
#   3. rebases each fork branch onto its target on a work branch
#      rebase/<version> (origin/<fork-branch> is the base; local branches and
#      the current checkout are only moved to the work branch, never lost);
#      the fork's `deps…bump` commits are DROPPED during replay — they would
#      conflict on gitlinks every time, and step 5 regenerates them;
#   4. re-syncs the never-forked submodules (winit, kymux, kyutil, vlc,
#      libvlcjni, kynput externals) to the pins upstream chose;
#   5. re-bumps the fork gitlinks bottom-up (kymedia -> kysdk -> kyber-desktop).
#
# Usage:
#   rebase-fork.sh <version> [fork-root]   start (e.g. rebase-fork.sh 0.27.1)
#   rebase-fork.sh --dry-run <version>     show targets + commits to replay
#   rebase-fork.sh --continue              resume after fixing a conflict
#   rebase-fork.sh --abort                 restore every repo as it was
#
# On a conflict the script stops with the repo path and options; typical
# resolutions: fix + `git add` + `--continue`, or `git rebase --skip` when a
# commit is already upstream (accepted MR — it happens, that's the goal).
# Nothing is ever pushed: the final report prints the push commands.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DEFAULT="$SCRIPT_DIR/../../kyber-desktop"

# name|path relative to root|fork branch|upstream url
#
# The fork branch is always "kyberfrog-dev" (2026-07-09 rename, all 7 repos,
# including kynput which used to be "feat/remote-desktop-fix"): scoped naming
# so it's never mistaken for — or accidentally based/pushed onto — upstream's
# own dev-ish branches when preparing an upstream MR.
REPOS='
kyber-desktop|.|kyberfrog-dev|git@gitlab.com:kyber.stream/apps/kyber-desktop.git
kysdk|kysdk|kyberfrog-dev|git@gitlab.com:kyber.stream/core/kysdk.git
kyctl|kysdk/kyctl|kyberfrog-dev|git@gitlab.com:kyber.stream/core/kyctl.git
kymedia|kysdk/kymedia|kyberfrog-dev|git@gitlab.com:kyber.stream/core/kymedia.git
kynput|kysdk/kynput|kyberfrog-dev|git@gitlab.com:kyber.stream/core/kynput.git
txproto|kysdk/kymedia/external/txproto|kyberfrog-dev|git@gitlab.com:kyber.stream/deps/txproto.git
vlc-rs|kysdk/kymedia/external/vlc-rs|kyberfrog-dev|git@gitlab.com:kyber.stream/deps/vlc-rs.git
'
# child -> parent whose target tree provides the child's target gitlink.
# Paths are NOT hardcoded: upstream renames submodule dirs (0.27.x moved
# kymedia external/ -> subprojects/), so gitlinks are located by basename.
PARENT_OF='kysdk=kyber-desktop kyctl=kysdk kymedia=kysdk kynput=kysdk
txproto=kymedia vlc-rs=kymedia'

ORDER='kyber-desktop kysdk kyctl kymedia kynput txproto vlc-rs'
BUMPS='kymedia kysdk kyber-desktop'
# git >= 2.52 writes todo lines as "pick <sha> # <subject>" — tolerate both.
DROP_BUMPS_EDITOR='sed -i -E "/^pick [0-9a-f]+ (# )?deps.*bump/d"'

field() { echo "$REPOS" | grep "^$1|" | cut -d'|' -f"$2"; }
repo_dir() { local p; p="$(field "$1" 2)"; [ "$p" = "." ] && echo "$ROOT" || echo "$ROOT/$p"; }

STATE=""   # set once ROOT is known
sget() { grep "^$1=" "$STATE" 2>/dev/null | tail -1 | cut -d= -f2-; }
sput() { echo "$1=$2" >> "$STATE"; }
done_step() { grep -q "^DONE=$1\$" "$STATE" 2>/dev/null; }

die() { echo "rebase-fork: $*" >&2; exit 1; }

# ---------------------------------------------------------------- fetch all
prepare() {
    local name dir url
    for name in $ORDER; do
        dir="$(repo_dir "$name")"; url="$(field "$name" 4)"
        [ -d "$dir/.git" ] || [ -f "$dir/.git" ] || die "$name not found at $dir"
        git -C "$dir" remote get-url upstream >/dev/null 2>&1 ||
            git -C "$dir" remote add upstream "$url"
        echo "fetch $name (origin + upstream)…"
        # --no-recurse-submodules: on-demand recursion would try to fetch
        # UPSTREAM gitlink pins from the FORK remotes of the submodules
        # ("not our ref"); every repo of the chain is fetched individually.
        git -C "$dir" fetch --quiet --prune --no-recurse-submodules origin ||
            die "$name: fetch origin failed"
        git -C "$dir" fetch --quiet --prune --tags --no-recurse-submodules upstream ||
            die "$name: fetch upstream failed"
    done
}

# Find where a tree records the gitlink of a repo, by basename ("txproto"
# matches both external/txproto and subprojects/txproto). Echoes "path sha".
gitlink_of() { # parent_dir committish child_name
    git -C "$1" ls-tree -r "$2" | awk -v n="$3" \
        '$1==160000 && ($4==n || $4 ~ ("/" n "$")) {print $4, $3; exit}'
}

# ------------------------------------------------- resolve all targets first
resolve_targets() {
    local version="$1" root_target="" cand
    for cand in "$version" "kyber-$version"; do
        if git -C "$ROOT" rev-parse -q --verify "refs/tags/$cand^{commit}" >/dev/null; then
            root_target="$(git -C "$ROOT" rev-parse "refs/tags/$cand^{commit}")"; break
        fi
    done
    [ -n "$root_target" ] || die "no tag '$version' (or 'kyber-$version') on kyber-desktop upstream"
    sput TARGET_kyber-desktop "$root_target"

    local entry child parent parent_target found path sha
    for entry in $PARENT_OF; do
        child="${entry%%=*}"; parent="${entry#*=}"
        parent_target="$(sget "TARGET_$parent")"
        found="$(gitlink_of "$(repo_dir "$parent")" "$parent_target" "$child")"
        [ -n "$found" ] || die "no gitlink named '$child' in $parent@${parent_target:0:9} — upstream layout changed, inspect 'git ls-tree -r' there"
        path="${found% *}"; sha="${found#* }"
        # the child needs that object locally (comes with its upstream fetch)
        git -C "$(repo_dir "$child")" cat-file -e "$sha^{commit}" 2>/dev/null ||
            die "$child: target ${sha:0:9} (pinned by $parent upstream at $path) not found locally — check the upstream remote"
        sput "TARGET_$child" "$sha"
        sput "TPATH_$child" "$path"
    done
}

show_plan() {
    local name dir branch target ncommits nbumps
    printf '%-14s %-28s %-28s %s\n' REPO "FORK BASE (origin)" "TARGET (upstream)" "COMMITS TO REPLAY"
    for name in $ORDER; do
        dir="$(repo_dir "$name")"; branch="$(field "$name" 3)"; target="$(sget "TARGET_$name")"
        ncommits="$(git -C "$dir" rev-list --count --no-merges "$target..origin/$branch")"
        nbumps="$(git -C "$dir" log --format=%s --no-merges "$target..origin/$branch" | grep -Ec '^deps.*bump' || true)"
        printf '%-14s %-28s %-28s %s\n' "$name" \
            "$(git -C "$dir" describe --tags --always "origin/$branch")" \
            "$(git -C "$dir" describe --tags --always "$target")" \
            "$((ncommits - nbumps)) (+$nbumps bumps dropped)"
    done
}

# ----------------------------------------------------------------- rebasing
conflict_stop() {
    local name="$1" dir="$2"
    cat >&2 <<EOF

rebase-fork: CONFLICT in $name ($dir)
$(git -C "$dir" status --short | head -20)

Options:
  - fix the files, 'git -C "$dir" add …', then rerun: rebase-fork.sh --continue
  - commit already upstream (accepted MR)? 'git -C "$dir" rebase --skip', then --continue
  - give up entirely: rebase-fork.sh --abort
EOF
    exit 2
}

rebase_one() {
    local name="$1" dir branch target
    dir="$(repo_dir "$name")"; branch="$(field "$name" 3)"; target="$(sget "TARGET_$name")"
    # =dirty: tolerate submodule *content* dirt (unavoidable mid-cascade),
    # block on real file changes — a rebase would drag them along. The
    # script's own state file lives at the fork root: not dirt.
    [ -n "$(git -C "$dir" status --porcelain --ignore-submodules=dirty -- ':(exclude).rebase-fork.state')" ] &&
        die "$name: working tree not clean — stash or commit first (fork-lint.sh warns about this)"
    sput "ORIG_$name" "$(git -C "$dir" symbolic-ref --short -q HEAD || git -C "$dir" rev-parse HEAD)"
    echo "== rebase $name: origin/$branch -> $(git -C "$dir" describe --tags --always "$target")"
    git -C "$dir" checkout -q -B "rebase/$VERSION" "origin/$branch"
    GIT_SEQUENCE_EDITOR="$DROP_BUMPS_EDITOR" git -C "$dir" rebase -i "$target" ||
        conflict_stop "$name" "$dir"
}

resume_rebase() {
    local name="$1" dir
    dir="$(repo_dir "$name")"
    if (cd "$dir" && [ -d "$(git rev-parse --git-path rebase-merge)" ]); then
        echo "== resume rebase in $name"
        git -C "$dir" rebase --continue || conflict_stop "$name" "$dir"
    fi
}

bump_one() {
    local name="$1" dir children child found path sha
    dir="$(repo_dir "$name")"
    case "$name" in
        kymedia)       children="txproto vlc-rs" ;;
        kysdk)         children="kyctl kymedia kynput" ;;
        kyber-desktop) children="kysdk" ;;
    esac
    for child in $children; do
        sha="$(git -C "$(repo_dir "$child")" rev-parse "rebase/$VERSION")"
        # Locate the gitlink in the parent's REBASED tree (paths may have
        # been renamed upstream) and update the index directly: the child's
        # worktree may not sit at the new path yet, the commit is what counts.
        found="$(gitlink_of "$dir" HEAD "$child")"
        [ -n "$found" ] || die "bump $name: no gitlink '$child' in rebased HEAD — the fork must (re)declare it in .gitmodules first"
        path="${found% *}"
        git -C "$dir" update-index --cacheinfo "160000,$sha,$path" ||
            die "bump $name: update-index failed for $path"
    done
    if git -C "$dir" diff --cached --quiet; then
        echo "== bump $name: nothing to do"
    else
        git -C "$dir" commit -q -m "deps: bump submodule pointers (rebase on kyber $VERSION)"
        echo "== bump $name: committed $(git -C "$dir" rev-parse --short HEAD)"
    fi
}

report() {
    local name dir branch
    echo
    echo "rebase-fork: DONE. Every repo sits on rebase/$VERSION; fork branches untouched."
    echo
    echo "Next steps (in order):"
    echo "  1. check committed .gitmodules of the rebased parents: forked"
    echo "     submodules (txproto, vlc-rs, kyctl, kymedia, kynput, kysdk) must"
    echo "     point at kyber-frog/* URLs, or a fresh CI clone dies (not our ref);"
    echo "     never-forked ones must keep the upstream URLs;"
    echo "  2. materialize worktrees for building: 'git submodule update --init'"
    echo "     per level (renamed paths clone fresh — subprojects/vlc is heavy);"
    echo "  3. build + validate the chain (build-win32.sh -p, then E2E smoke);"
    echo "  4. fast-forward each fork branch and push:"
    for name in $ORDER; do
        dir="$(repo_dir "$name")"; branch="$(field "$name" 3)"
        echo "       git -C \"$dir\" branch -f $branch rebase/$VERSION && git -C \"$dir\" push --force-with-lease origin $branch"
    done
    echo "  5. pin the resolved kyber-desktop SHA in packaging/versions.sh;"
    echo "  6. run fork-lint.sh before tagging anything."
}

# ------------------------------------------------------------------ main
MODE=run; VERSION=""; ROOT="$ROOT_DEFAULT"
case "${1:-}" in
    --dry-run)  MODE=dry;      VERSION="${2:-}"; ROOT="${3:-$ROOT_DEFAULT}" ;;
    --continue) MODE=continue; ROOT="${2:-$ROOT_DEFAULT}" ;;
    --abort)    MODE=abort;    ROOT="${2:-$ROOT_DEFAULT}" ;;
    -*|"")      die "usage: rebase-fork.sh <version>|--dry-run <version>|--continue|--abort" ;;
    *)          VERSION="$1";  ROOT="${2:-$ROOT_DEFAULT}" ;;
esac
ROOT="$(cd "$ROOT" && pwd)" || die "fork root not found"
STATE="$ROOT/.rebase-fork.state"

case "$MODE" in
dry)
    [ -n "$VERSION" ] || die "--dry-run needs a version"
    STATE="$(mktemp)"; trap 'rm -f "$STATE"' EXIT
    prepare; resolve_targets "$VERSION"; show_plan
    ;;
run)
    [ -f "$STATE" ] && die "a run is in progress (state: $STATE) — use --continue or --abort"
    : > "$STATE"; sput VERSION "$VERSION"
    prepare; resolve_targets "$VERSION"; show_plan; echo
    for name in $ORDER; do rebase_one "$name"; sput DONE "rebase:$name"; done
    for name in $BUMPS; do bump_one "$name"; sput DONE "bump:$name"; done
    report; rm -f "$STATE"
    ;;
continue)
    [ -f "$STATE" ] || die "no run in progress"
    VERSION="$(sget VERSION)"
    for name in $ORDER; do
        done_step "rebase:$name" && continue
        resume_rebase "$name"
        # ORIG_<name> is written when rebase_one starts: absent = never started.
        [ -z "$(sget "ORIG_$name")" ] && rebase_one "$name"
        sput DONE "rebase:$name"
    done
    for name in $BUMPS; do
        done_step "bump:$name" || { bump_one "$name"; sput DONE "bump:$name"; }
    done
    report; rm -f "$STATE"
    ;;
abort)
    [ -f "$STATE" ] || die "no run in progress"
    VERSION="$(sget VERSION)"
    for name in $ORDER; do
        dir="$(repo_dir "$name")"; orig="$(sget "ORIG_$name")"
        [ -n "$orig" ] || continue
        git -C "$dir" rebase --abort 2>/dev/null
        git -C "$dir" checkout -q "$orig"
        git -C "$dir" branch -q -D "rebase/$VERSION" 2>/dev/null
        echo "restored $name -> $orig"
    done
    rm -f "$STATE"; echo "rebase-fork: aborted, state cleaned."
    ;;
esac
