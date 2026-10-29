# ADR 0017: Native goal handoff around Relay jobs

## Decision

Use a waiting group required by default for an active Desktop goal, separate from job execution, sessions and deliveries. Ordinary terminal binding detects the active goal automatically; preparation comes from the actual Desktop terminal and pins its CLI image digest, Codex profile, logical chat, goal snapshot and original notification registration. Bindings carry only the opaque wait ID into shared MCP. Attach members in their admission transactions; freeze accepted membership before pausing. Preserve original Job/Run IDs and all execution idempotency semantics. Resume only after every original member is terminal and send one existing outbox delivery containing all member identities.

After admission, return `sealed` with an explicit native-host pause action. The originating Agent uses its own `update_goal(status="paused")`, then confirms success through revision-fenced `job_goal_wait(confirm_paused=true)`. Verify the original paused goal/binding before entering `waiting`. A separate app-server's storage-only pause does not notify Desktop's in-memory goal state and caused continued checks; it must never acknowledge waiting. Confirmation is a cooperative caller assertion, not independent host attestation. Missing confirmation expires after five minutes without changing goals or replaying work; earlier storage-only waits close with `attention`.

Use public Codex app-server initialize/get/status-only-set RPCs in bounded supervised processes for reads and restoration only. Never load/start/resume threads or send goal objectives/budgets. The existing Desktop adapter wakes the owning runtime on completion. No Agent or goal RPC polling occurs while jobs run; the native coordinator reads durable member state.

The current product contract replaces the original opt-in workflow: no special pause phrase or repeated permission question is required. Inactive/deleted goals are not adopted, and user interruption/cancellation remains effective.

## Fencing and recovery

Per-group process locks and SQLite revisions serialize control phases without holding database transactions over host I/O. Recheck local revision/cancellation/binding immediately before transmission, after controller startup and its final goal read. Persist mutation intent before sending it; lost acknowledgements and interrupted mutation phases close with `attention` instead of replay. Recover committed unsealed admissions under daemon exclusivity. Deleted/observably changed goals, binding/profile/image changes, explicit cancellation and manual member stops suppress future continuation. Cancellation cannot recall an already transmitted host effect. Backup restoration cancels control intents and grouped notifications. Archival preserves group membership/audits and unchanged-key retries without execution replay.

Pin groups permanently to their original registration. Ordinary same-chat recovery skips those rows explicitly and continues unrelated recovery; host replacement requires manual goal handling. Reject running `notify_agent` monitor rules for group members, retain local rules/automatic stops, preserve delivery expiry/history and distinguish host acceptance from observed execution.

## Compatibility limit

The current goal API has no generation/CAS and only second-resolution timestamps. Snapshot checks detect observable replacement and pause changes but cannot guarantee fencing identical same-second replacement, updates masked by final usage or a change between verification and mutation/message transmission. Status-only writes cannot resurrect a deleted goal, but can affect a replacement in that remaining race. Keep the host compatibility limits experimental while treating pause/restoration as the required requested Relay scheduling; do not claim strong host fencing. A future conditional host operation is required for that guarantee.

## Verification

Behavioral tests cover native-pause confirmation, stale/missing/repeated acknowledgement, storage-only pause refusal, confirmation expiry/legacy fences, batch partial admission, exact-run completion, stable retries, deletion/replacement/late deletion, lost restore acknowledgements, interrupted phases/admission, daemon loss with a live runner, cancellation/revisions, pinned recovery, archival and backup restoration. Actual source CLI/shared-MCP entry points use isolated fake hosts. The separate Linux check uses actual Codex and Relay with a simulated submitting Agent, Desktop endpoint and model; it requires the owning runtime's pause event before confirmation, zero model-request growth in paused windows and matching result execution. It does not qualify the actual Desktop goal UI, Windows controller or long-duration savings. See [testing](../testing.md) and [current evidence](../live-e2e.md#goal-waiting-protocol-proof).
