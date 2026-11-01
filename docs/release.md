# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

**Only maintainers publish, and every release starts manually.** Pushes, PRs, tags and Release events never publish. Versions come from Cargo.toml; package/plugin pins must match, and CHANGELOG supplies the reviewed notes. Publication always reuses the sealed candidate bytes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

## Current release handoff

**0.1.19 is the public npm/GitHub baseline**, published and rechecked on 2026-11-01. The original [sealed candidate **41291121309**](https://github.com/MorningFrog/agent-job-relay/actions/runs/41291121309), source `16e172aace7b8d15f6d640c1619e69253545909f`, was promoted through successful [Release All](https://github.com/MorningFrog/agent-job-relay/actions/runs/41297822279). The public [v0.1.19 Release](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.19), both npm latest versions, original candidate inventory and per-package publication receipts were checked. Linux release payloads require glibc 2.34 or newer. Preserve all published notes, dates, tags, candidate bytes and receipts.

The independent [Check Release **41298011740**](https://github.com/MorningFrog/agent-job-relay/actions/runs/41298011740) succeeded for that published source. The older 0.1.18 independent-check failure remains a separate historical-candidate follow-up in [TODO](TODO.md), without undoing its publication.

**0.1.20 is locally prepared, not published.** `finalize_release.py` passed all local gates and unused-version checks on 2026-11-02 and finalized the notes for feedback/recovery UI, history/log search, disk-pressure policy, bounded queuing and direct remote-alert feedback. Manifests/pins are synchronized. Remote preparation must still qualify and seal the candidate before publication. Source uses schema v14, IPC v1 and 27 MCP tools; published 0.1.19 uses v12 and 25 tools.

Current verification and package/loader limits are in [implementation status](IMPLEMENTATION_STATUS.md#current-verification). Local developer payloads can require newer glibc and cannot replace published bytes. Windows private-file fixtures and macOS backup/restore platform expectations are repaired. Current Linux, macOS portable and Rust 1.89 CI checks pass. Windows exposed a fake-host pipe-instance race; the fixture now waits for the replacement listener before registering its second chat. Requalify that native test before a new Release All dry-run. No formal 0.1.20 candidate has been sealed; retain the original sealed candidate once preparation succeeds for subsequent review and separately authorized publication.

Consumer CLI/plugin installation and updates use `ajr update`; native offline and OpenCode routes remain separate. Unreleased 0.1.20 migrates older profiles to v14 after older active runs finish. See [updates](updating.md) and [current support/evidence](IMPLEMENTATION_STATUS.md).

## Choose an entry point

| Workflow | Purpose | Main inputs |
|---|---|---|
| **Release All** (`release-all.yml`) | Coordinate preparation, publishing preview, publication and independent checking | dry_run=true by default; optional candidate_run_id; channel=latest for a new candidate; recover_candidate for tooling repairs |
| **Prepare Release** (`release.yml`) | Build/test Linux and Windows, package and seal a candidate without publishing credentials | channel=latest, or next for an experimental prerelease |
| **Publish Release** (`release-publish.yml`) | Preview OIDC access or submit npm/GitHub publication without waiting for synchronization | original candidate_run_id; dry_run=true by default; recover_candidate |
| **Check Release** (`release-check.yml`) | Read-only public-byte/channel/install/GitHub verification | original candidate_run_id; recover_candidate |

Publication and verification have separate outcomes: **Publication submitted** means npm accepted the uploads and GitHub publication completed; **Public verification passed** means the public bytes, channel and clean installation were checked. A failed check never republishes or rolls back a successful publication.

## Quick path: Release All

1. Update Cargo.toml and all pins, then run `python3 scripts/finalize_release.py`. It runs the local checks and automatically finalizes CHANGELOG only on success. Review/commit the changes and push main with maintainer authorization. Keep main unchanged during the release.
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

During development, keep changes under the single `## [Unreleased]` CHANGELOG heading. At formal release preparation, select the intended numeric x.y.z in Cargo.toml and synchronize Cargo.lock, both npm manifests, OpenCode's exact runtime dependency, plugin descriptors and both online MCP declarations/launchers. The stable skill loads the selected runtime workflow without a version pin. With Python 3.11+, Git, authenticated GitHub CLI maintainer access, Node/npm and the Rust toolchain available, run from the checkout:

```bash
python3 scripts/finalize_release.py
```

Expected: version/pin/notes validation and read-only local/remote tag, GitHub Release/draft/receipt and npm availability checks pass; frontend install/tests/build, release-flow and online-maintenance fixtures, fmt, all-target Clippy/tests and release build pass. Only then does CHANGELOG gain `## [x.y.z] - YYYY-MM-DD` beneath one empty Unreleased heading. The date defaults to Asia/Shanghai; `--date YYYY-MM-DD` explicitly pins a preparation date. Existing finalized dates are retained on retries; pending notes are never merged into an existing version. A failed check, unavailable inventory or changed tracked/untracked source leaves finalization unapplied. Review the diff and commit it before Actions; the command neither commits nor pushes/publishes. Its gates qualify the local native platform, not the remote Linux/Windows candidate or real hosts.

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
