# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Keep only current applicable acceptance and its limits here. Raw reports/private identifiers stay in ignored repository-local storage; historical runs remain in Git. Automated totals and candidate state belong in [implementation status](IMPLEMENTATION_STATUS.md).

## Linux ChatGPT Desktop

The user verified AgentJobRelay v0.1.8 local plugin/MCP submission and same-chat continuation after task completion. The user also confirms that current local goal mode pauses automatically after approval. That goal-pause observation does not independently qualify completion restoration, the new direct MCP form or a complete three-round workflow.

Use [published installation](installation.md#quick-install-the-latest-release) or [source installation](source-install.md) ([中文](source-install.zh-CN.md)), then [manual acceptance](manual-testing.md) for a relevant new change. Do not repeat an unchanged passed workflow merely to collect more evidence.

## Windows native runtime and MCP

The latest applicable Windows 11 MSVC external-task source baseline passes all four native gates under SYSTEM, with frontend/static-CRT build. It predates goal/schema v11. Earlier ordinary-user native acceptance remains applicable to the published monitor/Web/maintenance baseline, without qualifying newer source features.

Installed v0.1.8 with actual Codex 0.164.0 and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under SYSTEM and an ordinary interactive user: no prestarted daemon, one completed native Job/Run, expected output and doctor reporting `windows_cim`. Installed npm/package/loader checks use isolated registries/profiles and a simulated OpenCode model; they remain separate from real provider/host acceptance.

## Windows ChatGPT Desktop

The user verified v0.1.8 with official Desktop 26.1021.3705.0, bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and Full access. The actual chat binds, submits through MCP, ends its turn and automatically continues to inspect the matching result without another user message. A correctly reported failed workload still establishes result continuation, not workload success. Restricted-access single-command binding, current goal waiting and native confirmation UI remain unqualified.

## Distribution

npm/GitHub 0.1.11 is public and both npm latest tags resolve to it. Full read-only public-byte/install checker evidence remains at 0.1.10. Version 0.1.12 is prepared in source; its new native/package CI, sealed candidate and OIDC preview remain required. Published 0.1.11 candidate 40715744368 cannot qualify the changed payload. No official-directory submission occurred. See [release preparation](release.md).

## Runtime monitoring boundary

The bundled React/Vite panel has isolated Linux browser verification for live inspection, rules/application/alert evidence, progress and manual stopping. Windows native HTTP/package checks cover applicable monitor/Web/maintenance behavior. No current real Desktop/OpenCode mid-run continuation or Windows browser interaction is claimed. Completion continuation does not qualify mid-run alerts.

## Goal waiting protocol proof

**Observed host result:** the user confirms that local goal mode pauses automatically after approval. The supported UX asks one short question when permission is missing and reuses the actual reply for the bounded remaining jobs of that request/goal. Plugin text cannot grant permission; consent-free suspension remains unavailable.

**Isolated integration:** `scripts/check_codex_goal.py --wait-seconds 5` uses the actual Desktop-selected Codex executable with a simulated submitting Agent, endpoint and model. Scoped preflight requires confirmation before work; an explicitly simulated reply then permits admission. The fixture injects the owning runtime's native pause, observes its update event and requires zero model-request growth during confirmed pause, matching result execution and deleted/replaced-goal suppression. It does not prove real-model permission decisions or actual Desktop goal UI.

**Behavioral coverage:** Linux storage/CLI/MCP fixtures verify one answer for three separate original submissions, transactional limits/idempotent retries, scope/revision/expiry/deletion/revocation/restore fences, negotiated legacy forms, modern input-required responses and one Agent-question fallback. Retained-MCP tests cover current-image forwarding, a 91-second human wait and original profile/history preservation. Fake protocol clients are not real host form UI acceptance.

Direct MCP form UI, complete three-round real-model restoration, Windows schema v11/controller, changed-host goal recovery and long-duration savings remain unqualified. The host lacks generation/CAS and uses second-resolution timestamps, so identical same-second replacement and final verification/write/send races remain. No real chats/goals are created or changed by these fixtures. See [operation and limits](goal-waiting.md) and [backlog](TODO.md).

## External tasks (0.1.12 source)

Linux→Windows native execution through the existing authenticated VM channel under SYSTEM verifies one launch, cwd/env/output, injected transport and local observer/daemon loss, stable local/remote IDs after reconnect and observed exact-run cancellation. This is not SSH-server, ordinary-user, machine-restart or real Desktop acceptance. A completed real GitHub Actions attempt is observed read-only with matching conclusion/URL; status/auth/attempt transitions use fixtures. External-task same-chat continuation and prolonged reliability remain unqualified.
