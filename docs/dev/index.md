# Developer docs

KyberFrog is an **orchestration layer** on top of the `kyber-frog` fork of
[Kyber](https://kyber.stream) (QUIC video transport). It does **not** reimplement
Kyber — it generates configs and supervises the fork's binaries
(`kycontroller`, `kyavserver`, `kyclient`).

## Start here

- **[Architecture](architecture.md)** — crates, the one-config/two-halves model,
  the supervisor, the Win32 patterns, the gotchas.
- **[Building from source](building.md)** — the MinGW Docker workflow, and the
  cross-repo **fork build model** (how the bundled binaries are produced).
- **[Releasing & CI](releasing.md)** — versioning, tags, and the GitLab pipeline
  (`test` → `build-fork` → `installer` → `release`, plus `pages`).
- **[Contributing](contributing.md)** — workflow, tests, conventions.

## TL;DR for a new contributor

```sh
# From the workspace root (the dir with apps/, core/, …). Fast inner loop:
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local cargo test

# Build the single exe:
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo build --release --target x86_64-pc-windows-gnu
```

There is **no native Rust toolchain on the dev host** — everything
cross-compiles to Windows through the `kyber/debian-win64:local` image. On
Windows, mount with **PowerShell**, not git-bash (git-bash rewrites `-w /work`
and breaks the container).

## Project facts

- **Repo:** `git@gitlab.com:kyber-frog/kyberfrog.git`, branch `main`, public,
  AGPL-3.0.
- **Naming split:** the GitLab **group is `kyber-frog`** (hyphen — the bare
  `kyberfrog` namespace was taken); the **code name is `kyberfrog`** (no hyphen —
  crates, `%APPDATA%\kyberfrog`, the icon).
- **Backlog & tech debt:** [`IMPROVEMENTS.md`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/IMPROVEMENTS.md);
  the working plan is [`TODO.md`](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/TODO.md).
