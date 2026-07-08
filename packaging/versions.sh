#!/bin/bash
# Pinned upstream refs the KyberFrog release is built against.
# Sourced by .gitlab-ci.yml. Update + commit + tag to cut a release.

# Kyber fork "desktop" repo (kyber-frog/kyber-desktop): builds the bundle of
# kycontroller / kyavserver / kyclient + DLLs + libVLC plugins via build-win32.sh.
# A branch tracks the tip; pin to a commit SHA for a reproducible release.
KYBER_DESKTOP_REF="ac3d781a0a7a28b6b3726869407c4400b1decd28"  # dev @ kyber 0.27.1 rebase (2026-07-08/09)
