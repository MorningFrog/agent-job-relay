# Development contract

Read PROJECT_SPEC.md, docs/architecture.md and docs/IMPLEMENTATION_STATUS.md first.
The project specification governs scope; report limitations honestly.

- Keep job execution, MCP connections, sessions and deliveries independent.
- Add regression tests for changes to state machines, recovery or delivery.
- `accepted_by_host` never means `confirmed_resumed` without external evidence.
- Process trees, SQLite transactions and IPC permissions require behavioral tests.
- Keep OS-specific behavior in src/platform/. Unsupported must fail explicitly.
- Use UTF-8, isolated test state inside this repository; never touch Vanth state.
- Prefer documented public host interfaces. Document private API compatibility risks.
- Maintain practical CLI/setup/doctor/removal and synchronized bilingual READMEs.
- Installed plugins use shared-host MCP and a fresh current-terminal binding;
  never infer a tool caller from a shared process environment. Test installed
  entry points, not just the source binary; keep profiles separate from binaries.
- Preserve modified/foreign install and marketplace content. Plugin browser
  discovery and same-chat live execution need separate real-host evidence.
- Distribute through Agent plugins (Codex/Desktop, Claude Code, OpenCode targets).
  Independent OS installers/package repositories are cancelled. Official plugin
  platforms are the future goal; do not publish/submit during current development.
- Do not modify real user configuration/services, publish, push or rewrite history.
- Commit verifiable increments with English Conventional Commit titles and bodies.
- Run cargo fmt --check, cargo clippy --all-targets -- -D warnings,
  cargo test --all-targets and cargo build --release before final delivery.
