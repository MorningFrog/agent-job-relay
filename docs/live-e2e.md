# Current live verification

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Manual acceptance](manual-testing.md)

Live-verified baseline: **AgentJobRelay v0.1.8**, verified on **2026-10-27 (Asia/Shanghai)**. Published 0.1.10 has separate automated native/package/public-install checks. The 0.1.10 archive/backup/monitor/Web UI baseline has Linux/Windows native and candidate-package evidence; the 0.1.11 npm short CLI passes remote Linux/Windows native/package checks without new real-host continuation acceptance. Raw reports, logs and private identifiers stay in ignored repository-local storage; earlier results remain in Git history.

Unreleased batch submission has Linux native/CLI/MCP regressions and an actual OpenCode 1.19.4 loader/caller/session-loop check with a simulated model. These do not qualify real ChatGPT Desktop batch continuation, Windows batch execution or a real provider account; the live-verified baseline below remains unchanged.

Unreleased mid-run feedback has Linux runner/checker, actual CLI/MCP/HTTP and fake-Desktop routing/persistence coverage. It adds no real ChatGPT Desktop/OpenCode mid-run execution or Windows native acceptance; published completion-continuation evidence below does not qualify this new path. See [progress limits](progress.md).

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

Both npm 0.1.10 packages and the matching formal GitHub Release are public. The manual Release All pipeline and read-only public-byte/install checker passed; both packages expose npm provenance. These distribution checks are separate from real-host continuation acceptance. The original 0.1.11 baseline passes remote native/package CI, including the Windows npm short-command shim, and has a sealed candidate awaiting maintainer publication. Unreleased batch, external-task and goal-waiting changes require a future version and qualified candidate. No official-directory submission occurred. See [implementation status](IMPLEMENTATION_STATUS.md) and [release preparation](release.md).

## Runtime monitoring boundary

The 0.1.10 React/Vite panel has isolated local browser verification for live task inspection, rule editing/application, alert evidence and manual stopping. This is local UI/runtime evidence only. No real ChatGPT Desktop/OpenCode running-alert continuation was performed for these changes; the previously verified task-completion baseline remains unchanged. Windows monitor/Web UI/schema v9 native and installed-package checks pass; Windows browser interaction and real mid-run host continuation remain separate qualifications.

## Goal waiting protocol proof

The current **integrated source implementation** passes the isolated Linux check `scripts/check_codex_goal.py` with Desktop-bundled Codex 0.164.0-alpha.11.1, real Relay jobs and public app-server/queue interfaces against an explicit loopback owner. Three cases first observe repeated active-goal control turns, then cover successful resume, goal deletion and replacement while work runs. The product controller performs pause/restoration; the simulated notification endpoint only forwards the actual merged notification through `codex queue`. No bridge script manually changes goal state on Relay's behalf. After pause acknowledgement and in-flight turn completion, each two-second waiting window adds zero simulated-model requests while the real job remains running. Normal completion executes the matching result turn; deletion leaves no goal and sends no notification, while replacement preserves the unrelated paused goal and sends no notification. All three jobs complete successfully.

The separate controller uses the same disposable Codex profile without loading a thread or invoking a model. Behavioral fixtures additionally cover accounting preservation, deletion between recovery read and activation, lost mutation acknowledgements, cancellation, daemon/admission recovery, pinned-target recovery and archival/restore. A separate actual Codex loader check discovers 24 source tools from the bootstrapped source plugin without creating daemon/job state.

The Desktop endpoint and model are simulated. This establishes source integration with the actual Codex runtime, **not actual Desktop goal UI/Windows acceptance**, receiver deduplication, long-duration savings or complete replacement-goal fencing. A read-only query also found the actual Desktop chat under the same persisted thread/session identity, but the querying process reported it as not loaded; shared storage does not establish turn ownership. No real goal, configuration or existing daemon was changed. Current RPCs have no generation/CAS and use second-resolution timestamps, leaving identical same-second changes and final verification/write/send races. See [opt-in and operational limits](goal-waiting.md) and [remaining qualification](TODO.md).

## External tasks (unreleased source)

Linux→Windows 11 cross-host verification through the existing authenticated QEMU guest-agent channel passes under SYSTEM: one native launch, expected cwd/env/output, injected transport loss and local daemon/observer loss, original local/remote run IDs after reconnect and observed exact-run cancellation. This is actual remote execution with an injected channel failure, not SSH-server, ordinary-user, machine-restart or ChatGPT Desktop acceptance. Source MCP/CLI tests separately verify milestone/completion inbox routing after MCP exit. A real completed GitHub Actions attempt is observed read-only with matching final conclusion and URL; simulated transitions/auth/attempt failures remain separate evidence. No external-task same-chat live continuation or prolonged reliability claim is added. Current native gates/counts are in [implementation status](IMPLEMENTATION_STATUS.md).
