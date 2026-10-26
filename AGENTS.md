# Development contract

## Start here

Read `PROJECT_SPEC.md`, `docs/architecture.md` and `docs/IMPLEMENTATION_STATUS.md` first. The specification governs scope; report limitations honestly. This is one Rust package/executable with CLI, MCP, daemon and runner modes; Linux is the verified runtime.

## Architecture and verification

- Keep job execution, MCP connections, sessions and deliveries independent. Keep OS-specific behavior in `src/platform/`; unsupported operations must fail explicitly.
- Add regression tests for state-machine, recovery and delivery changes. Process trees, SQLite transactions and IPC permissions require behavioral tests.
- `accepted_by_host` never means `confirmed_resumed` without external evidence. Distinguish simulated tests, host connectivity, plugin browser discovery and observed same-chat execution.
- Installed plugins use shared-host MCP and a fresh current-terminal binding; never infer a caller from the shared process environment. Test installed entry points and keep state profiles separate from binaries.
- Recovery must preserve valid delivery IDs, expiry and attempt history, fence stale claims and exclude accepted/expired events. Renew opted-in leases only after verifying the original endpoint and current claim; never move targets implicitly.
- Explicit upgrade stops must select the installed profile and revalidate the original peer before any legacy signal; never use process-name/group kills or PID alone.
- Prefer documented public host interfaces and document private API compatibility risks. Preserve practical CLI/setup/doctor/removal and modified or foreign installation/marketplace content.

Run all delivery gates before final delivery; test state and fake endpoints must stay isolated inside this repository:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

## Documentation

- Keep documentation concise, focused and logically structured with meaningful headings. Remove redundant explanations and filler while retaining evidence and operational limits.
- Keep each Markdown paragraph on one source line; do not hard-wrap prose at fixed widths. Preserve line breaks required by lists, tables, code blocks and other Markdown syntax. Use UTF-8.
- After code changes, check `AGENTS.md`, both READMEs, `docs/TODO.md`, `docs/IMPLEMENTATION_STATUS.md` and other affected docs. Synchronize functionality, APIs, configuration, dependencies, architecture and workflows; change only relevant documentation.
- Keep the English and Chinese READMEs semantically aligned, including their bilingual introductions, language links, runnable commands and actual support boundaries.
- Use `docs/TODO.md` as the project backlog; update completed, changed and newly identified tasks without creating a duplicate root TODO. Output monitoring/interruption remains deferred until requested for implementation.
- Keep `docs/IMPLEMENTATION_STATUS.md` a current handoff: version, capabilities, verification, limitations and next work. Keep detailed historical evidence in `docs/live-e2e.md`, ADRs and Git history rather than duplicating a chronological work log.

## Git and operating boundaries

- Follow Conventional Commits: `<type>[optional scope]: <description>`, using appropriate types such as `feat`, `fix`, `docs`, `refactor`, `test` and `chore`. Write concise, descriptive subjects and any bodies in English.
- Commit each verifiable increment locally. Check `git status` and `git diff --check`, stage specific paths, and keep generated artifacts, private logs, databases and secrets out of Git.
- Do not modify real user configuration/services, publish, push or rewrite history during development. Never touch Vanth state; any reference repository is read-only.
- Distribute through Agent plugins around one Rust/MCP core, targeting Codex/Desktop, Claude Code and OpenCode. Independent OS installers/package repositories are cancelled; official plugin platforms remain future work with no publication/submission now.
