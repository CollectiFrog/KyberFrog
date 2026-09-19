# Releasing & CI

## Versioning — one source of truth

**The git tag is the version.** A `v<version>` tag cuts a release: CI passes it
to `packaging/build-installer.sh` and `packaging/linux/build-deb.sh`, which
export it as `KYBERFROG_VERSION` (baked into the binary by `build.rs`) and hand
it to NSIS / `dpkg-deb`. One value everywhere, nothing to keep in sync.

`version` under `[workspace.package]` in `Cargo.toml` is **only an offline
fallback**, used when there is no exact tag: local and MR builds are then named
`<cargo-version>-<short-sha>` (`<cargo-version>~<short-sha>` for the `.deb`, as
Debian gives `-` a meaning of its own). There is **no bump to do** when cutting
a release, and no check tying the tag to the Cargo version.

### Cut a release

```sh
git tag v0.5.2 && git push origin v0.5.2
```

CI then attaches to the [GitLab
Release](https://gitlab.com/kyber-frog/kyberfrog/-/releases):

- `KyberFrog-Setup-v0.5.2.exe` — the Windows installer;
- `kyberfrog_0.5.2_amd64.deb` — the Debian/Ubuntu package, **when the Linux
  chain succeeded** (see below).

Follow [SemVer](https://semver.org): patch for fixes, minor for features, major
for breaking config/CLI changes. Add the matching section to `CHANGELOG.md`
before tagging.

## The pipeline (`.gitlab-ci.yml`)

```mermaid
flowchart LR
    subgraph b["stage: build"]
        T["test"]
        U["build-ui"]
        FW["build-fork"]
        FL["build-fork-linux"]
        IMG["image-debian-linux<br/><i>only if its Dockerfile changed</i>"]
    end
    subgraph p["stage: package"]
        I["installer"]
        D["deb"]
    end
    subgraph r["stage: release — v* tags only"]
        R["release"]
        RD["release-deb"]
    end

    IMG -.->|"needs: optional"| FL
    T --> I
    U --> I
    FW --> I
    T --> D
    U --> D
    FL --> D
    I --> R
    D --> RD
    R --> RD
```

`pages` is not in this graph: it has `needs: []` and runs on the default branch
only, independently of the build chain.

| Job | What it does |
|-----|--------------|
| **test** | `cargo test --workspace --locked` on the Linux host target (Win32 → stubs). Fast; gates both packages. |
| **build-ui** | `npm ci && npm run build` → `ui/dist`, bundled next to the binary by both packagers. |
| **build-fork** | Clone + build the Kyber fork bundle for Windows (`kyber-desktop/build-win32.sh`). Heavy; cached in the Generic Package Registry keyed by the resolved `kyber-desktop` SHA. |
| **build-fork-linux** | Same for Linux amd64 (`build-linux.sh`), its own cache key. Checks the bundle really contains `kycontroller`/`kyavserver`/`kyclient` before anyone downstream trusts it. |
| **image-debian-linux** | Builds and pushes the Linux build image (Kaniko) — only when `ops/docker-images/debian-linux/` changed. |
| **installer** | `cargo build` `kyberfrog.exe`, then `makensis` → `KyberFrog-Setup.exe`. |
| **deb** | `cargo build` `kyberfrog`, then `packaging/linux/build-deb.sh` → `kyberfrog_<version>_amd64.deb`, checked with `lintian --fail-on error`. |
| **release** | On a `v*` tag: upload the installer to the package registry and create the GitLab Release linking it. |
| **release-deb** | On a `v*` tag: upload the `.deb` and attach it to that release as an extra asset. |
| **pages** | Build the MkDocs site → `public/` on the default branch. Independent (`needs: []`). |

### When pipelines run

The surface is declared once, in `workflow:rules`, not job by job:

- **merge requests**, pushes to **`dev`**, pushes to the **default branch**, and
  **tags**;
- **nothing on a working branch** (`feat/*`…) until it has a merge request — no
  minutes spent on code not yet proposed for integration;
- **no duplicates**: when a branch has an open MR, only the MR pipeline runs —
  a push to `dev` with an open MR to `main` yields one pipeline, not two.

Every job is **automatic** — there is no manual button anywhere in the chain.
Jobs that carry their own `rules` only *narrow* that surface: `release` and
`release-deb` to `v*` tags, `pages` to the default branch,
`image-debian-linux` to a change under `ops/docker-images/debian-linux/`.

### The Linux chain never holds back a Windows release

On a **tag**, `build-fork-linux` and `deb` are `allow_failure: true`: a broken
Linux build cannot stop `installer` → `release`. Everywhere else (MR, `dev`,
default branch) they are blocking, exactly like the Windows jobs — a Linux
regression has to be visible before the merge.

That is also why the `.deb` is attached by a **separate job** rather than being
one more entry in the `release:` block of the `release` job: that block is
static, so a tag whose Linux chain failed would publish a release adorned with a
dead link. `release-deb` only runs with a package in hand, and is itself
`allow_failure` — if the Release Links API ever refuses the job token, the
`.deb` is still published in the Generic Package Registry, at the URL printed in
the job trace.

### SaaS-runner notes

- Jobs are untagged so GitLab.com shared runners pick them up.
- The MinGW image must live in **this** project's registry — a `kyber-frog` CI
  token can't pull from `kyber.stream`'s private registry. Push it once:
  ```sh
  docker login registry.gitlab.com
  docker tag kyber/debian-win64:local "$CI_REGISTRY_IMAGE/debian-win64:latest"
  docker push "$CI_REGISTRY_IMAGE/debian-win64:latest"
  ```
  The Linux image is not in that situation: it is reproducible from this repo
  and CI builds it itself.
- The fork's submodules use SSH URLs; CI rewrites them to token HTTPS
  (`git config --global url."https://gitlab-ci-token:…".insteadOf "git@gitlab.com:"`).
- A fork build from scratch costs ~1 h 30 on a shared runner. Both fork jobs hit
  their cache as long as `packaging/versions.sh` doesn't move; the dev loop for
  the Linux bundle runs [on the workstation](building.md#building-for-linux-amd64),
  not here.

## Documentation site

The `pages` job builds the MkDocs Material site (`mkdocs.yml`, sources in
`docs/`) with `mkdocs build --strict` and publishes `public/` to GitLab Pages at
<https://kyber-anysource-b41fc4.gitlab.io/>. `--strict` fails the build on broken
links or nav, so keep internal links valid.
