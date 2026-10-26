# Current live verification

Latest applicable version: **0.1.5**, reviewed on **2026-10-26 (Asia/Shanghai)**, with user-reported **Desktop 26.1021.22027**. UI observations, measured state and observed Agent execution remain separate evidence levels; no adapter automatically records `confirmed_resumed`. Raw IDs, endpoints, reports and logs stay in ignored repository-local storage. Earlier results remain in Git history.

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
