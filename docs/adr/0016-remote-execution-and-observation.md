# ADR 0016: Pinned external execution and recoverable observation

Status: accepted.

## Context

A local SSH or `gh` child does not own the remote computation. Losing that child, its output or its launch acknowledgement cannot establish whether training/workflow execution started, completed or stopped. Remote support must preserve the initiating local Job/Run/session, retain bounded evidence and separate execution ownership from observation. A mandatory training framework or distributed scheduler would exceed the requested scope.

## Decision

Add remote and GitHub Actions command variants to the existing single Rust runtime. Use native remote AgentJobRelay as the reference durable owner, requiring only a copied executable/private profile. SSH or an explicitly configured authenticated transport invokes a one-request stdio helper. A small enforced contract permits future scheduler/Agent-authored adapters without treating arbitrary SSH exit as durable remote support. Native launch/attach/sample/stop advertise explicit capabilities; unsupported or malformed operations fail without inferring a workload result. Workflow observation is an independent read-only `gh api` adapter pinned to repository/run/attempt.

Provision a stable UUID for each remote profile outside backup-restored runtime state. Check protocol/target/request and exact remote Job/Run/attempt on responses. Reuse the immutable origin Job/Run as the remote launch key, preserving the native atomic admission and cross-archive idempotency rules. If cancellation arrives after an uncertain launch, only look up that key before stopping; never create another launch. Missing evidence stays unresolved rather than fabricating cancellation. Remote credentials remain in existing authentication facilities and workload data travels on stdin, never interpolated into the transport's shell command.

The independent local runner persists mapping, event cursor, connection freshness and bounded samples through existing runtime observations. Persist raw output before advancing its effective cursor, using file length as that cursor; retain native log caps and byte identity. Only external observer intents may be reclaimed under the existing per-run lock after process loss. Their remote execution identity and key survive; ordinary native jobs are never replayed. Restore keeps the existing orphan classification and cannot automatically relaunch or impersonate an old remote profile.

Default attachment is read-only. An explicit control attachment or owned launch can request graceful/forced stop of only its immutable remote run, and remains active until terminal evidence or an observation deadline. A remote natural completion wins a racing cancellation. Workflow/read-only stop cancels observation only. Observation timeout is orphaned with unknown external outcome, separate from the execution owner's timeout. Network silence cannot safely drive workload-silence rules; those belong in the remote runner, while local output-prefix/elapsed rules can notify the initiating session through the existing outbox. No checkpoint transfer, training restart, workflow dispatch/cancellation, public API or framework coupling is added.

## Consequences and regression requirements

Requests/responses/diagnostics and subprocess time are bounded; failures back off without Agent polling. Raw transport stderr is discarded and API failures retain fixed diagnostic categories. Custom adapters are trusted executable integrations that must independently qualify identity, durable deduplication, ownership, recovery and stop semantics. SSH authentication and shell deployments need separate native qualification from the VM transport used for cross-host verification.

Behavioral coverage must exercise lost launch acknowledgement and unchanged-key retry, cancellation lookup without relaunch, target/run/attempt refusal, observer death/reconnect with byte-identical output, read-only versus controlled stop, independent remote lifetime, and origin-session routing after MCP exit. Workflow tests must preserve final conclusion/URL, refuse a different attempt and distinguish auth/rate-limit/transport/observation timeout from workflow failure. Native Windows/cross-host evidence, simulations, real workflow reads and real same-chat continuation remain separate qualifications in the current status documents.
