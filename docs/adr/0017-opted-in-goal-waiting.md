# ADR 0017: Explicit goal waiting around Relay jobs

## Decision

Add an explicitly opted-in waiting group, separate from job execution, sessions and deliveries. Preparation comes from the actual Desktop terminal and pins its CLI image digest, Codex profile, logical chat, goal snapshot and original notification registration. Bindings carry only the opaque wait ID into shared MCP. Attach members in their admission transactions; freeze accepted membership before pausing. Preserve original Job/Run IDs and all execution idempotency semantics. Resume only after every original member is terminal and send one existing outbox delivery containing all member identities.

Use public Codex app-server initialize/get/status-only-set RPCs in bounded supervised processes. Never load/start/resume threads or send goal objectives/budgets. A separate controller can update persisted goals without model work, but requires the existing Desktop adapter to wake the owning runtime. No Agent or goal RPC polling occurs while jobs run; the native coordinator reads durable member state.

## Fencing and recovery

Per-group process locks and SQLite revisions serialize control phases without holding database transactions over host I/O. Recheck local revision/cancellation/binding immediately before transmission, after controller startup and its final goal read. Persist mutation intent before sending it; lost acknowledgements and interrupted mutation phases close with `attention` instead of replay. Recover committed unsealed admissions under daemon exclusivity. Deleted/observably changed goals, binding/profile/image changes, explicit cancellation and manual member stops suppress future continuation. Cancellation cannot recall an already transmitted host effect. Backup restoration cancels control intents and grouped notifications. Archival preserves group membership/audits and unchanged-key retries without execution replay.

Pin groups permanently to their original registration. Ordinary same-chat recovery skips those rows explicitly and continues unrelated recovery; host replacement requires manual goal handling. Reject running `notify_agent` monitor rules for group members, retain local rules/automatic stops, preserve delivery expiry/history and distinguish host acceptance from observed execution.

## Compatibility limit

The current goal API has no generation/CAS and only second-resolution timestamps. Snapshot checks detect observable replacement and pause changes but cannot guarantee fencing identical same-second replacement, updates masked by final usage or a change between verification and mutation/message transmission. Status-only writes cannot resurrect a deleted goal, but can affect a replacement in that remaining race. Keep the feature experimental and explicitly opted in; do not claim strong host fencing. A future conditional host operation is required for that guarantee.

## Verification

Behavioral tests cover batch partial admission, exact-run completion, stable retries, deletion/replacement/late deletion, lost acknowledgements, interrupted phases/admission, daemon loss with a live runner, cancellation/revisions, pinned recovery, archival and backup restoration. Actual source CLI/shared-MCP entry points use isolated fake hosts. The separate Linux check uses actual Codex and Relay with a simulated Desktop endpoint/model, verifying no model-request growth in paused windows and matching result execution. It does not qualify the actual Desktop goal UI, Windows controller or long-duration savings. See [testing](../testing.md) and [current evidence](../live-e2e.md#goal-waiting-protocol-proof).
