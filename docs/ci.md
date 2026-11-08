# Continuous integration

[Documentation index](README.md) · [Testing](testing.md) · [Release guide](release.md)

## Linux verification

`.woodpecker/linux.yaml` selects a dedicated `linux/amd64` Docker Agent with `cache=ajr-linux-v1` for pushes, pull requests and manual pipelines. It fixes the workspace at `/woodpecker/src`, matching local reproduction and keeping fixture Unix sockets within the OS path limit. `scripts/ci_linux.sh` supplies the same commands for local reproduction. The workflow requests no publication credentials, host mounts, Docker socket or privileged steps. The administrator supplies one project-only cache volume through the dedicated Agent. Fork PRs should retain the server's approval requirement.

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

## Validate before pushing

Install a matching Woodpecker CLI from the [official release instructions](https://woodpecker-ci.org/docs/cli/installation), verifying its published checksum. With Docker available, use a disposable checkout containing the proposed changes, then run from its root:

```bash
woodpecker-cli lint .woodpecker/
docker volume create woodpecker_ajr_ci_cache_trial_v1
woodpecker-cli --disable-update-check exec --local --backend-engine docker --backend-docker-volumes woodpecker_ajr_ci_cache_trial_v1:/ci-cache --pipeline-event push --commit-branch main --repo MorningFrog/agent-job-relay --timeout 60m .woodpecker/linux.yaml
```

Expected: `linux` completes all six steps successfully. A second fresh checkout with the same cache demonstrates warm reuse without retaining test state. Omit the backend volume flag for an uncached diagnostic run. `--local` mounts the checkout and writes generated files there; use a fresh disposable checkout for a full retry because packaging requires a new `.tmp/release-candidate` directory. Local execution validates containers and commands; it does not validate GitHub checkout, Webhooks, Agent scheduling, server timeout or GitHub status reporting. See [local execution](https://woodpecker-ci.org/docs/usage/local-execution) and [workflow syntax](https://woodpecker-ci.org/docs/usage/workflow-syntax).

After maintainer-authorized push, inspect the pipeline for that exact commit in Woodpecker and its reported GitHub checks. Keep the default repository config discovery enabled, allow the cold run sufficient time (currently 60 minutes), and leave fork approval and trust settings intact. Select required status checks from actual successful server runs rather than guessing their names.

## Migration boundary

GitHub Actions execution is unavailable in the maintainer environment. The existing `.github/workflows/` files remain as the reference for checks not yet migrated. The short-workspace and persistent-cache workflow passes the exact-commit server Push pipeline, including package installation and GitHub status reporting; current evidence is in [implementation status](IMPLEMENTATION_STATUS.md). The dedicated Worker is managed by the deployment's reusable Worker manager (create/list/show/up/status/logs/stop/restart/remove), keeping repository bindings, Agent identity and cache volumes separate. Worker removal retains identity/cache, and idle checks protect known active work; no deployment-specific paths or credentials are stored in this repository. Linux configuration alone does not restore Windows native/package tests or macOS compilation/portable tests. Windows needs its own Agent and commit-bound artifact transfer; macOS needs an appropriate worker. Track remaining work in [TODO](TODO.md).

Release preparation, preview, publication and independent verification still depend on Actions run identities, artifact storage and the npm GitHub OIDC publisher. They are not callable through this Linux workflow. Do not spoof Actions environment variables, reuse CI packages as release candidates or publish from push/PR/tag events; migrate those manual operations and their identity checks separately before attempting another release.
