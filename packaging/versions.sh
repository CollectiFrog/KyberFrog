#!/bin/bash
# Pinned upstream refs the KyberFrog release is built against.
# Sourced by .gitlab-ci.yml. Update + commit + tag to cut a release.

# Kyber fork "desktop" repo (kyber-frog/kyber-desktop): builds the bundle of
# kycontroller / kyavserver / kyclient + DLLs + libVLC plugins via build-win32.sh.
# A branch tracks the tip; pin to a commit SHA for a reproducible release.
KYBER_DESKTOP_REF="5b58c5e3b482e5bf6d55ad83851bf55a960d70cd"  # feat/arm64-triplet @ 2026-09-20 — ARCH_TRIPLET dérivé de uname -m (#35)
# Deux commits au-dessus de 643ee0e1 (l'ancien pin, kyberfrog-dev @ P0 Linux) :
# build-linux.sh dérive son triplet de `uname -m` dans kyber-desktop, kyctl et
# kymedia, et kymedia gate NVENC/oneVPL sur x86. Rien de tout ça n'est atteint
# sur un hôte x86_64 — mais le cache des deux jobs fork est keyé par ce SHA,
# donc ce bump coûte **un rebuild Windows et amd64 complets** (~1 h 30 chacun)
# au premier pipeline qui le voit. Les deux suivants retombent sur le cache.
