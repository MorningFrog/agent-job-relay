# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Live-verified baseline: **AgentJobRelay v0.1.8**, verified on **2026-10-27 (Asia/Shanghai)**. Published 0.1.10 has separate automated native/package/public-install checks. The 0.1.10 archive/backup/monitor/Web UI baseline has Linux/Windows native and candidate-package evidence; the 0.1.11 npm short CLI passes remote Linux/Windows native/package checks without new real-host continuation acceptance. Raw reports, logs and private identifiers stay in ignored repository-local storage; earlier results remain in Git history.

Unreleased batch submission has Linux native/CLI/MCP regressions and an actual OpenCode 1.19.4 loader/caller/session-loop check with a simulated model. These do not qualify real ChatGPT Desktop batch continuation, Windows batch execution or a real provider account; the live-verified baseline below remains unchanged.

## Linux ChatGPT Desktop

**v0.1.8 is verified with ChatGPT Desktop on Linux**, including the local plugin workflow and same-chat continuation after task completion. The user completed and confirmed this live acceptance.

See [latest published installation](installation.md#quick-install-the-latest-release) and [manual acceptance](manual-testing.md) for the installation and verification steps; [source installation](source-install.md) ([简体中文](source-install.zh-CN.md)) remains an alternative.

## Windows native runtime and MCP

The 0.1.11 Windows Actions run passes fmt, all-target Clippy, 59 native/portable tests and static-CRT release build. The separate 0.1.10 Windows 11 x64/Rust MSVC 1.99.1 VM baseline passes under SYSTEM and a non-elevated ordinary user; those identities were not rerun for 0.1.11. Coverage includes independent process lifetimes, monitoring/tree stop after daemon loss, Web HTTP auth/control, schema v9 archives/backups, private IPC, offline maintenance, automatic CIM startup and simulated ChatGPT Desktop recovery/renewal. Current test totals and candidate verification boundaries are maintained in [implementation status](IMPLEMENTATION_STATUS.md).

The separate v0.1.8 actual Codex 0.164.0 check with the installed offline plugin and `gpt-6.1-sol`/`low` passes cold-profile MCP submission under both SYSTEM and an ordinary interactive Windows user, without a prestarted daemon. Each test completes one Job/Run with exit 0 and the expected output; doctor reports `windows_cim`.

The local dual-platform 0.1.10 npm candidate passes Windows native execution, history after relocation, corrupt-payload refusal, embedded Web assets/authenticated inspection, installed CLI monitoring/stop/offline inspection and Codex 22-tool discovery under both SYSTEM and a non-elevated ordinary user. OpenCode 1.19.4 passes actual package loading, caller-context MCP and matching-session continuation with a simulated model; the control session stays untouched. These host-loader checks use a loopback registry for candidate bytes. Public 0.1.10 byte/install checks are recorded in [implementation status](IMPLEMENTATION_STATUS.md); they remain separate from real host/provider acceptance.

## Windows ChatGPT Desktop

The official ChatGPT Desktop **26.1021.3705.0** package is installed in the VM through winget and registered for the interactive Windows user. Its bundled Codex **0.164.0-alpha.11.1** successfully installs the local AgentJobRelay plugin and discovers **20 MCP tools** in an isolated profile.

**v0.1.8 is verified with ChatGPT Desktop on Windows**, using GPT-6 Luna / Low and user-selected Full access. The signed-in user's chat binds successfully, submits one real task through MCP `job_start`, ends its turn, and automatically continues in the same chat to read the matching result without another human message or CLI fallback. The user confirmed this acceptance.

The live task correctly reported a test failure caused by inherited `AJR_STATE_DIR` in the installation fixture. The fixture now isolates ambient runtime/host context; the corrected native gates are verified separately. The original live job remains failed and is not replayed. Restricted terminal access can refuse binding with error 5; single-command authorization in that mode remains unverified.

## Distribution

Both npm 0.1.10 packages and the matching formal GitHub Release are public. The manual Release All pipeline and read-only public-byte/install checker passed; both packages expose npm provenance. These distribution checks are separate from real-host continuation acceptance. The original 0.1.11 baseline passes remote native/package CI, including the Windows npm short-command shim, and has a sealed candidate awaiting maintainer publication. Unreleased batch changes require a future version and qualified candidate. No official-directory submission occurred. See [implementation status](IMPLEMENTATION_STATUS.md) and [release preparation](release.md).

## Runtime monitoring boundary

The 0.1.10 React/Vite panel has isolated local browser verification for live task inspection, rule editing/application, alert evidence and manual stopping. This is local UI/runtime evidence only. No real ChatGPT Desktop/OpenCode running-alert continuation was performed for these changes; the previously verified task-completion baseline remains unchanged. Windows monitor/Web UI/schema v9 native and installed-package checks pass; Windows browser interaction and real mid-run host continuation remain separate qualifications.
