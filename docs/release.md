# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

Only maintainers initiate releases. Prepare Release, Publish Release and Check Release run on manual dispatch from `main`. Cargo.toml determines the version, CHANGELOG supplies the reviewed notes, and every subsequent step uses the original sealed candidate. Experimental macOS ARM64 packaging remains development-only.

## Current release handoff

[GitHub Release 0.1.27](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.27) publishes the Goal native-envelope fix with its original Woodpecker evidence, candidate and publication receipt. Current npm channels and installed Desktop fix acceptance require separate verification. The GitHub Actions migration remains in Unreleased. Confirm publication state and select an unused version before formal preparation.

Retain each original provider, run, published asset and receipt. Check historical versions against their original evidence and use a successful GitHub Actions Run ID for new Actions candidates.

## Prepare and review

1. Run `python3 scripts/finalize_release.py --version <unused-x.y.z>`. It verifies tag, GitHub Release/draft, receipt and npm version availability, synchronizes Cargo/npm/plugin versions, runs the delivery gates and finalizes the dated CHANGELOG section. If gates fail, fix the failure and run the same command with the same version. Review and commit the result before submitting a workflow.
2. From the repository's Actions tab, run **Prepare Release** on `main`. Select `latest` for a formal release or `next` for an experimental Linux/Windows release. Wait for the entire run to succeed and record its Run ID. Review the sealed notes and SHA-256 inventory in the candidate artifact. Artifact expiry ends the normal promotion route, so retain the original reviewed artifact and its evidence.
3. Run **Publish Release** with `operation=preview` and the original Prepare Release Run ID. Expect matching source, package and channel selection plus a read-only check of any existing GitHub Release. The manual **Release All** workflow can combine steps 2 and 3 or reuse the original Run ID for preview. It ends after preview.

Preparation uses a repository Actions token for source/API reads. The local finalizer requires maintainer access to inspect draft Releases that the read-only workflow token may not see. Native and installed-package jobs get no npm publishing credentials. The `release-candidate-bundle` artifact contains a verified byte inventory and source/run seal. Earlier Actions candidates retain their original artifact format. Development packages cannot enter either route.

## Stage, approve and finalize

| Operation | Authority | Expected result |
|---|---|---|
| **Publish Release**, `stage` | Read-only GitHub token and environment secret `NPM_STAGE_TOKEN` | Stage or reuse the two exact npm tarballs, verify the staged bytes and show stage IDs. Wait for npm validation when necessary and rerun Stage against the same candidate to verify bytes before approval |
| **npm approval** | Maintainer npm account with 2FA | Approve verified runtime first, then OpenCode. Both versions and the sealed channel become publicly available |
| **Publish Release**, `finalize` | GitHub `release` environment and repository token with contents write | Verify both complete public npm tarballs and their channels, then publish the matching GitHub Release and candidate assets |
| **Check Release** | Read-only repository token | Independently verify public tarballs, channel, clean npm installation, GitHub Tag, notes and assets. Any retained legacy upload receipts are checked against their original Actions runs |

For **Stage**, configure the `npm-stage` environment on `main` with an expiring npm granular token limited to these two packages and the staging operations. Do not enable bypass 2FA. Keep the token in that environment secret and rotate it before expiry or after exposure. Configure `release` on `main` with the desired maintainer approval rules for GitHub finalization. Preview and Check Release require no npm credentials. See [npm stage command](https://docs.npmjs.com/cli/v11/commands/npm-stage/) and [GitHub deployment environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments).

## Recover the original candidate

Retry the failed phase using the same successful Prepare Release Run ID. Stage checks for matching existing stages and preserves their channel. After an uncertain upload, inspect npm's staging inventory before retrying. Finalize waits for matching public bytes and channels, and preserves matching draft assets and published bytes. If public registry propagation delays Check Release, rerun only that read-only check. A conflicting tag, channel, receipt, package or Release stops the workflow.

Use `recover_candidate` only for a descendant of the original revision whose diff is limited to the release-tooling and documentation allowlist. Runtime, build/package inputs, version or CHANGELOG changes require a new unused version. Preserve published notes, dates, tags, candidate bytes and receipts. An incomplete or failed verification never authorizes a rebuild or replacement of public artifacts.
