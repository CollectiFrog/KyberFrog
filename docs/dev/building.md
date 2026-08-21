# Building from source

For development only — end users use the [installer](../user/installation.md).

There is **no native Rust toolchain on the dev host**: everything is built in
Docker. The Windows binaries cross-compile through the same MinGW image used by
the rest of Kyber, `kyber/debian-win64:local` (a locally-built image — the
GitLab registry copy can't be pulled); the Linux ones build natively in
`kyber/debian-linux:local` (see [Building for Linux](#building-for-linux-amd64)).

!!! warning "Windows: use PowerShell, not git-bash"
    When mounting the volume on Windows, run the `docker` command from
    **PowerShell**. git-bash rewrites `-w /work` into a Windows path and breaks
    the container — unless you apply the two guards described in the Linux
    section below, which is what the Linux build scripts do.

## The single exe

```sh
# Build → target/x86_64-pc-windows-gnu/release/kyberfrog.exe
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu
```

The `x86_64-pc-windows-gnu` target matters: the Win32 code (tray, Job Object,
spout enumeration, icon loading) only compiles for Windows.

**Fast inner loop:** a plain `cargo check` / `cargo test` on the **Linux host
target** compiles everything except the Win32 modules (which fall back to no-op
stubs), so it catches all the non-Win32 logic quickly:

```sh
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test
```

The tray icon (`kyberfrog/assets/kyberfrog.ico`) is embedded in the exe at build
time; dropping a `kyberfrog.ico` next to the exe overrides it.

## The installer

`packaging/build-installer.sh` does the whole release locally — builds
`kyberfrog.exe`, stages it with the fork binaries bundle, and runs `makensis`
(both `cargo` and `makensis` live in the image). **Mount the workspace root**
(not just `apps/KyberFrog`) so the sibling fork bundle is reachable:

```sh
# from the workspace root (the dir that contains apps/, core/, …)
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  bash apps/KyberFrog/packaging/build-installer.sh
```

Out comes `apps/KyberFrog/dist/KyberFrog-Setup-<version>.exe`. By default it
reuses the prebuilt fork bundle at
`apps/kyber-desktop/kyberfrog-spout-e2e[.zip]` (build it with
`apps/kyber-desktop/build-win32.sh -p`). Flags: `-f <dir|zip>` to point
elsewhere, `-v <version>` to set the version, `-s` to skip the cargo build,
`-o <dir>` for the output dir.

## Building for Linux (amd64)

The Linux side has its own image, `ops/docker-images/debian-linux/` — the
counterpart of the MinGW one, carrying everything needed to compile the fork
chain *and* to package the `.deb`. CI builds and pushes it (job
`image-debian-linux`, with Kaniko); locally, `build-fork-local.sh -b` builds it
as `kyber/debian-linux:local`.

### The fork bundle — locally, not in CI

```sh
packaging/linux/build-fork-local.sh -b   # first time: build the image, then everything
packaging/linux/build-fork-local.sh -c   # dev loop: cargo check only, a few minutes
packaging/linux/build-fork-local.sh -f   # start clean after rebasing the fork chain
```

A full fork build costs **~20 min on a 12-core workstation against ~1 h 30 on a
shared CI runner**, and the CI cache is keyed on the whole `kyber-desktop` SHA —
a single fork commit invalidates it. So the dev loop runs here; CI only produces
the *official* artefact, from the SHA pinned in `packaging/versions.sh`.

Two rules the script already applies, worth knowing before driving docker by
hand:

- **Build in a docker volume, never in the bind mount.** The contrib tree
  (ffmpeg/VLC) is hundreds of thousands of files, and bind-mount I/O on Windows
  collapses under it. The sources are copied into the volume once; the
  cargo/contrib cache then persists from one run to the next.
- **Git Bash rewrites paths.** Use `cygpath -m` for *host* paths, and prefix the
  command with `MSYS_NO_PATHCONV=1` so *container* paths (`/src`, `-w /build/…`)
  survive untouched. Miss either and docker fails in confusing ways.

### The .deb

`packaging/linux/build-deb.sh` is the Linux mirror of `build-installer.sh`: it
builds `kyberfrog`, stages it with the web UI and the fork bundle, and runs
`dpkg-deb`. **Mount the workspace root** so the sibling bundle is reachable:

```sh
# from the workspace root, fork bundle already built
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-linux:local \
  bash apps/KyberFrog/packaging/linux/build-deb.sh \
    -f apps/KyberFrog/dist/kyber-linux-x86_64.tar.bz2
```

Out comes `apps/KyberFrog/dist/kyberfrog_<version>_amd64.deb`. Same flags as the
Windows script: `-f <dir|archive>`, `-v <version>`, `-o <dir>`, `-s` to skip the
cargo build. Without `-v` the version is `<cargo-version>~<short-sha>` — a
Debian version has to start with a digit, which a bare `git describe` does not
guarantee on a shallow clone.

!!! warning "Run it in the build image"
    The dependencies are **computed** by `dpkg-shlibdeps`, which needs the
    system libraries the bundle links against (libva, libgcrypt, libdav1d…) to
    be installed — the image brings them in through `apt build-dep vlc`.
    Anywhere else the computation fails and the script **stops**, rather than
    shipping a package whose dependencies were invented (it used to fall back
    to `Depends: libc6` in silence; the package then installed fine and died at
    runtime on a missing `.so`).

CI finishes the `deb` job with
`lintian --fail-on error --suppress-tags embedded-library`, worth running
locally too. `embedded-library` is expected and suppressed on purpose: the
bundle deliberately carries its own patched ffmpeg/VLC, exactly as the Windows
one does.

## The fork build model

KyberFrog only *orchestrates* pre-built Kyber binaries; building **them** means
building the **fork**, a nest of separate git repos wired by cargo
`[patch.crates-io]` + git submodules, under the GitLab group **`kyber-frog`**
(upstream = `kyber.stream`). Layout in the workspace (each dir = its own repo):

- **Build root for `kyclient.exe`:** `apps/kyber-desktop`
  (`kyber-frog/kyber-desktop`). Its `kyclient` crate owns the CLI (`clap`:
  `--port`, `--fullscreen`, …) and the `winit` window, and reaches the client
  engine via `kyc` + `kyclient-rs`.
    - submodules: `kysdk` → `core/kysdk`, `external/winit` → `deps/winit`.
    - `[patch.crates-io]`: `kyc`/`kyclient-rs`/`kynput-rs`/`kynput-sys` →
      `kysdk/kyctl/…` & `kysdk/kynput/…`; `winit` → `external/winit`.
- **SDK meta-repo:** `core/kysdk` (submodules: `kyctl`, `kymedia` — itself with
  `external/vlc-rs` + `external/txproto` —, `kynput`, `kymux`, `kyutil`).
  `core/kysdk/.cargo/config.toml` holds the `[patch.crates-io]` that redirects
  cross-crate deps to those submodule paths, **including
  `vlc-rs = { path = "./kymedia/external/vlc-rs" }`**.
- **Client video path:** `kyber-desktop/kyclient` (bin) → `kyclient-rs` (FFI) →
  **libkyclient** (C ABI, built from `kyctl/kyclient` Rust lib with the `capi`
  feature; `kyclient-sys/build.rs` finds it via **pkg-config**) → **kyvlcplayer**
  (libVLC, via the patched `vlc-rs`) → window / Spout.

!!! info "Key consequence"
    The standalone checkouts `core/kyctl`, `deps/vlc-rs` are the *canonical* fork
    repos, but the **build uses the submodule copies under `core/kysdk/**` and
    `apps/kyber-desktop/kysdk`**. A change in a sub-repo only reaches a build
    after the submodule pointers are bumped *up the chain*.

### Landing a cross-repo change

For example the Spout-output feature:

1. Push the feature branch to each fork: `kyctl`, `vlc-rs`, `kyber-desktop`.
2. In `core/kysdk`: bump the `kyctl` and `kymedia/external/vlc-rs` submodules to
   those commits, commit (on a branch).
3. In `apps/kyber-desktop`: bump the `kysdk` submodule, apply the CLI change,
   build libkyclient (kyctl `capi`) then `cargo build` the binary.

No `.cargo/config.toml` change is needed for `vlc-rs` (the patch already points
at its submodule — just update that submodule to the fork branch) nor for a new
crate that is a plain path-dep (resolved locally).

See also the worked example [E2E: Spout output](../E2E-spout-output.md).
