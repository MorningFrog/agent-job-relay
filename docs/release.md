# Maintainer release guide

**Only maintainers may publish releases, and every release action is started manually.** Pushes, pull requests, tags and GitHub Release events never publish. Contributors prepare code and changelog changes through normal review; the maintainer chooses when to publish. See [CONTRIBUTING.md](../CONTRIBUTING.md) for code and review conventions.

## Two separate workflows

| Workflow | File | Inputs | Result |
|---|---|---|---|
| **Prepare Release** | `.github/workflows/release.yml` | `channel`, default `latest` | Build/test and seal downloadable candidate files; no npm authentication or publication |
| **Publish Release** | `.github/workflows/release-publish.yml` | Original `candidate_run_id`, `dry_run` (default checked), optional `recover_candidate` | Check the actual publishing identity, preview, then publish the same candidate bytes when dry-run is unchecked |

Preparation and publication intentionally use different inputs and permissions. Preparation is the build/packaging dry-run. The publication dry-run checks npm OIDC from the exact workflow identity that will publish; a green preparation alone does not establish publishing access. Versions come from `Cargo.toml`; publication inherits the candidate's sealed version/channel and never rebuilds or repacks.

`latest` is the default formal channel: npm default installation and GitHub's latest normal Release use that version. Choose `next` explicitly for an experimental prerelease; it leaves npm `latest` unchanged. Both channels retain documented platform and host limitations.

## One-time setup for subsequent releases

1. Keep the GitHub `release` environment restricted to `main`. Limit repository write/Actions access to maintainers; add required environment reviewers if desired. The workflow requires a maintainer's manual invocation, and only Publish Release can write release state.
2. After the two npm packages exist, open **Settings → Trusted Publisher → GitHub Actions** for **each** package: `agent-job-relay-runtime` and `agent-job-relay-opencode-v1`.
3. Enter the following values and save the configuration:

   | Field | Value |
   |---|---|
   | Organization or user | `MorningFrog` |
   | Repository | `agent-job-relay` |
   | Workflow filename | **`release-publish.yml`** |
   | Environment | `release` |
   | Allowed actions | Enable **npm publish** and **npm dist-tag** |

   The filename is not `release.yml`: that workflow only prepares packages. An earlier Trusted Publisher configuration for the old combined workflow must be replaced or supplemented with this publishing workflow's identity.
4. Configure this near the next real release: npm requires the first successful publish within two days of creating a Trusted Publisher configuration. A dry-run does not satisfy that requirement; recreate an expired configuration when needed.
5. After the first successful OIDC release, revoke the temporary bootstrap npm token and remove `NPM_STAGE_TOKEN` from GitHub. The new workflows do not reference it or fall back to traditional publishing tokens.

Publish Release uses GitHub-hosted runners, Node 24, npm 11.21.1 and job-scoped `id-token: write`. Direct OIDC publication generates npm provenance for this public repository. Identity exchange and dry-run do not prove every actual upload, provenance or repository write rule; those are checked during real publication. See [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/).

## Routine update

### 1. Prepare source and release notes

Choose a new numeric `x.y.z` version in `Cargo.toml`. Synchronize the root package in `Cargo.lock`, both npm manifests, OpenCode's exact runtime dependency, the ChatGPT/local-plugin descriptors, both MCP declarations, both online launchers and the online skill pin. The source checker reports a mismatched path instead of changing source during Actions:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/release_actions.py source
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_release.py -v
```

Add exactly one nonempty `CHANGELOG.md` section for that version, such as `## [0.1.9] - YYYY-MM-DD`. Describe actual changes and limitations; keep unfinished work under `Unreleased`. Release Notes use this version's section plus generated distribution boundaries. Missing, duplicate or unfinished entries fail preparation.

Run the four [delivery gates](testing.md), commit/review the source, then push the approved change to `main`. Keep main unchanged until publication completes. A used npm version cannot be replaced with different bytes.

### 2. Build and review the candidate

Open **Actions → Prepare Release → Run workflow**, select branch `main`, and leave `channel=latest` for a formal release. No version or credentials input is needed.

Expect native Linux/Windows formatting, Clippy, tests and release builds; Ubuntu 22.04 packaging; installed-tarball execution/history/corrupt-payload checks; and actual Codex/OpenCode loader checks on both systems. OpenCode uses a simulated model. These checks do not claim new real-provider or ChatGPT Desktop UI acceptance.

After every check passes, the candidate job uploads **release-candidate**, retained for 14 days. Its Summary shows the Cargo-derived version, sealed channel, source revision and original numeric **Run ID**. Download the artifact and review `release-notes.md`, `release.json`, `candidate.json`, `SHA256SUMS` and the archives. Save the number after `/actions/runs/` in this preparation run's URL, rather than its run number or a publication run ID.

Preparation creates no npm stage/version/placeholder/dist-tag, Git tag or GitHub Release. Build jobs receive no publishing credentials.

### 3. Check the publishing identity without publishing

Open **Actions → Publish Release → Run workflow**:

| Field | Value |
|---|---|
| Branch | `main` |
| `candidate_run_id` | Original successful Prepare Release Run ID |
| `dry_run` | **Checked** |
| `recover_candidate` | Unchecked for unchanged main |

Expect OIDC exchange for both packages, original-run/repository/workflow/source checks, artifact inventory/SHA-256/npm integrity checks and npm publication previews using the sealed channel. Existing matching public versions are reused during a partial-release retry. The Summary must say **Publication dry-run passed**. It changes no npm package, stage, channel, Git tag or GitHub Release.

### 4. Publish the reviewed candidate

Run **Publish Release** again with the same preparation Run ID and recovery setting, but **uncheck `dry_run`**. This manual invocation authorizes publication. There is no channel or authentication selector: the channel is sealed, and OIDC is required.

The workflow verifies the candidate again, creates/reuses a GitHub draft, publishes the runtime before OpenCode, compares public tarballs with the candidate, performs a clean registry installation/native version/module-import check, verifies the npm channel and makes the GitHub Release public. `latest` creates a normal latest Release; `next` creates a prerelease. No npm-site 2FA approval is needed for this OIDC path.

Expect **Release complete** with the version, channel and GitHub URL. For a formal release, check that both `npm view <package>@latest version` results equal the intended version and GitHub no longer marks it as a prerelease.

## Failure and retry

GitHub and npm are separate systems; there is no atomic multi-service publication or automatic rollback. Both workflows share a concurrency group. Retry publication with the **original preparation Run ID**, preserving matching public bytes/assets; never rebuild a different package under a used npm version.

- A successful npm upload or channel update can precede public registry visibility. The workflow allows a two-minute propagation window with read-only checks every five seconds, without repeating the upload/tag write. Byte conflicts or newer/unknown channels stop immediately. A timeout preserves the upload and GitHub draft; retry the same candidate after propagation.
- If one package publishes and another fails, correct the reported access/connectivity problem, run Publish Release with dry-run checked, then publish the same candidate. Matching versions are verified and skipped.
- If npm succeeds but GitHub completion fails, retry the same candidate; public installation and GitHub completion repeat without uploading npm versions again.
- Conflicting npm integrity/channels, modified or foreign Release content, unexpected tag targets and missing/expired artifacts stop. No asset is clobbered/deleted, no stage is rejected and no tag is moved.
- Windows package failures retain bounded fixture-only status/events/output in **windows-package-diagnostics** for seven days. Databases, caches, credentials and real host state are excluded.
- If a publication-tooling repair changes main, choose a **new Publish Release run**, check `recover_candidate`, and retain the original preparation Run ID. The original commit must be an ancestor; only exact tooling/operational-documentation paths in `RECOVERY_PATHS` may differ. Runtime/package/build/version/CHANGELOG changes require a new candidate before publication. The old notes, bytes and source identity are retained.
- Authenticated pagination finds GitHub drafts; uploads target exact Release IDs. A matching empty duplicate from older tooling is preserved while reusing the populated original. Multiple populated drafts or foreign content stop.

An old combined Release run's Re-run keeps its old workflow. Use the current named workflows for subsequent versions.

## First-package bootstrap

If the package names do not yet exist, a maintainer may download a reviewed Prepare Release artifact and publish those tarballs interactively with an npm account and 2FA. Verify `SHA256SUMS`, then publish the runtime before OpenCode using `npm publish <tarball> --access public --ignore-scripts --tag latest`. Configure Trusted Publishing afterward. This bootstrap exception is maintainer-only; subsequent updates use the two workflows above.

## Consumer installation

Users install the latest published versions through the [quick installation guide](installation.md#quick-install-the-latest-release): npm latest selects the matching released GitHub marketplace, and OpenCode uses `agent-job-relay-opencode-v1@latest`. The GitHub Release ZIP is an alternative catalog source; both routes keep exact runtime/plugin dependencies together. No source checkout or Rust compiler is needed.

Node/npm and network are needed for online installation. Existing offline installations keep their recorded profiles; updates never implicitly stop daemons, replay commands or migrate profiles. Official directory submission is separate from publication.
