# Automated testing and evidence

[Documentation index](README.md) · [Contributing](../CONTRIBUTING.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

For reader-facing installation and ChatGPT Desktop acceptance steps, use [manual testing](manual-testing.md). This guide covers developer verification. Tests keep state, fake endpoints and logs under repository-local `.tmp/`; they do not modify personal settings, services, unrelated application state or real ChatGPT Desktop endpoints.

## Required checks

Run from the repository root:

```bash
npm ci --prefix web --ignore-scripts --no-audit --no-fund
npm test --prefix web
npm run build --prefix web
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

Current counts, toolchains and results are maintained in [implementation status](IMPLEMENTATION_STATUS.md). CI defines all-target Linux/Windows native tests, installed Linux/Windows package checks and macOS compilation/portable library checks. A configured matrix alone does not establish native OS support. CI keeps a separate Rust 1.89 minimum-version `cargo check --all-targets --locked` job; stable native behavior gates remain separate.

## Test selection and artifacts

Test behavior, correctness and regression risk through the existing Rust suites and fake hosts. Preserve the legacy daemon fixture and other required reference data. Do not add tests for README wording/structure or other implementation details that have no product-interface contract. Review documentation changes directly as part of normal maintenance.

Keep generated test state, outputs, reports and logs under ignored repository-local `.tmp/` or `target/`; never commit them. Versioned documentation records concise current results and verification limits. Historical runs are available in Git history, while required test cases/fixtures stay tracked.

The four configuration-integration fixtures run serially within their test binary. This prevents a parallel CLI fork from briefly inheriting another fixture's configuration lock before exec; each case keeps its own isolated paths and all assertions. Runtime concurrency cases continue to exercise their existing explicit concurrent workloads.

Plugin update/repair preservation checks checkpoint committed WAL pages before comparing complete database hashes. Terminal state and daemon-lock release can precede the last pooled connection closing; checkpoint timing alone must not look like data loss. The interrupted-repair fixture seeds committed WAL history before taking its snapshot and verifies that record remains readable after repair, alongside the existing backup/journal and database-preservation assertions.

## Embedded panel layout

Build the frontend before the native executable, then open `ui <job-id> --no-open` through the host’s embedded-browser tool using an isolated repository-local test profile. Check 360, 480, 761, 1250 and 1440 CSS-pixel widths. Expected: no page-wide horizontal overflow; an icon rail through 1250 pixels; mouse hover and keyboard/touch toggling reveal navigation without moving the main content or any navigation button/icon. Hover each collapsed icon at its original center, verify that the same button remains under the pointer, then click that coordinate and confirm the requested page. Buttons show distinct hover feedback and a visible keyboard-focus outline. Repeat at a 400-pixel height to catch flex shrinking; Escape/backdrop closes an explicitly opened menu. Selecting a task inserts exactly one detail panel directly after that row, switches cleanly to another row and collapses on a second activation. Wider windows place the same task detail beside the list. Search/filter must not leave a hidden selected task’s detail below the narrow list; direct task URLs must retain the requested job even outside the latest history page.

Verify stdout/stderr, rules/form controls, events and a stop request against fixture jobs. Expected: the rule’s saved/applied state and eventual terminal status remain visible in the inline panel. Reset temporary viewport overrides afterward. The runtime workflow must open one exact-profile task panel after accepted submission and before yielding/native goal pause; pending/rejected submissions do not open one, and UI failures do not resubmit or cancel accepted execution. Runtime-instruction inspection plus a manually exercised host-panel tool establishes the opening mechanism, while a newly updated real host Agent following it remains separate acceptance.

For panel reuse, start two fixture jobs and obtain their fresh `ui <job-id> --no-open` URLs from that same isolated profile. List this chat's embedded tabs before opening; create the first panel only if none matches the exact origin/path. Reuse its ID for A → B → B → A and an overview URL, navigating then showing that same tab. Expected: one tab ID throughout, the requested detail/output after each task navigation, no detail in overview, cleared credential fragments and no authentication error. Repeat with search/state filters set before navigation; the requested task must become visible. An inventory/open failure must not automatically create another panel. Node navigation tests cover fragment consumption, repeated selection, credential refresh and listener cleanup; actual host navigation is a separate browser check.

## Behavior coverage

| Suite | Coverage | Boundary |
|---|---|---|
| Library | MCP schemas, legacy launch-readiness and interrupted replacement recovery/refusal/locks | Local checks plus deterministic filesystem fault states |
| Monitoring and Web | Literal chunks after prefix cap, daemon-independent rule/tree stop, silence dedupe/rearm, revision/terminal fences, offline reads, sequence migration, HTTP auth/origin/operation restrictions and installed launcher UI | Linux/Windows native behavior plus portable storage checks; real browser interaction verified separately |
| Runtime | Execution/output/cursors, idempotency, process trees, cancellation/timeout, daemon/runner independence, launch recovery, quotas, IPC/security; batch overlap/options, partial failures, quota races and lost-response/restart retry | Real Linux children and injected failure windows |
| Reliability | Profile-scoped log-write/log-sync/WAL-write ENOSPC, delayed buffered-write error propagation, unknown recovery/non-replay, adopted session-escaped children and unrelated-process preservation, concurrent resource bounds/restarts/active backup/restore | Linux native processes and explicit test-only I/O faults; no physical disk/power-loss qualification |
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

For inspection sampling, run `cargo test --lib web::sampling::tests`, `cargo test --test data archived_inspection` and `cargo test --test monitor shared_web_streams`. Expected: eight same-task viewers reuse one serialized sample; task/health expiry, selection changes, bounded navigation, profile isolation and error recovery behave independently; explicit doctor remains fresh; archived detail pages match hot history before and after rehydration and refuse corrupt snapshots; real authenticated SSE connections retain their selections and observe a direct stop. Storage fixtures never execute their placeholder commands; the Linux HTTP regression uses one owned native workload.

For 0.1.12 batch changes, run `cargo test --test runtime batch` and `cargo test --test convenience batch`. Expected: three runtime and two CLI/MCP regressions pass, including independent monitor stopping, invalid-session rollback, stable IDs after a discarded response/restart and exact fake-Desktop caller destinations. With the isolated OpenCode toolchain described in [host plugins](host-plugins.md#account-free-actual-loader-check), run `python3 scripts/check_opencode_plugin.py --batch`. Expected: the actual loader exposes job_start_batch, two independent items complete and notify the actual caller while one invalid item is rejected, forged model-supplied caller fields are overridden, the control session stays empty and the simulated model runs after host acceptance. This adds no real-provider, Windows or real ChatGPT Desktop batch evidence.

The daemon shutdown regression holds a real SQLite writer while an owned queued runner forces reconciliation to await it, then sends SIGINT before releasing the writer. It keeps the original short exit deadline, verifies the existing workload survives, and checks notification dispatch after restart. Control requests must remain responsive during that writer wait, and SIGINT still stops the daemon without stopping independent runners. Another regression occupies all four general database workers and requires status to use reserved read capacity. Development/test profiles retain line tables to keep installed fixtures below the existing 128-MiB image budget; CI uses no debug symbols. All behavior cases remain enabled.

Run `cargo test --test daemon_upgrade --test legacy_upgrade --test plugin` for automatic update behavior. A real native fixture with changed package-version bytes exercises concurrent startup/UI handover, preserved running Job/Run/logs and launch count, newer-peer reuse and schema-migration refusal without stopping old work. Installed update checks cover preview/no-op, active-job refusal, the new daemon's loaded path and retained history/cache snapshots; legacy checks exercise actual pidfd/SIGINT handover. The IPC library regression proves a changed process identity receives no shutdown bytes. These checks use isolated Linux profiles; they do not qualify all historical releases, Windows handover or real host continuation.

Run `cargo test --lib interrupted_doctor_probe` for the deterministic concurrent-start interruption window. Expected: after the first Doctor connection closes without a reply, the client waits for the held startup lock and rechecks the authenticated endpoint; a ready current peer is reused without launching another daemon or creating a database. The incompatible-protocol regression in `tests/convenience.rs` must still fail without entering startup or replacing the peer.

Run `cargo test --lib progress::` and `cargo test --test monitor` for source progress. Expected: bounded split-stream framing and invalid/flood handling; sequence/cooldown/revision/terminal fences; transactional rollback and archive/backup preservation; actual CLI threshold stopping/shared HTTP snapshots; and the reference checker delivering mid-run to its exact fake Desktop chat while remaining independent of daemon lifetime. Fake-host acknowledgement is not observed real Agent execution; no new Windows/real-provider acceptance is claimed.

For feedback receipts, run `cargo test --test feedback`, the existing monitor/goal-wait suites and `cargo test --test data archived_inspection`. Expected: current persisted rule changes survive unchanged-key retries, receipt diagnostics cannot turn admitted work into an apparent submission failure, inbox/source/application evidence never claims Agent execution, explicit target rebind/lease state is followed without host probes, one-shot/progress exhaustion and goal restrictions remain visible, CLI/shared MCP/HTTP carry the receipt, and hot/cold detail remains equivalent without rehydration. These tests qualify receipts and protocol behavior; natural-language adoption of changed guidance needs real-Agent acceptance.

For temporary goal feedback, run `cargo test --test goal_wait feedback` and `cargo test --test data schema`. Expected: repeated restoration/native re-pause with stable IDs/accounting; coalesced delivery history and group cooldown/budget; stale/lost confirmation and late intermediate ACK fences; final completion before transmission and during reporting; uncertain restore, timeout, cancellation/deletion and backup/restore without replay; schema v11 migration with unchanged history. `python3 scripts/check_codex_goal.py --wait-seconds 2` adds an actual-Codex feedback case: a native checkpoint produces a model turn after delivery-time renewal of an expired opted-in session, an explicitly simulated owning-host pause is confirmed, no model requests occur during re-paused waiting, and final continuation completes the same run. The Agent/model/endpoint are simulated; no real Desktop goal is changed.

## Manual feature prompts

Copy the short task requests in [English](test-prompts.md) or [中文](test-prompts.zh-CN.md) for directory scans, file searches, builds, progress updates and task management.

## Actual Codex plugin-loader check

After the release build, with Codex CLI and Python available:

```bash
python3 scripts/check_codex_plugin.py
```

This exports and relocates an offline bundle, bootstraps in disposable repository-local HOME/CODEX_HOME, removes the delivery artifact, installs through the actual loader, checks its returned cache path/profile and no-op refresh, and requires MCP discovery including job_recover_session, job_start_batch, job_goal_consent and job_goal_wait (25 tools in current source). It asserts that no daemon/workload state was created. Reports/logs remain in the printed ignored `.tmp/loader-...` directory. Python is a developer-check dependency only.

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

The goal-controller image regression rewrites an equal-length executable while preserving its modification time and requires validation to reject the changed bytes, including filesystems that round consecutive writes into one timestamp tick.

The portable progress test verifies archival on every build platform, explicit atomic-backup refusal on unsupported platforms, and complete backup/restore evidence on Linux/Windows. macOS compilation and portable tests do not establish native runtime support.

The Python MCP probes allow at most 1 MiB per JSON line so the full tool catalog fits (25 tools in the current runtime); asyncio's default 64-KiB reader limit is too small for the expanded schemas. The Claude scaffold check retains its request deadlines, required-tool checks, explicit caller-isolation assertions and inbox-only boundary.

## Installed release-candidate checks

Follow [release preparation](release.md) for `prepare_release.py` and `check_release.py`. Unlike a source-linked import check, they install the real npm tarballs, use their packaged native executable, preserve job history across runtime relocation and refuse a corrupt payload. For 0.1.11 and newer, the checks also install into an isolated npm global prefix, invoke the generated `ajr` command/shim, verify default/explicit profile selection and environment overrides, submit a real command and inspect its same-profile history through the authenticated Web UI. The Windows checks exercise npm's `.cmd` shim; running those checks on Windows remains necessary to qualify that platform. OpenCode uses its actual loader/MCP/agent loop with a simulated model. Codex adds the extracted self-hosted catalog through its real CLI and discovers the tools through a loopback fixture registry; it does not claim public npm availability or ChatGPT Desktop UI installation. Prepare Release repeats both host checks; Publish Release dry-run checks OIDC and previews the sealed candidate. Push/PR CI performs the Linux installed OpenCode check and assembles the same-revision Linux/Windows images for the actual installed Windows package/loader check. Windows failures include bounded job/status/output diagnostics and a seven-day fixture-only diagnostic artifact; native-only CI is not a substitute for the installed-package test. Generated artifacts/reports remain ignored.

Release-state regression checks run with `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_release.py -v`; CI, Prepare Release and Publish Release run the same checks. They cover version/changelog errors, credential-free preparation, the exact OIDC identity, no-poll publication, acceptance receipt/source/hash validation, interrupted receipt/public-metadata recovery, independent read-only propagation checking, bounded timeouts, fresh-state/credential-free ETARGET installation retries, immediate permanent/native/import/integrity failure, exact child dispatch/reuse after a lost response, child metadata initialization without redispatch or weakened identity checks, failed-child continuation, main/source fences and foreign-content refusal. Legacy stage validation/approval behavior, immutable inventories and draft pagination/exact-ID uploads remain covered. Service fixtures, a simulated clock and a loopback HTTP server exercise these cases without real writes or accounts. Validate CI and all four release workflows with actionlint. Preserve the original candidate bytes; see [release handoff](release.md#current-release-handoff) and [implementation status](IMPLEMENTATION_STATUS.md) for current candidate and public-verification results.

## Windows native verification

Windows behavior lives in `tests/windows.rs`, with portable library, delivery and integration checks alongside it. Coverage includes command arguments/exit codes, PowerShell/cmd, capped raw logs, split-output rules, detection/tails after the prefix cap, silence rearming/revision fences, daemon-independent monitoring/tree stop, bounded stop observation, default Web assets/auth/origin/control, tree cancellation/timeouts, runner-death containment, daemon stop/crash, MCP exit, cold-start capture-handle isolation, private DACL/junction refusal, bounded busy-pipe probing with connected-instance authentication, Node/Bun pipe owner/process fencing, malformed/duplicate pipes, inbox isolation, simulated ChatGPT Desktop routing/recovery/renewal and data-preserving offline maintenance. The installation fixture clears inherited `AJR_STATE_DIR` and host caller context so background execution cannot select the outer runtime's profile; an ambient-profile regression verifies this isolation. Current native results are in [implementation status](IMPLEMENTATION_STATUS.md).

Cargo's Windows cleanup Job Object forbids breakaway. The harness launches only its owned fixture processes through local CIM under the same SID outside that job, then controls the exact recorded daemon instance; runner spawning remains native CreateProcess with inheritance disabled. Daemon startup now supports a verified same-user local CIM helper when a cleanup job denies direct creation; native regressions exercise this actual product route, cold environment/identity isolation, host job destruction, daemon-crash restart without replay and policy/token/peer refusal. Tests cover nested inner-breakaway/outer-restrictive jobs, pre-execution detachment checks, direct-only refusal without a database/workload, and busy host pipes authenticated on the actual connection. Tests keep state under the disposable source snapshot's ignored `.tmp/`. These native checks are separate from verified ChatGPT Desktop live acceptance. See [Windows operation](windows.md) and [ADR 0011](adr/0011-windows-native-runtime.md).

Actual Windows Codex MCP/model acceptance has passed from a cold profile with no prestarted daemon, using `gpt-6.1-sol`/`low`; doctor reported `windows_cim`. The cold-profile MCP test passes under both SYSTEM and an ordinary interactive Windows user. Windows ChatGPT Desktop same-chat continuation is verified separately with GPT-6 Luna / Low and Full access. Account-free catalog discovery also passed with the original portable-plus-compatibility declarations. See [live evidence](live-e2e.md) for the version-specific result; credentials and raw transcripts stay out of reports/source.

Windows installed-package reproduction uses `scripts/check_windows_packages.mjs`; see [release preparation](release.md). It tests actual npm installation/native execution, relocation/history preservation, embedded JS/CSS loading, unauthenticated API refusal, authenticated same-profile inspection and corrupt-payload refusal. For 0.1.10 it also exercises installed CLI rule creation, alert inspection, bounded stopping and offline inspection. Codex uses the original portable-plus-compatibility catalog with only registry transport replaced by a loopback fixture; OpenCode uses the real Bun package-name loader, MCP caller hook and public session API with a simulated model. A matching-session response and empty control session are required. This does not establish public npm availability or ChatGPT Desktop acceptance.

## Data-maintenance verification

Run `cargo test --test data` for data-maintenance scenarios and `cargo test --test delivery --test runtime` for reserved-read and control-response regressions. Tests reconstruct archive rollback/post-commit cleanup and backup sealed/unsealed states, verify exact IDs/expiry/attempts and checksum/foreign-file refusal, and restore active intents as orphaned without execution. The daemon case proves maintenance runs without a manual call. These controlled state/fault tests pass on Linux and Windows; physical disk/power faults and prolonged load need separate qualification. See [data maintenance](data-maintenance.md).

## Local browser acceptance

Use only an isolated profile under .tmp/, run ui --no-open, then open its returned access URL. Expected: real task rows update, selecting a task shows bounded stdout/stderr, editing a rule first shows pending then applied/triggered, and the event view retains evidence. Stopping updates the task to its observed terminal outcome. Verify the same history through inspect --json, then stop only this test profile's daemon. This is browser/runtime acceptance, not ChatGPT Desktop or another host's same-chat continuation. React/Vite build output and browser screenshots stay ignored.

## External-task verification (0.1.12+)

Run `cargo test --test remote` for native reference-helper and observer behavior. Linux covers lost acknowledgement/idempotency, lookup-only cancellation, target/run/attempt fences, raw byte cursors, transport/observer/remote-runner loss, read-only versus controlled stopping, source shared-MCP/CLI retry plus origin inbox alerts/results, observer storage failure and terminal/recovery races. Windows runs the four applicable native cases using the existing verified daemon bootstrap outside Cargo’s cleanup Job Object. Workflow status/auth/attempt changes use a bounded fake `gh`; tests assert GET-only calls, current-step/count snapshots, partial matrices, attempt-specific URLs, detail failure preserving terminal evidence and no workflow mutation. Portable unit coverage fences job run/attempt identities and caps retained names/pages.

For a real remote host, follow [remote task setup](remote-execution.md), choose isolated profiles, emit a unique milestone and record a launch counter. Expected: disconnecting the transport or losing the local observer preserves remote work; reconnect keeps the original run IDs and byte cursors, the counter remains one, read-only stop leaves work alive and a control stop observes the exact remote terminal result. Use only the created profile’s daemon/owned workload for cleanup. SSH authentication/shell deployments, real host continuation and prolonged/machine-loss tests need separate qualification. For Actions, select a real existing repository/run/attempt and compare retained conclusion/URL with a read-only API result; never dispatch or cancel a workflow for this check.

## Goal-waiting verification (experimental)

Run `cargo test --test goal_wait` and `cargo test --lib goal_control`. Isolated Linux coverage includes structured pre-consent gating, one scoped approval for three separate original submissions with a blocked→host-Resume transition after the first result, unchanged-key budget preservation, blocked single/batch admission, inactive native-question suppression, controller-loss preservation/local revocation, stale-observation revision fencing, scope/token fences, legacy/modern native input, one Agent-question fallback, expiry/deletion/revocation/restore and native-pause/accounting/recovery/uncertain-mutation cases. No test touches a real user goal.

After a release build, on Linux with Python and a Node version providing built-in WebSocket:

```bash
python3 scripts/check_codex_goal.py --codex "<absolute-goal-capable-codex-executable>" --wait-seconds 5
```

Omit `--codex` to use the actual terminal’s Desktop-selected runtime, falling back to PATH. Expected: scoped preflight returns pending input without jobs/waits, then an explicitly simulated user answer permits admission. The fixture injects native pause on the owning runtime connection. Resume/deleted/replaced cases must observe that pause event, zero simulated-model requests per confirmed five-second pause window, one matching result turn and no deleted/replaced goal wake. The resume case additionally injects a post-result blocker, requires gated reconnection with unchanged approval/budget, then uses host Resume and reconnects with the original request. The endpoint, submitting Agent and model are simulated; this does not qualify real-model dialogue or Desktop form UI. Reports stay in `.tmp/`; real chats/configuration/installations remain untouched.

The user already confirms local automatic pause after approval. For remaining manual acceptance, use a user-created disposable goal chat with the updated source plugin/profile and three separately requested bounded runs without pause wording. Expected: one short native form or permitted host question, no admission before reply and no duplicate question/blocked status. A natural affirmative answer permits each round through the same request ID, with native pause/acknowledgement, matching result inspection and restoration. Put requested delays inside the workload. Initial permission skips the question; denial, no response or plugin text never grants it. Native form UI and complete three-round real-model restoration remain separate acceptance, alongside Windows schema v12/controller and long-duration savings.

Run `cargo test --test plugin retained_mcp` for verified current-runtime forwarding. A retained standard MCP connection spans native image updates, executes one durable command and preserves history. Fixture coverage includes caller/catalog routing, writer exclusion, one image notice, lost-ack non-replay, tamper/downgrade/profile refusal, a 91-second native human wait and modern input-required retry across worker processes. Human wait is excluded from the 90-second active transport budget. Fake protocol clients do not prove actual host UI support. For actual update acceptance, migrate historical entries once, then retain the current connection across later verified updates and compare MCP workflow digest with launcher `instructions`; no cache scan/rewrite or profile movement is needed. See [goal waiting](goal-waiting.md).

## Native reliability and sustained load

Run `cargo test --test reliability` on Linux. The suite reuses the native runtime rig and compiles `tests/fixtures/storage-fault.c` with `cc` into ignored `.tmp/`. Only explicitly selected test daemons/runners receive that preload library; an exact fixture-profile prefix plus activation file gates ENOSPC for stdout writes/syncs or SQLite WAL writes. A separate evidence file proves the intended I/O boundary was reached. The production runtime has no test fault switches. Expected: failed log persistence never reports success, database-outage recovery preserves original IDs and unknown classification, retries never relaunch, and live-runner adopted-child cleanup does not signal an unrelated process. Existing runner-death tests retain their separate unknown-effects boundary.

The default sustained-load case lasts three seconds. For ten minutes of eight concurrent native workloads, repeated daemon crashes/restarts, output beyond each retained prefix, structured progress, bounded per-runner RSS/file descriptors, unchanged-key retries, exact-run completion and an active backup restored without execution replay:

```bash
AJR_RELIABILITY_SECONDS=600 AJR_RELIABILITY_REPORT="$PWD/.tmp/reliability-soak.json" cargo test --test reliability native_soak -- --nocapture
```

Expected: eight original Job/Run pairs complete once, each workload launch file contains one entry, stdout/stderr stay within 16 MiB each, per-runner RSS growth stays within 64 MiB and descriptor growth within 32, and restored active jobs are orphaned under their original IDs. Reports must stay under repository `.tmp/`; fixture state/children are owned and cleaned by the rig. This is bounded Linux soak evidence, not multi-day reliability, disk-reservation, independent-storage backup or physical power-loss qualification.

Frontend `npm test --prefix web` checks split component rendering against runtime fixtures and stream parsing, size bounds, visibility/retry/authentication handling, stale task-response fencing and rule revision refusal. These are deterministic transport/render tests; actual browser interaction remains a separate acceptance.

## Unified online maintenance (0.1.14+)

`cargo test --test online_update` checks native profile/installation exclusion, schema/downgrade fences, compatible active-run handover, idle restart/history, default uninstall shutdown, forced task stops without purge, read-only preview and lost-controller behavior. `node --test scripts/test_online_update.mjs scripts/test_online_uninstall.mjs` checks public-host command sequences, shared/foreign content protection and uncertain-acknowledgement recovery without actual host writes. Real Git fixtures also cover Codex's generated installation record, exact release provenance, malformed/oversized/stale/foreign/sparse records, preserved tracked/staged/untracked changes and Linux symlink refusal. Diagnostics cover exact paths/status, rename pairs, escaped filenames, capped inventories and specific record-validation reasons without record values. CI runs the Node fixtures on all build platforms. The Linux installed checker includes the generated record in both marketplace revisions and prepares its older-version image through an atomic sibling-file replacement, preserving any runner still mapped to the previous image after terminal persistence; `test_loaded_native_fixture_replacement` verifies the kernel busy-file case, original process/inode survival and executable permissions.

After preparing a repository-local candidate, `python3 scripts/check_release.py --release <candidate-directory> --codex <codex-executable>` installs it in a disposable global npm prefix. The same-schema older-image fixture and loopback registry verify actual staged-tarball replacement while an independent runner continues, original IDs/history, no workload replay, daemon-only upgrade with current package files, partial failure/forward retry, active-task uninstall refusal and forced self-removal with retained history. Host operations in that check use a public-CLI model. The separate actual Codex loader checks catalog/MCP discovery and public plugin/cache removal without a model turn. These checks do not establish Windows self-update/self-removal or real Desktop refresh/continuation. All generated state and helper images stay in repository-local ignored directories.
