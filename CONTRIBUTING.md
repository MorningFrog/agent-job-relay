# Contributing

PRs and reproducible test reports are welcome. **Claude Code needs community help:** the maintainer has no account, so its plugin is an unverified host framework with an inbox-only capability. OpenCode V1 has an actual-loader/MCP/continuation check using a local simulated model; real-provider sessions, V2, restart recovery and more versions still need contributors. Docker-based account-free checks and version matrices are welcome.

## Start with a bounded change

Read [AGENTS.md](AGENTS.md), [PROJECT_SPEC.md](PROJECT_SPEC.md), [architecture](docs/architecture.md) and [implementation status](docs/IMPLEMENTATION_STATUS.md). Use [TODO.md](docs/TODO.md) as the backlog. Keep one Rust/MCP core, exact caller/session ownership and independent command/delivery lifetimes. Do not turn host acceptance into a confirmed-resume claim.

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

Use concise English Conventional Commits and describe the concrete behavior change, validation and material limits. Keep generated artifacts under ignored `.tmp/` or `target/`. Preserve modified/foreign installation content and default data retention. Release preparation and the manual-only GitHub candidate/draft workflow are in scope; see [release guidance](docs/release.md). Actual pushes/publication still need maintainer confirmation. Do not add automatic push/tag publication, release credentials, independent OS installers or official directory submissions.
