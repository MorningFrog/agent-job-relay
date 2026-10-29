# ADR 0017: Native goal handoff around Relay jobs

## Decision

Use a waiting group separate from execution, sessions and deliveries. Scoped consent is prepared from the actual Desktop terminal and pins the controller image/profile, logical chat, goal and notification registration; [ADR 0020](0020-scoped-goal-consent.md) governs the one-question admission flow. Approved reconnection returns an opaque wait ID for shared MCP. Attach members in their admission transactions, then freeze accepted membership. Preserve original Job/Run IDs and execution idempotency. Restore only after every original member is terminal, and send one existing outbox delivery containing all member identities.

After admission, return `sealed` with an explicit native-host pause action. Only under an independent user/system pause request, the originating Agent uses its own `update_goal(status="paused")`, then confirms success through revision-fenced `job_goal_wait(confirm_paused=true)`. Verify the original paused goal/binding before entering `waiting`. A separate app-server's storage-only pause does not notify Desktop's in-memory goal state and caused continued checks; it must never acknowledge waiting. Confirmation is a cooperative caller assertion, not independent host attestation. Missing confirmation expires after five minutes without changing goals or replaying work; earlier storage-only waits close with `attention`.

Use public Codex app-server initialize/get/status-only-set RPCs in bounded supervised processes for reads and restoration only. Never load/start/resume threads or send goal objectives/budgets. The existing Desktop adapter wakes the owning runtime on completion. No Agent or goal RPC polling occurs while jobs run; the native coordinator reads durable member state.

Native pause requires actual user consent, so use one short confirmation when absent and reuse an affirmative answer across this request’s remaining rounds on the same goal. Prefer the host’s suitable native question interface; preselected options and plugin-generated messages cannot authorize work. Do not require exact wording, repeat questions per round, mark blocked/completed or poll. Consent-free pause remains unsupported. Expose `automatic_goal_pause_supported=false` separately from the supported explicit native-confirmation protocol. Inactive/deleted goals are not adopted and interruptions/cancellation remain effective.

## Fencing and recovery

Per-group process locks and SQLite revisions serialize control phases without holding database transactions over host I/O. Recheck local revision/cancellation/binding immediately before transmission, after controller startup and its final goal read. Persist mutation intent before sending it; lost acknowledgements and interrupted mutation phases close with `attention` instead of replay. Recover committed unsealed admissions under daemon exclusivity. Deleted/observably changed goals, binding/profile/image changes, explicit cancellation and manual member stops suppress future continuation. Cancellation cannot recall an already transmitted host effect. Backup restoration cancels control intents and grouped notifications. Archival preserves group membership/audits and unchanged-key retries without execution replay.

Pin groups permanently to their original registration. Ordinary same-chat recovery skips those rows explicitly and continues unrelated recovery; host replacement requires manual goal handling. Reject running `notify_agent` monitor rules for group members, retain local rules/automatic stops, preserve delivery expiry/history and distinguish host acceptance from observed execution.

## Compatibility limit

The current goal API has no generation/CAS and only second-resolution timestamps. Snapshot checks detect observable replacement and pause changes but cannot guarantee fencing identical same-second replacement, updates masked by final usage or a change between verification and mutation/message transmission. Status-only writes cannot resurrect a deleted goal, but can affect a replacement in that remaining race. Keep the explicit protocol experimental and the unavailable automatic scheduling requirement visible; do not claim strong host fencing. A future conditional host operation is required for that guarantee.

## Verification

Isolated native tests cover consent/pre-admission gating, native acknowledgement/retries/revisions, storage-only refusal, expiry/legacy fences, batch membership, exact-run completion, deletion/replacement/accounting, uncertain restoration, interruption/recovery, cancellation and archival/restore. Actual Codex integration uses a simulated submitting Agent/endpoint/model. The user separately confirms local goal pause after approval; direct native form UI, full multi-round restoration and Windows controller remain separate. See [testing](../testing.md) and [current evidence](../live-e2e.md#goal-waiting-protocol-proof).
