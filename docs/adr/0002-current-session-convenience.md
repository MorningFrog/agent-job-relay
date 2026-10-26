# ADR 0002: current-context wake and on-demand daemon

Accepted, 2026-10-26. Normal users should not copy thread IDs, endpoints and
registration keys for each task. Keep the explicit APIs for debugging and other
hosts; make an opted-in same-session start the normal Desktop path.

The CLI `start --wake` and MCP `job_start` with wake_me=true read only the calling
process's inherited CODEX_THREAD_ID and CODEX_APP_TOOLS_PIPE_PATH. No global
latest-session record, conversation scan or daemon-inherited identity is used.
The endpoint's owner and fingerprint are checked, registration uses that actual
fingerprint as host instance, the lease is refreshed to 24 hours and host tools
are probed before persisting an execution intent. Failure prevents execution;
there is no silent fallback. Explicit session and automatic wake are exclusive.

This is client behavior, shared in client.rs. MCP processes need per-session
inherited identity. Shared processes or hosts omitting context cannot be inferred;
the Agent can instead invoke the same one-command CLI from its own terminal tool.
The environment is trusted caller input, not cryptographic host attestation.

Submissions auto-start a detached native daemon if its socket is absent/refused.
A private startup lock coordinates concurrent callers; the normal daemon lock
still prevents duplicate owners. Permission/protocol/database errors are never
interpreted as absence. There is an eight-second readiness bound; failure directs
the user to foreground diagnostics. No login service or PATH change is made.
Host identity variables are removed from the newly launched daemon environment.
Doctor and ordinary queries do not auto-start state. `daemon stop` gracefully
stops dispatch/IPC while runners and data continue independently; next submission
starts the daemon again. Shutdown is not exposed as an MCP tool.

The underlying durable start/outbox protocol and accepted_by_host semantics are
unchanged. Endpoint replacement creates a new registration; old targets are not
automatically moved. Long jobs beyond the lease and explicit host replacement
still require refresh/rebind. Shortcuts have fake-host and real CLI/MCP process
tests. On 2026-10-26 the user also confirmed CLI live first start, registration
reuse and idle-daemon stop/auto-start, each continuing in the same chat without
another human message; the control chat stayed quiet. Private artifacts were
correlated read-only (live-e2e.md). In-flight interruption, Desktop restart/rebind
and real-host MCP wake_me=true remain separate validation work.
