# Automated testing and evidence

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

For reader-facing installation and ChatGPT Desktop acceptance steps, use [manual testing](manual-testing.md). This guide covers developer verification. Tests keep state, fake endpoints and logs under repository-local `.tmp/`; they do not modify personal settings, services, unrelated application state or real ChatGPT Desktop endpoints.

## Required checks

Run from the repository root:

```bash
npm ci --prefix web --ignore-scripts --no-audit --no-fund
npm run build --prefix web
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

Current counts, toolchains and results are maintained in [implementation status](IMPLEMENTATION_STATUS.md). CI defines all-target Linux/Windows native tests plus macOS compilation/portable library checks. Current Linux/Windows native jobs and macOS portable checks pass remotely; see [implementation status](IMPLEMENTATION_STATUS.md). A configured matrix alone does not establish native OS support. The declared Rust 1.89 minimum is not independently verified.

## Test selection and artifacts

Test behavior, correctness and regression risk through the existing Rust suites and fake hosts. Preserve the legacy daemon fixture and other required reference data. Do not add tests for README wording/structure or other implementation details that have no product-interface contract. Review documentation changes directly as part of normal maintenance.

Keep generated test state, outputs, reports and logs under ignored repository-local `.tmp/` or `target/`; never commit them. Versioned documentation records concise current results and verification limits. Historical runs are available in Git history, while required test cases/fixtures stay tracked.

The four configuration-integration fixtures run serially within their test binary. This prevents a parallel CLI fork from briefly inheriting another fixture's configuration lock before exec; each case keeps its own isolated paths and all assertions. Runtime concurrency cases continue to exercise their existing explicit concurrent workloads.

Plugin update/repair preservation checks checkpoint committed WAL pages before comparing complete database hashes. Terminal state and daemon-lock release can precede the last pooled connection closing; checkpoint timing alone must not look like data loss. The interrupted-repair fixture seeds committed WAL history before taking its snapshot and verifies that record remains readable after repair, alongside the existing backup/journal and database-preservation assertions.

## Behavior coverage

| Suite | Coverage | Boundary |
|---|---|---|
| Library | MCP schemas, legacy launch-readiness and interrupted replacement recovery/refusal/locks | Local checks plus deterministic filesystem fault states |
| Monitoring and Web | Literal chunks after prefix cap, daemon-independent rule/tree stop, silence dedupe/rearm, revision/terminal fences, offline reads, sequence migration, HTTP auth/origin/operation restrictions and installed launcher UI | Linux/Windows native behavior plus portable storage checks; real browser interaction verified separately |
| Runtime | Execution/output/cursors, idempotency, process trees, cancellation/timeout, daemon/runner independence, launch recovery, quotas, IPC/security; batch overlap/options, partial failures, quota races and lost-response/restart retry | Real Linux children and injected failure windows |
| Data maintenance | Automatic daemon archival/backup, age/count/log budgets, hot/cold history/cursors/idempotency, binary logs, protected recovery/claims, explicit retry, migration refusal, archive rollback/cleanup, WAL backup/restore without replay, sealed/unsealed recovery and ownership/rotation | Linux/Windows fixtures; no physical power-loss or ENOSPC qualification |
| Delivery | Terminal/outbox transactions, migrations, exact routing, fenced attempts, retries/TTL/dead letters, read-only idle claims, pooled transaction/profile isolation and per-chat claim exclusion | Real SQLite and persistence simulations |
| Goal waiting | Native host pause confirmation/retries/revisions, storage-only refusal, missing acknowledgement expiry and legacy notification fences, partial/frozen admission, exact-run/merged completion, deletion/replacement/late deletion, cancellation/recovery, CLI/shared-MCP binding and cold/backup preservation | Linux native Relay plus fake goal/Desktop hosts; actual Codex check below uses a simulated submitting Agent/host/model |
| Public plugin bridge | Exact caller/session/delivery echoes, frame limits, endpoint replacement, scoped recovery and same-endpoint renewal | Real SQLite plus fake owned socket host |
| ChatGPT Desktop | Framing, catalog/identity checks, endpoint replacement, fenced lease renewal and four-chat dispatch with same-chat exclusion | Fake hosts; no real restart/multi-day proof |
| Convenience | CLI/MCP binding, multi-caller routing, installed shared MCP, loaded-image spawn, recovery and long-job renewal | Real CLI/MCP processes plus fake hosts; includes terminal workspace bindings, minimal start arguments, conflicting-key refusal, compact/full output and unified connect recovery |
| Integration | Preview/apply, comments/foreign-entry preservation, ownership refusal and removal | Isolated files |
| Onboarding | Installed launcher/profile, manifests, marketplace setup, automatic idle-daemon update, active-job refusal and data-preserving removal | Isolated installed processes/files |
| Plugin | Relocated/toolchain-free bootstrap, package validation, preserved DB/marketplace, custom-root defaults, profile/downgrade rules, cache refresh/refusal, actual CLI repair and retained SQLite history | Linux behavioral tests; includes unified install/update profile/history preservation and local payload/resource-change fixtures |
| Legacy upgrade | Verified same-profile stop, peer identity checks, lock release and immediate replacement | Std-only legacy fixture; no unrelated peer signals |
| Recovery | Scoped preview/apply, pagination, rollback/audit, preserved delivery IDs/history/TTL, accepted/expired exclusion and stale-claim fencing | Isolated storage/endpoint fixtures |

Focused commands are `cargo test --test <suite>`, using the lower-case suite filenames under `tests/`. Waits have deadlines; harnesses stop/reap only their own isolated processes. Output-open faults and injected expiry/boot/lock conditions are not physical ENOSPC, power-loss or multi-day tests.

For unreleased batch changes, run `cargo test --test runtime batch` and `cargo test --test convenience batch`. Expected: three runtime and two CLI/MCP regressions pass, including independent monitor stopping, invalid-session rollback, stable IDs after a discarded response/restart and exact fake-Desktop caller destinations. With the isolated OpenCode toolchain described in [host plugins](host-plugins.md#account-free-actual-loader-check), run `python3 scripts/check_opencode_plugin.py --batch`. Expected: the actual loader exposes job_start_batch, two independent items complete and notify the actual caller while one invalid item is rejected, forged model-supplied caller fields are overridden, the control session stays empty and the simulated model runs after host acceptance. This adds no real-provider, Windows or real ChatGPT Desktop batch evidence.

The daemon shutdown regression holds a real SQLite writer while an owned queued runner forces reconciliation to await it, then sends SIGINT before releasing the writer. It keeps the original short exit deadline, verifies the existing workload survives, and checks notification dispatch after restart. Control requests must remain responsive during that writer wait, and SIGINT still stops the daemon without stopping independent runners. Another regression occupies all four general database workers and requires status to use reserved read capacity. Development/test profiles retain line tables to keep installed fixtures below the existing 128-MiB image budget; CI uses no debug symbols. All behavior cases remain enabled.

Run `cargo test --test daemon_upgrade --test legacy_upgrade --test plugin` for automatic update behavior. A real native fixture with changed package-version bytes exercises concurrent startup/UI handover, preserved running Job/Run/logs and launch count, newer-peer reuse and schema-migration refusal without stopping old work. Installed update checks cover preview/no-op, active-job refusal, the new daemon's loaded path and retained history/cache snapshots; legacy checks exercise actual pidfd/SIGINT handover. The IPC library regression proves a changed process identity receives no shutdown bytes. These checks use isolated Linux profiles; they do not qualify all historical releases, Windows handover or real host continuation.

Run `cargo test --lib progress::` and `cargo test --test monitor` for source progress. Expected: bounded split-stream framing and invalid/flood handling; sequence/cooldown/revision/terminal fences; transactional rollback and archive/backup preservation; actual CLI threshold stopping/shared HTTP snapshots; and the reference checker delivering mid-run to its exact fake Desktop chat while remaining independent of daemon lifetime. Fake-host acknowledgement is not observed real Agent execution; no new Windows/real-provider acceptance is claimed.

## Manual feature prompts

Copy the short task requests in [English](test-prompts.md) or [中文](test-prompts.zh-CN.md) for directory scans, file searches, builds, progress updates and task management.

## Actual Codex plugin-loader check

After the release build, with Codex CLI and Python available:

```bash
python3 scripts/check_codex_plugin.py
```

This exports and relocates an offline bundle, bootstraps in disposable repository-local HOME/CODEX_HOME, removes the delivery artifact, installs through the actual loader, checks its returned cache path/profile and no-op refresh, and requires MCP discovery including job_recover_session, job_start_batch and job_goal_wait (24 tools in current source). It asserts that no daemon/workload state was created. Reports/logs remain in the printed ignored `.tmp/loader-...` directory. Python is a developer-check dependency only.

The recorded loader is Codex CLI 0.164.0. Portable root and PLUGIN_ROOT command/args/cwd behavior differed from documentation; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). The loader check starts no model turn and establishes neither ChatGPT Desktop browser discovery nor real MCP task/wake behavior.

## Evidence boundaries and live acceptance

| Evidence | What it establishes |
|---|---|
| Configuration preview/match | Owned files are configured as intended |
| Local stdio self-check | Executable can initialize and expose tools |
| Actual loader discovery | The tested loader finds the plugin's MCP declarations/tools |
| Host `job_doctor` | A real host tool call reached MCP |
| `accepted_by_host` | Host acknowledged that delivery |
| Observed same-chat continuation | Agent actually resumed for the matching result |

AgentJobRelay v0.1.8 is verified with ChatGPT Desktop on Linux and Windows, including same-chat continuation after task completion; Windows verification uses Full access. See [live verification](live-e2e.md). Follow [manual acceptance](manual-testing.md) for a new installation or relevant change, without repeating completed acceptance just to collect more evidence. Results completed while ChatGPT Desktop is closed and real multi-day renewal remain separate scenarios.

Resource-only updates and downgrade fixtures deliberately modify package metadata/hashes to test management behavior. They do not authenticate release provenance. No automatic adapter records confirmed_resumed; host acceptance and observed execution stay separate. Deferred native-platform, containment, disk/power-loss and artifact-provenance work is tracked in [TODO](TODO.md).

The developer-only `scripts/check_desktop_restart.py` provides a bounded FIFO handshake for the explicitly requested real restart/rebind acceptance in [manual testing](manual-testing.md#chatgpt-desktop-restart-and-same-chat-rebinding). The isolated wait/release check verifies only the script. The separate coordinated real-host pass is recorded in [live evidence](live-e2e.md), with UI attestations distinguished from measured endpoint/recovery state and observed continuation.

## Account-free host-package checks

[Host plugins](host-plugins.md) provides dependency/setup commands for `scripts/check_opencode_plugin.py` and `scripts/check_claude_scaffold.py`. OpenCode 1.19.4 uses the actual loader/MCP/session API and agent loop with a local simulated model, checking a completed response and an untouched second session. Claude uses synthetic callers and the actual MCP launcher only; its caller-isolation check requests `full=true` to inspect the stored session and inbox-only capability. Its loader/skill substitution/account/model remain untested. Docker recipes/version matrices and account-backed Claude tests are welcome through [community contributions](../CONTRIBUTING.md).

The Python MCP probes allow at most 1 MiB per JSON line so the full tool catalog fits (24 tools in current source, 22 in sealed 0.1.11); asyncio's default 64-KiB reader limit is too small for the expanded schemas. The Claude scaffold check retains its request deadlines, required-tool checks, explicit caller-isolation assertions and inbox-only boundary.

## Installed release-candidate checks

Follow [release preparation](release.md) for `prepare_release.py` and `check_release.py`. Unlike a source-linked import check, they install the real npm tarballs, use their packaged native executable, preserve job history across runtime relocation and refuse a corrupt payload. For 0.1.11 and newer, the checks also install into an isolated npm global prefix, invoke the generated `ajr` command/shim, verify default/explicit profile selection and environment overrides, submit a real command and inspect its same-profile history through the authenticated Web UI. The Windows checks exercise npm's `.cmd` shim; running those checks on Windows remains necessary to qualify that platform. OpenCode uses its actual loader/MCP/agent loop with a simulated model. Codex adds the extracted self-hosted catalog through its real CLI and discovers the tools through a loopback fixture registry; it does not claim public npm availability or ChatGPT Desktop UI installation. Prepare Release repeats both host checks; Publish Release dry-run checks OIDC and previews the sealed candidate. Push/PR CI performs the Linux installed OpenCode check and assembles the same-revision Linux/Windows images for the actual installed Windows package/loader check. Windows failures include bounded job/status/output diagnostics and a seven-day fixture-only diagnostic artifact; native-only CI is not a substitute for the installed-package test. Generated artifacts/reports remain ignored.

Release-state regression checks run with `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_release.py -v`; CI, Prepare Release and Publish Release run the same checks. They cover version/changelog errors, credential-free preparation, the exact OIDC identity, no-poll publication, acceptance receipt/source/hash validation, interrupted receipt/public-metadata recovery, independent read-only propagation checking, bounded timeouts, fresh-state/credential-free ETARGET installation retries, immediate permanent/native/import/integrity failure, exact child dispatch/reuse after a lost response, child metadata initialization without redispatch or weakened identity checks, failed-child continuation, main/source fences and foreign-content refusal. Legacy stage validation/approval behavior, immutable inventories and draft pagination/exact-ID uploads remain covered. Service fixtures, a simulated clock and a loopback HTTP server exercise these cases without real writes or accounts. Validate CI and all four release workflows with actionlint. Published 0.1.10 passed the complete manual Prepare/Publish/Check pipeline, including the read-only public installation check. Source 0.1.11 passes release-revision native/package CI, manual Prepare Release and the actual OIDC publisher dry-run; the downloaded sealed candidate also passes checksum/inventory/source review. Publication and public checking remain separate; see [release handoff](release.md#current-release-handoff) and [implementation status](IMPLEMENTATION_STATUS.md).

## Windows native verification

Windows behavior lives in `tests/windows.rs`, with portable library, delivery and integration checks alongside it. Coverage includes command arguments/exit codes, PowerShell/cmd, capped raw logs, split-output rules, detection/tails after the prefix cap, silence rearming/revision fences, daemon-independent monitoring/tree stop, bounded stop observation, default Web assets/auth/origin/control, tree cancellation/timeouts, runner-death containment, daemon stop/crash, MCP exit, cold-start capture-handle isolation, private DACL/junction refusal, bounded busy-pipe probing with connected-instance authentication, Node/Bun pipe owner/process fencing, malformed/duplicate pipes, inbox isolation, simulated ChatGPT Desktop routing/recovery/renewal and data-preserving offline maintenance. The installation fixture clears inherited `AJR_STATE_DIR` and host caller context so background execution cannot select the outer runtime's profile; an ambient-profile regression verifies this isolation. Current native results are in [implementation status](IMPLEMENTATION_STATUS.md).

Cargo's Windows cleanup Job Object forbids breakaway. The harness launches only its owned fixture processes through local CIM under the same SID outside that job, then controls the exact recorded daemon instance; runner spawning remains native CreateProcess with inheritance disabled. Daemon startup now supports a verified same-user local CIM helper when a cleanup job denies direct creation; native regressions exercise this actual product route, cold environment/identity isolation, host job destruction, daemon-crash restart without replay and policy/token/peer refusal. Tests cover nested inner-breakaway/outer-restrictive jobs, pre-execution detachment checks, direct-only refusal without a database/workload, and busy host pipes authenticated on the actual connection. Tests keep state under the disposable source snapshot's ignored `.tmp/`. These native checks are separate from verified ChatGPT Desktop live acceptance. See [Windows operation](windows.md) and [ADR 0011](adr/0011-windows-native-runtime.md).

Actual Windows Codex MCP/model acceptance has passed from a cold profile with no prestarted daemon, using `gpt-6.1-sol`/`low`; doctor reported `windows_cim`. The cold-profile MCP test passes under both SYSTEM and an ordinary interactive Windows user. Windows ChatGPT Desktop same-chat continuation is verified separately with GPT-6 Luna / Low and Full access. Account-free catalog discovery also passed with the original portable-plus-compatibility declarations. See [live evidence](live-e2e.md) for the version-specific result; credentials and raw transcripts stay out of reports/source.

Windows installed-package reproduction uses `scripts/check_windows_packages.mjs`; see [release preparation](release.md). It tests actual npm installation/native execution, relocation/history preservation, embedded JS/CSS loading, unauthenticated API refusal, authenticated same-profile inspection and corrupt-payload refusal. For 0.1.10 it also exercises installed CLI rule creation, alert inspection, bounded stopping and offline inspection. Codex uses the original portable-plus-compatibility catalog with only registry transport replaced by a loopback fixture; OpenCode uses the real Bun package-name loader, MCP caller hook and public session API with a simulated model. A matching-session response and empty control session are required. This does not establish public npm availability or ChatGPT Desktop acceptance.

## Data-maintenance verification

Run `cargo test --test data` for data-maintenance scenarios and `cargo test --test delivery --test runtime` for reserved-read and control-response regressions. Tests reconstruct archive rollback/post-commit cleanup and backup sealed/unsealed states, verify exact IDs/expiry/attempts and checksum/foreign-file refusal, and restore active intents as orphaned without execution. The daemon case proves maintenance runs without a manual call. These controlled state/fault tests pass on Linux and Windows; physical disk/power faults and prolonged load need separate qualification. See [data maintenance](data-maintenance.md).

## Local browser acceptance

Use only an isolated profile under .tmp/, run ui --no-open, then open its returned access URL. Expected: real task rows update, selecting a task shows bounded stdout/stderr, editing a rule first shows pending then applied/triggered, and the event view retains evidence. Stopping updates the task to its observed terminal outcome. Verify the same history through inspect --json, then stop only this test profile's daemon. This is browser/runtime acceptance, not ChatGPT Desktop or another host's same-chat continuation. React/Vite build output and browser screenshots stay ignored.

## External-task verification (unreleased source)

Run `cargo test --test remote` for native reference-helper and observer behavior. Linux covers lost acknowledgement/idempotency, lookup-only cancellation, target/run/attempt fences, raw byte cursors, transport/observer/remote-runner loss, read-only versus controlled stopping, source shared-MCP/CLI retry plus origin inbox alerts/results, observer storage failure and terminal/recovery races. Windows runs the four applicable native cases using the existing verified daemon bootstrap outside Cargo’s cleanup Job Object. Workflow status/auth/attempt changes use a bounded fake `gh`; tests assert GET-only calls and no workflow mutation.

For a real remote host, follow [remote task setup](remote-execution.md), choose isolated profiles, emit a unique milestone and record a launch counter. Expected: disconnecting the transport or losing the local observer preserves remote work; reconnect keeps the original run IDs and byte cursors, the counter remains one, read-only stop leaves work alive and a control stop observes the exact remote terminal result. Use only the created profile’s daemon/owned workload for cleanup. SSH authentication/shell deployments, real host continuation and prolonged/machine-loss tests need separate qualification. For Actions, select a real existing repository/run/attempt and compare retained conclusion/URL with a read-only API result; never dispatch or cancel a workflow for this check.

## Goal-waiting verification (experimental source)

Run `cargo test --test goal_wait` for the Linux behavioral fixtures and `cargo test --lib goal_control` for controller image-replacement detection. The 16 integration cases include ordinary automatic goal binding/wake submissions, stripped-binding refusal, inactive/invalid goal-state handling, native host pause confirmation/retries/revisions, storage-only refusal, missing acknowledgement expiry and legacy waiting/restored notification fences, source CLI/shared MCP, frozen partial admission, exact-run/merged completion, deletion/replacement/late deletion, uncertain restoration, cancellation (including during the final controller read), committed-admission and daemon-loss recovery, pinned recovery, archival and backup restore. All state is isolated; no actual Desktop goal is changed.

After a release build, on Linux with Python and a Node version providing built-in WebSocket:

```bash
python3 scripts/check_codex_goal.py --codex "<absolute-goal-capable-codex-executable>" --wait-seconds 5
```

Omit `--codex` to use the actual terminal's Desktop-selected runtime, falling back to Codex on PATH. Expected: admission returns `sealed` with an active goal; a simulated submitting Agent pauses on the actual owning connection, which must emit its pause notification before Relay confirmation. Resume/deleted/replaced all pass, with a live Relay job and zero simulated-model requests in each five-second paused window, one matching resumed result turn and no wake/modification for deleted/replaced goals. The source controller performs reads/restoration through public goal RPCs, and the simulated Desktop adapter forwards only the resulting notification through an explicit loopback `codex queue`. Reports/wire traces stay in the printed `.tmp/goal-codex-...` directory. The script does not use real chats/accounts or alter installations/configuration. Actual Desktop pause/restored-goal UI, Windows behavior and long-duration savings remain separate qualifications; host generation/CAS races cannot be proven absent by these tests.

For manual Desktop acceptance, use a user-created disposable goal chat and an explicitly updated matching source plugin/profile advertising `goal_wait_host_pause`. Submit a normal background request without adding any pause/restoration authorization phrase, using a bounded command that waits for a test marker. Expected: ordinary `connect` supplies a goal binding automatically, admission returns `sealed`, the Agent's native goal tool pauses the Desktop UI without another approval question, confirmation returns `waiting`, and the Agent ends its turn without subsequent checking turns while the command remains running. Release the marker: expect one same-chat notification with the original Job/Run IDs, a matching result turn and the restored goal state. Repeat in a separate disposable goal chat, deleting the goal before releasing its marker: expect the job result to persist, with no goal recreation or completion wake. Record actual client/runtime versions and UI/turn evidence separately for Linux and Windows; a shared-database read alone does not pass acceptance.

For retained native connections, run `cargo test --test plugin retained_mcp`. The same standard MCP connection survives same-version native image updates, executes one durable command through a real native worker, and observes the replacement workflow through an unchanged cached launcher. A bounded fixture covers catalog/caller routing, one notice per image, concurrent replacement exclusion, lost-ack non-replay and modified/downgraded/profile-moved refusal. Job/profile history remains unchanged after read-only/uncertain fixture operations. For actual Desktop acceptance, complete the one-time historical-entry migration/reconnection, retain that connection across a later owned-runtime update, then compare MCP `runtime_workflow_sha256` with launcher `instructions` and verify normal goal delegation uses native pause without extra wording. Host notification handling and Windows require separate evidence.
