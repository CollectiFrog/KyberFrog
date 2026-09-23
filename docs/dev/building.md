# Building from source

For development only — end users use the [installer](../user/installation.md).
First time here? [Dev environment setup](setup.md) gets a machine ready in one
command; this page is the reference behind `./dev.sh`.

There is **no native Rust toolchain on the dev host**: everything is built in
Docker. The Windows binaries cross-compile in `kyber/debian-win64:local` (MinGW),
the Linux ones build natively in `kyber/debian-linux:local`. Both are published
in this project's registry, pullable without a login; `./dev.sh` pulls the one
it needs on first use and tags it with the local name the scripts expect:

| Local name | Pulled from |
|---|---|
| `kyber/debian-win64:local` | `registry.gitlab.com/kyber-frog/kyberfrog/debian-win64:latest` |
| `kyber/debian-linux:local` | `registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-amd64` |

The Linux image is reproducible from `ops/docker-images/debian-linux/`. The
win64 one is not: its `Dockerfile` lives in upstream's private ops repo, and
the registry copy is the only source. An image that already exists under the
local name is never replaced.

!!! warning "Typing docker commands by hand on Windows"
    `./dev.sh` takes care of this. By hand from Git Bash, `docker run` breaks:
    Git Bash rewrites `-w /work` into a Windows path. Run such commands from
    PowerShell, or prefix them with `MSYS_NO_PATHCONV=1` and pass host paths
    through `cygpath -m`.

## The single exe

```sh
./dev.sh exe     # → target/x86_64-pc-windows-gnu/release/kyberfrog.exe
./dev.sh check   # cargo check of the whole workspace for that target
```

The `x86_64-pc-windows-gnu` target matters: the Win32 code (tray, Job Object,
spout enumeration, icon loading) only compiles for Windows.

**Fast inner loop:** `cargo test` on the **Linux host target** compiles
everything except the Win32 modules (which fall back to no-op stubs), so it
catches all the non-Win32 logic quickly:

```sh
./dev.sh test                                      # the whole suite
./dev.sh test -p kyberfrog-shared config_round     # extra args go to cargo
```

The tray icon (`kyberfrog/assets/kyberfrog.ico`) is embedded in the exe at build
time; dropping a `kyberfrog.ico` next to the exe overrides it.

## The installer

`packaging/build-installer.sh` does the whole release locally — builds
`kyberfrog.exe`, stages it with the web UI and the fork binaries bundle, and
runs `makensis` (both `cargo` and `makensis` live in the image):

```sh
./dev.sh installer   # → dist/KyberFrog-Setup-<version>.exe
```

Without `-f`, the bundle is the one CI built for the pinned `kyber-desktop` SHA
([below](#the-fork-bundle)). Flags, passed through: `-f <dir|zip>` for another
bundle — typically a local fork build —, `-v <version>`, `-s` to skip the cargo
build, `-o <dir>` for the output dir.

## The fork bundle

KyberFrog depends on the fork as **binaries** — `kycontroller`, `kyavserver`,
`kyclient` and their libraries —, never as source. The CI fork jobs build that
bundle once per `kyber-desktop` SHA and publish it in this project's Generic
Package Registry, which is public. `packaging/fork-bundle.sh` downloads the one
matching the pin (the [`vendor/kyber-desktop` gitlink](#the-fork-build-model))
into `dist/fork-bundle/<sha>/`, and the packaging scripts call it whenever `-f`
is absent:

```sh
./dev.sh bundle                # win64 (default)
./dev.sh bundle linux-amd64    # or linux-arm64
```

**Pros.** A clone with no fork checkout builds the installer and the `.deb`; the
default is exactly what a release ships, never a leftover local build from
another pin; nothing to authenticate.

**Cons.** After a pin bump, the bundle only exists once the first CI pipeline
has built it (~1 h 30; arm64 is [uploaded by hand](releasing.md#the-arm64-fork-bundle-is-built-here-not-in-ci)) —
until then `fork-bundle.sh` fails and says so, and a local fork build passed
with `-f` is the way through. Each pin leaves ~90 MB per arch in `dist/`.

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
the *official* artefact, from the SHA pinned by the `vendor/kyber-desktop`
gitlink. The script builds the initialised submodule, or a sibling
`../kyber-desktop` checkout if there is none; `-s <path>` picks another one.

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
`dpkg-deb`:

```sh
./dev.sh deb                                  # pinned bundle, fetched
./dev.sh deb -f dist/kyber-linux-x86_64.tar.bz2   # a local fork build
```

Out comes `dist/kyberfrog_<version>_amd64.deb`. Same flags as the
Windows script: `-f <dir|archive>`, `-v <version>`, `-o <dir>`, `-s` to skip the
cargo build. Without `-v` the version is `<cargo-version>~<short-sha>` — a
Debian version has to start with a digit, which a bare `git describe` does not
guarantee on a shallow clone.

!!! warning "Run it in the build image"
    The dependencies are **computed** by `dpkg-shlibdeps`, which needs the
    system libraries the bundle links against (libva, libgcrypt, libdav1d…) to
    be installed — the image brings them in through `apt build-dep vlc`.
    Anywhere else the computation fails and the script **stops**, rather than
    shipping a package whose dependencies were invented — such a package would
    install fine and die at runtime on a missing `.so`.

CI finishes the `deb` job with
`lintian --fail-on error --suppress-tags embedded-library`, worth running
locally too. `embedded-library` is expected and suppressed on purpose: the
bundle deliberately carries its own patched ffmpeg/VLC, exactly as the Windows
one does.

### arm64

Same scripts, one flag: `-a arm64`. The bundle is built in an **emulated**
`linux/arm64` container (qemu, through Docker Desktop's binfmt) because neither
CI nor the workstation has an ARM machine:

```sh
packaging/linux/build-fork-local.sh -a arm64 -b   # image + bundle, allow a night
```

It produces `dist/kyber-linux-aarch64.tar.bz2` and prints the command that
uploads it to the Generic Package Registry, keyed by the `kyber-desktop` SHA.
**That upload is the arm64 build**: CI never compiles the fork for ARM, it only
takes the cache hit — see [Releasing](releasing.md#the-arm64-fork-bundle-is-built-here-not-in-ci)
and [arm64](plan-linux-amd64.md#arm64) for why.

Everything downstream follows the same flag; `-a arm64` already maps to the
`aarch64` bundle name and the `aarch64-unknown-linux-gnu` target:

```sh
bash packaging/linux/build-deb.sh -a arm64 -f dist/kyber-linux-aarch64.tar.bz2
```

!!! warning "The .deb itself wants a real arm64 environment"
    `dpkg-shlibdeps` resolves against *installed* packages, so packaging has to
    run where those packages are arm64 — the emulated image, or the
    `saas-linux-small-arm64` runner the `deb-arm64` job uses. Cross-packaging
    from the amd64 image would fail the dependency computation, and
    `build-deb.sh` treats that as fatal on purpose.

## The fork build model

KyberFrog only *orchestrates* pre-built Kyber binaries; building **them** means
building the **fork**, a nest of separate git repos wired by cargo
`[patch.crates-io]` + git submodules, under the GitLab group **`kyber-frog`**
(upstream = `kyber.stream`). Layout (each dir = its own repo):

- **Build root for `kyclient.exe`:** `kyber-desktop`
  (`kyber-frog/kyber-desktop`), the submodule **`vendor/kyber-desktop`** of
  `kyberfrog`. It stays empty in a plain clone — `./dev.sh setup --fork` checks
  out the whole chain (~1 min, ~740 MB). Its
  `kyclient` crate owns the CLI (`clap`: `--port`, `--fullscreen`, …) and the
  `winit` window, and reaches the client engine via `kyc` + `kyclient-rs`.
    - submodules: `kysdk`, `external/winit`.
    - `[patch.crates-io]`: `kyc`/`kyclient-rs`/`kynput-rs`/`kynput-sys` →
      `kysdk/kyctl/…` & `kysdk/kynput/…`; `winit` → `external/winit`.
- **SDK meta-repo:** `kyber-desktop/kysdk` (submodules: `kyctl`, `kymedia` —
  itself with `subprojects/vlc-rs` + `subprojects/txproto` —, `kynput`, `kymux`,
  `kyutil`). `kysdk/.cargo/config.toml` holds the `[patch.crates-io]` that
  redirects cross-crate deps to those submodule paths, **including
  `vlc-rs = { path = "./kymedia/subprojects/vlc-rs" }`**.
- **Client video path:** `kyber-desktop/kyclient` (bin) → `kyclient-rs` (FFI) →
  **libkyclient** (C ABI, built from `kyctl/kyclient` Rust lib with the `capi`
  feature; `kyclient-sys/build.rs` finds it via **pkg-config**) → **kyvlcplayer**
  (libVLC, via the patched `vlc-rs`) → window / Spout.

!!! info "Key consequence"
    There is exactly **one checkout of each repo** — no standalone-vs-submodule
    duplication. Each submodule tree under `kyber-desktop/kysdk/**` **is** the
    canonical fork repo, so editing in place is enough for a *local* build. A
    change only reaches **other clones and CI** once it is pushed on the
    sub-repo's own branch and the submodule pointers are bumped *up the chain*,
    up to `kyberfrog`'s own gitlink.

**`kyberfrog-dev` is the fork branch on all seven fork repos** (kyber-desktop,
kysdk, kyctl, kymedia, kynput, txproto, vlc-rs). New fork work branches off it
and merges back into it. KyberFrog pins a **commit SHA** of `kyber-desktop`:
the `vendor/kyber-desktop` gitlink *is* the pin. `packaging/versions.sh` reads it
from the index (`KYBER_DESKTOP_REF`), initialised or not, and that SHA is what
CI builds and what `fork-bundle.sh` downloads. How the chain is maintained and
rebased on upstream: [fork chain process](plans-fork-restructure.md).

**Pros.** One source of truth for the pin, visible in `git log -- vendor/`; a
plain clone reveals the fork's location without paying for it; every build runs
from one repo root.

**Cons.** Windows: the chain nests ~175 characters below the repo root, and
git's HTTPS transport refuses a git dir past 260 even with `core.longpaths` —
so the fork chain only checks out from a repo root of ~80 characters at most.
`setup --fork` checks that first, and turns on `core.longpaths` in each repo for
the rest of git. A sibling
`../kyber-desktop` checkout is still accepted by the fork scripts, so older
workspaces keep working — but then the gitlink and that checkout can disagree,
and only the gitlink counts.

### Landing a cross-repo change

1. Commit and push the change on each affected sub-repo (e.g. `kyctl`,
   `vlc-rs`), merged into that repo's `kyberfrog-dev`.
2. Bump the submodule pointers up the chain — `kymedia` (if a `subprojects/`
   repo moved), then `kysdk`, then `kyber-desktop` — each on `kyberfrog-dev`:
   in the parent, `git add` the child's path alone and commit. `.gitmodules`
   changes never go in a bump commit.
3. Build libkyclient (kyctl `capi`) then `cargo build`, or a release bundle in
   the meson-fixed image `setup --fork` derives:

    ```sh
    MSYS_NO_PATHCONV=1 docker run --rm -v "$(cygpath -m "$PWD/vendor/kyber-desktop"):/work" \
      -w /work kyber/debian-win64:local-0.27 \
      bash -c "KYBER_STAGING_DIRECTORY=kyberfrog-fork-bundle ./build-win32.sh -p"
    ```

4. Move KyberFrog's pin: with `vendor/kyber-desktop` on the new SHA,
   `git add vendor/kyber-desktop`, commit, and run `packaging/fork-lint.sh`. The
   first pipeline on that commit builds and publishes the new bundles.

No `.cargo/config.toml` change is needed for `vlc-rs` (the patch already points
at its submodule — just update that submodule to the fork branch) nor for a new
crate that is a plain path-dep (resolved locally).

See also the worked example [E2E: Spout output](../E2E-spout-output.md).
