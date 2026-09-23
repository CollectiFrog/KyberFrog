# Setting up the dev environment

From a machine with nothing on it to a KyberFrog installer you built yourself.
Nothing is compiled on the host — every build runs in Docker, and `./dev.sh`
types the `docker run` lines for you.

```sh
git clone https://gitlab.com/kyber-frog/kyberfrog.git
cd kyberfrog
./dev.sh setup        # PowerShell / cmd: .\dev setup
./dev.sh installer    # → dist/KyberFrog-Setup-<version>.exe
```

The rest of this page explains what those commands need and what they do.
[Building from source](building.md) is the reference behind them.

## What the machine needs

| Tool | Why | Notes |
|---|---|---|
| **Docker Desktop** (WSL2 backend on Windows) | every build runs in a container | ~15 GB of disk: the Windows build image is 7.2 GB, the Linux one 4.9 GB, pulled only when a command needs it |
| **Git** — Git for Windows on Windows | the repo, and the shell `dev.sh` runs in | `.\dev` in PowerShell finds Git Bash by itself |

That is all. No Rust, Node, meson, NSIS or MinGW: they live in the images. No
GitLab account either, until you push — the repo, the images and the prebuilt
fork bundle are all public.

## What `./dev.sh setup` does

1. Checks that Docker is running.
2. Pulls the Windows build image from this project's registry and tags it
   `kyber/debian-win64:local`. An image that already exists under that name is
   left alone.
3. Downloads the **fork bundle** — `kycontroller`, `kyavserver`, `kyclient`, the
   libraries and libVLC plugins — that CI built for the pinned `kyber-desktop`
   commit. ~90 MB, into `dist/fork-bundle/<sha>/`.
4. Builds the dashboard into `ui/dist`.

On this project's reference workstation that is under a minute once the image is
there; the first pull takes as long as 7 GB takes on your line.

!!! info "The Kyber fork stays out of your way"
    KyberFrog runs the fork's programs; it never compiles against the fork's
    code. So the fork is a submodule, `vendor/kyber-desktop`, that a plain clone
    leaves **empty** — its commit is only a pin. Step 3 fetches the binaries
    built from that pin, which is all an app change needs.

## The daily loop

```sh
./dev.sh test        # cargo test on the Linux target — ~3-4 min cold, then fast
./dev.sh check       # cargo check for Windows: compiles the Win32 code too
./dev.sh ui          # rebuild the dashboard
./dev.sh installer   # the Windows installer
./dev.sh deb         # the Debian/Ubuntu package (pulls the Linux image once)
./dev.sh docs        # this documentation, live, on http://localhost:8000
./dev.sh help        # everything else
```

Extra arguments go through: `./dev.sh test -p kyberfrog-shared config_round`,
`./dev.sh installer -v 1.2.3`.

Run the result by installing it: the installer puts the exe, `ui/dist` and the
fork binaries where the app expects them and adds itself to PATH; the dashboard
opens on <http://localhost:7700>. Config lives in `%APPDATA%\kyberfrog\` (XDG
paths on Linux) — start a dev build with `APPDATA` pointed elsewhere to keep it
away from a production install's config.

## Working on the fork itself

Only when a change has to happen inside Kyber — `kyctl`, `kymedia`, `txproto`…:

```sh
./dev.sh setup --fork
```

On top of the above, it checks out the whole fork chain into
`vendor/kyber-desktop` at the pinned commits (~1 min, ~740 MB, 16 nested
repos), and derives the image fork builds need
(`kyber/debian-win64:local-0.27`, meson ≥ 1.10). Without an SSH key on GitLab it
clones over HTTPS and tells you how to keep fetching that way.

On Windows the repo root must be **80 characters at most**: the chain nests ~175
characters deeper, and 260 is a hard limit for git's HTTPS transport. The
command checks it before downloading anything.

Every sub-repo then sits on a detached HEAD — the pin. Put the one you edit on
the fork branch before committing:

```sh
git -C vendor/kyber-desktop/kysdk/kyctl switch kyberfrog-dev
```

How a fork change travels back up to KyberFrog's pin:
[Landing a cross-repo change](building.md#landing-a-cross-repo-change).

## When something goes wrong

| Symptom | Cause, fix |
|---|---|
| `Docker is installed but not running` | Start Docker Desktop, wait for the whale to settle, retry. |
| `fork-bundle: no win64 bundle in the registry` | The pin was just moved and CI has not built that commit yet (~1 h 30 after the push). Wait for the pipeline, or build the fork locally and pass it with `./dev.sh installer -f <bundle>`. |
| `setup --fork`: `the repo root is N characters long` | Windows' 260-character limit: the fork chain nests ~175 characters below the repo root, and git's HTTPS transport has no way around it. Clone into a directory of 80 characters at most — `C:\Users\<you>\Workspace\kyberfrog` is fine, a deep OneDrive folder is not. |
| `.\dev` is not recognised in PowerShell | Run it from the repo root: `.\dev setup`. |
| A `docker run` typed by hand fails on paths | Git Bash rewrites them. Use `./dev.sh`, or see [Building from source](building.md). |

Before your first push: [Contributing](contributing.md) — branch off `dev`, open
an MR, take your item from the [backlog](backlog.md).
