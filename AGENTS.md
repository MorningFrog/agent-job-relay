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
- Packaging, package repositories and release automation are deferred in docs/TODO.md.
- Do not modify real user configuration/services, publish, push or rewrite history.
- Commit verifiable increments with English Conventional Commit titles and bodies.
- Run cargo fmt --check, cargo clippy --all-targets -- -D warnings,
  cargo test --all-targets and cargo build --release before final delivery.
