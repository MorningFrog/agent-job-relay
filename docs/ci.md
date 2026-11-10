# Continuous integration

[Documentation index](README.md) · [Testing](testing.md) · [Release guide](release.md)

## Linux verification

`.woodpecker/_ci-linux.yaml` selects a dedicated `linux/amd64` Docker Agent with `cache=ajr-linux-v1` for pushes to `main`, pull requests and manual pipelines. Feature-branch pushes are verified through their PR or a manual run, avoiding duplicate Push/PR jobs on the single-worker cache. It fixes the workspace at `/woodpecker/src`, matching local reproduction and keeping fixture Unix sockets within the OS path limit. `scripts/ci_linux.sh` supplies the same commands for local reproduction. The workflow requests no publication credentials, host mounts, Docker socket or privileged steps. The administrator supplies one project-only cache volume through the dedicated Agent. Fork PRs should retain the server's approval requirement.

The Node 24.21.0 Debian Bookworm image is pinned by digest. Its Python 3.11, Git, C compiler and binutils support the existing release/package fixtures. Rust 1.99.1 is the pinned stable verification toolchain; Rust 1.89.0 independently checks the declared minimum. Update pins deliberately and revalidate the workflow when upgrading.

| Step | Checks |
|---|---|
| `toolchain` | Validate prerequisites; restore compiler artifacts and reuse/install both Rust toolchains, including rustfmt and Clippy for stable |
| `frontend` | Locked frontend installation, panel tests, bundled UI build and online-update/uninstall fixtures |
| `msrv` | Cache-input regressions, content/mode-checked input timestamps and Rust 1.89 all-target locked compilation in `target/msrv/` |
| `native` | Format, all-target Clippy/tests, release build and CLI help with Rust 1.99.1 |
| `packages` | Source metadata and release fixtures, isolated OpenCode 1.19.4 tools, development-only npm/marketplace packaging, installed-package/OpenCode checks with a simulated model and Claude scaffold verification |
| `upload-native` | Send the tested Linux executable to the private append-only CI exchange, bound to this repository/run/commit |
| `cache` | Save compiler artifacts and input fingerprints after success or failure; never retain fixture profiles, installed candidates or successful-test receipts |

All steps run serially. The dedicated Agent must keep capacity **1** and be restricted to this repository. Its Docker backend mounts `woodpecker_ajr_ci_cache_v1:/ci-cache`; the ordinary worker remains unchanged. Configure `WOODPECKER_AGENT_LABELS=repo=MorningFrog/agent-job-relay,cache=ajr-linux-v1` and `WOODPECKER_BACKEND_DOCKER_VOLUMES=woodpecker_ajr_ci_cache_v1:/ci-cache`, with server-side repo filters where available. This administrator-supplied mount does not require enabling repository Trusted Volumes. Keep fork approval enabled; approved code can write this CI cache, which must remain separate from formal release builds. See [Agent configuration](https://woodpecker-ci.org/docs/administration/configuration/agent) and [Docker backend](https://woodpecker-ci.org/docs/administration/configuration/backends/docker).

The image/toolchain namespace persists Rust installations, Cargo/npm downloads and compiler output. Compiler artifacts are copied into a fresh workspace so hard-link/process-replacement tests keep the same filesystem; source and generated UI mtimes are restored only for matching bytes/modes/inventory. Changed inputs are made newer than the cached build. All tests rerun, including after a previously failed pipeline. Build jobs follow `nproc`; `AJR_CI_BUILD_JOBS` can override this for constrained workers. Incremental compilation is enabled. A missing cache is a cold run, not permission to skip gates.

Generated fixture state remains in the disposable `.tmp/`, `web/node_modules/` and `web/dist/`; only compiler output under `target/` is saved. The measured cache occupies about 6.1 GiB; monitor storage and remove only this project's generated cache when invalidating it. This Bookworm build is for CI verification, not a sealed release or evidence of the release payload's glibc 2.34 compatibility.

## Windows native verification

`.woodpecker/_ci-windows.yaml` selects the dedicated `windows/amd64` Local-backend Worker with `worker=ajr-win`. Its events match Linux: main Push, pull request and manual, excluding Tags and duplicate feature-branch Push runs. `image: pwsh` selects PowerShell 7 on this backend. The deployed Worker runs as LocalService with capacity **1**, isolated workflow home directories and no publication credentials. Local backend code runs directly with that account's permissions; labels and directories do not isolate mutually untrusted repositories. Keep fork approval enabled.

`scripts/ci_windows.ps1` validates Node 24 and Rust **1.99.1** with rustfmt/Clippy, discovers the installed Visual Studio C++ x64 tools through `vswhere`, and imports the compiler environment into the current process. It changes no machine-wide PATH or toolchain installation. The administrator must provision these tools, Git and clone plugin **2.12.1** before running the Agent; the Local backend uses that installed plugin executable. Clone disables partial fetching and unused Git LFS because the private MinGit distribution has no LFS executable and this repository has no LFS objects. If LFS is introduced, provision Git LFS and reenable its checkout deliberately. Minimum-Rust compilation remains a separate Linux gate.

| Step | Checks |
|---|---|
| `toolchain` | Validate tools and source revision, initialize MSVC and report compiler parallelism/cache location |
| `frontend` | Locked frontend install, panel tests/UI build and online-update/uninstall fixtures |
| `native` | Format, all-target Clippy/tests, release build, CLI help and native Windows version/hash/source descriptor |
| `upload-native` | Send the exact tested executable and its descriptor to the private CI exchange |

The administrator supplies a project-only `CARGO_HOME`. The script derives a versioned sibling directory for Cargo/npm downloads and a stable `CARGO_TARGET_DIR`; `AJR_CI_CACHE_ROOT` can explicitly select it. Without either setting it uses disposable `.tmp/ci-windows/`. Capacity one protects shared compiler output. Fixture profiles/results stay in each disposable checkout; every gate reruns, and changed source remains subject to Cargo's normal fingerprint checks. No input timestamps or test-success receipts are restored on Windows. Keep compiler output and fixture directories on the same volume when exercising hard-link fixtures. `AJR_CI_BUILD_JOBS` can override the processor-count default.

After successful native gates, `target/release/agent-job-relay.exe` and `target/windows-native.json` identify that run's executable and Git revision. `describe_native.ps1 -Revision <commit>` supplies source identity explicitly; its default remains compatible with the legacy Actions caller. The upload step sends these exact workspace files before the Local backend removes the checkout. Do not substitute an older executable or use mutable compiler-cache files as artifact identities.

## Cross-worker development packages

The four-workflow graph runs native Linux and Windows checks concurrently. `package-windows-test` waits for both, downloads their exact native bytes on the Linux Worker, and invokes `prepare_release.py --allow-unreleased --expected-revision <CI commit>` to create development-only dual-platform npm packages and a marketplace archive. `windows-packages` then downloads those packages on the Windows Worker and runs `ci_windows_packages.ps1`: isolated OpenCode 1.19.4 and Codex 0.164.0 tools, actual native/npm installation and actual host-loader checks. OpenCode uses a simulated model; Codex discovery needs no account/model. No workflow publishes or creates a formal release candidate. Current preparation/activation evidence is in [implementation status](IMPLEMENTATION_STATUS.md).

| Workflow | Depends on | Result |
|---|---|---|
| `linux` | — | Full Linux gates and `linux-native` |
| `windows` | — | Full Windows gates and `windows-native` |
| `package-windows-test` | `linux`, `windows` | Verified `windows-packages` from the same run |
| `windows-packages` | `package-windows-test` | Installed native/npm/Codex/OpenCode checks; bounded failure diagnostics |

The deployment administrator must enable the private HTTPS CI exchange and four repository secrets before pushing this graph. The exchange is separate from compiler caches and release storage. Secrets are injected only into upload/download/packaging steps; installed-package/host checks receive no artifact credentials. Keep the existing fork-approval policy.

| Repository secret | Purpose |
|---|---|
| `ci_artifact_endpoint` | Private HTTPS origin reachable from both Workers |
| `ci_artifact_ca` | Explicit trusted PEM certificate; never disable TLS verification |
| `ci_artifact_linux_token` | This repository's Linux artifact role: write Linux native/development packages, read both native objects |
| `ci_artifact_windows_token` | This repository's Windows artifact role: write Windows native/diagnostics, read development packages |

`scripts/ci_artifacts.mjs` derives repository name/ID, pipeline number/creation timestamp and full commit from the actual Woodpecker environment, checks the clean tracked checkout and validates source/run/producer identity, file inventory, lengths and SHA-256 before writing any downloaded files. Native Windows metadata must match the expected source and bytes. Package bundles must be marked `development=true` and have an exact dual-platform inventory/checksum set. Bundle names are flat and portable; directories, duplicate case-folded names, traversal, truncation and trailing bytes are rejected. The exchange publishes complete objects without replacement; an identical lost-ack retry succeeds, conflicting bytes fail. Objects are bounded to 256 MiB and the deployment store to 20 GiB; no automatic deletion is configured. Operator retention/backup belongs to the deployment, outside this project's runtime.

The exchange roles authenticate trusted repository code, not individual CI steps or adversarial runs. Run/commit fencing prevents accidental mixing; it is not signed build provenance or isolation from approved malicious source. Keep CI credentials separate from publication accounts. Woodpecker 3.20.1 restarts allocate a new pipeline number, so a rerun gets a new namespace without overwriting an earlier run. Failure diagnostics are stored privately under the same identity; they are not printed into public checks or uploaded to a Release.

## Validate before pushing

With Node and Python available, run transfer/source regressions and lint with the matching verified Woodpecker CLI:

```bash
node --test scripts/test_ci_artifacts.mjs scripts/test_online_update.mjs scripts/test_online_uninstall.mjs
python3 -B -m unittest discover -s scripts -p test_release.py -v
woodpecker-cli lint .woodpecker/
```

Run the native commands in a disposable checkout using the provisioned tool configuration: Linux uses the pinned workflow image and `sh scripts/ci_linux.sh <step>`; Windows uses PowerShell 7 and `./scripts/ci_windows.ps1 <step>`. Every native gate must pass before uploading bytes. Use a separate fixture exchange and synthetic run namespace for local integration tests; never test against formal release storage. Full CLI workflow execution also needs the four private secrets, positive repository/run/creation metadata and the actual checkout commit. For CLI 3.20.1, provide an explicit metadata file with `repo.owner`, `repo.name` and `repo.id`; its `--repo` flag does not reliably populate the repository name in this version. The CLI does not orchestrate this mixed-OS dependency graph; invoke stages in dependency order on their real platforms with the same fixture identity.

After separately authorized service activation and push, inspect the Server pipeline for the exact PR head, all four workflow results and GitHub checks. This is required to validate scheduling, real dependency ordering, secret injection, authenticated checkout and cross-Worker networking; local fixture transfers do not establish those properties. Keep default repository config discovery, the existing 60-minute timeout, fork approval and trust settings intact. Select required checks from actual successful Server runs.

## Main-branch protection

Changes to main require a pull request against the latest main and all four successful checks: `ci/woodpecker/pr/_ci-linux`, `ci/woodpecker/pr/_ci-windows`, `ci/woodpecker/pr/_ci-package-windows-test` and `ci/woodpecker/pr/_ci-windows-packages`. No administrator bypass is configured. Use a branch/PR for subsequent source, CI and release-preparation edits; do not push those changes directly to main. After merging, verify the main Push pipeline independently. See [current verification](IMPLEMENTATION_STATUS.md).

## Migration boundary

GitHub Actions execution is unavailable in the maintainer environment. The legacy `.github/workflows/ci.yml` keeps only `workflow_dispatch` as a manual reference for checks not yet migrated; automatic main-Push/PR CI runs through Woodpecker. Other Actions release workflows remain manual references. Linux passes exact-commit server CI, including package installation and GitHub status reporting; Windows native migration evidence is recorded separately in [implementation status](IMPLEMENTATION_STATUS.md). The deployment's reusable Worker manager keeps repository bindings, Agent identity and cache directories/volumes separate for Docker and Windows Local workers. Worker removal retains identity/cache, and idle checks protect known active work; no deployment-specific paths or credentials are stored in this repository. The private exchange and repository secrets are enabled. PR #3 is merged with the native Windows changes from PR #2; both exact-head PR and independent main Push acceptance pass the full Windows package/loader graph. An [experimental macOS ARM64 cross-build entry](macos-arm64.md) is available without running tests, but actual target compilation awaits the SDK/toolchain and native/portable execution still requires a Mac environment. Track remaining work in [TODO](TODO.md).

The separate manual Woodpecker preparation graph and default shared-login local publication route are accepted end to end for 0.1.24; see [release handoff](release.md#current-release-handoff). PR #7/main four-check CI and formal #42 source/native/package/seal checks pass. Maintainer npm 2FA and separately authorized GitHub finalization preserve the original bytes; the independent public asset/channel/Tag/receipt and isolated installation check passes. New projects reference shared platform/profile bindings while retaining their own repository/Worker/cache/artifact roles; they do not repeat platform setup or the reference project's full rehearsal. Human login/approval can use the deployment desktop handoff without copied commands. Per-project PATs, App registration and CI npm tokens are not the default setup. Legacy manual Actions references remain for separate cleanup; macOS migration and real-host support gaps remain explicit. Push/PR/Tag events never publish, and development CI packages never become formal candidates.
## Workflow layout

The active directory contains four automatic verification YAML files and six explicit manual preparation YAML files. Publication now uses the deployment-side Run Release entry with default dry-run; the six former publish workflows are retired. See [release operations](release.md#start-a-release) and `.woodpecker/_README.md`. Shared templates belong in deployment tooling. Prefix reusable support files with `_` or `.`, but use a non-YAML extension or keep them outside the active path: Woodpecker does not exclude YAML merely because of the prefix.
