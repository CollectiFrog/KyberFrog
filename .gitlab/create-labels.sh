#!/usr/bin/env sh
# Create the backlog's scoped labels on the project. Idempotent: re-running it
# leaves existing labels alone.
#
# The two axes mirror docs/dev/backlog.md — state:: is where an item stands,
# access:: is what you need to *have* to take it. Scoped labels are mutually
# exclusive in GitLab, which is exactly right for both axes.
#
# Needs an authenticated glab:  glab auth login
# (the VS Code GitLab extension's OAuth does NOT count — glab has its own
# credential store.)

set -eu

label() {
  if glab label create --name "$1" --color "$2" --description "$3" >/dev/null 2>&1; then
    echo "  created  $1"
  else
    echo "  exists   $1"
  fi
}

echo "state:: — where an item stands"
label "state::ready"       "#2C6E49" "Someone can pick this up today"
label "state::in progress" "#1F6FB2" "Someone is on it — check the linked MR before starting"
label "state::blocked"     "#99590C" "Waiting on an upstream change or on hardware nobody has yet"
label "state::decision"    "#56499F" "Writing code before the call is made would be a mistake"
label "state::icebox"      "#6E7A82" "Kept on purpose, no expressed need — do not start it"

echo "access:: — what you need to have"
label "access::laptop"     "#0E6F68" "Rust + React and the MinGW image. Nothing else"
label "access::fork-chain" "#8B5A2B" "The 7 nested fork repos, ~1h30 for a full build"
label "access::hardware"   "#A03E3E" "A second machine, a vertical screen, an AMD GPU, a Linux VM…"
label "access::operator"   "#6E7A82" "A product call, not an implementation"

echo "Done. See docs/dev/contributing.md → Taking an item."
