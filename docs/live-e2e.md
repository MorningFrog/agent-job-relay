# Current live verification

Latest applicable version: **0.1.4**, reviewed on **2026-10-26 (Asia/Shanghai)**. This summary distinguishes user observations from persisted-record checks; it does not provide an automatic `confirmed_resumed` capability. Raw reports, IDs and logs remain in ignored local storage. Earlier runs are available in Git history.

## Installed MCP workflow

The user confirmed that a real background task was submitted through MCP, the submitting turn ended, and the completion notification resumed the same chat to inspect the result, without CLI fallback. Bounded read-only MCP queries matched the supplied job, successful exit, completed event and one `accepted_by_host` delivery. The current runtime check reported 0.1.4, Linux, explicit current-terminal binding and matching installed/active state.

This qualifies the installed MCP submission/continuation workflow for that user-confirmed run. The originating tool calls and actual UI continuation were not independently observed. Runtime delivery remains `accepted_by_host`; user-attested continuation is separate evidence. No workload replay, delivery retry or host message was used during review.

## Desktop UI/restart acceptance

The requested acceptance is in progress against the unchanged installed **0.1.4** runtime; source **0.1.5** has separate isolated loader/recovery evidence. Available computer tools expose browsers but no native Desktop window, so plugin-browser installation, app version and normal restart require operator observation. No personal installation/configuration, daemon or other plugin has been replaced or stopped for this check.

A developer-only bounded FIFO probe stages an existing MCP job for same-chat recovery after reopening Desktop, then explicit release and completion-triggered continuation. The isolated FIFO handshake passed. Real old/new host identity comparison, recovered target correlation, UI/restart and continuation remain pending; private probe artifacts/IDs stay in ignored repository-local storage. Follow [manual acceptance](manual-testing.md#desktop-restart-and-same-chat-rebinding). This check does not establish a 0.1.5 live upgrade or a result completed while the host was closed.

## Remaining scope

- The current Desktop version, OS distribution, plugin-browser installation and native/cache upgrade integrity were not independently rechecked.
- Real Desktop restart/rebind, multi-day renewal, receiver deduplication and additional host/platform versions remain unqualified.
- Keep passed runs complete; collect new evidence only for a relevant change or suitable use case, without repeating equivalent workloads or requiring a manual crash matrix.

See [manual acceptance](manual-testing.md) for the procedure and [implementation status](IMPLEMENTATION_STATUS.md) for current automated results and operating limits.
