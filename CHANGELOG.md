# Changelog

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
- User confirmed post-install Desktop MCP connectivity following the host job_doctor procedure; no new MCP background-task/wake evidence is implied.

## 0.1.1 — 2026-10-26 (unreleased development repair)

- Fix Linux Runner/daemon spawning after a rebuild unlinks the executable, using the mapped image; reproduce the prior ENOENT with a real-process regression.
- Detect affected legacy daemons before submission and expose explicit idle- restart guidance, without automatic restart, workload replay or state switching.
- Install the tested Codex manifest/MCP layout with a stable absolute local native command. An isolated actual Codex CLI 0.164.0 loader discovers all 19 tools.
- Retain a custom installed state profile on updates without explicit overrides; preserve legacy ownership receipts and modified/foreign-file refusal.
- Require honest CLI fallback reporting. Record the user's real 45-test release task as installed skill/CLI wake evidence, including its first failure/workaround.
- Pass 47 Rust tests, strict Clippy, formatting and locked release build.
- Revise distribution to host-specific Agent plugins around one Rust/MCP core: Claude Code/OpenCode planned; independent OS installers/repositories cancelled. Official plugin platforms are future work; no publication during development.

## 0.1.0 — 2026-10-26 (unreleased development milestone)

- Independent Linux runner and process groups with durable launch claims, conservative recovery, capped logs, cancellation and timeout.
- SQLite jobs/runs/events, session leases, transactional delivery outbox, fenced attempts, retry/TTL/dead letters and exact target rebinding.
- Official rmcp stdio tools, Codex configuration and notification inbox.
- Opt-in experimental Desktop Unix socket transport with automated fake-host tests; user-confirmed basic live E2E on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04, with local artifacts cross-checked (docs/live-e2e.md).
- One-flag `start --wake` / MCP wake_me=true: current-context registration, lease refresh and preflight; on-demand detached daemon and safe `daemon stop`.
- User-confirmed CLI shortcut live first start, session reuse and idle-daemon stop/auto-start on the same Desktop version/OS, with no manual continuation or wrong control-chat notification; local proofs and binary hash cross-checked.
- Dry-run setup, backed-up owned configuration edits, safe integration removal, CLI diagnostics and bilingual onboarding.
- Stable local native/Plugin install and uninstall with owned-file SHA-256 checks, retained state profiles/backups, relocatable launcher and local marketplace setup.
- Installed shared-host MCP uses current-terminal bindings, rejecting stale process identity; workflow skill hides tool/state/session paths from everyday prompts.
- Separate configuration, daemon, bounded stdio self-check and real tool-connectivity diagnostics. Installed/plugin UI live acceptance remains pending.
- Linux behavior tests and compile/test CI definition; future packaging roadmap.

No packages published or Windows/macOS runtime support claimed. Basic live wake validation is limited to the recorded client/OS; no automated confirmed-resume adapter exists. See docs/IMPLEMENTATION_STATUS.md for validation evidence.
