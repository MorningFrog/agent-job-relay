# Contributing

PRs and reproducible test reports are welcome. **Claude Code needs community help:** the maintainer has no account, so its plugin is an unverified host framework with an inbox-only capability. OpenCode V1 has an actual-loader/MCP/continuation check using a local simulated model; real-provider sessions, V2, restart recovery and more versions still need contributors. Docker-based account-free checks and version matrices are welcome.

## Start with a bounded change

Read [AGENTS.md](AGENTS.md), [PROJECT_SPEC.md](PROJECT_SPEC.md), [architecture](docs/architecture.md) and [implementation status](docs/IMPLEMENTATION_STATUS.md). Use [TODO.md](docs/TODO.md) as the backlog. Keep one Rust/MCP core, exact caller/session ownership and independent command/delivery lifetimes. Do not turn host acceptance into a confirmed-resume claim.

## Code and documentation conventions

- Keep one Rust package/executable. Put CLI syntax/dispatch in `src/cli/`, shared behavior in library modules and OS behavior in `src/platform/`. Unsupported operations must fail explicitly.
- Follow `cargo fmt` and warning-free Clippy. Validate inputs and preserve caller/session/profile ownership; avoid implicit command replay, profile changes or unrelated process cleanup.
- Add behavioral tests for meaningful correctness/regression risk, especially state transitions, recovery, SQLite transactions, process trees and IPC permissions. Reuse existing fixtures; avoid tests that merely copy implementation details or check README wording.
- Use isolated `.tmp/` or `target/` test state. Keep credentials, databases, raw host transcripts and generated reports out of commits. Never modify real user configuration/services as test setup or cleanup.
- Update relevant guides and implementation status when behavior changes. Keep English/Chinese READMEs aligned, Markdown paragraphs on one source line, and paths portable. Maintain only the latest applicable evidence; use Git history for old runs.

## Required checks

Before submitting a code change, run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

For host work, also run the applicable account-free checks in [host plugins](docs/host-plugins.md), then the actual host acceptance when you have authorized access. Add meaningful behavioral regression coverage for routing, recovery, transactions or IPC changes. README wording tests and generated run logs do not belong in the suite or repository.

## Community host acceptance

Please report these separately:

| Evidence | What to include |
|---|---|
| Environment | OS/architecture, exact host/SDK/core version, V1 versus V2, local or container |
| Loader | Actual manifest/plugin discovery, namespace and MCP tool discovery; distinguish a local protocol self-check |
| Caller routing | How the real caller ID was obtained; two sessions must keep independent targets |
| Execution | Explicit argv/cwd, harmless command outcome, logs/events and same retained job/run; no secrets |
| Notification | Inbox recording versus host admission, delivery retries/deduplication and target correlation |
| Continuation | Whether a real provider or a simulated model ran in the same session without another user prompt |
| Lifecycle | Host exit/restart/rebind if tested; command replay and implicit profile switches are failures |

For Claude Code, verify `--plugin-dir`, manifest validation, real `${CLAUDE_SESSION_ID}` skill substitution, actual MCP submission and inbox inspection before proposing wake support. A future hook/channel PR needs a documented public interface and account-backed session/idle/failure evidence. The existing framework must continue to report `can_resume=false` until that implementation is verified.

Use disposable repository-local `.tmp/` state and cleanly stop only your test instances. Keep real user and unrelated application state untouched. Do not publish raw logs, transcripts, socket paths, session keys, credentials or databases; share a concise sanitized result and the reproducing script. A Docker pass is useful interface evidence but does not prove ChatGPT Desktop UI or an account-backed host run.

## PR scope and history

Use concise English Conventional Commits and describe the concrete behavior change, validation and material limits. Keep generated artifacts under ignored `.tmp/` or `target/`. Preserve modified/foreign installation content and default data retention. Release preparation and the separate manual preparation/publication workflows are in scope; see [release guidance](docs/release.md). Actual pushes/publication still need maintainer confirmation. Do not add automatic push/tag publication, release credentials, independent OS installers or official directory submissions.

## Maintainer-only manual releases

**Publishing npm packages, changing release channels and publishing GitHub Releases are maintainer-only operations, performed manually.** Contributor pushes, PRs, tags and Release events must never trigger publication. GitHub's `release` environment is restricted to `main`; repository/Actions access and any environment approval rules define who may authorize a release. Contributors can prepare and test a change but cannot treat a merged PR or green CI as permission to publish.

The normal update sequence is:

1. Update the version in `Cargo.toml`, synchronize the lockfile and package/plugin pins, and add the matching version section to `CHANGELOG.md`. Complete the checks above and obtain review before pushing the approved source to main.
2. Manually run **Prepare Release** with `channel=latest` for a formal release. It builds/tests Linux and Windows and seals the downloadable candidate, without publishing credentials. Review its files/Release Notes and save its original Run ID.
3. Manually run **Publish Release** with that Run ID and `dry_run` checked. This validates the candidate and npm OIDC from the actual publishing workflow. A successful preparation alone is not a credential check.
4. After review, manually run **Publish Release** with the same Run ID and `dry_run` unchecked. It submits the exact reviewed files, retains upload-acceptance receipts and publishes GitHub without waiting for public registry synchronization.
5. Manually run **Check Release** with the same candidate ID to verify public bytes, channel and installation. This check has no remote write permission; a check failure requires only a check retry.

**Release All** is the unified manual shortcut for this sequence. Its default dry-run prepares/reuses a candidate and checks the actual publisher identity. Unchecking dry-run also publishes and starts a separate checking job. Reuse the original candidate ID after review or partial failure; successful child runs are retained, and failed children resume their original Run IDs. The wrapper dispatches release-publish.yml so npm Trusted Publisher configuration stays unchanged.

Use `next` only for an intentional experimental prerelease; it does not change npm's default `latest`. Keep main unchanged during a release, and never replace bytes under a staged/published version. A documented tooling-only recovery mode can reuse the original candidate after a release-script repair; it cannot admit payload/version changes.

For npm Trusted Publisher settings, exact Actions inputs, retry limits and the maintainer's one-time manual first-package bootstrap, follow the [maintainer release guide](docs/release.md). Subsequent automated releases use OIDC configured for **`release-publish.yml`**; they do not use a stored npm publishing token. An Agent may prepare code, tests and artifacts, but the maintainer remains responsible for manually authorizing release execution.
