# Continuous integration

[Documentation index](README.md) · [Testing](testing.md) · [Release guide](release.md)

## Linux verification

`.woodpecker/linux.yaml` selects a dedicated `linux/amd64` Docker Agent with `cache=ajr-linux-v1` for pushes to `main`, pull requests and manual pipelines. Feature-branch pushes are verified through their PR or a manual run, avoiding duplicate Push/PR jobs on the single-worker cache. It fixes the workspace at `/woodpecker/src`, matching local reproduction and keeping fixture Unix sockets within the OS path limit. `scripts/ci_linux.sh` supplies the same commands for local reproduction. The workflow requests no publication credentials, host mounts, Docker socket or privileged steps. The administrator supplies one project-only cache volume through the dedicated Agent. Fork PRs should retain the server's approval requirement.

The Node 24.21.0 Debian Bookworm image is pinned by digest. Its Python 3.11, Git, C compiler and binutils support the existing release/package fixtures. Rust 1.99.1 is the pinned stable verification toolchain; Rust 1.89.0 independently checks the declared minimum. Update pins deliberately and revalidate the workflow when upgrading.

| Step | Checks |
|---|---|
| `toolchain` | Validate prerequisites; restore compiler artifacts and reuse/install both Rust toolchains, including rustfmt and Clippy for stable |
| `frontend` | Locked frontend installation, panel tests, bundled UI build and online-update/uninstall fixtures |
| `msrv` | Cache-input regressions, content/mode-checked input timestamps and Rust 1.89 all-target locked compilation in `target/msrv/` |
| `native` | Format, all-target Clippy/tests, release build and CLI help with Rust 1.99.1 |
| `packages` | Source metadata and release fixtures, isolated OpenCode 1.19.4 tools, development-only npm/marketplace packaging, installed-package/OpenCode checks with a simulated model and Claude scaffold verification |
| `cache` | Save compiler artifacts and input fingerprints after success or failure; never retain fixture profiles, installed candidates or successful-test receipts |

All steps run serially. The dedicated Agent must keep capacity **1** and be restricted to this repository. Its Docker backend mounts `woodpecker_ajr_ci_cache_v1:/ci-cache`; the ordinary worker remains unchanged. Configure `WOODPECKER_AGENT_LABELS=repo=MorningFrog/agent-job-relay,cache=ajr-linux-v1` and `WOODPECKER_BACKEND_DOCKER_VOLUMES=woodpecker_ajr_ci_cache_v1:/ci-cache`, with server-side repo filters where available. This administrator-supplied mount does not require enabling repository Trusted Volumes. Keep fork approval enabled; approved code can write this CI cache, which must remain separate from formal release builds. See [Agent configuration](https://woodpecker-ci.org/docs/administration/configuration/agent) and [Docker backend](https://woodpecker-ci.org/docs/administration/configuration/backends/docker).

The image/toolchain namespace persists Rust installations, Cargo/npm downloads and compiler output. Compiler artifacts are copied into a fresh workspace so hard-link/process-replacement tests keep the same filesystem; source and generated UI mtimes are restored only for matching bytes/modes/inventory. Changed inputs are made newer than the cached build. All tests rerun, including after a previously failed pipeline. Build jobs follow `nproc`; `AJR_CI_BUILD_JOBS` can override this for constrained workers. Incremental compilation is enabled. A missing cache is a cold run, not permission to skip gates.

Generated fixture state remains in the disposable `.tmp/`, `web/node_modules/` and `web/dist/`; only compiler output under `target/` is saved. The measured cache occupies about 6.1 GiB; monitor storage and remove only this project's generated cache when invalidating it. This Bookworm build is for CI verification, not a sealed release or evidence of the release payload's glibc 2.34 compatibility.

## Windows native verification

`.woodpecker/windows.yaml` selects the dedicated `windows/amd64` Local-backend Worker with `worker=ajr-win`. Its events match Linux: main Push, pull request and manual, excluding Tags and duplicate feature-branch Push runs. `image: pwsh` selects PowerShell 7 on this backend. The deployed Worker runs as LocalService with capacity **1**, isolated workflow home directories and no publication credentials. Local backend code runs directly with that account's permissions; labels and directories do not isolate mutually untrusted repositories. Keep fork approval enabled.

`scripts/ci_windows.ps1` validates Node 24 and Rust **1.99.1** with rustfmt/Clippy, discovers the installed Visual Studio C++ x64 tools through `vswhere`, and imports the compiler environment into the current process. It changes no machine-wide PATH or toolchain installation. The administrator must provision these tools, Git and clone plugin **2.12.1** before running the Agent; the Local backend uses that installed plugin executable. Clone disables partial fetching and unused Git LFS because the private MinGit distribution has no LFS executable and this repository has no LFS objects. If LFS is introduced, provision Git LFS and reenable its checkout deliberately. Minimum-Rust compilation remains a separate Linux gate.

| Step | Checks |
|---|---|
| `toolchain` | Validate tools and source revision, initialize MSVC and report compiler parallelism/cache location |
| `frontend` | Locked frontend install, panel tests/UI build and online-update/uninstall fixtures |
| `native` | Format, all-target Clippy/tests, release build, CLI help and native Windows version/hash/source descriptor |

The administrator supplies a project-only `CARGO_HOME`. The script derives a versioned sibling directory for Cargo/npm downloads and a stable `CARGO_TARGET_DIR`; `AJR_CI_CACHE_ROOT` can explicitly select it. Without either setting it uses disposable `.tmp/ci-windows/`. Capacity one protects shared compiler output. Fixture profiles/results stay in each disposable checkout; every gate reruns, and changed source remains subject to Cargo's normal fingerprint checks. No input timestamps or test-success receipts are restored on Windows. Keep compiler output and fixture directories on the same volume when exercising hard-link fixtures. `AJR_CI_BUILD_JOBS` can override the processor-count default.

After successful native gates, `target/release/agent-job-relay.exe` and `target/windows-native.json` retain that run's exact executable and Git revision within the disposable workspace. `describe_native.ps1 -Revision <commit>` supplies source identity explicitly; its default remains compatible with the legacy Actions caller. These files are not yet uploaded or bound to a cross-worker pipeline identity. Installed Windows package/loader checks remain pending until that transfer is implemented; do not substitute an older Windows executable or treat mutable compiler-cache files as a sealed artifact.

## Validate before pushing

Install a matching Woodpecker CLI from the [official release instructions](https://woodpecker-ci.org/docs/cli/installation), verifying its published checksum. With Docker available, use a disposable checkout containing the proposed changes, then run from its root:

```bash
woodpecker-cli lint .woodpecker/
docker volume create woodpecker_ajr_ci_cache_trial_v1
woodpecker-cli --disable-update-check exec --local --backend-engine docker --backend-docker-volumes woodpecker_ajr_ci_cache_trial_v1:/ci-cache --pipeline-event push --commit-branch main --repo MorningFrog/agent-job-relay --timeout 60m .woodpecker/linux.yaml
```

Expected: `linux` completes all six steps successfully. A second fresh checkout with the same cache demonstrates warm reuse without retaining test state. Omit the backend volume flag for an uncached diagnostic run. `--local` mounts the checkout and writes generated files there; use a fresh disposable checkout for a full retry because packaging requires a new `.tmp/release-candidate` directory. Local execution validates containers and commands; it does not validate GitHub checkout, Webhooks, Agent scheduling, server timeout or GitHub status reporting. See [local execution](https://woodpecker-ci.org/docs/usage/local-execution) and [workflow syntax](https://woodpecker-ci.org/docs/usage/workflow-syntax).

After maintainer-authorized push, inspect the pipeline for that exact commit in Woodpecker and its reported GitHub checks. Keep the default repository config discovery enabled, allow the cold run sufficient time (currently 60 minutes), and leave fork approval and trust settings intact. Select required status checks from actual successful server runs rather than guessing their names.

For Windows, use the verified Windows CLI of the same Woodpecker version, a disposable checkout and the same low-privilege account/tool configuration as the Worker:

```powershell
$env:CARGO_HOME = Join-Path $PWD '.tmp/ci-downloads/cargo'
woodpecker-cli --disable-update-check exec --local --backend-engine local --backend-local-isolated-home --pipeline-event push --commit-branch main --repo MorningFrog/agent-job-relay --timeout 60m .woodpecker/windows.yaml
```

Expected: all three Windows steps succeed, with native tests and a version/hash/revision descriptor. Reuse the compiler/download cache for a second fresh checkout to measure warm behavior; keep fixture state disposable. This local execution does not establish Server scheduling, authenticated checkout or GitHub status reporting, which require an authorized exact-commit PR run.

## Main-branch protection

Changes to main require a pull request and successful `ci/woodpecker/pr/linux` against the latest main. No administrator bypass is configured. Use a branch/PR for subsequent source, CI and release-preparation edits; do not push those changes directly to main. After merging, verify the main Push pipeline independently. See [current verification](IMPLEMENTATION_STATUS.md).

## Migration boundary

GitHub Actions execution is unavailable in the maintainer environment. The legacy `.github/workflows/ci.yml` keeps only `workflow_dispatch` as a manual reference for checks not yet migrated; automatic main-Push/PR CI runs through Woodpecker. Other Actions release workflows remain manual references. Linux passes exact-commit server CI, including package installation and GitHub status reporting; Windows native migration evidence is recorded separately in [implementation status](IMPLEMENTATION_STATUS.md). The deployment's reusable Worker manager keeps repository bindings, Agent identity and cache directories/volumes separate for Docker and Windows Local workers. Worker removal retains identity/cache, and idle checks protect known active work; no deployment-specific paths or credentials are stored in this repository. Windows package/loader tests still need commit/run-bound cross-worker artifact transfer. macOS compilation/portable tests need an appropriate worker. Track remaining work in [TODO](TODO.md).

Release preparation, preview, publication and independent verification still depend on Actions run identities, artifact storage and the npm GitHub OIDC publisher. They are not callable through this Linux workflow. Do not spoof Actions environment variables, reuse CI packages as release candidates or publish from push/PR/tag events; migrate those manual operations and their identity checks separately before attempting another release.
