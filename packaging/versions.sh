#!/bin/bash
# Pinned upstream refs the KyberFrog release is built against.
# Sourced by .gitlab-ci.yml. Update + commit + tag to cut a release.

# Kyber fork "desktop" repo (kyber-frog/kyber-desktop): builds the bundle of
# kycontroller / kyavserver / kyclient + DLLs + libVLC plugins via build-win32.sh.
# A branch tracks the tip; pin to a commit SHA for a reproducible release.
KYBER_DESKTOP_REF="643ee0e1adf3746bdf59c5ea5add835d88c14d33"  # kyberfrog-dev @ P0 Linux fix (2026-08-17) — camera_device cfg(linux) réparé
