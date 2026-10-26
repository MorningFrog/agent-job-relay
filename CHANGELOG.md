# Changelog

## 0.1.4 — 2026-10-26 (unreleased plugin runtime lifecycle)

- Export a relocatable offline native plugin bundle; bootstrap validates the bundle and installation/marketplace ownership before writing, with no recipient Rust/Python/Node toolchain.
- Update from checked local bundles while retaining the recorded state and refusing downgrades, active runtimes and modified/foreign files; generate plugin version metadata from the actual package version.
- Route cached terminal and MCP entry points to the same managed native runtime; add explicit cache status and backed-up snapshot refresh, including update preflight across selected caches. Never scan/delete caches or change host enablement.
- Qualify offline bootstrap/cache handling with the actual isolated Codex loader and 20 MCP tools, without touching personal settings or a real Desktop endpoint.
- Record the user-confirmed installed MCP submission/same-chat continuation pass without CLI fallback; read-only review matches successful completion and one accepted delivery. Host continuation remains user-attested; no automatic confirmed-resume capability or publication is claimed.
- Remove README wording/structure assertions, retain behavioral tests/fixtures, and keep current verification concise with generated artifacts ignored.

## 0.1.3 — 2026-10-26 (unreleased notification recovery)

- Add preview/apply same-chat recovery through CLI/MCP with atomic audits, preserved delivery IDs/history/TTL, and no accepted-event or command replay.

- Retry temporary delivery failures until TTL with jitter capped at five minutes; pause changed identities for explicit recovery and stop permanent failures.
- Preserve delivery IDs/history and expose failure kind and next-attempt time; migrate existing delivery records to schema v6 without data loss.

- Renew opted-in long-job session leases at delivery only after a same-endpoint capability probe and fenced transaction/audit; refuse old daemons before binding.
- Keep task output monitoring/interruption recorded for a later increment.

## 0.1.2 — 2026-10-26 (unreleased upgrade repair)

- Add `daemon stop --install-root` to select the recorded installation profile regardless of stale shell state exports.
- Gracefully stop verified early 0.1.0 daemons that reject IPC shutdown, using their existing Ctrl-C handler with PID-reuse-safe Linux pidfds. Refuse unknown peers, stale identities and unrelated errors; never use a group signal/force kill.
- Wait for daemon lock release before successful stop returns, allowing immediate update; improve active-install errors with a concrete profile-aware command.
- Add legacy-upgrade, unrelated-peer and non-shutdown-error regression coverage.

## 0.1.1 — 2026-10-26 (unreleased development repair)

- Fix Linux Runner/daemon spawning after a rebuild unlinks the executable, using the mapped image; reproduce the prior ENOENT with a real-process regression.
- Detect affected legacy daemons before submission and expose explicit idle- restart guidance, without automatic restart, workload replay or state switching.
- Install the tested Codex manifest/MCP layout with a stable absolute local native command. Verify discovery through the actual isolated Codex CLI loader.
- Retain a custom installed state profile on updates without explicit overrides; preserve legacy ownership receipts and modified/foreign-file refusal.
- Require honest CLI fallback reporting and preserve failed jobs instead of silently switching state or replaying work.
- Revise distribution to host-specific Agent plugins around one Rust/MCP core: Claude Code/OpenCode planned; independent OS installers/repositories cancelled. Official plugin platforms are future work; no publication during development.

## 0.1.0 — 2026-10-26 (unreleased development milestone)

- Independent Linux runner and process groups with durable launch claims, conservative recovery, capped logs, cancellation and timeout.
- SQLite jobs/runs/events, session leases, transactional delivery outbox, fenced attempts, retry/TTL/dead letters and exact target rebinding.
- Official rmcp stdio tools, Codex configuration and notification inbox.
- Opt-in experimental Desktop Unix socket transport with explicit identity/capability checks and fake-host coverage.
- One-flag `start --wake` / MCP wake_me=true: current-context registration, lease refresh and preflight; on-demand detached daemon and safe `daemon stop`.
- Dry-run setup, backed-up owned configuration edits, safe integration removal, CLI diagnostics and bilingual onboarding.
- Stable local native/Plugin install and uninstall with owned-file SHA-256 checks, retained state profiles/backups, relocatable launcher and local marketplace setup.
- Installed shared-host MCP uses current-terminal bindings, rejecting stale process identity; workflow skill hides tool/state/session paths from everyday prompts.
- Separate configuration, daemon, bounded stdio self-check and real tool-connectivity diagnostics. Installed/plugin UI live acceptance remains pending.
- Linux behavior tests and compile/test CI definition; future packaging roadmap.

No packages published or Windows/macOS runtime support claimed. Support is limited to the documented verification scope; no automated confirmed-resume adapter exists. See docs/IMPLEMENTATION_STATUS.md for validation evidence.
