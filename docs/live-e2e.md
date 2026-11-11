# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Keep only current applicable acceptance and its limits here. Raw reports/private identifiers stay in ignored repository-local storage; historical runs remain in Git. Automated totals and candidate state belong in [implementation status](IMPLEMENTATION_STATUS.md).

## User-reported sustained real-work use

On 2026-11-02, the user confirmed the following results from sustained use in actual work:

| Workflow | Observed result |
|---|---|
| Ordinary mode | Wake-up after task completion works normally |
| Goal mode | Wake-up works normally; pause/resume works in the vast majority of cases, with occasional inability to resume |
| Remote tasks | Remote execution and wake-up at completion work normally |
| Intermediate feedback | Mid-run feedback works normally for ordinary-mode and remote tasks |

These are user-reported real-work results, separate from simulated-host acknowledgements and bounded automated tests. The report does not specify OS, host/runtime versions, model, MCP/CLI route, remote transport/adapter, exact duration or task/failure counts; no release/platform attribution or numerical reliability rate is inferred. Goal-mode intermediate feedback/native re-pause, a specific direct `remote_alert` subscription and instrumented multi-day lease/restart/load behavior are not established by this report. Occasional goal resume failure remains open; its cause and relationship to the earlier reported `blocked` state are undetermined.

## Linux ChatGPT Desktop

The user verified AgentJobRelay v0.1.8 local plugin/MCP submission and same-chat continuation after task completion, and separately confirmed local automatic goal pause after approval. The broader sustained-use results above have no platform/version attribution; the new direct MCP form and a specifically observed complete three-round workflow remain separate acceptance items.

Use [published installation](installation.md#quick-install-the-latest-release) or [source installation](source-install.md) ([中文](source-install.zh-CN.md)), then [manual acceptance](manual-testing.md) for a relevant new change. Do not repeat an unchanged passed workflow merely to collect more evidence.

## Windows native runtime and MCP

The published 0.1.24 candidate passes Woodpecker Windows MSVC gates, frontend/static-CRT build and installed-package/loader checks. These automated gates do not qualify Windows self-update/self-removal with held images, browser interaction or goal/controller behavior. Earlier Windows 11 VM external-task verification under SYSTEM and ordinary-user monitor/Web/maintenance acceptance remain applicable to their baselines; Windows goal/controller and native confirmation UI still require separate host evidence.

Installed v0.1.8 with actual Codex 0.164.0 and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under SYSTEM and an ordinary interactive user: no prestarted daemon, one completed native Job/Run, expected output and doctor reporting `windows_cim`. Installed npm/package/loader checks use isolated registries/profiles and a simulated OpenCode model; they remain separate from real provider/host acceptance.

## Windows ChatGPT Desktop

The user verified v0.1.8 with official Desktop 26.1021.3705.0, bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and Full access. The actual chat binds, submits through MCP, ends its turn and automatically continues to inspect the matching result without another user message. A correctly reported failed workload still establishes result continuation, not workload success. Restricted-access single-command binding, current goal waiting and native confirmation UI remain unqualified.

## Woodpecker observation (0.1.25 source)

On 2026-11-10, an isolated Linux source release build observes this project's existing public Woodpecker pipeline **#42**, global ID **69**, source `baf9a7b18a219e298354f52330c2aebbc354538d`. The observer reports completed with verified Server `success`, all six workflow states and exact step counts matching a separate read-only API query. CLI, shared stdio MCP and authenticated local HTTP return the same snapshot; unchanged-key CLI/MCP submission retains the original Job/Run, and compact MCP inspection includes identity/status/link. The check uses native HTTPS without account credentials and starts no CI work. This establishes public read/inspection behavior, not private-account authentication, native Windows/macOS or real-Agent completion continuation. Browser automation was unavailable in this session; frontend rendering is covered separately by automated tests.

## Distribution

Publication, candidate receipts and independent verification status are maintained in [release handoff](release.md#current-release-handoff) and [implementation status](IMPLEMENTATION_STATUS.md). They do not extend version-specific Desktop/Windows goal qualification or establish official-directory approval. Preserve published notes, dates and candidate bytes.

The 0.1.19 Linux development-package baseline performs actual staged-tarball replacement in an isolated global prefix while the original runner continues with one workload launch and unchanged Job/Run. It verifies daemon-only upgrades with current package files, partial-update recovery, active-task uninstall refusal and forced self-removal with retained history, including Codex-generated marketplace installation records. Host mutations in that check use a public-CLI model. Separate actual Codex 0.166.0 loader checks verify 25 packaged tools, missing-CLI refusal and public removal of the exact plugin/cache. This does not qualify Windows self-update/self-removal or actual Desktop refresh/reconnection.

## Runtime monitoring boundary

The published-source layout baseline has isolated Linux in-app-browser checks at 360/1280 CSS-pixel widths for terminal-first task details: compact title/status/stop controls, stdout/stderr, automatic following and manual-scroll pause/resume, keyboard tab navigation, separate progress/feedback views, failed tasks and collapsed command/identity information. No horizontal overflow was observed. Earlier applicable source evidence covers archived-history search, archived-log byte locations/context and disk-capacity notices; rendering fixtures retain source/target, recovery and backup variants. Tests use a private profile without changing user tasks or host goals. CLI/MCP/authenticated-HTTP and native storage evidence is in [implementation status](IMPLEMENTATION_STATUS.md). Updated installed-Agent adoption, actual Goal recovery and Windows browser behavior remain separate.

## Real-Agent mid-run feedback

On 2026-10-30, the user confirmed completion of the staged real-Agent feedback acceptance: the Agent submits one background task and yields, automatically reports the first two stages while that original Job/Run is still running, yields after each report, and gives a final summary after exit, without another human message, an Agent polling loop or workload replay. See the reproducible prompt in [English](test-prompts.md#verify-real-agent-mid-run-feedback) or [简体中文](test-prompts.zh-CN.md#验证真实-agent-中途反馈闭环) and [acceptance criteria](manual-testing.md#optional-real-agent-mid-run-feedback).

The 2026-11-02 sustained-use report above additionally confirms normal mid-run feedback for ordinary-mode and remote tasks. Neither report specifies OS, host/runtime versions, model or MCP/CLI route, so neither is attributed to a release or platform-specific matrix. Additional host/version combinations, OpenCode real-provider behavior and Windows installed checker workflows still need their own qualification. These reports do not establish temporary goal restoration/native re-pause for intermediate feedback, periodic summaries or instrumented restart/lease recovery. Autonomous adoption of the published 0.1.19 long-task planning guidance remains unqualified; passive visibility diagnostics have isolated native regression coverage.

The source 0.1.23 cooldown-retention, combined-alert context and Goal diagnostic changes have separate source regression coverage in [implementation status](IMPLEMENTATION_STATUS.md). They have no new real-model/Desktop or native Windows acceptance and do not resolve the reported occasional Goal resume failure.

## Goal waiting protocol proof

**Observed host result:** the user's 2026-11-02 sustained-work report confirms normal goal-mode wake-up and pause/resume in the vast majority of cases, with occasional inability to resume. Earlier local acceptance confirms automatic pause after approval; an earlier report describes occasional `blocked` state after completion when continuing/submitting the next round. The cause of the resume failures and any connection to that host transition remain undetermined. Source recovery retains unexpired consent and IDs instead of permanently invalidating them on an inactive status/read failure; it does not automatically clear unexplained blockers. The supported UX asks one short question when permission is missing and reuses the actual reply for the bounded remaining jobs of that request/goal. Plugin text cannot grant permission; consent-free suspension remains unavailable.

**Isolated integration:** The 0.1.19 goal baseline passes `scripts/check_codex_goal.py --wait-seconds 5` with actual Desktop-selected Codex **0.166.0-alpha.5.1** and a simulated submitting Agent, endpoint and model. One explicitly simulated approval covers three separately executed rounds in the same loaded thread, with original request/Job/Run IDs, unchanged-key retries, exact admission accounting and zero model-request growth during each confirmed pause. An injected post-first-result blocker requires host Resume; round two restarts Relay between owning native pause and confirmation, then reconnects through the original `goal_wait_pending`/`confirm_pause` revision without another question or binding. All three result turns execute in the original thread. The separate feedback case verifies temporary restoration, expired opted-in lease renewal, originating native re-pause and final continuation; deletion/replacement cases suppress obsolete notifications. All four cases pass and isolated cleanup succeeds. This establishes the protocol, not real-model permission/reporting decisions, Desktop UI, Windows behavior or the source of the reported blocker; no real user chat was changed.

**Behavioral coverage:** Linux storage/CLI/MCP fixtures verify one answer for three separate original submissions, transactional limits/idempotent retries, scope/revision/expiry/deletion/revocation/restore fences, negotiated legacy forms, modern input-required responses and one Agent-question fallback. Runtime 0.1.19 recovery coverage includes interrupted native-pause handoff, compatibility paths without scoped IDs, delayed versus prompt final receipts, blocked/replaced-goal refusal, exhausted-budget feedback re-pause and compact/panel action diagnostics. Retained-MCP tests cover current-image forwarding, the human-wait/active-budget boundary through a paused clock and short actual transport, and original profile/history preservation. Fake protocol clients are not real host form UI acceptance.

Direct MCP form UI, a specifically observed complete three-round real-model workflow, actual Desktop goal-feedback/re-pause, Windows goal/controller, changed-host goal recovery and measured long-duration savings remain unqualified. The host lacks generation/CAS and uses second-resolution timestamps, so identical same-second replacement and final verification/write/send races remain. No real chats/goals are created or changed by these fixtures. See [operation and limits](goal-waiting.md) and [backlog](TODO.md).

## External tasks (0.1.12+)

Linux→Windows native execution through the existing authenticated VM channel under SYSTEM verifies one launch, cwd/env/output, injected transport and local observer/daemon loss, stable local/remote IDs after reconnect and observed exact-run cancellation. This is not SSH-server, ordinary-user, machine-restart or real Desktop acceptance. Current Linux source observes a completed real GitHub Actions attempt read-only with matching conclusion, all five jobs, step counts and an attempt-specific URL; the embedded browser shows its details and opens that exact GitHub page on click. Long-name/current-step/partial-matrix cards pass 360/1280-pixel checks without horizontal overflow. In-progress/auth/attempt/detail-failure transitions use fixtures. The sustained-use report above separately confirms remote execution, completion wake-up and mid-run feedback; transport/adapter, host/version and fault-recovery attribution remain unspecified.

Runtime 0.1.20+ direct remote-alert forwarding has Linux native-runner/source-MCP and simulated Desktop endpoint coverage: two intermediate notifications and a final notification reach the original target across transport disconnection and observer/daemon restart, with original IDs and one workload launch. This does not establish actual Desktop/OpenCode Agent continuation, Goal feedback UI, native Windows or SSH deployment acceptance for the new subscription. See [direct remote feedback](remote-execution.md#send-intermediate-feedback-to-the-origin-0120).
