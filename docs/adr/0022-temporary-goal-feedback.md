# ADR 0022: Temporary goal restoration for intermediate feedback

## Decision

Allow requested notifying monitor rules on a goal-wait member. Keep execution, original request budget, frozen Job/Run membership and final notification state unchanged. Source 0.1.17/schema v12 adds current feedback event/delivery IDs, cycle count and last-restoration time to the existing wait. Reuse `waiting → resuming → resumed → waiting`; a non-null feedback identity distinguishes temporary restoration from final completion. Existing wait audits and ordinary event/delivery histories retain earlier cycle evidence.

Select one original-run alert addressed to the original active target, coalesce older unsent alerts while retaining their IDs/history, persist intent before status-only goal restoration and send metadata identifying feedback rather than all-member completion. Intermediate acceptance never sets final `notified`. At most 32 temporary restorations are permitted per wait, at least 60 seconds apart. Rule cooldowns and existing job budgets remain independent. No Agent polling, new admission or per-cycle consent question is introduced; the user must have requested feedback and authorized the original pause/restoration workflow.

After reporting, the originating Agent must invoke its native goal pause and confirm the current wait revision through the existing `confirm_paused` operation. Only that acknowledgement returns the same wait to `waiting` with a fresh paused snapshot; an external controller never performs the pause. The five-minute deadline is fixed, not renewed by retries. Lost restore acknowledgement, interrupted `resuming`, unconfirmed re-pause or explicit cancellation closes automatic control while workloads remain independent. Never replay an uncertain native mutation.

All-member completion wins before intermediate transmission: after verifying the active original goal, the dispatcher can fence its own untransmitted claim, retain attempt evidence and admit the single merged final notification without another restoration. Once a feedback effect may have been transmitted, finish the native re-pause handshake first, then perform the ordinary final restoration. An accepted transport effect is not actual Agent execution. Live claims retain existing per-chat exclusion; a late intermediate ACK cannot consume final notification state or reopen an earlier cycle.

## Recovery and compatibility

Schema v12 migration requires the selected old daemon to stop and old launching/running jobs to finish. Restore accepts supported earlier backups, migrates forward and closes restored goal controls without replay. Current selection fields remain durable across daemon loss; original delivery IDs, expiry, attempts and audits are preserved. Goal deletion/replacement, original-host changes, consent revocation and cancellation retain existing fences. Closed waits permit local monitoring only. No automatic rebinding, objective/budget replacement or new execution is added.

The host still lacks goal generation/CAS. Identical same-second replacement, accounting-masked changes and final verification/write/send races remain limits. Capability `goal_wait_feedback=true` describes the runtime protocol, not actual Desktop UI or model acceptance. Actual Codex with a simulated endpoint/model qualifies the owning pause event, temporary feedback turn, re-pause and idle behavior separately from real-model Desktop and Windows acceptance.

## Verification

Reuse native fake-host/controller tests for repeated cycles, coalescing/cooldown/budget, stale confirmations, late ACKs, completion before transmission and during reporting, timeout/cancellation/deletion, uncertain control non-replay and backup/restore. Keep schema migration/history coverage and the isolated actual-Codex fixture. See [current evidence](../IMPLEMENTATION_STATUS.md) and [goal workflow](../goal-waiting.md).
