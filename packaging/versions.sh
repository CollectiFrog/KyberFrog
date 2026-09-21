#!/bin/bash
# Pinned upstream refs the KyberFrog release is built against.
# Sourced by .gitlab-ci.yml. Update + commit + tag to cut a release.

# Kyber fork "desktop" repo (kyber-frog/kyber-desktop): builds the bundle of
# kycontroller / kyavserver / kyclient + DLLs + libVLC plugins via build-win32.sh.
# A branch tracks the tip; pin to a commit SHA for a reproducible release.
KYBER_DESKTOP_REF="38d64ebf782f8c828869d227588fa2bca5af3d35"  # feat/arm64-triplet @ 2026-09-21 — ARCH_TRIPLET (4 dépôts) + portage aarch64 (#35)
# Au-dessus de 643ee0e1 (l'ancien pin, kyberfrog-dev @ P0 Linux), le strict
# nécessaire pour qu'un bundle aarch64 existe :
#   * build-linux.sh dérive son triplet de `uname -m` dans les **quatre**
#     dépôts qui en ont un (kyber-desktop, kyctl, kymedia, kynput) au lieu de
#     coder x86_64-linux-gnu en dur, et les wrappers run_*.sh livrés dans le
#     bundle lisent le leur à l'exécution ;
#   * kymedia gate NVENC et oneVPL sur x86 — aucun des deux n'a de cible
#     aarch64 ;
#   * txproto-rs porte `va_list` (tableau sur x86_64, struct sur aarch64) et le
#     signe de `c_char`, sans quoi les crates Rust ne compilent pas sur ARM.
# Rien de tout ça n'est atteint sur un hôte x86_64 — mais le cache des deux
# jobs fork est keyé par ce SHA, donc ce bump coûte **un rebuild Windows et
# amd64 complets** (~1 h 30 chacun) au premier pipeline qui le voit. Les
# suivants retombent sur le cache.
