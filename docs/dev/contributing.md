# Contributing

## Workflow

- Repo: `git@gitlab.com:kyber-frog/kyberfrog.git`, branch `main`, AGPL-3.0.
- Work on a **branch**, open a **Merge Request** against `main`.
- There is no `glab`/`gh` CLI on the dev host; use `git` + the GitLab web UI.
- Keep the [backlog](https://gitlab.com/kyber-frog/kyberfrog/-/blob/main/IMPROVEMENTS.md)
  honest: when you ship a numbered item, move it to **Shipped** (keep its
  number) rather than deleting it — `CLAUDE.md`, commits and MRs reference those
  numbers.

## Before you push

Run the test suite (it gates the CI `installer`):

```sh
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo test --workspace --locked

# A single test by name:
docker run --rm -v "${PWD}:/work" -w /work kyber/debian-win64:local \
  cargo test -p kyberfrog-shared config_round_trips_both_halves
```

`cargo test` on the Linux host target compiles the whole workspace (Win32 → no-op
stubs) and runs the unit tests. The MinGW image and the PowerShell-not-git-bash
caveat are covered in [Building from source](building.md).

## Where to put tests

The `shared` crate is **pure** (no Win32), so it's the natural home for unit
tests, and they run on the Linux container target. Good targets for new tests
(`IMPROVEMENTS.md` #14):

- `shared/src/config.rs` — `Globals::kyclient_args()` ordering and flags, round-trips.
- `shared/src/gen.rs` — `render_config()` layering edge cases.
- `kyberfrog/src/app.rs` — `resolve_port`, `resolve_viewer_id` (testable logic
  that doesn't touch Win32).

## Conventions

These bite if ignored — see [Architecture → Conventions & gotchas](architecture.md#conventions-gotchas):

- **kyclient arg order** is strict: the positional server IP goes **last**.
- **Binaries resolve via PATH**; don't hard-code absolute paths.
- **MinGW `HANDLE` is `*mut c_void`** — null-check with `is_null()`, never `== 0`.
- **Advanced settings are file-only** — don't surface auth/encoder/TLS in the web
  UI without a deliberate decision (`IMPROVEMENTS.md` #3).
- **Mutations go through one `op_*` function** so the web UI and tray stay in
  lockstep; take locks **config before manager**.

## Documentation

User-facing and developer docs live in `docs/` and are published to GitLab Pages
by the `pages` job. Edit the relevant `.md`, keep internal links valid
(`mkdocs build --strict` is enforced in CI), and the site rebuilds on merge to
`main`. Optionally preview locally:

```sh
docker run --rm -v "${PWD}:/work" -w /work -p 8000:8000 \
  squidfunk/mkdocs-material serve -a 0.0.0.0:8000
```
