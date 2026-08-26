# Contributing

## Workflow

- Repo: `git@gitlab.com:kyber-frog/kyberfrog.git`, branch `main`, AGPL-3.0.
- Work on a **branch**, open a **Merge Request** against `main`.
- There is no `glab`/`gh` CLI on the dev host; use `git` + the GitLab web UI.
- Keep the [backlog](backlog.md) honest: when you ship a numbered item, move it
  to the [archive](backlog-archive.md) — keeping its number — rather than
  deleting it. `CLAUDE.md`, commits and MRs reference those numbers, so a
  number is never renumbered and never reused.

## Taking an item

The [backlog](backlog.md) is the source of truth, not the issue tracker. There is
deliberately **no issue for most items** — 24 open issues nobody reads is worse
than one page that is true.

When you actually start on something:

1. Open an issue from the **Backlog item** template, quoting the item's `#N`.
2. Set its labels (see below).
3. On the board, link the issue from the item's row and flip its state to
   `🚧 in progress`, so nobody starts the same thing.
4. When it ships, move the item to the [archive](backlog-archive.md) with its
   number and close the issue.

Labels mirror the board's two axes, so a board row and an issue always say the
same thing:

| Scoped label | Values |
|---|---|
| `state::` | `ready`, `in progress`, `blocked`, `decision`, `icebox` |
| `access::` | `laptop`, `fork-chain`, `hardware`, `operator` |

Scoped labels are mutually exclusive in GitLab, which is exactly what these two
axes need — an item is in one state and needs one kind of access.

Items in the [validation queue](backlog.md#validation-queue-no-code-just-a-run)
do not need an issue at all: they are single runs, not work. Report the result
on the board.

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
(the open backlog item is [#38](backlog.md#38-broader-unit-test-coverage);
`#14` is the *shipped* CI job that runs them):

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
  UI without a deliberate decision ([backlog](backlog.md) #3).
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
