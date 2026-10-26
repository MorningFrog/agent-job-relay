# Implementation status

## Current milestone

Last verified: **2026-10-26 (Asia/Shanghai)**. Source version **0.1.6**, unreleased; Linux x86_64, kernel 7.0.0-38-generic, Rust/Cargo 1.99.1, edition 2024. Cargo.lock pins rmcp 3.7.0, Tokio 1.54.1 and rusqlite 0.41.0. The declared Rust 1.89 MSRV has not been independently tested.

One Rust package/executable provides CLI, stdio MCP, daemon and runner modes. SQLite schema is **v6**, local IPC is **v1**. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. Historical increments/results remain in Git; concise current live verification is in [live-e2e.md](live-e2e.md).

Stages 0–2 have locally verified design, Linux execution, MCP/delivery and setup implementations. Stage 3 has the version-specific host evidence described below; Stage 4 has passed local quality/documentation gates. Native Windows/macOS runtime and remaining live-host acceptance are still open. The prior remote CI failed on Windows unused-field compilation and Linux installation-test timeouts; fixes are included, but updated workflows have not run remotely.

## Implemented and tested

| Area | Current behavior |
|---|---|
| Execution | Explicit argv or opt-in shell, validated cwd/env/timeout, durable idempotency, separate Job/Run identities and attempt=1; deliberate reruns need a new job/key |
| Lifecycle | Detached runner, per-run lock/CAS startup, independent workload groups, conservative orphan classification and no automatic execution replay |
| Storage/output | SQLite WAL/FULL/foreign keys, bounded blocking workers, short transactions, v1/v4→v6 migrations, capped raw stdout/stderr, byte cursors and lossy UTF-8 reads |
| Cancellation/security | Graceful then bounded forced process-group stop, subreaper, private Unix socket/directories, peer checks, bounded requests/pages and symlink refusal |
| Delivery | Atomic terminal state/Event/Outbox, stable delivery IDs, fenced 10-second claims, three-second attempt deadline, temporary retries until 24-hour event TTL, backoff/jitter capped at five minutes, explicit retry and dead letters |
| Recovery/renewal | Same-chat preview/apply with immutable target transfer, original IDs/history/TTL, atomic audit, live-claim exclusion and stale-ack fencing; opted-in delivery-time renewal verifies the same endpoint and claim without periodic host polling |
| MCP/session binding | 20 tools; explicit host caller binding, never shared-process identity. Desktop uses current terminal; OpenCode V1 injects actual tool context; Claude scaffold uses explicit skill session substitution and inbox-only capability |
| Installation/diagnostics | Stable local native/plugin bundle, pinned separate state profile, owned configuration/marketplace preview/apply/removal, modified/foreign-content refusal, data preservation and separate configuration/stdio/host-connectivity evidence |
| Release candidates | Shared Linux x86_64/glibc runtime and OpenCode npm tarballs, external stable profiles, integrity/glibc checks, ChatGPT self-hosted catalog, manual-only artifacts/optional prerelease draft workflow; npm account/names/publication pending |
| Plugin lifecycle | Offline export/bootstrap/update, profile preservation and downgrade refusal; explicit checked cache refresh; durable directory-replacement journals and `plugin repair --path` preview/apply, retained backups/history, altered/missing-file refusal and active-runtime fencing |
| Public host bridge | Owned endpoint/caller/instance and exact delivery metadata checks, bounded frames/deadlines, existing renewal/recovery fences with adapter isolation; OpenCode V1 public session API |
| Upgrade | Linux `/proc/self/exe` launch survives pathname replacement; old daemon capabilities fail preflight; explicit profile-aware legacy stop revalidates the original peer and uses pidfd/SIGINT, never PID alone or a process-group kill |

Submissions and explicit session registrations start the per-state daemon on demand. Doctor/status do not start it; `daemon stop` leaves runners and persisted results independent. Config and ownership-receipt writes are individually atomic, not a cross-file transaction.

## Verification and evidence boundaries

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --all-targets` | **81 passed, 0 failed, 0 ignored** |
| Test groups | 8 library, 4 public-plugin bridge, 13 convenience/installed MCP, 10 delivery/migration, 7 Desktop simulation/renewal, 4 integration, 5 onboarding, 10 offline plugin/cache, 3 legacy-upgrade, 4 scoped-recovery, 13 Linux runtime |
| `cargo build --release --locked` | Passed; `target/release/agent-job-relay`, version 0.1.6 |
| Plugin loader | Source 0.1.6: actual Codex CLI 0.164.0 in isolated HOME/CODEX_HOME bootstraps a relocated offline bundle, checks the host-created cache and discovers 20 tools, including `job_recover_session`; no model turn, daemon, workload or real Desktop endpoint |
| Live installed MCP | 0.1.5 active-job restart/rebind/continuation passed: original Job/Run, exit 0, one accepted delivery to the recovered target and observed same-chat execution; Desktop 26.1021.22027/UI/restart user-attested, endpoint change measured |
| OpenCode V1 | Actual Bun name/version install of candidate tarballs from a loopback registry, bundled native runtime, 1.19.4 loader, caller-context MCP submission, owned-session public callback and subsequent agent-loop response passed with a local simulated model; control session untouched, no real provider/account |
| Self-hosted marketplace | Actual Codex 0.164.0 catalog installation and 20 MCP tools with a loopback npm registry; cached launcher uses the same pinned version; no public npm write, Desktop contact or model turn |
| Packaged runtime | Actual npm tarball install, native execution, retained job history after package relocation and pre-execution corrupt-native refusal passed; local build requires glibc 2.39, release workflow targets Ubuntu 22.04 |
| Claude scaffold | Synthetic caller separation/literal-placeholder rejection and actual launcher/MCP discovery/submission passed; no Claude account, loader, skill substitution or model run verified |
| Windows VM compile | Windows 11 x64, winget machine-installed Rust MSVC 1.99.1: fmt, all-target Clippy, 2 portable library tests, release build and CLI help passed; native daemon request correctly refused as Linux-only |
| Test policy | README wording/structure assertions removed; behavioral suites and required fixtures preserved. Generated reports/logs stay ignored; documentation keeps concise current results |

Automated Desktop transport, host replacement, scoped recovery and lease-renewal suites use fake hosts. A separate real Desktop active-job restart/rebind/continuation check passed as described below. The long-job test injects expiry 48 hours into the past; real multi-day renewal remains unqualified. Tests keep state/endpoints under ignored repository-local `.tmp/` and do not touch Vanth state.

The installed 0.1.5 MCP probe survived the user-reported Desktop 26.1021.22027 restart. Saved bindings establish a changed bridge instance/endpoint/session key with the same logical chat/caller/host. MCP recovery preview/apply created the matching immutable target and audit. The original Job/Run completed once with exit 0 after release; one completed event and one accepted delivery/attempt matched the new target, then the notification triggered observed same-chat execution without another human message. See [live verification](live-e2e.md).

`notification_recorded`, `accepted_by_host` and observed Agent execution remain distinct. No adapter automatically confirms resume, and receiver deduplication is unproven. UI enablement/version/restart remain user-attested. Clean installation, broader compatibility, events completed while the host is closed and real multi-day renewal are separate open checks; do not repeat this passed active-job scenario merely to refill evidence.

Installation recovery coverage reconstructs persisted states before/between/after directory renames for fresh installs, upgrades and caches. An actual CLI repair restores a missing installation, preserves SQLite bytes and reloads stdio MCP. Modified/foreign/missing files, symlinks, locks, legacy receipts and repeated repairs are covered; physical power-loss/ENOSPC remains unqualified. See [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Installed runtime and provenance

Current read-only checks confirm the installed native **0.1.5**, matching recorded/active state, explicit current-terminal binding mode and actual MCP connectivity. The user reports Desktop **26.1021.22027** with plugin **0.1.5** enabled in its UI. These are separate evidence levels; they do not freshly audit native/cache hashes or history preservation across the user's upgrade. The active-job restart check used the same installation/state and retained the original Job/Run; no developer-driven configuration update or daemon stop was performed.

Source development does not update installed/running programs or modify real configuration/services. Earlier installation audits and backup decisions remain in Git history; they are not a fresh audit of the current installation.

## Limitations and next work

- Linux is the only verified background runtime. A Windows 11 VM passed compilation/portable tests and explicit unsupported-runtime refusal; Windows pipes/SID/DACL/Job Objects and macOS native identity/lifecycle remain unimplemented; updated remote CI remains unverified; the CI definition alone is not platform evidence.
- Runner death or descendants escaping process groups can leave unknown external effects. Power-loss and real ENOSPC VM tests, stronger containment, retention/backup/export and crash-atomic multi-file installation remain pending. See [limitations](limitations.md) and [security](security.md).
- Desktop uses an experimental private interface and trusted caller identity, not host attestation. Codex CLI inbox is notification-only. Cached MCP needs the original stable installation path.
- Offline bootstrap/update and explicit cache refresh are local development features, not a marketplace ZIP, authenticated download or automatic host bootstrap. Hashes do not establish publisher provenance or cross-distribution libc compatibility. Legacy caches without the new profile require host refresh. Per-directory journal repair finishes recorded replacements forward; multi-directory/configuration writes, uninstall and pre-journal/legacy remnants still require separate handling. See [ADR 0008](adr/0008-interrupted-plugin-replacement.md).
- **Task output monitoring and interruption is deferred**, as requested on 2026-10-26. Existing log reads/follow and manual stop do not provide it; design and implementation are tracked in [TODO.md](TODO.md).
- Desktop 26.1021.22027/plugin 0.1.5 active-job restart, changed-endpoint recovery and same-chat continuation passed. Native UI actions/version are user-attested; saved identities, recovery audit and completed-event/delivery correlation are measured, and the Agent continuation was observed. Clean UI installation, results completed while the host is closed and real multi-day renewal remain separate. See [live evidence](live-e2e.md).
- Follow [TODO.md](TODO.md) for host-driven bootstrap/artifact provenance, community Claude account tests, OpenCode real-provider/restart/V2 checks (Docker/version-matrix PRs welcome), data maintenance and native OS support. Independent OS packages are cancelled. Shared npm/self-hosted marketplace and manual release preparation are authorized; publication/push await final confirmation. Official directory submission and history rewrite are not authorized.

## Host-plugin handoff

Source packages, official API research, reproducible checks and community acceptance are in [host plugins](host-plugins.md), [ADR 0009](adr/0009-public-host-plugin-bridge.md) and [CONTRIBUTING.md](../CONTRIBUTING.md). OpenCode uses actual V1 1.19.4 interfaces with a simulated model; Claude is inbox-only with no maintainer account and needs community tests/PRs. V2 has a researched migration map, not a runnable entry. Linux CI now packages candidates and checks the actual installed OpenCode entry; manual release also checks the Codex catalog. The old remote run failed before host checks; updated workflows need a remote run. The existing Codex offline bundle does not include these source packages; personal installed/running 0.1.5 is unchanged. SQLite v6/IPC v1 remain, while older binaries cannot interpret plugin_bridge records.

## Release handoff

The GitHub repository is public. `scripts/prepare_release.py` produces allowlisted npm tarballs, a self-hosted marketplace ZIP and checksums; `scripts/check_release.py` exercises installed packages, the actual Bun package-name loader and Codex catalog using a loopback registry. All generated files/reports remain ignored. `release.yml` has only a manual trigger, defaults to artifacts and can optionally create a prerelease draft after gates. No npm publishing workflow, credentials, remote push, official listing or package publication was performed. The user's npm account is awaiting verification; current unscoped names are provisional. See [release preparation](release.md) and [ADR 0010](adr/0010-plugin-release-candidates.md).
