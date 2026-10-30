# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Keep only current applicable acceptance and its limits here. Raw reports/private identifiers stay in ignored repository-local storage; historical runs remain in Git. Automated totals and candidate state belong in [implementation status](IMPLEMENTATION_STATUS.md).

## Linux ChatGPT Desktop

The user verified AgentJobRelay v0.1.8 local plugin/MCP submission and same-chat continuation after task completion. The user also confirms that current local goal mode pauses automatically after approval. That goal-pause observation does not independently qualify completion restoration, the new direct MCP form or a complete three-round workflow.

Use [published installation](installation.md#quick-install-the-latest-release) or [source installation](source-install.md) ([中文](source-install.zh-CN.md)), then [manual acceptance](manual-testing.md) for a relevant new change. Do not repeat an unchanged passed workflow merely to collect more evidence.

## Windows native runtime and MCP

The 0.1.15 candidate passes GitHub Windows MSVC gates, frontend/static-CRT build and installed-package/loader checks. These automated gates do not qualify Windows self-update/self-removal with held images, browser interaction or goal/controller behavior. Earlier Windows 11 VM external-task verification under SYSTEM and ordinary-user monitor/Web/maintenance acceptance remain applicable to their baselines; Windows goal/controller and native confirmation UI still require separate host evidence.

Installed v0.1.8 with actual Codex 0.164.0 and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under SYSTEM and an ordinary interactive user: no prestarted daemon, one completed native Job/Run, expected output and doctor reporting `windows_cim`. Installed npm/package/loader checks use isolated registries/profiles and a simulated OpenCode model; they remain separate from real provider/host acceptance.

## Windows ChatGPT Desktop

The user verified v0.1.8 with official Desktop 26.1021.3705.0, bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and Full access. The actual chat binds, submits through MCP, ends its turn and automatically continues to inspect the matching result without another user message. A correctly reported failed workload still establishes result continuation, not workload success. Restricted-access single-command binding, current goal waiting and native confirmation UI remain unqualified.

## Distribution

npm/GitHub 0.1.14 is public and both npm latest tags resolve to it, rechecked on 2026-10-30. Its [Release All](https://github.com/MorningFrog/agent-job-relay/actions/runs/41039827099) and [independent Check Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/41040159441) passed, including public-byte/channel/clean-install/GitHub verification. See [implementation status](IMPLEMENTATION_STATUS.md) and [release preparation](release.md). No official-directory submission occurred.

The retained 0.1.15 candidate passes [exact-revision CI](https://github.com/MorningFrog/agent-job-relay/actions/runs/41045437813), [Release All dry-run](https://github.com/MorningFrog/agent-job-relay/actions/runs/41046893086), [Linux/Windows candidate preparation](https://github.com/MorningFrog/agent-job-relay/actions/runs/41046910320) and the [actual OIDC publisher preview](https://github.com/MorningFrog/agent-job-relay/actions/runs/41048797044). Its original candidate bytes/identity and packaged updater were checked after download. Publication jobs were skipped. Source 0.1.16 adds feedback receipts and guidance beyond those sealed bytes; it has no new remote candidate or changed-guidance real-Agent acceptance. No new public version is claimed.

The 0.1.15 Linux npm check performs actual staged-tarball replacement in an isolated global prefix while the original runner continues with one workload launch and unchanged Job/Run. It verifies daemon-only upgrades with current package files, partial-update recovery, active-task uninstall refusal and forced self-removal with retained history, including Codex-generated marketplace installation records. Host mutations in that check use a public-CLI model. Separate actual Codex loader checks verify packaged tool discovery, missing-CLI refusal and public removal of the exact plugin/cache. This does not qualify Windows self-update/self-removal or actual Desktop refresh/reconnection.

## Runtime monitoring boundary

The current bundled React/Vite panel has isolated Linux embedded-browser verification at 360–1440 CSS-pixel widths: compact hover/toggle navigation, details immediately after the selected row, wide adjacent columns, no horizontal overflow, search/collapse, output/events, rules/application and observed stopping. The 0.1.14 sidebar fix additionally passes six-width native-bundle checks for stable button/icon coordinates and original pointer targets; source-browser checks cover short-height layout, keyboard focus and backdrop closure. Current-source manual Codex host-tool checks keep one IAB/provider tab ID through A → B → B → A → overview, with correct details, reset search and removed credential fragments. Exact-job URLs and the updated discover/navigate/show workflow are verified; autonomous reuse by a newly updated installed Agent remains separate acceptance. Windows native HTTP/package checks cover applicable monitor/Web/maintenance behavior. Windows browser interaction remains unqualified; real-Agent feedback acceptance is recorded separately below.

## Real-Agent mid-run feedback

On 2026-10-30, the user confirmed completion of the staged real-Agent feedback acceptance: the Agent submits one background task and yields, automatically reports the first two stages while that original Job/Run is still running, yields after each report, and gives a final summary after exit, without another human message, an Agent polling loop or workload replay. See the reproducible prompt in [English](test-prompts.md#verify-real-agent-mid-run-feedback) or [简体中文](test-prompts.zh-CN.md#验证真实-agent-中途反馈闭环) and [acceptance criteria](manual-testing.md#optional-real-agent-mid-run-feedback).

This is user-confirmed execution, distinct from fake-host acknowledgement. The confirmation did not specify OS, host/runtime versions, model or MCP/CLI route, so it is not attributed to a release or platform-specific matrix. Additional host/version combinations, OpenCode real-provider behavior and Windows installed checker workflows still need their own qualification. Goal waiting continues to reject notifying rules; this acceptance does not change that contract or establish periodic summaries, restart recovery or prolonged reliability.

## Goal waiting protocol proof

**Observed host result:** the user confirms that local goal mode pauses automatically after approval, and reports occasional `blocked` state after task completion when continuing/submitting the next round. The source of that host transition is not yet established. Source recovery retains unexpired consent and IDs instead of permanently invalidating them on an inactive status/read failure; it does not automatically clear unexplained blockers. The supported UX asks one short question when permission is missing and reuses the actual reply for the bounded remaining jobs of that request/goal. Plugin text cannot grant permission; consent-free suspension remains unavailable.

**Isolated integration:** `scripts/check_codex_goal.py --wait-seconds 5` uses the actual Desktop-selected Codex executable with a simulated submitting Agent, endpoint and model. Scoped preflight requires confirmation before work; an explicitly simulated reply then permits admission. The fixture injects the owning runtime's native pause, observes its update event and requires zero model-request growth during confirmed pause, matching result execution and deleted/replaced-goal suppression. After a result, an injected host blocker gates the next round while retaining approval/budget; host Resume permits the same request to reconnect. It does not prove real-model permission decisions, actual Desktop goal UI or the source of the reported blocker.

**Behavioral coverage:** Linux storage/CLI/MCP fixtures verify one answer for three separate original submissions, transactional limits/idempotent retries, scope/revision/expiry/deletion/revocation/restore fences, negotiated legacy forms, modern input-required responses and one Agent-question fallback. Retained-MCP tests cover current-image forwarding, a 91-second human wait and original profile/history preservation. Fake protocol clients are not real host form UI acceptance.

Direct MCP form UI, complete three-round real-model restoration, Windows goal/controller, changed-host goal recovery and long-duration savings remain unqualified. The host lacks generation/CAS and uses second-resolution timestamps, so identical same-second replacement and final verification/write/send races remain. No real chats/goals are created or changed by these fixtures. See [operation and limits](goal-waiting.md) and [backlog](TODO.md).

## External tasks (0.1.12+)

Linux→Windows native execution through the existing authenticated VM channel under SYSTEM verifies one launch, cwd/env/output, injected transport and local observer/daemon loss, stable local/remote IDs after reconnect and observed exact-run cancellation. This is not SSH-server, ordinary-user, machine-restart or real Desktop acceptance. Current Linux source observes a completed real GitHub Actions attempt read-only with matching conclusion, all five jobs, step counts and an attempt-specific URL; the embedded browser shows its details and opens that exact GitHub page on click. Long-name/current-step/partial-matrix cards pass 360/1280-pixel checks without horizontal overflow. In-progress/auth/attempt/detail-failure transitions use fixtures. External-task same-chat continuation and prolonged reliability remain unqualified.
