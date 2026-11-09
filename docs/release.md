# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

**Only maintainers publish, and every release starts manually.** Pushes, PRs, tags and Release events never publish. Versions come from Cargo.toml; package/plugin pins must match, and CHANGELOG supplies the reviewed notes. Publication always reuses the sealed candidate bytes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

**CI/CD migration:** GitHub Actions execution is unavailable in the maintainer environment. [Woodpecker CI](ci.md) now verifies Linux/Windows native and installed development packages on main. The manual Woodpecker preparation graph passes exact-source PR Server acceptance and is normally merged into main. The existing Actions publisher/independent check and its npm identity remain unmigrated; they cannot consume the new provider-specific seal. No new release is authorized.

## Current release handoff

**0.1.22 is the public npm/GitHub baseline**, published on 2026-11-03 (Asia/Shanghai). Both npm latest tags, the [public v0.1.22 Release](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.22), original candidate inventory and both npm publication receipts were rechecked on 2026-11-06. The original [candidate **41575201488**](https://github.com/MorningFrog/agent-job-relay/actions/runs/41575201488) pins source `89b30452092a6762867e95c48d08f6ff946613d1`, channel latest and package hashes. Both receipts match the successful [Publish Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/41594041341); independent [Check Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/41594216844) and parent [Release All](https://github.com/MorningFrog/agent-job-relay/actions/runs/41593913378) succeeded. This metadata/receipt review does not rerun public-byte installation checks or extend actual Desktop acceptance.

**0.1.23 local preparation passed on 2026-11-06** for deferred stage/remote feedback, bounded coalesced Goal context and read-only recovery diagnostics. Later CI/CD migration changes remain under Unreleased and are outside that finalization; prepare a new unused version after the migration rather than rewriting the existing 0.1.23 notes. Require exact-revision Linux/Windows CI, a manually sealed candidate and publisher preview before publication review. Source remains schema v14, IPC v1 and 27 MCP tools; Linux release payloads require glibc 2.34 or newer. Goal resume-failure attribution and new actual-host acceptance remain open.

Preserve every published version's notes, dates, tags, candidate bytes and receipts. Earlier 0.1.21/0.1.18 independent-check failures remain separate backlog items and are not repaired by a later release. Consumer CLI/plugin installation and updates use `ajr update`; native offline and OpenCode routes remain separate. Older profiles migrate to schema v14 after older active runs finish. See [updates](updating.md) and [current support/evidence](IMPLEMENTATION_STATUS.md).

## Woodpecker preparation

**Current acceptance:** PR #4 source `4f55deb` passes the four PR checks (pipeline #30) and all six manual preparation workflows (pipeline #28, 661 seconds). Linux native/actual glibc 2.34, Windows native, both installed-package/loader checks and the immutable rehearsal seal were independently checked. This includes Windows Worker queue time alongside PR CI; it is not a standalone performance benchmark. The Worker, private exchange and manual-only configuration are enabled. PR #4 is normally merged as `c6d13b0`; independent main Push #31 also passes the four automatic workflows in 433 seconds. The six-workflow rehearsal was not repeated after merge. No formal version or publication was prepared.

The six `release-*.yaml` workflows run only for an explicitly requested manual `prepare` or `rehearse` task. Ordinary main Push/PR CI remains the four-check verification graph; a manual preparation does not queue that graph a second time. Source preflight precedes parallel native Linux/Windows checks, followed by dual-platform packaging/Linux installed-loader checks, actual Windows installed-package/loader checks, then sealing. No step publishes packages, tags or GitHub Releases.

| Manual parameter | Required value |
|---|---|
| `AJR_RELEASE_TASK` | `rehearse` for migration acceptance; `prepare` only after formal unused-version finalization |
| `EXPECTED_RELEASE_REVISION` | Full reviewed commit; must equal the actual pipeline checkout |
| `AJR_RELEASE_CHANNEL` | Explicit `latest` or `next`; stored in the seal |

`rehearse` permits a reviewed feature branch and Unreleased notes, and retains `development=true` in both packages and the resulting seal. `prepare` requires main, finalized notes and a version absent from tags, all Releases/drafts and both npm packages. Source/main/version availability is checked before native builds and again before formal sealing. Preparation never finalizes versions or edits source; use the existing local finalizer and a reviewed PR first. Do not reuse 0.1.23 for migration changes.

The administrator must provision a dedicated `worker=ajr-release` Linux Docker Worker with its own project cache volume and a separate private HTTPS release exchange. Windows retains the trusted LocalService Worker at capacity one, with a separate release compiler-cache namespace. The pinned Bookworm builder reruns native gates; its exact executable must require at most glibc 2.34 and pass version/help/SQLite initialization in the pinned AlmaLinux 9/glibc 2.34 image before upload. This is a compatibility smoke check, not full old-distribution runtime qualification.

| Manual-only secret | Scope |
|---|---|
| `release_artifact_endpoint`, `release_artifact_ca` | Separate release exchange origin and validated certificate |
| `release_artifact_linux_token` | Preflight/native/packages/candidate writer and preparation reader |
| `release_artifact_windows_token` | Windows native/report writer; preflight/package reader |
| `release_source_github_token` | Read-only repository contents, including drafts; injected only into source-preflight/formal-seal steps, never builds or installed-host tests. Required for `prepare`; rehearsal uses an explicit `UNCONFIGURED` marker with no account permissions. Formal preparation refuses that marker before making requests |

Release objects cannot enter the ordinary CI exchange, and CI credentials cannot enter release storage. Complete flat inventories, lengths, source/run/producer identity and SHA-256 are verified before extraction. Windows success evidence binds the exact consumed package bundle. The final candidate has `format=2`, `provider=woodpecker`, server/repository/run/creation/commit/mode/channel identity, notes and an asset inventory. Existing Actions candidates remain unchanged (`format=1`); the old publisher refuses the new seal. An uploaded seal alone does not prove pipeline success: independently verify the exact manual Server run and all six workflow results before acceptance. Credentials authorize trusted repository code, not signed step provenance.

After the reviewed implementation and administrator setup are enabled, start a rehearsal against the exact reviewed main/feature pipeline:

```bash
woodpecker-cli pipeline create --branch <reviewed-branch> --workflow release-seal --var AJR_RELEASE_TASK=rehearse --var AJR_RELEASE_CHANNEL=next --var EXPECTED_RELEASE_REVISION=<full-reviewed-commit> <repo-id>
```

Use the reviewed branch itself; an existing PR ref is refused. `release-seal` includes all its hard dependencies. This command is a preparation request, not publication authorization. Inspect all six Server workflows and the immutable candidate. Restarting allocates a new run identity; never overwrite earlier objects. Publisher preview, publication identity and independent public-byte verification still need migration before formal publication.

For focused local checks, run the preparation/protocol tests and lint once; native delivery gates are defined in AGENTS.md. Local CLI execution needs an explicit metadata file containing repository owner/name/id, plus explicit `--system-name woodpecker --system-url <fixture-origin>` when that file lacks system metadata. Do not falsify provider variables in production or treat fixture results as a successful Server run.

## Choose an entry point

| Workflow | Purpose | Main inputs |
|---|---|---|
| **Release All** (`release-all.yml`) | Coordinate preparation, publishing preview, publication and independent checking | dry_run=true by default; optional candidate_run_id; channel=latest for a new candidate; recover_candidate for tooling repairs |
| **Prepare Release** (`release.yml`) | Build/test Linux and Windows, package and seal a candidate without publishing credentials | channel=latest, or next for an experimental prerelease |
| **Publish Release** (`release-publish.yml`) | Preview OIDC access or submit npm/GitHub publication without waiting for synchronization | original candidate_run_id; dry_run=true by default; recover_candidate |
| **Check Release** (`release-check.yml`) | Read-only public-byte/channel/install/GitHub verification | original candidate_run_id; recover_candidate |

Publication and verification have separate outcomes: **Publication submitted** means npm accepted the uploads and GitHub publication completed; **Public verification passed** means the public bytes, channel and clean installation were checked. A failed check never republishes or rolls back a successful publication.

## Quick path: Release All

1. Run `python3 scripts/finalize_release.py --version <unused-x.y.z>` to synchronize release versions/pins, run the local checks and automatically finalize CHANGELOG only on success. Review/commit the changes and push main with maintainer authorization. Keep main unchanged during the release.
2. Open **Actions → Release All → Run workflow**, branch main. For a new unused version, leave candidate_run_id empty and channel=latest. Keep dry_run checked. The prepare job outputs the original candidate Run ID; the preview job checks credentials through the real Publish Release workflow. Download/review the candidate and notes.
3. Run **Release All** again with that original candidate_run_id and dry_run unchecked. The candidate is reused, then preview, publication and checking run as separate jobs. Channel comes from the sealed candidate; the channel selector is ignored for reuse.
4. A maintainer may choose dry_run unchecked with an empty candidate ID for a single manual prepare → preview → publish → check operation on a new unused version. The default remains dry-run for review. Never leave the ID empty for a partially published version.

Release All dispatches the existing manual workflows with the repository's automatic GitHub token; it does not publish npm itself. This preserves the configured npm identity **release-publish.yml**. Its token needs Actions write permission, but no new secret or npm Trusted Publisher is needed. Each child URL/Run ID appears in the parent Summary. Internal pipeline_key/expected_revision inputs in child workflows are filled automatically; leave them empty when operating those workflows directly.

Stable per-parent step keys recover the exact child runs after a lost dispatch response or parent re-run. A new child's default title or empty metadata may need initialization; the coordinator waits up to one minute using its exact Run ID, without dispatching again or advancing before all identity/source fields match. Populated mismatches fail immediately and identify the field. Successful preparation is reused; a failed child resumes its original Run ID. Parent and child concurrency locks differ to avoid deadlock. Changes to main are fenced before dispatch and in the child before release operations. If source/tools change, start a new Release All run on main with the original candidate and recover_candidate checked. Cancelling a parent does not cancel/roll back an already dispatched child; inspect its linked run before resuming.

## One-time npm/GitHub setup

Keep GitHub's release environment restricted to main and repository/Actions write access limited to maintainers. Optional environment reviewers still apply to Publish Release when invoked from Release All.

For **each** package, agent-job-relay-runtime and agent-job-relay-opencode-v1, configure **Settings → Trusted Publisher → GitHub Actions**:

| Field | Value |
|---|---|
| Organization or user | MorningFrog |
| Repository | agent-job-relay |
| Workflow filename | release-publish.yml |
| Environment | release |
| Allowed actions | npm publish and npm dist-tag |

Prepare Release, Check Release and Release All are not npm publishing identities. The publisher uses GitHub-hosted runners, Node 24, npm 11.21.1 and job-scoped id-token: write. It has no traditional-token fallback; NPM_STAGE_TOKEN is unused. Configure near the next release because npm requires a first successful publish within two days of creating a Trusted Publisher; dry-run does not satisfy that condition. After a successful complete OIDC release, revoke the bootstrap token and remove its unused secret. See [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/) and [GitHub workflow dispatch](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event).

## Separate operations

### Prepare and review

During development, keep changes under the single `## [Unreleased]` CHANGELOG heading. Describe the final changes relative to the previous release, consolidating related work and omitting fixes or repeated adjustments to features introduced in the same development version. Use `Fixed` for problems present in the previous release; published sections remain immutable.

At formal release preparation, pass the intended unused numeric x.y.z to the local entry point. It synchronizes Cargo.toml/Cargo.lock, both public npm manifests, OpenCode's exact runtime dependency, plugin descriptors and both online MCP declarations/launchers. The private web package retains its independent development version; the stable skill loads the selected runtime workflow without a version pin. With Python 3.11+, Git, authenticated GitHub CLI maintainer access, Node/npm and the Rust toolchain available, run from the checkout:

```bash
python3 scripts/finalize_release.py --version <unused-x.y.z>
```

Expected: the proposed metadata/notes and read-only local/remote tag, GitHub Release/draft/receipt and npm availability checks pass before version edits are applied. Frontend install/tests/build, release-flow and online-maintenance fixtures, fmt, all-target Clippy/tests and release build then pass. Only then does CHANGELOG gain `## [x.y.z] - YYYY-MM-DD` beneath one empty Unreleased heading, after a second availability/source check. The date defaults to Asia/Shanghai; `--date YYYY-MM-DD` explicitly pins a preparation date. Existing finalized dates are retained on retries; pending notes are never merged into an existing version. Review the diff and commit it before Actions; the command neither commits nor pushes/publishes. Its gates qualify the local native platform, not the remote Linux/Windows candidate or real hosts.

If checks fail after synchronization, version edits remain for review and CHANGELOG finalization stays unapplied. Fix the failure and repeat the same command. Replacements are atomic per file, not across the group; an interrupted version update can be completed with the same explicit `--version`. Unavailable inventory or invalid metadata before synchronization leaves all files untouched. Omitting `--version` retains the validate-current-Cargo-version behavior and requires already synchronized pins. See the [script responsibility index](../scripts/README.md) for packaging, Actions and verification entry points.

This local step runs before the existing manual workflows so every job checks out the same committed notes and source revision. Actions preflight remains strict; candidate reuse, publication and checking never rewrite CHANGELOG or rebuild a used version. To validate versions during development without finalizing, use `python3 scripts/release_actions.py source --allow-unreleased`.

Run **Prepare Release** on main, channel=latest for a formal release. It runs native Linux/Windows gates, Ubuntu 22.04 packaging and actual Codex/OpenCode installed-package/loader checks. OpenCode uses a simulated model; this does not establish new provider/UI acceptance. Preparation refuses a version already public or represented by a GitHub draft/release so a used version is not rebuilt.

Review the **release-candidate** artifact: release-notes.md, release.json, candidate.json, SHA256SUMS and archives. It is retained for 14 days. Save the numeric **Run ID** after /actions/runs/ in this preparation URL, not a publication Run ID or run number. No npm stage/version/channel, Git tag or GitHub Release is created during preparation.

### Preview credentials

Run **Publish Release**, branch main, original candidate_run_id, dry_run checked. Leave recover_candidate unchecked for unchanged main; check it only for allowed tooling/documentation repairs. Expect **Publication dry-run passed** after the actual publication identity exchanges OIDC tokens for both packages and previews the sealed bytes/channel. Dry-run does not prove eventual writes or public propagation and changes no remote state.

### Submit publication

Run **Publish Release** with the same candidate ID and dry_run unchecked. It checks all visible conflicts before mutation, creates/reuses the exact GitHub draft/assets, submits runtime before OpenCode, records accepted npm uploads, and makes the GitHub Release public. It never waits for public registry metadata, downloads or installation after those writes. latest produces a normal GitHub Release; next produces a prerelease.

Each accepted npm upload adds an immutable npm-publish-receipt-<run>-<attempt>-<package>.json asset. The receipt binds the original candidate, tarball SHA-256, version/channel and actual publisher run. These are supplemental acceptance records, not candidate payloads or proof of public availability. Existing matching receipts let retries skip accepted uploads even while public metadata is absent. Candidate notes, archives and checksums are never rewritten.

Expect **Publication submitted**. Public synchronization is still a separate result.

### Check public availability

Run **Check Release** with the same original candidate ID/recovery setting. It has only contents/actions read permissions and no npm publishing credential/OIDC permission. Metadata visibility and clean npm installation each have a two-minute read-only propagation retry window: npm's install metadata can lag its version/channel endpoints. Only installation ETARGET errors are retried, with a fresh install directory/cache and online metadata preference each time. Byte conflicts, newer/unknown channels, other installation errors and native/import failures stop immediately. It verifies public npm integrity/tarball SHA-256, the sealed channel, isolated registry installation/native version/OpenCode import, public GitHub notes/assets and the original source tag.

Expect **Public verification passed**. If synchronization is pending, rerun only Check Release (or only Release All's failed check job). After a checking-tool repair, start a new Check Release run on main with the original candidate ID and recover_candidate checked; re-running the old job uses its old code. No publication is repeated by the checker.

## Failure and recovery

- Preserve the **original successful preparation Run ID** and candidate bytes. Never rebuild, change source payloads or bump a partially published version as a repair.
- In Release All, use **Re-run failed jobs** to resume failed children; completed prepare/publish jobs are retained. A check-only failure requires only the check job. Standalone Publish Release also safely reuses matching public packages, acceptance receipts and GitHub assets.
- If npm accepted an upload but its receipt response was interrupted, the next run can recover from matching public metadata. If neither proof is available yet, a duplicate-version error remains a stop: inspect/wait and retry the same candidate; never overwrite, reject or unpublish it.
- If tooling changes main, use a **new** manual run on main with the original candidate_run_id and recover_candidate checked. The original source must be an ancestor, and only exact operational paths in RECOVERY_PATHS may differ, including configuration-integration fixture isolation. Runtime, package, build, version and CHANGELOG changes are refused; other Rust test paths are not generally allowed. Old run re-runs keep their old code.
- Foreign/modified notes, assets, receipts or tags, multiple populated releases, newer/unknown channels, integrity conflicts and expired candidates stop without deletion or replacement. Matching empty duplicate drafts are preserved while the populated original is reused. Uploads target exact Release IDs.
- GitHub/npm are independent services with no atomic transaction or automatic rollback. A published Release can briefly precede npm public availability; checker failure is reported separately and never retracts publication.
- Windows package failures retain bounded fixture-only diagnostics for seven days. Databases, credentials, caches and real host state are excluded.

## First-package bootstrap

If a name does not yet exist, a maintainer may download reviewed candidate tarballs, verify SHA256SUMS and publish interactively with npm login/2FA using npm publish <tarball> --access public --ignore-scripts --tag latest, runtime before OpenCode. Configure Trusted Publishing afterward. Subsequent updates use the manual workflows above.

## Consumer installation

Users follow [latest published installation](installation.md#quick-install-the-latest-release). npm latest selects the matching released GitHub marketplace; OpenCode uses agent-job-relay-opencode-v1@latest. No source checkout or Rust compiler is needed. Existing offline profiles remain separate; runtime 0.1.12+ entries replace older daemons within the selected profile; older versions retain the manual stop fallback. Updates never replay commands or migrate profiles implicitly. Official-directory approval is separate from publication.
