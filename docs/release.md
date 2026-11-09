# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

**Only maintainers publish, and every release starts manually.** Pushes, PRs, tags and Release events never publish. Versions come from Cargo.toml; package/plugin pins must match, and CHANGELOG supplies the reviewed notes. Publication always reuses the sealed candidate bytes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

**CI/CD migration:** GitHub Actions execution is unavailable in the maintainer environment. [Woodpecker CI](ci.md) now verifies Linux/Windows native and installed development packages on main. The manual Woodpecker preparation graph passes exact-source PR Server acceptance and is normally merged into main. The new publisher/independent-check implementation is local and awaits deployment/read-only Server inspection plus real credential setup. Existing Actions candidates/receipts retain their original format and are not relabeled. No new release is authorized.

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

## Publication entry points

npm currently supports OIDC Trusted Publishing only on GitHub-hosted Actions, GitLab.com shared runners and CircleCI cloud; self-hosted Workers/Woodpecker are unsupported. This backend therefore uses a restricted granular **stage-only** token and maintainer 2FA approval. The stage-only setting prevents direct new-version publication; it still permits other npm writes, so this controller deliberately makes no dist-tag/deprecation/approval calls. See [trusted publishers](https://docs.npmjs.com/trusted-publishers/), [token capabilities](https://docs.npmjs.com/about-access-tokens/) and [staged publishing](https://docs.npmjs.com/staged-publishing/). These policies were checked on 2026-11-09; recheck them before changing authentication.

| Manual task / workflow | Credentials and effects |
|---|---|
| `inspect` / `publish-inspect` | Read-only candidate/status role; inspect an accepted formal or development preparation without executing it. A development inspection never authorizes publication |
| `identity` / `publish-identity` | npm account and GitHub repository read check only; useful after credential setup, without a formal candidate. It does not prove write permissions |
| `preview` / `publish-preview` | Formal candidate only, account/source/conflict checks and `npm stage publish --dry-run`; no npm or GitHub mutation |
| `stage` / `publish-stage` | Formal original tarballs only; reuse matching existing stages, verify staged bytes when validation is ready, then stop for maintainer 2FA. No GitHub draft or automatic npm approval |
| `finalize` / `publish-finalize` | After both public npm tarballs and their channel match, create/resume the exact GitHub draft, upload original assets/receipt and publish it. No npm write or rebuild |
| `check` / `publish-check` | Independent public npm/GitHub byte, channel, tag and clean-registry installation check. No staging or publication credentials reach installed code |

All operations require an explicit manual event and full expected workflow revision. Only inspection permits a reviewed feature branch; identity/preview/stage/finalize/check run from main. Formal mutation requires `format=2`, `provider=woodpecker`, `development=false`, original main preparation and six successful workflows. Channel/version come from the seal and cannot be overridden. The original source metadata is read from fixed Git blobs, so inspecting/checking a prior candidate does not rebuild it or execute old source. Publication source changes are restricted to allowlisted tooling/documentation repairs; otherwise select the original reviewed workflow source.

The publisher runs on a dedicated `worker=ajr-publish` Docker Worker. Its artifact role can only read a sealed candidate and that repository's bounded status snapshot. It cannot write candidates, native files or diagnostics. The private status gateway opens Woodpecker SQLite in read-only/query-only mode and returns a fixed pipeline/workflow field list; no Server management token or arbitrary SQL interface reaches the Worker. The reviewed deployment update and read-only status-directory mount are enabled, preserving existing storage and Linux/Windows role credentials. Publisher Worker #5 passes real manual inspection #33 of accepted development rehearsal #28 in 9 seconds; PR #5 is normally merged as `fe623c3`, with all four independent main Push #34 checks passing in 462 seconds. Account-backed publication remains unqualified. See [current evidence](IMPLEMENTATION_STATUS.md).

### Configure credentials privately

| Repository secret | Use |
|---|---|
| `release_artifact_publisher_token` | Candidate/status reads only; event manual |
| `release_source_github_token` | Existing source credential, restricted to this repository's read-only contents including draft metadata; preparation and publisher preview/stage/check |
| `publish_github_token` | Fine-grained contents-write for this repository only; identity check and GitHub finalization, never builds or installed tests |
| `publish_npm_stage_token` | Selected runtime/OpenCode packages only, read/write **stage only**, without bypass 2FA; identity/preview/stage only |

Credentials must be entered through private local files/secret configuration, never chat, Git or command arguments. npm approval is performed by the maintainer in npmjs.com or their own interactive CLI; the Worker never receives an OTP/session credential or calls `npm stage approve`/`reject`. Stage-only is a registry capability configured by the maintainer: identity/dry-run cannot non-destructively prove all server-side staging/write policy. See [npm's command/2FA matrix](https://docs.npmjs.com/cli/v11/commands/npm-stage/).

No publisher credentials are currently configured by this migration step. Preparation's source credential still has an explicit unconfigured marker; formal preparation will refuse it. The existing published 0.1.22 baseline and all its notes/tags/candidates/receipts remain untouched.

## Prepare and review

1. Check actual tags, registry versions, GitHub Releases/drafts and publication receipts before selecting an unused version. Run `python3 scripts/finalize_release.py --version <unused-x.y.z>`; review/commit the synchronized versions and finalized notes, then use an authorized PR. Existing dated 0.1.23 notes are not reused for later migration changes.
2. Run the manual `release-seal` preparation graph with `AJR_RELEASE_TASK=prepare`, the full reviewed main commit and the intended channel. Keep its original pipeline number and private candidate bytes. The default migration rehearsal remains development-only and is never a publishable substitute.
3. Review the candidate, notes and original successful six-workflow Server execution. Run publisher preview against that original pipeline. It checks account identity and conflicts but performs no upload.
4. After explicit maintainer authorization, select `stage`. It stages exact reviewed tarballs; if npm is still validating, rerun only the same publisher step/candidate after validation. Matching existing stages are read/verified, not overwritten. Never start a fresh preparation to recover an uncertain staging response.
5. Once staged bytes are verified, review and approve runtime first and OpenCode second using npm's 2FA UI/interactive CLI. This is the npm public-publication step and always requires human intervention. No automatic polling/approval is performed by the publisher.
6. After public npm bytes/channel match, authorize `finalize` for that same original candidate. It only publishes GitHub state after those checks. Existing public assets/tags/notes must match and are never replaced; ambiguous/foreign Releases or receipts stop the operation for inspection.
7. Run the independent read-only `check`. A failed check does not undo publication: retry verification or resume the same original finalization, never rebuild, relabel or republish the version.

Use the known repository ID, original candidate pipeline number, full current reviewed workflow revision and expected npm username as public parameters. For example, after setting `release_repo_id`, `release_candidate_number`, `release_workflow_revision` and `release_npm_user` in your shell:

```bash
woodpecker-cli pipeline create --branch main --workflow publish-preview \
  --var AJR_RELEASE_TASK=preview \
  --var "EXPECTED_RELEASE_REVISION=$release_workflow_revision" \
  --var "AJR_CANDIDATE_PIPELINE_NUMBER=$release_candidate_number" \
  --var "NPM_EXPECTED_USER=$release_npm_user" "$release_repo_id"
```

Select the matching workflow/task pair for each subsequent operation. These are independent short operations; they do not rerun native CI or preparation. Default CI is excluded for recognized publication tasks. `identity` requires no candidate number; it is optional after setup because preview already checks identity. To validate the migration with the accepted development rehearsal, use `publish-inspect` / `inspect` on a reviewed branch: it must report development-only and publication unauthorized, with no upload.

## Recovery and evidence

Keep the original immutable candidate, source commit and Server run metadata. A seal upload alone is not successful preparation: inspect the exact Server run and all six workflow results. Publisher status reads are scoped by repository/pipeline and use a coherent read-only snapshot; secrets/log payloads are excluded.

Preview is read-only. Staging can reserve the npm version even before public approval; once any package is public the version is frozen. An uncertain upload must be inspected/retried against the same original candidate. Existing matching npm versions/stages are skipped or verified. No changed candidate may claim the same stage/version, and channel movement is not automated.

GitHub finalization retains the original candidate assets and adds a provider-specific receipt that public npm bytes were confirmed. It never claims that a failed final verification unpublished anything. Authenticated GitHub asset redirects drop Authorization before following signed HTTPS downloads; private response bodies and signed URLs are withheld from errors. Independent checks retrieve actual public bytes and install in isolated state without publishing credentials; they do not repeat the candidate's real host-loader/model-boundary qualification.

Legacy `.github/workflows/release*.yml`, `release_actions.py` and `release_pipeline.py` remain manual references for original format-1 Actions candidates/receipts while migration acceptance is incomplete. The new controller does not spoof Actions environment variables, import old candidates as new provider runs or alter published history. Consumer CLI/plugin installation continues through [published installation](installation.md) and `ajr update`; data profiles remain outside binary/plugin caches.
