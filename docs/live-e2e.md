# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Keep only current applicable acceptance and its limits here. Raw reports/private identifiers stay in ignored repository-local storage; historical runs remain in Git. Automated totals and candidate state belong in [implementation status](IMPLEMENTATION_STATUS.md).

## Linux ChatGPT Desktop

The user verified AgentJobRelay v0.1.8 local plugin/MCP submission and same-chat continuation after task completion. The user also confirms that current local goal mode pauses automatically after approval. That goal-pause observation does not independently qualify completion restoration, the new direct MCP form or a complete three-round workflow.

Use [published installation](installation.md#quick-install-the-latest-release) or [source installation](source-install.md) ([中文](source-install.zh-CN.md)), then [manual acceptance](manual-testing.md) for a relevant new change. Do not repeat an unchanged passed workflow merely to collect more evidence.

## Windows native runtime and MCP

The sealed 0.1.12 baseline passes GitHub Windows MSVC gates with 69 tests, frontend/static-CRT build and schema v11 storage/progress checks. Subsequent Actions detail changes are verified on Linux only. Earlier Windows 11 VM external-task verification under SYSTEM and ordinary-user monitor/Web/maintenance acceptance remain applicable to their baselines; Windows goal/controller and native confirmation UI still require separate host evidence.

Installed v0.1.8 with actual Codex 0.164.0 and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under SYSTEM and an ordinary interactive user: no prestarted daemon, one completed native Job/Run, expected output and doctor reporting `windows_cim`. Installed npm/package/loader checks use isolated registries/profiles and a simulated OpenCode model; they remain separate from real provider/host acceptance.

## Windows ChatGPT Desktop

The user verified v0.1.8 with official Desktop 26.1021.3705.0, bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and Full access. The actual chat binds, submits through MCP, ends its turn and automatically continues to inspect the matching result without another user message. A correctly reported failed workload still establishes result continuation, not workload success. Restricted-access single-command binding, current goal waiting and native confirmation UI remain unqualified.

## Distribution

npm/GitHub 0.1.12 is public and both npm latest tags resolve to it. Publication succeeded; the original Actions checker timed out on registry propagation. A local read-only audit on 2026-10-30 verifies public package bytes/channels/clean installation, matching GitHub notes/assets/receipts and the original source tag against candidate 40908803482. The failed Actions run remains unchanged. Source 0.1.14 additionally implements unified online installation/update/uninstall and requires new CI, candidate and publisher preview. See [implementation status](IMPLEMENTATION_STATUS.md) and [release preparation](release.md). No official-directory submission occurred.

The local 0.1.14 npm check performs actual staged-tarball replacement in an isolated global prefix while the original Linux runner continues with one workload launch and unchanged Job/Run. It verifies daemon-only upgrades with current package files, partial-update recovery, active-task uninstall refusal and forced self-removal with retained history. Host mutations in that check use a public-CLI model. A separate actual Codex loader verifies packaged tool discovery, missing-CLI refusal and public removal of the exact plugin/cache. This does not qualify Windows self-update/self-removal or actual Desktop refresh/reconnection.

## Runtime monitoring boundary

The current bundled React/Vite panel has isolated Linux embedded-browser verification at 360–1440 CSS-pixel widths: compact hover/toggle navigation, details immediately after the selected row, wide adjacent columns, no horizontal overflow, search/collapse, output/events, rules/application and observed stopping. Current-source manual Codex host-tool checks keep one IAB/provider tab ID through A → B → B → A → overview, with correct details, reset search and removed credential fragments. Exact-job URLs and the updated discover/navigate/show workflow are verified; autonomous reuse by a newly updated installed Agent remains separate acceptance. Existing progress/alert evidence remains applicable. Windows native HTTP/package checks cover applicable monitor/Web/maintenance behavior. No current real Desktop/OpenCode mid-run continuation or Windows browser interaction is claimed. Completion continuation does not qualify mid-run alerts.

## Goal waiting protocol proof

**Observed host result:** the user confirms that local goal mode pauses automatically after approval. The supported UX asks one short question when permission is missing and reuses the actual reply for the bounded remaining jobs of that request/goal. Plugin text cannot grant permission; consent-free suspension remains unavailable.

**Isolated integration:** `scripts/check_codex_goal.py --wait-seconds 5` uses the actual Desktop-selected Codex executable with a simulated submitting Agent, endpoint and model. Scoped preflight requires confirmation before work; an explicitly simulated reply then permits admission. The fixture injects the owning runtime's native pause, observes its update event and requires zero model-request growth during confirmed pause, matching result execution and deleted/replaced-goal suppression. It does not prove real-model permission decisions or actual Desktop goal UI.

**Behavioral coverage:** Linux storage/CLI/MCP fixtures verify one answer for three separate original submissions, transactional limits/idempotent retries, scope/revision/expiry/deletion/revocation/restore fences, negotiated legacy forms, modern input-required responses and one Agent-question fallback. Retained-MCP tests cover current-image forwarding, a 91-second human wait and original profile/history preservation. Fake protocol clients are not real host form UI acceptance.

Direct MCP form UI, complete three-round real-model restoration, Windows goal/controller, changed-host goal recovery and long-duration savings remain unqualified. The host lacks generation/CAS and uses second-resolution timestamps, so identical same-second replacement and final verification/write/send races remain. No real chats/goals are created or changed by these fixtures. See [operation and limits](goal-waiting.md) and [backlog](TODO.md).

## External tasks (0.1.12+)

Linux→Windows native execution through the existing authenticated VM channel under SYSTEM verifies one launch, cwd/env/output, injected transport and local observer/daemon loss, stable local/remote IDs after reconnect and observed exact-run cancellation. This is not SSH-server, ordinary-user, machine-restart or real Desktop acceptance. Current Linux source observes a completed real GitHub Actions attempt read-only with matching conclusion, all five jobs, step counts and an attempt-specific URL; the embedded browser shows its details and opens that exact GitHub page on click. Long-name/current-step/partial-matrix cards pass 360/1280-pixel checks without horizontal overflow. In-progress/auth/attempt/detail-failure transitions use fixtures. External-task same-chat continuation and prolonged reliability remain unqualified.
