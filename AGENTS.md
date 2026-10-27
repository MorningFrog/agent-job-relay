# Development contract

## Start here

Read `PROJECT_SPEC.md`, `docs/architecture.md` and `docs/IMPLEMENTATION_STATUS.md` first. The specification governs scope; report limitations honestly. This is one Rust package/executable with CLI, MCP, daemon and runner modes; Linux and Windows have verified native runtime cores; Windows requires breakaway permission from enclosing host jobs.

## Architecture and behavior

- Keep job execution, MCP connections, sessions and deliveries independent. Keep OS-specific behavior in `src/platform/`; unsupported operations must fail explicitly.
- `accepted_by_host` never means `confirmed_resumed` without external evidence. Distinguish simulated tests, host connectivity, plugin browser discovery and observed same-chat execution.
- Installed plugins use shared-host MCP and a fresh binding from the actual host caller (Desktop terminal or host tool context); never infer a caller from the shared process environment. Test installed entry points and keep state profiles separate from binaries.
- Interrupted install/cache replacement repair must validate its recorded snapshots, preserve backups/history and finish forward only; never infer operations by scanning caches or roll back a used schema.
- Offline plugin bundles carry no selected user profile. Preserve recorded state on plugin updates, reject downgrades, and verify explicit cache snapshots before replacement; never scan/delete host caches or stop daemons implicitly.
- Same-chat notification recovery must preserve valid delivery IDs, expiry and attempt history, fence stale claims and exclude accepted/expired events. Renew opted-in leases only after verifying the original endpoint and current claim; never move targets implicitly.
- Explicit upgrade stops must select the installed profile and revalidate the original peer before any legacy signal; never use process-name/group kills or PID alone.
- Host-plugin checks must distinguish the actual loader/session API from a simulated model. Claude Code has no maintainer account: keep its scaffold inbox-only and request community tests/PRs until real host evidence exists.
- Prefer documented public host interfaces and document private API compatibility risks. Preserve practical CLI/setup/doctor/removal and modified or foreign installation/marketplace content.

## Testing and artifacts

- Add targeted tests only for meaningful behavior, correctness or regression risk. State-machine, recovery/delivery, process-tree, SQLite transaction and IPC-permission changes require behavioral coverage; reuse the existing Rust/fake-host infrastructure.
- Avoid redundant, trivial or implementation-coupled tests. Do not test README wording, section order or specific text unless documentation itself is a product/interface under test. Keep tests of actual CLI/MCP/configuration behavior and preserve required cases, fixtures and reference data.
- Keep generated test state, endpoints, outputs, reports and logs in ignored repository-local `.tmp/` or `target/`. Do not commit temporary or historical test artifacts; never clean real user/Vanth state as test cleanup.
- Document only concise results for the latest applicable version and material verification limits. Do not accumulate execution logs or historical test reports in documentation; past results remain in Git history.

Run all delivery gates before final delivery:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

## Documentation

- Keep documentation concise, focused and logically structured with meaningful headings. Remove redundant explanations and filler while retaining evidence and operational limits.
- Write reader-facing guides in task order. Present the quickest working installation before source builds; keep README focused on getting started and navigation, put detailed usage/maintenance in their guides, and give manual test steps explicit expected results. Keep historical evidence out of installation instructions.
- Keep each Markdown paragraph on one source line; do not hard-wrap prose at fixed widths. Preserve line breaks required by lists, tables, code blocks and other Markdown syntax. Use UTF-8.
- After code changes, check `AGENTS.md`, both READMEs, `docs/TODO.md`, `docs/IMPLEMENTATION_STATUS.md` and other affected docs. Synchronize functionality, APIs, configuration, dependencies, architecture and workflows; change only relevant documentation.
- Keep the English and Chinese READMEs semantically aligned, including their bilingual introductions, language links, runnable commands and actual support boundaries.
- Use `docs/TODO.md` as the project backlog; update completed, changed and newly identified tasks without creating a duplicate root TODO. Output monitoring/interruption remains deferred until requested for implementation.
- Keep `docs/IMPLEMENTATION_STATUS.md` a current handoff and `docs/live-e2e.md` a concise current live-verification summary. ADRs preserve design decisions and regression rationale, not run logs; use Git history for past results.

## Git and operating boundaries

- Follow Conventional Commits: `<type>[optional scope]: <description>`, using appropriate types such as `feat`, `fix`, `docs`, `refactor`, `test` and `chore`. Write concise, descriptive subjects and any bodies in English.
- Commit each verifiable increment locally. Check `git status` and `git diff --check`, stage specific paths, and keep generated artifacts, private logs, databases and secrets out of Git.
- Do not modify real user configuration/services or rewrite history. Release preparation is authorized; pushing, publishing and submission still require final user confirmation. Explicitly requested Windows VM toolchain installation is allowed for verification. Never touch Vanth state; any reference repository is read-only.
- Distribute through Agent plugins around one Rust/MCP core, targeting Codex/Desktop, Claude Code and OpenCode. Independent OS installers/package repositories are cancelled; prepare OpenCode npm delivery and a ChatGPT self-hosted marketplace; official directory submissions remain future work. GitHub release operations must use a manual workflow, never push/tag triggers.
