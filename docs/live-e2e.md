# Current live verification

Latest applicable version: **0.1.5**, reviewed on **2026-10-26 (Asia/Shanghai)**. This summary distinguishes user observations from persisted-record checks; it does not provide an automatic `confirmed_resumed` capability. Raw reports, IDs and logs remain in ignored local storage. Earlier runs are available in Git history.

## Installed MCP workflow

The user confirmed that a real background task was submitted through MCP, the submitting turn ended, and the completion notification resumed the same chat to inspect the result, without CLI fallback. Bounded read-only MCP queries matched the supplied job, successful exit, completed event and one `accepted_by_host` delivery. The current runtime check reported 0.1.4, Linux, explicit current-terminal binding and matching installed/active state.

This qualifies the installed MCP submission/continuation workflow for that user-confirmed run. The originating tool calls and actual UI continuation were not independently observed. Runtime delivery remains `accepted_by_host`; user-attested continuation is separate evidence. No workload replay, delivery retry or host message was used during review.

## Desktop UI/restart acceptance

The user reports **Desktop 26.1021.22027** and AgentJobRelay **0.1.5** visibly enabled in the Plugins interface. This qualifies UI enablement for that user-attested environment, not an independently observed clean installation. Read-only checks confirm installed native version 0.1.5, matching recorded/active state and a responding real MCP call. The daemon was stopped before this retry; submission starts it normally. Native/cache hashes and upgrade preservation were not independently audited.

A new coordinated restart/rebind attempt is in progress at the user's request. The bounded FIFO command is submitted through installed MCP with a fresh current-chat binding; after normal Desktop restart, compare host identities, recover the retained target, release that existing command and observe its same-chat result. The prior expired job is retained without replay. Private artifacts/IDs remain ignored and local. Real restart, changed bridge identity, recovered target correlation and completion-triggered continuation remain pending. See [manual acceptance](manual-testing.md#desktop-restart-and-same-chat-rebinding). This active-job check does not establish a result completed while the host was closed or receiver deduplication.

## Remaining scope

- Desktop version and plugin enablement are user-attested; OS distribution, clean UI installation and native/cache upgrade integrity remain independently unverified.
- Real Desktop restart/rebind, multi-day renewal, receiver deduplication and additional host/platform versions remain unqualified.
- Keep passed runs complete; collect new evidence only for a relevant change or suitable use case, without repeating equivalent workloads or requiring a manual crash matrix.

See [manual acceptance](manual-testing.md) for the procedure and [implementation status](IMPLEMENTATION_STATUS.md) for current automated results and operating limits.
