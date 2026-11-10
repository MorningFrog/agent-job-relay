# Maintainer release guide

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

**Only maintainers publish, and every release starts manually.** Pushes, PRs, tags and Release events never publish. Versions come from Cargo.toml; package/plugin pins must match, and CHANGELOG supplies the reviewed notes. Publication always reuses the sealed candidate bytes. See [CONTRIBUTING.md](../CONTRIBUTING.md).

**CI/CD migration:** GitHub Actions execution is unavailable in the maintainer environment. [Woodpecker CI](ci.md) runs automatic Linux/Windows checks and explicit manual candidate preparation. The default publisher uses shared official local CLI authentication, maintainer npm 2FA and separately confirmed GitHub finalization; the complete 0.1.24 route is accepted below. Publication is orchestrated by the Web UI release workflow and deployment-side coordinator; the six obsolete individual CI publication workflows have been removed. Existing Actions candidates/receipts retain their original format.

Experimental macOS ARM64 cross-build inputs are accepted only by the [development packaging path](macos-arm64.md), with `--allow-unreleased` and an explicit untested descriptor. They are not added to the formal preparation graph or published support matrix.

## Current release handoff

**0.1.24 is public on npm and [GitHub](https://github.com/MorningFrog/agent-job-relay/releases/tag/v0.1.24).** Both npm latest tags select 0.1.24; complete public tarballs match original formal Woodpecker candidate #42. GitHub v0.1.24 is public/latest and its Tag targets source `baf9a7b18a219e298354f52330c2aebbc354538d`; all eight original assets and one truthful local maintainer receipt match. One independent read-only check passed public versions/channels/bytes, Tag/Release/assets/receipt and isolated credential-free npm installation, native version and OpenCode module import on Linux. Preserve this version's original notes/source/bytes and receipts, regardless of later verification outcomes.

**0.1.24 preparation passed on 2026-11-10.** PR #7 passed four required checks (#39, 507 seconds) and normally merged as `baf9a7b`; automatic main #41 passed in 432 seconds. Formal #42 passed all six workflows in 544 seconds, including actual unused-version/Tag/Release/draft checks, glibc 2.34, both native/installed-package platforms and the formal seal. Maintainer login, local preview, original-byte staging and human 2FA preceded separately authorized GitHub finalization and the independent check. No candidate rebuild, npm re-publication or CI/Windows rerun was performed for finalization/check. Existing 0.1.23 notes and earlier releases remain unchanged. Schema v14, IPC v1, 27 MCP tools and actual-host/macOS limits are unchanged.

Preserve every published version's notes, dates, tags, candidate bytes and receipts. Earlier 0.1.21/0.1.18 independent-check failures remain separate backlog items and are not repaired by a later release. Consumer CLI/plugin installation and updates use `ajr update`; native offline and OpenCode routes remain separate. Older profiles migrate to schema v14 after older active runs finish. See [updates](updating.md) and [current support/evidence](IMPLEMENTATION_STATUS.md).

## Start a release

Open the repository in **Woodpecker Web UI → New pipeline**, choose branch **main** and workflow **release**, fill the automatically displayed Release parameters, then click Run. Select `release` explicitly; internal `_ci-*` / `_release-*` workflows are available through Show internal workflows. The form provides safe mode/channel defaults and labeled candidate/recovery inputs; no variable names, desktop launcher, command or account token need entering. Advanced native variables remain available.

| Variable | Default / meaning |
|---|---|
| `RELEASE_DRY_RUN` | `true`; publication requires explicit `false` |
| `RELEASE_CANDIDATE` | Empty prepares the finalized unused version; an original successful preparation number reuses its exact bytes |
| `RELEASE_CHANNEL` | `latest` or `next`, only for a new candidate; an existing seal determines its own channel |
| `RELEASE_RESUME` | Optional original **release parent pipeline** number to resume the same local task/candidate/channel; retain its original dry-run choice |

The physical helper names are distinct from the immutable artifact producer/seal roles. Original candidates keep their old exact graph; new preparations use the exact `_release-*` graph.

The single `release-all` step shows phase, original candidate and final success/failure. It prepares or reuses, inspects original Server success/bytes, previews, then (only for non-dry-run) stages, waits for maintainer npm approval, finalizes GitHub and independently verifies public bytes/installation. Login/2FA opens explicit official prompts on the deployment host's active desktop; no commands or stage IDs need copying. Account sessions stay on that host and are not exported into CI. A missing desktop/login or a failed stage reports failure instead of pretending publication succeeded.

For a safe existing-version preview, set `RELEASE_CANDIDATE=42` and retain dry-run. A new version must first be finalized, reviewed and merged into main as described in [prepare and review](#prepare-and-review). The workflow does not choose versions or edit source. A new-candidate dry-run **does run preparation CI and save private artifacts**, but never uploads npm packages or creates Tag/Release; reuse avoids preparation. A pure deployment-side `release plan` does neither.

After interruption, start the same workflow with `RELEASE_RESUME=<original parent number>` and the same dry-run value. The deployment coordinator preserves its original local task and candidate; unknown writes are not blindly retried. Parent cancellation stops before later phases/approval, but cannot undo in-flight writes or already-public objects; child preparation is preserved. A current-main/configuration change requires review rather than editing the original intent. The deployment `onboarding/RELEASE.md` covers setup, success criteria, desktop prerequisites and recovery. The old local panel remains an optional entry, not the normal trigger.

## Woodpecker preparation

**Current acceptance:** Formal candidate #42 for source `baf9a7b` passes all six workflows, source availability, glibc 2.34, Linux/Windows native and installed-package checks, immutable sealing and independent original-run/byte inspection. Its exact bytes were published through the default local route and pass independent public checking. Old #28 remains development-only and is not promoted; no old rehearsal is rerun merely for merge, documentation or verification.

The six `_release-*.yaml` workflows run only for an explicitly requested manual `prepare` or `rehearse` task. Ordinary main Push/PR CI remains the four-check verification graph; a manual preparation does not queue that graph a second time. Source preflight precedes parallel native Linux/Windows checks, followed by dual-platform packaging/Linux installed-loader checks, actual Windows installed-package/loader checks, then sealing. No step publishes packages, tags or GitHub Releases.

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
woodpecker-cli pipeline create --branch <reviewed-branch> --workflow _release-seal --var AJR_RELEASE_TASK=rehearse --var AJR_RELEASE_CHANNEL=next --var EXPECTED_RELEASE_REVISION=<full-reviewed-commit> <repo-id>
```

Use the reviewed branch itself; an existing PR ref is refused. `_release-seal` includes all its hard dependencies. This command is a preparation request, not publication authorization. Inspect all six Server workflows and the immutable candidate. Restarting allocates a new run identity; never overwrite earlier objects. Existing preparation/inspection evidence does not need another rehearsal; formal candidate #42 and the default local account-backed publication are accepted.

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

The Web UI `release` workflow is the default entry, bound to the deployment-side `platform_manager.py release` coordinator. Its pure local `plan` is separate from `run`; run defaults to dry-run, and actual publication requires `--no-dry-run --apply`. It pins the reviewed main tools in a separate clean worktree and invokes this repository's local adapter. Shared authentication and orchestration remain outside business code. Local intent, stage and result records are private and retain original candidate identity.

Four automatic, six manual-preparation YAML files and one Release All entry are active under `.woodpecker/`; see its `_README.md`. The former six `publish-*.yaml` workflows are retired. The publication core remains because the local adapter imports it; keeping a Python helper does not create a separately executable CI workflow. Shared templates live in deployment tooling. Woodpecker filters workflow files by YAML extension, so merely naming a YAML file with `_` or `.` does not disable it; reusable non-workflow support files use another extension or stay outside that directory.

npm Trusted Publishing currently does not support self-hosted Woodpecker Workers. The default route uses the official shared local session and human staged-publishing approval, requiring no npm CI token per project. Session reuse depends on actual account/package permissions and expiry; it is not permanent validity. No account publication session is copied into CI. See [trusted publishers](https://docs.npmjs.com/trusted-publishers/) and [staged publishing](https://docs.npmjs.com/staged-publishing/); recheck platform support before changing authentication.

## Prepare and review

1. Check actual tags, registry versions, GitHub Releases/drafts and receipts before choosing an unused version. Run `python3 scripts/finalize_release.py --version <unused-x.y.z>` and its delivery gates, review the synchronized versions/final notes and merge through a PR. Reusing an original candidate does not repeat finalization.
2. In Woodpecker Web UI select main / release. Select a new candidate and dry-run to prepare once and review, or select formal publication to continue through the entire flow after confirmation. For an existing successful formal candidate, select it from the list; its version/channel/bytes remain immutable.
3. The coordinator checks actual main/version availability before a new preparation and original six-workflow success/complete bytes before preview. An unpublished preview checks official expected accounts and runs `npm stage publish --dry-run`; an already-public preview verifies public bytes/channel without requiring npm login.
4. Formal publication uploads only missing original packages or reads matching existing stages, waits for npm validation, downloads and verifies exact bytes, then prompts for human runtime/OpenCode approval in order. Cancel leaves the original stages intact. No automatic 2FA, direct publish fallback or channel mutation is used.
5. Only after public npm bytes/channel match does GitHub finalization create/resume the matching Release and upload original assets/local receipt. Existing tags/assets/notes are never replaced. The independent credential-free public-byte/install check must pass before the full task reports completion.

New preparation dispatch has a persisted unique request identity. Unknown admission is observed without a second POST; unknown stage results are reconciled read-only; unknown finalization stops automatic replay and requires original-state review. Resume uses the same local task. Actual main or binding changes stop the task rather than silently authorizing new source. The operator sequence above remains available for focused recovery, not required for normal desktop use.

## Recovery and evidence

Keep the original immutable candidate, source commit and Server run metadata. A seal upload alone is not successful preparation: inspect the exact Server run and all six workflow results. Publisher status reads are scoped by repository/pipeline and use a coherent read-only snapshot; secrets/log payloads are excluded.

Preview is read-only. Staging can reserve the npm version even before public approval; once any package is public the version is frozen. An uncertain upload must be inspected/retried against the same original candidate. Existing matching npm versions/stages are skipped or verified. No changed candidate may claim the same stage/version, and channel movement is not automated.

GitHub finalization retains the original candidate assets and adds a provider-specific receipt that public npm bytes were confirmed. It never claims that a failed final verification unpublished anything. Authenticated GitHub asset redirects drop Authorization before following signed HTTPS downloads; private response bodies and signed URLs are withheld from errors. Independent checks retrieve actual public bytes and install in isolated state without publishing credentials; they do not repeat the candidate's real host-loader/model-boundary qualification.

Legacy `.github/workflows/release*.yml`, `release_actions.py` and `release_pipeline.py` remain manual references for original format-1 Actions candidates/receipts for original published format-1 history. The new controller does not spoof Actions environment variables, import old candidates as new provider runs or alter published history. Consumer CLI/plugin installation continues through [published installation](installation.md) and `ajr update`; data profiles remain outside binary/plugin caches.
