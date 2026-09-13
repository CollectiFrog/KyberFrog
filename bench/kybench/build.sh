#!/bin/bash
# Build kybench.exe in the MinGW image. From PowerShell, at bench/kybench:
#
#   docker run --rm -v "${PWD}:/src" -v kybench-cargo-registry:/cargo/registry `
#     -v kybench-target:/target -w /src kyber/debian-win64:local-0.27 bash build.sh
#
# The target dir lives in a docker volume (bind-mount I/O is slow on Windows);
# the exe is copied back next to this script.
set -euo pipefail
export CARGO_TARGET_DIR=/target
cargo test --release --quiet            # idcode unit tests, Linux host target
cargo build --release --target x86_64-pc-windows-gnu
cp /target/x86_64-pc-windows-gnu/release/kybench.exe /src/kybench.exe
ls -l /src/kybench.exe
