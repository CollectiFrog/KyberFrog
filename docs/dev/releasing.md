# Releasing & CI

## Versioning — one source of truth

The version is `version` under `[workspace.package]` in `Cargo.toml`. A
`v<version>` git tag cuts a release. Local dev builds without an exact tag are
named `<cargo-version>-<short-sha>`.

### Cut a release

1. Bump `version` in `Cargo.toml` (e.g. `0.1.0` → `0.2.0`), commit.
2. Tag it **matching the Cargo version** and push:
   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```
3. CI builds and attaches `KyberFrog-Setup-v0.2.0.exe` to the
   [GitLab Release](https://gitlab.com/kyber-frog/kyberfrog/-/releases).

The CI `installer` job **fails fast if the tag ≠ the Cargo version**, so the two
can't drift. Follow [SemVer](https://semver.org): patch for fixes, minor for
features, major for breaking config/CLI changes.

## The pipeline (`.gitlab-ci.yml`)

```
 stage build      stage package   stage release      stage pages
┌──────────┐
│   test   │──┐
└──────────┘  │   ┌───────────┐   ┌───────────┐
┌──────────┐  ├──▶│ installer │──▶│  release  │  (tag v* only)
│build-fork│──┘   └───────────┘   └───────────┘
└──────────┘
┌──────────┐
│  pages   │  (default branch only, needs: [])
└──────────┘
```

| Job | What it does |
|-----|--------------|
| **test** | `cargo test --workspace --locked` on the Linux host target (Win32 → stubs). Fast, gates the installer (a red test blocks package + release). |
| **build-fork** | Clone + build the Kyber fork bundle via `kyber-desktop/build-win32.sh`. Heavy; cached in the Generic Package Registry keyed by the resolved `kyber-desktop` SHA. `timeout: 1h30m`. |
| **installer** | `cargo build` `kyberfrog.exe`, then `makensis` → `KyberFrog-Setup.exe` (`packaging/build-installer.sh`). On a tag, checks tag = Cargo version. |
| **release** | On a `v*` tag: upload the setup to the package registry and create a GitLab Release linking it. |
| **pages** | Build the MkDocs site → `public/` on the default branch. Independent (`needs: []`). |

Jobs run on **MR**, the **default branch**, and **tags** (`.win-rules`), except
`release` (tags `v*`) and `pages` (default branch).

### SaaS-runner notes

- Jobs are untagged so GitLab.com shared runners pick them up.
- The MinGW image must live in **this** project's registry — a `kyber-frog` CI
  token can't pull from `kyber.stream`'s private registry. Push it once:
  ```sh
  docker login registry.gitlab.com
  docker tag kyber/debian-win64:local "$CI_REGISTRY_IMAGE/debian-win64:latest"
  docker push "$CI_REGISTRY_IMAGE/debian-win64:latest"
  ```
- The fork's submodules use SSH URLs; CI rewrites them to token HTTPS
  (`git config --global url."https://gitlab-ci-token:…".insteadOf "git@gitlab.com:"`).

## Documentation site

The `pages` job builds the MkDocs Material site (`mkdocs.yml`, sources in
`docs/`) with `mkdocs build --strict` and publishes `public/` to GitLab Pages at
<https://kyber-frog.gitlab.io/kyberfrog/>. `--strict` fails the build on broken
links or nav, so keep internal links valid.
