# Changelog

## [Unreleased]

### Changed

- Reuse bounded SQLite connections, wake workers on local writes, and back off idle reconciliation/delivery checks to one second without acquiring a writer lock for an empty notification queue.
- Dispatch up to four chat notifications concurrently while transactionally keeping each logical destination's attempts serial across registration instances.
- Separate maintainer-only Prepare Release and Publish Release workflows. Default formal candidates to latest, validate OIDC from the actual publication workflow in dry-run, and reuse matching partial publications without rebuilding.
- Document code conventions, required checks and manual release ownership in CONTRIBUTING.md, with detailed subsequent-version and first-package instructions in the release guide.

### Fixed

- Confirm npm channel updates are publicly visible at the reviewed version before completing GitHub publication.

## [0.1.8] - 2026-10-27

### Added

- Automatically bootstrap the same Windows daemon profile through local CIM when a cleanup Job Object blocks native startup, with no login task/service registration or routine manual preparation. Keep native runner containment unchanged.
- Authenticate both process instances/images/SIDs/integrity levels and private IPC before transferring the exact Unicode environment/cwd in memory. Reject restricted/AppContainer/low-integrity tokens, preserve caller isolation and expose direct-only policy plus copyable fallback diagnostics.
- Cover host job destruction, cold start, crash restart without replay, environment preservation and refusal boundaries with native behavioral tests; package the matching runtime and plugin pins.

### Distribution

- Unify manual GitHub Release and the two npm main packages under one Actions workflow. Infer the version from Cargo.toml, enforce matching package/plugin pins, and extract this version's CHANGELOG section as Release Notes.
- Build and check Linux/Windows candidates in dry-run, then promote those exact reviewed artifacts without rebuilding. First-time npm staging uses a stage-only token and human 2FA approval; later direct publication uses OIDC Trusted Publishing.
- Preserve matching uploads across retries, verify staged/public tarball bytes, refuse changed or foreign assets/tags, and keep GitHub drafts private until public npm installation passes.

- Preserve bounded Windows installed-package failure diagnostics and run the installed native/host checks in ordinary CI as well as manual release dry-runs.

### Fixed

- Validate complete Windows Job Object detachment on a newly created suspended daemon/runner before resuming it. Nested-host partial breakaway now triggers the existing daemon-only CIM path; direct-only mode refuses before state/workload execution. Preserve underlying runner-launch errors.

- Wait asynchronously for a busy Windows host pipe, then authenticate the actual connected instance before Desktop/plugin exchanges. Avoid a separate fingerprint connection that could fail before the bounded wait; Unix socket checks remain unchanged.

### Known limitations

- Linux and Windows x86_64 are supported; macOS and ARM native runtimes are unavailable. OpenCode V1 targets 1.19.4 with actual-loader tests using a simulated model; Claude Code remains an inbox-only scaffold.
- ChatGPT Desktop same-chat continuation is verified on Linux and Windows, using an experimental private interface. Windows acceptance uses Full access. Results completed while the host is closed and real multi-day renewal remain separate scenarios.

## 0.1.7 — 2026-10-27 (unreleased Windows runtime and plugin candidate)

- Add private SID/DACL named pipes, peer process-instance checks, non-inheriting native daemon/runner launch and runner-owned Job Objects for Windows x86_64; reuse durable state, delivery/recovery, bounded logs, cancellation/timeouts and offline maintenance.
- Add explicit `daemon start [--install-root]` without a job/session side effect. Windows Codex 0.164.0 MCP forbids breakaway, so it needs same-profile startup from an outside terminal; no runtime service/WMI escape is used.
- Bundle Linux and Windows in the shared npm runtime with per-platform integrity metadata, static Windows CRT, `.cmd` terminal support and portable pinned MCP startup. Secure OpenCode's Windows Node/Bun pipe before registering callers.
- Verify actual Windows npm installation, Codex catalog/MCP discovery and OpenCode matching-session continuation with a simulated model. Separately verify an installed Codex MCP task through Codex-Manager using `gpt-6.1-sol`/`low` and a prestarted daemon under guest-agent SYSTEM; ordinary-user/Windows ChatGPT Desktop continuation remains open.
- Extend native Windows CI and manual release preparation to assemble and check both platforms from one revision. Remote workflow acceptance and publication remain pending.

## 0.1.6 — 2026-10-26 (unreleased plugin delivery candidate)

- Retain the daemon SIGINT receiver while reconciliation awaits SQLite, preventing lost graceful-stop signals; reproduce the blocked-reconciliation window with an owned-process/real-SQLite regression.
- Prepare shared native-runtime/OpenCode npm tarballs and a self-hosted ChatGPT marketplace with matching version pins, external persistent profiles, native integrity/glibc checks and no install scripts. Package names/account verification and publication remain pending.
- Check installed tarballs with actual OpenCode and Codex loaders using simulated-model/loopback-registry fixtures; verify history after package relocation and corrupt-native refusal.
- Add manual-only GitHub candidate artifacts/optional prerelease draft workflow with Node.js 24 action versions; no push/tag release or npm publishing. Fix Windows unused executable metadata and separate installation-test deadlines from runtime deadlines.

- Add owner-checked, bounded plugin bridge transport with exact session/instance/delivery echoes; reuse fenced renewal and adapter-scoped recovery. Explicit registrations start an absent daemon without starting a job.
- Add OpenCode V1 caller-context binding to the existing MCP core and public session continuation. Target 1.19.4; actual loader/MCP/agent-loop checks use a local simulated model, with real-provider/restart/V2 evidence still open.
- Add Claude Code manifest/MCP/skill/inbox scaffold and synthetic helper/MCP-entry checks. No Claude account or actual host test is available; community testing and PRs are welcome. No Claude wake support is claimed.
- Document V1/V2 differences, community acceptance and local account-free checks; add Linux host-check CI definitions without publication or credentials.

## 0.1.5 — 2026-10-26 (unreleased interrupted replacement recovery)

- Durably stage installation/cache replacements and publish a sibling operation journal before renaming directories; block overlapping lifecycle changes until repair.
- Add `plugin repair --path` preview/apply to finish the recorded replacement, validate original/staged/backup files and profiles, retain backups/history and respect active-runtime locks. No automatic daemon stop, schema rollback or global cache scan.
- Defer Claude Code/OpenCode adapters until a later environment, potentially using Docker. ChatGPT Desktop UI/restart verification requires separate observed evidence.

## 0.1.4 — 2026-10-26 (unreleased plugin runtime lifecycle)

- Export a relocatable offline native plugin bundle; bootstrap validates the bundle and installation/marketplace ownership before writing, with no recipient Rust/Python/Node toolchain.
- Update from checked local bundles while retaining the recorded state and refusing downgrades, active runtimes and modified/foreign files; generate plugin version metadata from the actual package version.
- Route cached terminal and MCP entry points to the same managed native runtime; add explicit cache status and backed-up snapshot refresh, including update preflight across selected caches. Never scan/delete caches or change host enablement.
- Qualify offline bootstrap/cache handling with the actual isolated Codex loader and 20 MCP tools, without touching personal settings or a real ChatGPT Desktop endpoint.
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
- Opt-in experimental ChatGPT Desktop Unix socket transport with explicit identity/capability checks and fake-host coverage.
- One-flag `start --wake` / MCP wake_me=true: current-context registration, lease refresh and preflight; on-demand detached daemon and safe `daemon stop`.
- Dry-run setup, backed-up owned configuration edits, safe integration removal, CLI diagnostics and bilingual onboarding.
- Stable local native/Plugin install and uninstall with owned-file SHA-256 checks, retained state profiles/backups, relocatable launcher and local marketplace setup.
- Installed shared-host MCP uses current-terminal bindings, rejecting stale process identity; workflow skill hides tool/state/session paths from everyday prompts.
- Separate configuration, daemon, bounded stdio self-check and real tool-connectivity diagnostics. Installed/plugin UI live acceptance remains pending.
- Linux behavior tests and compile/test CI definition; future packaging roadmap.
