# Current live verification

Current version: **AgentJobRelay v0.1.8**, verified on **2026-10-27 (Asia/Shanghai)**. Raw reports, logs and private identifiers stay in ignored repository-local storage; earlier results remain in Git history.

## Linux ChatGPT Desktop

**v0.1.8 is verified with ChatGPT Desktop on Linux**, including the local plugin workflow and same-chat continuation after task completion. The user completed and confirmed this live acceptance.

See [source installation](source-install.md) ([简体中文](source-install.zh-CN.md)) and [manual acceptance](manual-testing.md) for the installation and verification steps.

## Windows native runtime and MCP

Windows 11 x64/Rust MSVC 1.99.1 passes all four delivery gates: fmt, all-target Clippy, **35 tests** (20 Windows-specific plus 15 portable), and static-CRT release build. Coverage includes independent process lifetimes, cancellation/timeouts, private IPC, offline maintenance, automatic CIM startup and simulated ChatGPT Desktop recovery/renewal.

Actual Codex 0.164.0 with the installed offline plugin and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under both SYSTEM and an ordinary interactive Windows user, without a prestarted daemon. Each test completes one Job/Run with exit 0 and the expected output; doctor reports `windows_cim`.

Windows npm-package checks pass native execution, history after relocation, corrupt-payload refusal and Codex 20-tool discovery. OpenCode 1.19.4 passes actual package loading, caller-context MCP and matching-session continuation with a simulated model; the control session stays untouched. These checks use a loopback registry for unpublished candidates.

## Windows ChatGPT Desktop

The official ChatGPT Desktop **26.1021.3705.0** package is installed in the VM through winget and registered for the interactive Windows user. Its bundled Codex **0.164.0-alpha.11.1** successfully installs the local AgentJobRelay plugin and discovers **20 MCP tools** in an isolated profile.

**v0.1.8 is verified with ChatGPT Desktop on Windows**, using GPT-6 Luna / Low and user-selected Full access. The signed-in user's chat binds successfully, submits one real task through MCP `job_start`, ends its turn, and automatically continues in the same chat to read the matching result without another human message or CLI fallback. The user confirmed this acceptance.

The live task correctly reported a test failure caused by inherited `AJR_STATE_DIR` in the installation fixture. The fixture now isolates ambient runtime/host context; the corrected native gates are verified separately. The original live job remains failed and is not replayed. Restricted terminal access can refuse binding with error 5; single-command authorization in that mode remains unverified.

## Distribution

No npm publication, GitHub release or official submission occurred. Updated remote native Windows CI and the manual dual-platform release workflow remain to be run. See [implementation status](IMPLEMENTATION_STATUS.md) and [release preparation](release.md).
