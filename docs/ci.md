# Continuous integration

[Documentation index](README.md) · [Testing](testing.md) · [Release guide](release.md)

## Linux verification

`.woodpecker/linux.yaml` runs on a `linux/amd64` Docker Agent for pushes, pull requests and manual pipelines. `scripts/ci_linux.sh` supplies the same commands for local reproduction. The workflow has no publication credentials, host mounts, Docker socket or privileged steps. Fork PRs should retain the server's approval requirement.

The Node 24.21.0 Debian Bookworm image is pinned by digest. Its Python 3.11, Git, C compiler and binutils support the existing release/package fixtures. Rust 1.99.1 is the pinned stable verification toolchain; Rust 1.89.0 independently checks the declared minimum. Update pins deliberately and revalidate the workflow when upgrading.

| Step | Checks |
|---|---|
| `toolchain` | Validate prerequisites; install the two Rust toolchains under workspace-local `.tmp/ci/`, including rustfmt and Clippy for stable |
| `frontend` | Locked frontend installation, panel tests, bundled UI build and online-update/uninstall fixtures |
| `msrv` | Rust 1.89 all-target locked compilation, with separate `target/msrv/` output |
| `native` | Format, all-target Clippy/tests, release build and CLI help with Rust 1.99.1 |
| `packages` | Source metadata and release fixtures, isolated OpenCode 1.19.4 tools, development-only npm/marketplace packaging, installed-package/OpenCode checks with a simulated model and Claude scaffold verification |

All steps run serially in one workflow and share its workspace. No cross-pipeline cache or persistent host volume is required. Cold runs download toolchains and dependencies; generated state stays under `.tmp/`, `target/`, `web/node_modules/` and `web/dist/`. This Bookworm build is for CI verification, not a sealed release or evidence of the release payload's glibc 2.34 compatibility.

## Validate before pushing

Install a matching Woodpecker CLI from the [official release instructions](https://woodpecker-ci.org/docs/cli/installation), verifying its published checksum. With Docker available, use a disposable checkout containing the proposed changes, then run from its root:

```bash
woodpecker-cli lint .woodpecker/
woodpecker-cli --disable-update-check exec --local --backend-engine docker --pipeline-event push --commit-branch main --repo MorningFrog/agent-job-relay --timeout 60m .woodpecker/linux.yaml
```

Expected: `linux` completes all five steps successfully. `--local` mounts the checkout and writes generated files there; use a fresh disposable checkout for a full retry because packaging requires a new `.tmp/release-candidate` directory. Local execution validates containers and commands; it does not validate GitHub checkout, Webhooks, Agent scheduling, server timeout or GitHub status reporting. See [local execution](https://woodpecker-ci.org/docs/usage/local-execution) and [workflow syntax](https://woodpecker-ci.org/docs/usage/workflow-syntax).

After maintainer-authorized push, inspect the pipeline for that exact commit in Woodpecker and its reported GitHub checks. Keep the default repository config discovery enabled, allow the cold run sufficient time (currently 60 minutes), and leave fork approval and trust settings intact. Select required status checks from actual successful server runs rather than guessing their names.

## Migration boundary

GitHub Actions execution is unavailable in the maintainer environment. The existing `.github/workflows/` files remain as the reference for checks not yet migrated. Linux configuration alone does not restore Windows native/package tests or macOS compilation/portable tests. Windows needs its own Agent and commit-bound artifact transfer; macOS needs an appropriate worker. Track remaining work in [TODO](TODO.md).

Release preparation, preview, publication and independent verification still depend on Actions run identities, artifact storage and the npm GitHub OIDC publisher. They are not callable through this Linux workflow. Do not spoof Actions environment variables, reuse CI packages as release candidates or publish from push/PR/tag events; migrate those manual operations and their identity checks separately before attempting another release.
