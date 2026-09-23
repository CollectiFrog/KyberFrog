#!/bin/bash
# Pinned upstream refs the KyberFrog release is built against.
# Sourced by .gitlab-ci.yml and the packaging scripts.

# Kyber fork "desktop" repo (kyber-frog/kyber-desktop): builds the bundle of
# kycontroller / kyavserver / kyclient + DLLs + libVLC plugins.
#
# The pin IS the gitlink of the `vendor/kyber-desktop` submodule — read from the
# index, so it works whether or not the submodule is initialised (CI never
# initialises it; most dev clones don't either). To move it:
#
#   git -C vendor/kyber-desktop checkout <sha>
#   git add vendor/kyber-desktop && git commit
#
# Why each pin moved lives in those commits: `git log -p -- vendor/kyber-desktop`
# (and, before the submodule, `git log -p -- packaging/versions.sh`).
#
# Outside a git checkout (source tarball), export KYBER_DESKTOP_REF beforehand.
# `safe.directory`: inside a build container the bind-mounted checkout belongs to
# another uid, and git refuses to read it otherwise.
if [ -z "${KYBER_DESKTOP_REF:-}" ]; then
    KYBER_DESKTOP_REF="$(git -c safe.directory='*' \
        -C "$(dirname "${BASH_SOURCE[0]}")/.." \
        ls-files --stage -- vendor/kyber-desktop 2>/dev/null | awk '$1 == "160000" { print $2 }')"
fi
if [ -z "$KYBER_DESKTOP_REF" ]; then
    echo "versions.sh: no gitlink at vendor/kyber-desktop and KYBER_DESKTOP_REF unset." >&2
    return 1 2>/dev/null || exit 1
fi
