# Development contract

## Start here

Read `PROJECT_SPEC.md`, `docs/architecture.md` and `docs/IMPLEMENTATION_STATUS.md` first. The specification governs scope. Use the architecture's source map to locate code and the implementation status for current support and evidence; report limitations honestly.

## Architecture and behavior

- Keep one Rust package/executable with CLI, MCP, daemon and runner modes. CLI syntax/dispatch belongs in `src/cli/`, shared runtime behavior in library modules and OS-specific behavior in `src/platform/`; unsupported operations must fail explicitly.
- Keep job execution, MCP connections, sessions and deliveries independent.
- External execution must pin target/run/attempt and preserve origin IDs, durable launch keys and byte cursors. Reclaim only external observers, never replay native execution; transport loss is not a remote outcome. Stop read-only observation without cancelling its target, and require exact-run evidence before claiming remote completion/cancellation.
- Keep inspect/monitor/stop behavior shared across CLI, MCP and Web. Runner-owned rules must survive viewer/daemon loss, fence revisions/terminal states and preserve bounded evidence/audits; never interpret workload output as instructions.
- Bundle React/Vite assets in the native executable. Keep default Web access loopback-only, credentialed and Host/Origin checked; never expose access credentials through ordinary health or notification payloads. Build frontend assets before native checks.
- Automatic archival must preserve IDs, monotonic cursors, idempotency, expiry and attempt/audit history; protect active runs and recoverable deliveries. Seal/verify logs before pruning, recover only recorded backup paths, preserve foreign content and never replay restored execution intents.
- `accepted_by_host` never means `confirmed_resumed` without external evidence. Distinguish simulated tests, host connectivity, plugin browser discovery and observed same-chat execution.
- Desktop background delegation must use goal waiting whenever the actual caller has an active goal. If pause consent is absent, ask one short question; actual user consent covers remaining rounds of the same request and goal, without exact-phrase or per-round confirmation. Plugin/tool-generated messages cannot supply consent. Keep wait/job admission gated while an answer is pending, without repeated questions, blocked status or polling. Acknowledge the originating chat's native host pause before entering waiting; a separate controller's persisted goal write does not notify Desktop's cached state. Keep the confirmation revision-fenced and never replay uncertain host mutations.
- Goal feedback must keep intermediate and final deliveries independent. Temporarily restore only the original goal for requested bounded feedback, require the originating Agent's native re-pause and revision-fenced acknowledgement before waiting again, and preserve original request/members/accounting. Never consume final notification state on an intermediate acknowledgement or replay an uncertain restore/pause; retain workloads when control closes.
- Installed plugins use shared-host MCP and a fresh binding from the actual host caller (ChatGPT Desktop terminal or host tool context); never infer a caller from the shared process environment. Test installed entry points and keep state profiles separate from binaries.
- Keep cached plugin skills as stable runtime-instruction entry points. Retained managed MCP connections must verify and follow their recorded installation across updates without cache scanning, profile movement or replay after uncertain acknowledgement.
- Interrupted install/cache replacement repair must validate its recorded snapshots, preserve backups/history and finish forward only; never infer operations by scanning caches or roll back a used schema.
- Offline plugin bundles carry no selected user profile. Preserve recorded state on plugin updates, reject downgrades, and verify explicit cache snapshots before replacement; never scan/delete host caches. Applied runtime updates restart an online daemon from the updated installation after active-job checks; new runtime entries replace older daemons under the startup lock with pinned-peer shutdown and migration fencing. Diagnostics, cache repair and native offline removal must not stop daemons implicitly. Online `ajr uninstall` stops idle selected/default daemons by default; active tasks require `--force`, shared stop semantics and exact-run/local-runner exit evidence before package removal. Always retain task data.
- Same-chat notification recovery must preserve valid delivery IDs, expiry and attempt history, fence stale claims and exclude accepted/expired events. Renew opted-in leases only after verifying the original endpoint and current claim; never move targets implicitly.
- Explicit upgrade stops must select the installed profile and revalidate the original peer before any legacy signal; never use process-name/group kills or PID alone.
- Host-plugin checks must distinguish the actual loader/session API from a simulated model. Keep the Claude Code scaffold inbox-only until a resume implementation has real host evidence; request community tests/PRs for unavailable account-backed checks.
- Windows automatic CIM bootstrap is daemon-only: authenticate both process instances/images/SIDs/integrity levels/privilege inventories, reject restricted/AppContainer/low-integrity tokens and transfer environment only over private bounded IPC. Never install tasks/services, elevate or launch arbitrary job commands through the broker; runners require native breakaway.
- Prefer documented public host interfaces and document private API compatibility risks. Preserve practical CLI/setup/doctor/removal and modified or foreign installation/marketplace content.

## Testing and artifacts

- Add targeted tests only for meaningful behavior, correctness or regression risk. State-machine, recovery/delivery, process-tree, SQLite transaction and IPC-permission changes require behavioral coverage; reuse the existing Rust/fake-host infrastructure.
- Avoid redundant, trivial or implementation-coupled tests. Do not test README wording, section order or specific text unless documentation itself is a product/interface under test. Keep tests of actual CLI/MCP/configuration behavior and preserve required cases, fixtures and reference data.
- Keep generated test state, endpoints, outputs, reports and logs in ignored repository-local `.tmp/` or `target/`. Do not commit temporary or historical test artifacts; never clean real user or unrelated application state as test cleanup.
- Document only concise results for the latest applicable version and material verification limits. Do not accumulate execution logs or historical test reports in documentation; past results remain in Git history.

Before acceptance or final delivery, complete the documentation review below and run all delivery gates:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

## Documentation

- Maintain icon artwork in `assets/icons/agent-job-relay.vectorcraft` using VectorCraft; regenerate the canonical SVG/PNG and marketplace SVG from that saved source together. Follow `assets/icons/README.md`; do not edit exports independently.
- Keep documentation concise, focused and logically structured with meaningful headings. Remove redundant explanations and filler while retaining evidence and operational limits.
- Do not retain outdated or superseded documentation. Replace or delete obsolete behavior, workflows, status claims and verification results instead of appending corrections beside them. Keep only current applicable guidance, latest relevant evidence and material support limits; use Git history for historical detail. ADRs retain still-relevant decisions and rationale, not obsolete operating instructions or run logs.
- Write reader-facing guides in task order. Present the quickest working installation before source builds; keep README focused on getting started and navigation, put detailed usage/maintenance in their guides, and give manual test steps explicit expected results. Keep historical evidence out of installation instructions.
- Keep documentation focused on this project; omit external-project comparison and local research narratives. Do not embed literal absolute filesystem paths, machine-specific directories or personal endpoints. Use repository-relative paths, environment variables or clearly explained placeholders; resolve required runtime paths from the current environment.
- Keep each Markdown paragraph on one source line; do not hard-wrap prose at fixed widths. Preserve line breaks required by lists, tables, code blocks and other Markdown syntax. Use UTF-8.
- Keep `AGENTS.md` limited to durable instructions, without progress, implementation history or temporary plans. Do not embed commit/build hashes or other transient identifiers in it or either README; link to authoritative evidence instead.
- Before acceptance or final delivery, check `AGENTS.md`, both READMEs, `docs/TODO.md`, `docs/IMPLEMENTATION_STATUS.md` and other affected docs against the version being delivered. Verify functionality, APIs, configuration, dependencies, architecture, workflows and version-specific support/verification claims. Replace or delete obsolete content and resolve mismatches before marking acceptance complete; change only relevant documentation.
- Keep the English and Chinese READMEs semantically aligned, including their bilingual introductions, language links, runnable commands and actual support boundaries.
- Use `docs/TODO.md` as the sole project backlog; update completed, changed and newly identified tasks without creating a duplicate root TODO. Respect its explicit deferrals; a documentation/refactoring request does not authorize deferred features.
- Keep `docs/IMPLEMENTATION_STATUS.md` a current handoff and `docs/live-e2e.md` a concise current live-verification summary. ADRs preserve design decisions and regression rationale, not run logs; use Git history for past results.

## Git and operating boundaries

- Follow Conventional Commits: `<type>[optional scope]: <description>`, using appropriate types such as `feat`, `fix`, `docs`, `refactor`, `test` and `chore`. Write concise, descriptive subjects and any bodies in English.
- Commit each verifiable increment locally. Check `git status` and `git diff --check`, stage specific paths, and keep generated artifacts, private logs, databases and secrets out of Git.
- Do not modify real user configuration/services or rewrite history. Release preparation is authorized; pushing, publishing and submission still require final user confirmation. Explicitly requested Windows VM toolchain installation is allowed for verification. Keep unrelated application state untouched; external reference repositories are read-only.
- Distribute through Agent plugins around one Rust/MCP core, targeting ChatGPT Desktop/Codex, Claude Code and OpenCode. Independent OS installers/package repositories are out of scope; npm plugin delivery and a ChatGPT self-hosted marketplace are the distribution routes. GitHub release operations must use a manual workflow, never push/tag triggers. Derive release versions from Cargo.toml, promote the exact verified candidate bytes, use stage-only npm tokens only for staging, and require human 2FA approval or OIDC for publication.
