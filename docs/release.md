# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

**Only maintainers publish, and every release starts manually.** Pushes, PRs, tags and Release events never publish. Versions come from Cargo.toml; package/plugin pins must match, and CHANGELOG supplies the reviewed notes. Publication always reuses the sealed candidate bytes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

**CI/CD migration:** GitHub Actions execution is unavailable in the maintainer environment. [Woodpecker CI](ci.md) runs automatic Linux/Windows checks and explicit manual candidate preparation. The default publisher uses shared official local CLI authentication, maintainer npm 2FA and separately confirmed GitHub finalization; the complete 0.1.24 route is accepted below. The optional CI publisher has no configured account publishing credentials. Existing Actions candidates/receipts retain their original format.

## Current release handoff

**0.1.24 is public on npm and [GitHub](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.24).** Both npm latest tags select 0.1.24; complete public tarballs match original formal Woodpecker candidate #42. GitHub v0.1.24 is public/latest and its Tag targets source `baf9a7b18a219e298354f52330c2aebbc354538d`; all eight original assets and one truthful local maintainer receipt match. One independent read-only check passed public versions/channels/bytes, Tag/Release/assets/receipt and isolated credential-free npm installation, native version and OpenCode module import on Linux. Preserve this version's original notes/source/bytes and receipts, regardless of later verification outcomes.

**0.1.24 preparation passed on 2026-11-10.** PR #7 passed four required checks (#39, 507 seconds) and normally merged as `baf9a7b`; automatic main #41 passed in 432 seconds. Formal #42 passed all six workflows in 544 seconds, including actual unused-version/Tag/Release/draft checks, glibc 2.34, both native/installed-package platforms and the formal seal. Maintainer login, local preview, original-byte staging and human 2FA preceded separately authorized GitHub finalization and the independent check. No candidate rebuild, npm re-publication or CI/Windows rerun was performed for finalization/check. Existing 0.1.23 notes and earlier releases remain unchanged. Schema v14, IPC v1, 27 MCP tools and actual-host/macOS limits are unchanged.

Preserve every published version's notes, dates, tags, candidate bytes and receipts. Earlier 0.1.21/0.1.18 independent-check failures remain separate backlog items and are not repaired by a later release. Consumer CLI/plugin installation and updates use `ajr update`; native offline and OpenCode routes remain separate. Older profiles migrate to schema v14 after older active runs finish. See [updates](updating.md) and [current support/evidence](IMPLEMENTATION_STATUS.md).

## Start a release

Pushes and tags never publish. Explicitly ask the deployment operator/Agent to prepare this project's next unused version, with publication held for review. The existing entry points handle version finalization and the reviewed PR/main checks, one manual formal preparation, original-candidate inspection, explicitly authorized staging, maintainer npm 2FA, separately authorized GitHub finalization and one independent check. The maintainer reviews prompts and completes official authentication without copying commands or stage IDs.

The deployment desktop helper currently handles npm login/approval only; it does not start the complete release graph. Without an operator/Agent, the current unified CLI still requires administrator orchestration. Administrator/Agent examples follow below; a complete standalone one-click release panel is not implemented.

## Woodpecker preparation

**Current acceptance:** Formal candidate #42 for source `baf9a7b` passes all six workflows, source availability, glibc 2.34, Linux/Windows native and installed-package checks, immutable sealing and independent original-run/byte inspection. Its exact bytes were published through the default local route and pass independent public checking. Old #28 remains development-only and is not promoted; no old rehearsal is rerun merely for merge, documentation or verification.

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
| `release_source_github_token` | Runtime value from the deployed OAuth Secret Extension, bound to this repository/main/manual context and referenced only by source-check steps. Reads main, tags and Release/draft metadata; the underlying OAuth may have account write permissions. The stored UNCONFIGURED marker remains unchanged and is refused by formal checks if forwarding is unavailable |

Release objects cannot enter the ordinary CI exchange, and CI credentials cannot enter release storage. Complete flat inventories, lengths, source/run/producer identity and SHA-256 are verified before extraction. Windows success evidence binds the exact consumed package bundle. The final candidate has `format=2`, `provider=woodpecker`, server/repository/run/creation/commit/mode/channel identity, notes and an asset inventory. Existing Actions candidates remain unchanged (`format=1`); the old publisher refuses the new seal. An uploaded seal alone does not prove pipeline success: independently verify the exact manual Server run and all six workflow results before acceptance. Credentials authorize trusted repository code, not signed step provenance.

After the reviewed implementation and administrator setup are enabled, start a rehearsal against the exact reviewed main/feature pipeline:

```bash
woodpecker-cli pipeline create --branch <reviewed-branch> --workflow release-seal --var AJR_RELEASE_TASK=rehearse --var AJR_RELEASE_CHANNEL=next --var EXPECTED_RELEASE_REVISION=<full-reviewed-commit> <repo-id>
```

Use the reviewed branch itself; an existing PR ref is refused. `release-seal` includes all its hard dependencies. This command is a preparation request, not publication authorization. Inspect all six Server workflows and the immutable candidate. Restarting allocates a new run identity; never overwrite earlier objects. Existing preparation/inspection evidence does not need another rehearsal; actual formal candidate and account-backed publication acceptance remain pending.

For focused local checks, run the preparation/protocol tests and lint once; native delivery gates are defined in AGENTS.md. Local CLI execution needs an explicit metadata file containing repository owner/name/id, plus explicit `--system-name woodpecker --system-url <fixture-origin>` when that file lacks system metadata. Do not falsify provider variables in production or treat fixture results as a successful Server run.

## Authentication policy and local manual publication

Reuse platform authentication across projects; do not treat reuse as a promise that credentials never expire. Human local operations use the host/account login managed by `gh auth login --web` and `npm login --auth-type=web`, without copying either session into Woodpecker. A login can serve multiple repositories/packages only where that account already has access. Organization SSO, package permissions and repository authorization still apply.

Manual CI source checks reuse the existing Woodpecker GitHub OAuth through an official Secret Extension maintained in deployment tooling. The administrator enables one shared internal adapter and binds each project's repository ID/name, branch, manual event and secret reference; new projects need no additional App, PAT or copy of local gh credentials. The adapter verifies signed requests and the body digest, forwards the current Server NETRC value only in memory and does not record tokens. OAuth retains the original account's permissions, potentially including writes; source-check commands only read, and this is not a repository-scoped token. Push/PR/Tag workflows receive no source secret from this extension. OAuth expiry, refresh and revocation remain under the existing platform login; reusable does not mean permanent. See [official Secret Extensions](https://woodpecker-ci.org/docs/usage/extensions/secret-extension).

The shared adapter and this repository's extension settings are enabled. Manual pipeline #38 runs only release-preflight in rehearse mode: 14 seconds execution, 26 seconds including waiting, with one verified request and one forwarded secret. That rehearsal verifies transport only. The later formal #42 run independently passed version/Tag/Release/draft checks, both native/package graphs and formal sealing; no publication occurred. The stored UNCONFIGURED value is preserved because the official extension's same-name response takes runtime precedence. If the extension is stopped, formal checks reject that marker. Restore the repository's prior extension/NETRC settings to disable this binding; platform maintenance and full rollback belong to the deployment guide. The unused GitHub App has been deleted; optional App tooling is not a required setup step.

For npm, self-hosted Woodpecker is outside current OIDC Trusted Publishing support. Prefer local interactive staging and maintainer approval, with no npm publishing secret in CI. `npm login` supplies a two-hour session usable across the account's permitted packages; reauthentication after expiry is a platform requirement, not per-project token setup. Classic npm tokens were revoked in December 2025. Granular write tokens have a maximum 90-day lifetime, and direct granular-token publication is scheduled for removal in January 2027. Stage-only tokens remain a supported optional automation route, but are not the default credential request for this deployment. See [npm authentication changes](https://github.blog/changelog/2025-12-09-npm-classic-tokens-revoked-session-based-auth-and-cli-token-management-now-available/), [token policy](https://docs.npmjs.com/about-access-tokens/) and [Trusted Publishing support](https://docs.npmjs.com/trusted-publishers/). Checked on 2026-11-09; no future policy permanence is guaranteed.

| Credential | Scope and lifecycle | Default use |
|---|---|---|
| GitHub CLI login | Local host/account; reusable for authorized repositories, subject to expiry/revocation/SSO | Human GitHub operations; keep in the CLI credential store, never export to CI |
| Woodpecker GitHub OAuth | Shared Server/account identity; original permissions, login/refresh/revocation lifecycle | Repository integration and explicitly bound manual source API checks; no new project PAT or App, no permanent-validity claim |
| npm CLI login | Local account/session; multiple permitted packages, two-hour expiry and human 2FA | Manual npm staging/approval; never import session into CI |
| Woodpecker CLI login | User on one Server; reusable for authorized projects, revocable | Local pipeline control/inspection, separate from Worker credentials |
| Worker registration credential | Deployment/Server scope; locally managed rotation | Reusable Worker manager; never expose to workflow commands |
| Artifact exchange roles | Project/service/role-specific; locally generated and managed | Keep CI, preparation and publisher roles separate; no manual third-party token creation per project |
| CI GitHub/npm publisher bindings | Existing optional workflow interface; currently unconfigured | Do not fill the earlier three-token file request as the default setup |

The accepted #28 rehearsal remains development-only and must never be staged or published. `scripts/release_local.py` implements that local adapter: each invocation retrieves the live bounded status snapshot and immutable candidate through the existing publisher read role, independently verifies the six successful original workflows/source/full inventory, and reads only original Git metadata blobs. It records a local UUID/time/workflow revision/account without inventing a Server run, preserves both existing Server and local receipts, and gates GitHub finalization on both public npm byte/channel matches. A reviewed current-main revision and formal candidate are required for every operation except inspection; source is checked again immediately before writes. The original 0.1.24 npm/GitHub publication and independent public check are accepted. Do not publish the rehearsal.

### Operator/Agent operation sequence

The deployment-side shared registry supplies platform/profile/project bindings; the project entry point remains separate from the generic manager. Set `WOODPECKER_DEPLOYMENT` to the deployment root and `release_workflow_revision` to the full reviewed checkout commit:

```bash
python3 -B "$WOODPECKER_DEPLOYMENT/platform_manager.py" local-release agent-job-relay inspect \
  --workspace "$PWD" --pipeline <original-preparation-number> \
  --expected-revision "$release_workflow_revision"
```

Only `inspect` accepts the development rehearsal on a reviewed feature branch. All other operations require the clean reviewed tool revision to match actual GitHub main, a formal original main preparation and explicit expected accounts. The shared configuration supplies expected usernames; direct script use requires `--expected-github-user` and, for preview/stage, `--expected-npm-user`. Existing `gh` login is used through the official CLI without extracting/exporting its token. Noninteractive CI token/OIDC environment variables are removed from local platform subprocesses.

1. Inspect the formal candidate using the original pipeline number, then review its version/channel/notes/hashes. No package rebuild, installation or publication occurs in inspection.
2. Use the deployment-side desktop handoff when login is needed. It explicitly prompts, opens the official npm interactive login and supplies the project/account context; maintainers complete authentication without entering commands or copying tokens. The local adapter uses the official CLI's private userconfig outside project npm configuration; administrator CLI options remain available for an explicitly selected nondefault account config.
3. Select local `preview` with the same command parameters. It checks expected GitHub/npm identity and conflicts, then runs `npm stage publish --dry-run` on the original tarballs. Requires npm >=11.15 and Node >=22.14; no upload occurs.
4. After explicit final authorization, select `stage --apply`. It lists/views existing stages, verifies metadata and exact downloaded bytes through `npm stage download`, reuses matching stages and uploads only missing original packages. It never calls approve/reject, direct publish or dist-tag mutation. Validation-pending stages stop for later inspection of the same candidate.
5. Review the exact staged bytes through the deployment desktop prompt, then explicitly confirm runtime first and OpenCode second. The helper fills the verified stage IDs and opens official interactive npm approval/2FA; maintainers do not enter commands. Confirmation publicly publishes the selected package. No OTP/session is passed through chat or CI, and no approval runs without the human confirmation.
6. After both public npm bytes/channels match, explicitly authorize local `finalize --apply` for that same candidate. It creates/resumes only a matching GitHub draft, uploads original assets plus an honest local receipt and publishes it. The shared `gh` login performs the requests. Existing or conflicting assets are never replaced.
7. Select local `check` to independently download public bytes, check channel/tag/Release and install in isolated credential-free state. Failure does not undo publication; inspect/retry the original candidate, never rebuild or republish.

Every invocation keeps an intent/result record and the verified candidate under ignored `.tmp/local-publish-*/`. A stopped mutating command is treated as potentially having written; query remote state before resuming. `stage`/`finalize` require `--apply`, which read-only operations reject. No generic `npm publish` fallback, automated 2FA approval or new build is part of this flow. Local publication does not inherit a hosted provider's OIDC provenance; original hashes/receipts are not relabeled as that attestation.

## Publication entry points

This section describes the existing **optional CI backend**. The default shared-login/local adapter and deployed manual source OAuth binding are above; no GitHub App issuer is required.

npm currently supports OIDC Trusted Publishing only on GitHub-hosted Actions, GitLab.com shared runners and CircleCI cloud; self-hosted Workers/Woodpecker are unsupported. The optional CI staging backend therefore requires a restricted granular **stage-only** token and maintainer 2FA approval; the default local route above requires no CI npm token. The stage-only setting prevents direct new-version publication; it still permits other npm writes, so this controller deliberately makes no dist-tag/deprecation/approval calls. See [trusted publishers](https://docs.npmjs.com/trusted-publishers/), [token capabilities](https://docs.npmjs.com/about-access-tokens/) and [staged publishing](https://docs.npmjs.com/staged-publishing/). These policies were checked on 2026-11-09; recheck them before changing authentication.

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

### Optional CI credential bindings

| Repository secret | Use |
|---|---|
| `release_artifact_publisher_token` | Candidate/status reads only; event manual |
| `release_source_github_token` | Shared OAuth forwarded in the bound repository/main/manual context; used for source reads by preparation and publisher preview/stage/check. Retains the original account permissions |
| `publish_github_token` | Fine-grained contents-write for this repository only; identity check and GitHub finalization, never builds or installed tests |
| `publish_npm_stage_token` | Selected runtime/OpenCode packages only, read/write **stage only**, without bypass 2FA; identity/preview/stage only |

Credentials must be entered through private local files/secret configuration, never chat, Git or command arguments. npm approval is performed by the maintainer in npmjs.com or their own interactive CLI; the Worker never receives an OTP/session credential or calls `npm stage approve`/`reject`. Stage-only is a registry capability configured by the maintainer: identity/dry-run cannot non-destructively prove all server-side staging/write policy. See [npm's command/2FA matrix](https://docs.npmjs.com/cli/v11/commands/npm-stage/).

These bindings document the existing optional CI implementation, not a request to create three tokens for every project. Source reads already use the shared OAuth extension; do not substitute a personal local CLI session into CI. No CI account publishing credentials are configured. The source binding's stored marker remains unchanged and is rejected if the extension cannot supply the runtime value. Formal availability/draft checks passed in #42; original npm staging/download verification passes; public publication remains pending. The existing published baseline and all its notes/tags/candidates/receipts remain untouched.

## Prepare and review

The formal version/candidate requirements below apply to both routes. Steps selecting `identity`, `preview`, `stage`, `finalize` or `check` describe the existing optional CI backend; use the shared local authentication policy above for the default route through the separate local entry point.

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
