#!/bin/sh
# Lit CHANGELOG.md pour la CI.
#
#   sh packaging/changelog.sh extract 0.6.0   affiche le corps de cette section
#                                             (ce dont la Release GitLab est remplie)
#
# Le texte d'une release est la section du changelog, écrite pendant le
# développement — pas une liste de sujets de commits générée ici.
#
# POSIX sh + awk seulement : tourne dans l'image release-cli (busybox) comme
# dans git-bash sur le poste. Sort en silence (statut 0, rien sur stdout) quand
# la section n'existe pas : un tag de pré-version (v0.6.0-rc1) n'en a pas, et il
# ne doit pas faire échouer la release pour autant.

set -eu

CHANGELOG="${CHANGELOG:-CHANGELOG.md}"

case "${1:-}" in
  extract)
    [ $# -eq 2 ] || { echo "usage: $0 extract <version>" >&2; exit 2; }
    [ -f "$CHANGELOG" ] || { echo "$CHANGELOG introuvable (lancer depuis la racine)" >&2; exit 1; }
    # Corps d'une section, lignes vides retirées aux deux bouts. S'arrête au
    # titre `## ` suivant.
    awk -v want="## [$2]" '
      index($0, want) == 1 { inside = 1; next }
      /^## / { inside = 0 }
      inside { lines[n++] = $0 }
      END {
        first = 0; last = n - 1
        while (first <= last && lines[first] == "") first++
        while (last >= first && lines[last] == "") last--
        for (i = first; i <= last; i++) print lines[i]
      }
    ' "$CHANGELOG"
    ;;
  *)
    sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
