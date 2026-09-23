# Developer docs

KyberFrog is an **orchestration layer** on top of the `kyber-frog` fork of
[Kyber](https://kyber.stream) (QUIC video transport). It does **not** reimplement
Kyber — it generates configs and supervises the fork's binaries
(`kycontroller`, `kyavserver`, `kyclient`).

## Start here

- **[Dev environment setup](setup.md)** — first stop for a new contributor:
  from an empty machine to an installer you built, in three commands.
- **[Architecture](architecture.md)** — crates, the one-config/two-halves model,
  the supervisor, the Win32 patterns, the gotchas.
- **[Building from source](building.md)** — the MinGW Docker workflow, and the
  cross-repo **fork build model** (how the bundled binaries are produced).
- **[Releasing & CI](releasing.md)** — versioning, tags, and the GitLab pipeline
  (`test` → `build-fork` → `installer` → `release`, plus `pages`).
- **[Contributing](contributing.md)** — workflow, tests, conventions.

## TL;DR for a new contributor

```sh
git clone https://gitlab.com/kyber-frog/kyberfrog.git && cd kyberfrog
./dev.sh setup       # once: build image, pinned fork bundle, dashboard
./dev.sh test        # the inner loop
./dev.sh installer   # dist/KyberFrog-Setup-<version>.exe
```

In PowerShell or cmd, `.\dev` instead of `./dev.sh`. There is **no native Rust
toolchain on the dev host** — every build runs in Docker, and `dev.sh` owns the
`docker run` lines. Details: [Dev environment setup](setup.md).

## Project facts

- **Repo:** `git@gitlab.com:kyber-frog/kyberfrog.git`, public, AGPL-3.0 —
  `dev` for integration, `main` for releases.
- **Naming split:** the GitLab **group is `kyber-frog`** (hyphen — the bare
  `kyberfrog` namespace was taken); the **code name is `kyberfrog`** (no hyphen —
  crates, `%APPDATA%\kyberfrog`, the icon).
- **Backlog & tech debt:** [Backlog](backlog.md) — everything that is open, with
  a *state* and an *access* label on every item, so you can tell at a glance what
  you are able to pick up. Shipped items keep their number in the
  [archive](backlog-archive.md).
