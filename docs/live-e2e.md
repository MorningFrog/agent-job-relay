# Current live verification

Latest installed Linux Desktop evidence: **0.1.5**, reviewed on **2026-10-26 (Asia/Shanghai)**, with user-reported **Desktop 26.1021.22027**. UI observations, measured state and observed Agent execution remain separate evidence levels; no adapter automatically records `confirmed_resumed`. Raw IDs, endpoints, reports and logs stay in ignored repository-local storage. Earlier results remain in Git history.

## Installed MCP workflow

The coordinated restart probe was submitted through installed MCP with a fresh current-terminal binding, without CLI submission fallback. The Agent ended its turn, the user reported a normal Desktop restart, and the existing command remained running. After recovery and explicit release, the Agent ended its turn again; the completion notification triggered a new same-chat execution without another human continuation message. Bounded MCP status/events/logs/delivery queries correlated successful exit code 0, the original Job/Run with attempt=1, one completed event and one `accepted_by_host` delivery/attempt to the recovered target.

This verifies the installed MCP active-job restart/rebind/continuation workflow for this environment. Runtime delivery remains `accepted_by_host`; the resulting Agent execution is separate observed evidence. No workload replay, delivery retry, state-profile switch or daemon stop was used.

## Desktop UI and restart evidence

The user reports plugin **0.1.5** visibly enabled in Desktop **26.1021.22027** and confirms the normal restart. Native Desktop UI is unavailable to current computer tools, so clean UI installation is not independently observed. Read-only checks establish installed native version 0.1.5, matching recorded/active state and actual MCP connectivity; native/cache hashes and upgrade history preservation were not freshly audited.

Saved before/after bindings show a changed bridge instance, endpoint fingerprint and session key, while logical chat, caller and host label stay identical. Recovery preview selected only the retained active job, with no in-flight claim. MCP apply created an immutable replacement target and recovery audit; the following page was empty. The original Job/Run remained unchanged, then its completed event was accepted by the new target and triggered this chat. This qualifies real changed-endpoint active-job recovery, beyond fake-host transport tests. The FIFO probe itself deliberately does not attest to UI actions.

## Remaining scope

- OS distribution, clean UI installation, native/cache upgrade integrity and additional host/platform versions remain independently unverified.
- Results completed while Desktop is closed, real multi-day renewal and receiver deduplication remain unqualified; this was an active-job recovery before completion.
- Keep this passed scenario complete. Do not repeat equivalent workloads or require a manual crash matrix merely to refill evidence.

See [manual acceptance](manual-testing.md#desktop-restart-and-same-chat-rebinding) for the procedure and [implementation status](IMPLEMENTATION_STATUS.md) for automated results and operating limits.

The source 0.1.7 OpenCode actual-host/simulated-model checks and Claude synthetic scaffold checks are separate evidence in [host plugins](host-plugins.md). They do not replace this Desktop 0.1.5 live result or establish a real provider/account session.

## Current package and Windows qualification

Source 0.1.7 dual-platform candidates pass real npm installation/native execution, retained history after relocation and corrupt-payload refusal on Linux and Windows. OpenCode 1.19.4 loads the exact package name/version through a loopback registry, submits through actual caller-context MCP and persists a matching-session agent-loop response with a local simulated model; the control session remains untouched. Codex 0.164.0 installs the original self-hosted catalog and discovers 20 tools on both OSes without a model turn or Desktop contact. These are package/interface checks, not public npm availability or clean Desktop UI acceptance.

Windows 11 x64/Rust MSVC 1.99.1 passes fmt, all-target Clippy, 30 tests (15 native plus 15 portable) and static-CRT release build. Native execution, tree cancellation/timeout, runner/daemon independence, cold capture-handle isolation, private IPC/Node-bridge ACLs, inbox, simulated Desktop recovery/renewal and offline maintenance are covered. Explicit outside-host `daemon start` starts/reuses the selected profile without creating a job/session. Actual Codex 0.164.0 MCP has a non-breakaway cleanup job, so cold auto-start refuses; model acceptance must state this prerequisite.

The updated native Windows CI and dual-platform release workflow await a remote run. Earlier [remote CI](https://github.com/MorningFrog/agent-job-relay/actions/runs/40356375386) covered portable Windows/macOS compilation. Release remains manual-only and untriggered; no npm publication, GitHub release or official submission occurred.

A separate Windows live model check passed through Codex-Manager using `gpt-6.1-sol`/`low`: actual installed MCP performed exactly one `job_start`, one bounded `job_wait`, `job_status` and `job_logs`; the original native Job/Run completed with exit 0 and the expected fixed log marker. The daemon was explicitly started for the installed profile outside Codex first. Gateway records confirm the requested model/effort. The isolated profile uses the existing DPAPI-protected gateway key only in the child process environment, without ChatGPT sign-in or copied account tokens. Execution was through guest-agent SYSTEM; the VM had no logged-in interactive user. Ordinary-user and Windows Desktop continuation remain unverified. The VM's current gateway address was substituted only in this disposable test profile; the previous user connection was not modified.
