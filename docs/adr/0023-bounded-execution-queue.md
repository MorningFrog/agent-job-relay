# ADR 0023: Opt-in bounded execution queue

## Decision

Keep one daemon/profile and the existing `queued → launching → running` vocabulary. An immutable optional `queue_timeout_seconds` permits admission beyond the default eight unfinished jobs, up to eight execution slots plus 64 additional waiting records. Do not add priorities, dependencies, automatic workload retries or configurable concurrency.

Reserve a slot in one IMMEDIATE transaction: reread the original job, expire its first-launch deadline if necessary, verify fewer than eight launching/running records and no older queued record, then persist launching and its existing nonce. The daemon's full-queue snapshot skips no-op write transactions; the claim always rechecks authoritative state. Submission/cancellation/claim races therefore remain serialized, and original sequence order governs slot reservation rather than process-output order.

Persist the option in the existing specification and derive the absolute deadline from original creation time. Omit absent values to preserve historical idempotency bytes. Expose bounded queue identity/deadline/position through shared results; positions are snapshots. Queue expiry is a terminal timeout/event/Outbox transaction without launching a runner. Execution timeout and monitoring clocks remain independent. Before first launch, opted-in remote attachment cancellation stays local; started observers reconnect the original run without applying the first-launch deadline again.

Use schema v13 as a durable semantic fence despite needing no new table. An older daemon's launch loop cannot safely own an expanded queue. Existing exclusive migration/old-run fences apply, and old executables refuse the newer profile. Development metadata advances to an unpublished version; published candidates stay immutable.

## Recovery and authorization

Restart continues only never-launched queued intents with original IDs, order and deadlines. Launch uncertainty retains orphan classification without native replay. Backup restore orphans all unfinished records, including queued work. The daemon must run to schedule/expire; clock changes affect wall-clock deadlines. Existing runner/daemon/host lifetimes remain independent.

Goal budget consumption, membership, rules and session targets remain in the admission transaction. Queueing neither bypasses pending consent nor authorizes new Goal controls. Every admitted member, including a queued timeout, participates in the original all-terminal completion decision. Cancellation and host-control closure retain existing semantics.

## Regression boundaries

Cover concurrent capacity/duplicate claims, FIFO and slot release, unchanged-key deadlines at full capacity, queued expiry/cancellation without execution, original identities through daemon restart, archived retry/restore non-replay, external observer/control boundaries and Goal membership/accounting. Check actual CLI/shared MCP results and Web rendering separately from real installed-host continuation. Native Windows and installed-host queue acceptance require their own qualification.
