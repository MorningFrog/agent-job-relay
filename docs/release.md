# Manual GitHub and npm releases

All publication runs through **Actions → Release → Run workflow** on `main`. Pushes, PRs, tags and Release events never trigger publication. The workflow publishes `agent-job-relay-runtime` and `agent-job-relay-opencode-v1`, plus the reviewed marketplace ZIP, npm tarballs, Release Notes, candidate manifest and checksums on GitHub. There are no scoped duplicates or independent OS installers. The current packages remain unpublished until a maintainer performs the steps below.

## One-time setup

1. In repository **Settings → Environments**, create `release`. Restrict deployment branches to `main`. Required reviewers are optional; a personal repository can use the manual workflow invocation as its publication decision.
2. For initial package creation, add the short-lived npm **Read and write (stage only)** token as environment secret `NPM_STAGE_TOKEN`. It does not need Bypass 2FA. The account must have a verified email, enabled 2FA and permission to create the two package names. If unpublished names cannot be selected individually, use temporary All Packages access and revoke the token after migration to OIDC. Organization management access is unnecessary.
3. Optionally add environment variable `NPM_EXPECTED_USER` with the actual npm username; if set, credential preflight must match it. No GitHub PAT is needed: Actions supplies `GITHUB_TOKEN` automatically.
4. Merge the workflow onto the default `main` branch before trying to run it. Workflow/configuration changes require the repository's normal review and push authorization; credential configuration does not itself authorize publication.

The npm token value is never committed or stored in artifacts. It is supplied only to the authentication/preview/staging steps. Build/dependency-install jobs receive no npm publication token. Temporary npm configuration references an environment variable and is removed afterward; OIDC mode removes inherited npm tokens rather than silently falling back to them.

## Prepare a version

`Cargo.toml` `[package].version` is the sole version authority. Use numeric `x.y.z`. Before committing, synchronize the root package in `Cargo.lock`, the two npm manifests, OpenCode's exact runtime dependency, the ChatGPT/local-plugin manifests, both MCP declarations, both online terminal launchers and the online skill pin. The source check reports a mismatched path instead of rewriting files during publication:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/release_actions.py source
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_release.py -v
```

Add a unique nonempty section to `CHANGELOG.md`, for example `## [0.1.8] - 2026-10-27`. Put changes under `Added`, `Changed`, `Fixed` and `Known limitations` as applicable. Keep future work under `Unreleased`. Actions extracts only the selected version, rejects missing/duplicate/unfinished sections, and appends platform/package/install boundaries. It does not infer version bumps, generate unreviewed claims from commits, edit source, or push a release commit. Historical sections stay in the changelog.

Commit/review the version and changelog, then push the approved source to `main`. Keep `main` unchanged until this candidate is fully published. Promotion requires exactly the same source commit; after unrelated changes, generate a new dry-run before any upload. If a candidate has already been staged, finish it before changing source so the immutable npm version is not stranded.

## First release: stage-only bootstrap

### 1. Generate a dry-run candidate

Open **Actions → Release → Run workflow** and enter:

| Field | Value |
|---|---|
| Branch | `main` |
| `dry_run` | Checked |
| `candidate_run_id` | Empty |
| `channel` | `next` for the experimental release, or `latest` for a deliberately stable release |
| `npm_auth` | `stage` |

No version input is needed. Expect credential/source checks, all four native delivery gates on Linux and Windows, installed-tarball/native/history/corrupt-payload checks, and actual Codex/OpenCode loader checks on both systems. OpenCode uses a simulated model; these checks do not claim real-provider or new ChatGPT Desktop UI acceptance. Linux builds use Ubuntu 22.04/glibc 2.35, Windows uses MSVC/static CRT. Native Windows bytes must match this run's source revision/hash.

After all checks pass, the `candidate` job uploads **release-candidate** for 14 days. Its Summary shows the version, source revision, channel, original numeric **Run ID**, and CHANGELOG-derived Release Notes. Download the artifact to review `release-notes.md`, `release.json`, `candidate.json` and `SHA256SUMS` alongside the actual archives. Save that original Run ID; it is the number in the run URL after `/actions/runs/`, not the run number or a later staging run's ID.

Dry-run creates no npm version, stage, placeholder, dist-tag, Git tag or GitHub Release. It does create ordinary Actions artifacts and summaries. Token preflight verifies npm account authentication, not the final write permission/new-name eligibility. OIDC preflight exchanges a GitHub identity for a short-lived token for each package, without uploading a version; it does not prove direct-publish/provenance acceptance. GitHub read access and tag conflicts are checked, but actual write rules remain untested until publication. Missing credentials and failed exchanges stop the workflow; successful package previews do not override failed authentication.

### 2. Stage the reviewed candidate

Run **Release** again on unchanged `main`:

| Field | Value |
|---|---|
| `dry_run` | Unchecked |
| `candidate_run_id` | Original successful dry-run Run ID |
| `channel` | Ignored; promotion inherits the sealed candidate channel |
| `npm_auth` | `stage` |

Promotion downloads only the original verified artifact; no native rebuild or repack occurs. It checks the successful manual run/repository/workflow/main/source identity, both platforms, version, complete file inventory, SHA-256 and npm integrity. Expired/missing candidates stop instead of rebuilding. GitHub/npm API errors other than an explicit not-found stop rather than being interpreted as permission to create something.

Expect a GitHub **draft** with the reviewed files, then runtime/OpenCode npm stages. Each stage is downloaded with authentication and compared byte-for-byte with the candidate. Summary lists stage IDs and says **Waiting for npm maintainer approval**. A green run at this stage means staging succeeded, not that publication is complete. For new names, npm creates a publicly visible `0.0.0-stage` placeholder; the staged version's contents remain private until approval. This is a real write and happens only with `dry_run` unchecked.

### 3. Approve the npm stages once

Sign into npm with the publishing account, open **Staged Packages**, and review package name, version, channel and the stage IDs printed by Actions. Approve the runtime first and complete 2FA; confirm its exact version is public. Then approve the OpenCode package and complete 2FA. Never approve a stage whose bytes failed the Actions verification. A stage-only token cannot approve these stages, and the workflow never requests or stores an OTP.

### 4. Finish the same release

Run **Release** a third time with `dry_run` unchecked, the **same original dry-run Run ID**, and `npm_auth=stage`. Keep the stage token available until bootstrap finishes. Matching public versions are retained without another upload; pending matching stages are reused only after channel/byte checks. The workflow downloads public tarballs, compares them with the candidate, performs a clean npm-name installation/native version/module-import check, checks the channel, and finally makes the GitHub Release public. `next` creates a prerelease; `latest` creates a normal latest Release.

Completion Summary must say **Release complete** with the GitHub URL and npm channel. A green staging run alone is not this result. Do not manually publish the GitHub draft ahead of the package checks.

## Switch subsequent releases to OIDC

Configure this shortly before the next real version publication: npm requires a new Trusted Publisher configuration to complete a first successful publish within two days. Dry-run exchange is not that successful publish; recreate an expired configuration if necessary.

For **each** npm package, open **Settings → Trusted Publisher → GitHub Actions** and enter:

| Field | Value |
|---|---|
| Organization or user | `MorningFrog` |
| Repository | `agent-job-relay` |
| Workflow filename | `release.yml` (filename only) |
| Environment | `release` |
| Allowed actions | Allow `npm publish` and `npm dist-tag` |

The workflow uses GitHub-hosted runners, Node 24, npm 11.21.1 and job-scoped `id-token: write`; the package repository URL must match this repository. Public OIDC publishing generates npm provenance automatically. The initial token-staged release is not claimed to have that OIDC provenance. See [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/), [staging](https://docs.npmjs.com/staged-publishing/) and [stage-only tokens](https://docs.npmjs.com/about-access-tokens/).

For the next version, update source/pins/CHANGELOG and run the dry-run with `npm_auth=oidc`. Review its artifact and Run ID, then run with `dry_run` unchecked, that original Run ID and `npm_auth=oidc`. Actions publishes the runtime before the OpenCode package, verifies public installation, and completes GitHub Release without an npm-site approval. After that actual OIDC publication succeeds, revoke the temporary npm token and delete environment secret `NPM_STAGE_TOKEN`. Optionally restrict package publishing access to require 2FA and disallow traditional tokens. GitHub's automatic token is independent of npm token deprecation.

## Failure and retry

GitHub and npm are separate systems; there is no atomic multi-service publish or automatic rollback. The workflow is serialized and retains its draft/used versions across failures. Always retry with the original candidate Run ID and auth mode; never rebuild different bytes under a used npm version.

- If one stage/upload succeeds and another fails, inspect Summary/error code, correct access or connectivity, and retry. Existing matching public bytes/stages/assets are verified and reused.
- If stages await 2FA, approve on npm, then run the same promotion again. Pending stages are not repeatedly uploaded. A stage's channel is immutable; a conflicting channel/content requires explicit maintainer investigation, not automatic rejection or replacement.
- If both packages are public but GitHub completion fails, retry the same promotion. npm uploads are skipped; public installation and GitHub completion repeat.
- A changed npm integrity, foreign/modified Release body/assets, unexpected tag target or missing candidate stops publication. No release asset is clobbered/deleted and no tag is moved. Fixing different published bytes requires a new version.
- If a Windows installed-package check fails, inspect the bounded status/events/stdout/stderr in the job log and download the `windows-package-diagnostics` artifact (retained for seven days). It contains only isolated fixture reports/captures, not databases, caches, credentials or real host state.
- If public registry propagation is incomplete, retry after the exact versions are readable. The workflow does not keep polling or silently claim success.

## Local package checks

From Linux x86_64/glibc with Rust, Node/npm, Python 3.11+ and readelf:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
python3 scripts/prepare_release.py --output .tmp/release-candidate-new
npm install --prefix .tmp/opencode-toolchain --cache .tmp/npm-cache --ignore-scripts --no-audit --no-fund opencode-ai@1.19.4 @opencode-ai/plugin@1.19.4 @opencode-ai/sdk@1.19.4 @openai/codex@0.164.0
python3 scripts/check_release.py --release .tmp/release-candidate-new --opencode .tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode --codex "$PWD/.tmp/opencode-toolchain/node_modules/.bin/codex"
```

Choose a new `.tmp/` output path each time. To include Windows, run its native gates from the same source, describe the executable with `scripts/describe_native.ps1`, and pass the checked bytes/descriptor using `--windows-binary` and `--windows-metadata`. On Windows, `scripts/check_windows_packages.mjs` checks the resulting candidate and actual host loaders. Local preparation never publishes, authenticates to a personal npm account or changes host configuration. Promotion is restricted to the manual main-branch Actions context. Hashes establish byte integrity; OIDC provenance and actual host execution are separate evidence.

## Consumer installation after publication

Only advertise these routes after the exact runtime/plugin versions exist and public installation passes. OpenCode V1 selects `agent-job-relay-opencode-v1@<published-version>` in its `plugin` list. ChatGPT Desktop/Codex users add the self-hosted marketplace:

```bash
codex plugin marketplace add MorningFrog/agent-job-relay
```

Restart ChatGPT Desktop, select AgentJobRelay in Plugins and enable it in a new local chat. CLI users can use `codex plugin add agent-job-relay@agent-job-relay` for tools/inbox; this does not establish automatic CLI model continuation. Node/npm and network are needed on first online use. An offline installation has a separate recorded profile: review it before explicitly selecting AJR_STATE_DIR. Updates/removal never automatically stop a daemon, replay commands or purge history. See [installation](installation.md) and [host plugins](host-plugins.md). Official directory submissions remain separate future work.
