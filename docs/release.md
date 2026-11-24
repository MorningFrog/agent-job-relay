# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

Only maintainers initiate releases. Release All and its standalone Prepare Release, Publish Release and Check Release workflows run on manual dispatch from `main`. Cargo.toml determines the version, CHANGELOG supplies the reviewed notes, and every subsequent step uses the original sealed candidate. Experimental macOS ARM64 packaging remains development-only.

## Current release handoff

[GitHub Release 0.1.27](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.27) remains the published baseline with its original candidate and publication receipt. Version 0.1.28 includes the GitHub Actions release route, Goal-lifetime default pause approval and compact MCP guidance and results. Use a reviewed commit with finalized notes and successful CI to prepare a new candidate. The [earlier preview](https://github.com/MorningFrog/agent-job-relay/actions/runs/45325117673/attempts/1) refers to the original candidate before the MCP changes. See the [current verification status](IMPLEMENTATION_STATUS.md#current-verification).

Retain each original provider, run, published asset and receipt. Check historical versions against their original evidence and use a successful GitHub Actions Run ID for new Actions candidates.

## Prepare and review

1. Run `python3 scripts/finalize_release.py --version <unused-x.y.z>`. It verifies tag, GitHub Release/draft, receipt and npm version availability, synchronizes Cargo/npm/plugin versions, runs the delivery gates and finalizes the dated CHANGELOG section. If gates fail, fix the failure and run the same command with the same version. Review and commit the result before submitting a workflow.
2. In GitHub Actions, run **Release All** on `main` with **Dry run** checked (the default). Leave `candidate_run_id` empty to build a new candidate. Select `latest` for the stable channel or `next` for the experimental Linux/Windows channel. To preview an existing candidate, enter its original successful Prepare Release Run ID. A reused candidate keeps its sealed channel.
3. Wait for preparation and the read-only preview to succeed. Record the **Prepare Release** child Run ID and review the sealed notes and SHA-256 inventory in its artifact. Artifact expiry ends the normal promotion route, so retain the original reviewed artifact and its evidence. You can also run **Prepare Release** and **Publish Release** with `operation=preview` separately.

Preparation uses a repository Actions token for source/API reads. The local finalizer requires maintainer access to inspect draft Releases that the read-only workflow token may not see. Native and installed-package jobs get no npm publishing credentials. The `release-candidate-bundle` artifact contains a verified byte inventory and source/run seal. Earlier Actions candidates retain their original artifact format. Development packages cannot enter either route.

## Stage, approve and finalize

1. To publish, run **Release All** on `main` and uncheck **Dry run**. Enter the reviewed **Prepare Release** Run ID to publish the same bytes. Leaving the field empty builds and previews a new candidate before staging.
2. Wait for the Stage child to succeed. Open its summary for the verified npm stage IDs. Review the candidate and approve those stages on npm with maintainer 2FA, runtime first and then OpenCode. Release All waits up to one hour for both versions to appear on the sealed channel.
3. Once both versions are visible, Release All dispatches GitHub finalization and then independent **Check Release**. Expect Finalize to verify both full npm tarballs and their channels before writing to GitHub. A successful Check Release verifies the public downloads, clean npm installation, tag, notes and assets.

The standalone workflows can run each phase separately for inspection or recovery.

| Operation | Authority | Expected result |
|---|---|---|
| **Publish Release**, `stage` | Read-only GitHub token and environment secret `NPM_STAGE_TOKEN` | Stage or reuse the two exact npm tarballs, verify the staged bytes and show stage IDs. Wait for npm validation when necessary and rerun Stage against the same candidate to verify bytes before approval |
| **npm approval** | Maintainer npm account with 2FA | Approve verified runtime first, then OpenCode. Both versions and the sealed channel become publicly available |
| **Publish Release**, `finalize` | GitHub `release` environment and repository token with contents write | Verify both complete public npm tarballs and their channels, then publish the matching GitHub Release and candidate assets |
| **Check Release** | Read-only repository token | Independently verify public tarballs, channel, clean npm installation, GitHub Tag, notes and assets. Any retained legacy upload receipts are checked against their original Actions runs |

For **Stage**, configure the `npm-stage` environment on `main` with an expiring npm granular token limited to these two packages and the staging operations. Do not enable bypass 2FA. Keep the token in that environment secret and rotate it before expiry or after exposure. Configure `release` on `main` with the desired maintainer approval rules for GitHub finalization. Preview and Check Release require no npm credentials. See [npm stage command](https://docs.npmjs.com/cli/v11/commands/npm-stage/) and [GitHub deployment environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments).

## Recover the original candidate

Use **Re-run failed jobs** for failures or approval timeouts on the same revision. The coordinator reuses matching child runs and retries failed children by their original Run ID. If a dispatched child is temporarily absent from the Actions API, it waits up to one minute for the exact Run ID to become visible. Keep `main` on the selected revision through publication because a change fences subsequent dispatch. After committing a source fix for a failed preparation, dispatch a fresh dry-run from the updated `main`.

Standalone retries use the same successful Prepare Release Run ID. Stage reuses matching stages and preserves their channel. After an uncertain upload, inspect npm's staging inventory before retrying. Finalize preserves matching draft assets and published bytes. If public registry propagation delays Check Release, rerun only that read-only check. A conflicting tag, channel, receipt, package or Release stops the workflow.

Use `recover_candidate` only for a descendant of the original revision whose diff is limited to the release-tooling and documentation allowlist. Runtime, build/package inputs, version or CHANGELOG changes require a new unused version. Preserve published notes, dates, tags, candidate bytes and receipts. An incomplete or failed verification never authorizes a rebuild or replacement of public artifacts.
