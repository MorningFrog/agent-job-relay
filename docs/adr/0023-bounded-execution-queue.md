# ADR 0023: Default bounded execution queue

## Decision

Keep one daemon/profile and the existing `queued → launching → running` vocabulary. New requests default to FIFO waiting, with eight execution slots and 64 additional waiting records. The maximum wait defaults to 24 hours; `queue_timeout_seconds` customizes it up to seven days, or zero rejects when full. Do not add priorities, dependencies, automatic workload retries or configurable concurrency.

Reserve a slot in one IMMEDIATE transaction: reread the job, expire its first-launch deadline if necessary, verify fewer than eight launching/running records and no older queued record, then persist launching and its existing nonce. A full queue skips no-op write transactions; the claim always rechecks authoritative state. Original sequence governs slot reservation rather than process-output order.

Persist the resolved timeout in `jobs.queue_timeout_seconds` at admission, independently of the immutable caller specification. SQL NULL means waiting disabled. Keep absent caller fields absent so original idempotency bytes/hashes stay stable. Schema v14 migrates previously explicit queue durations and preserves legacy absent/no-deadline jobs. Retrying old hot/cold records returns their original policy; current defaults cannot add or extend a deadline. Old cold snapshots reconstruct only policies evidenced by their original specification and Job; a missing resolved default is refused.

Queue expiry uses the terminal timeout/event/Outbox transaction without launching a runner. Execution timeout and monitoring clocks remain independent. Before first launch, jobs with waiting enabled cancel locally even for control-enabled remote attachments; started observers reconnect the original run without reapplying the first-launch deadline.

## Recovery and authorization

Restart continues only never-launched queued intents with original IDs, order and resolved deadlines. Launch uncertainty remains orphaned without native replay. Backup restore orphans all unfinished records. The daemon must run to schedule/expire; clock changes affect wall-clock deadlines. Schema v14 and exclusive migration/old-run fences prevent older runtimes from interpreting changed queue policy. Published candidates remain immutable.

Goal budget consumption, membership, rules and targets stay in the admission transaction. Default waiting supplies no consent or extra budget. Every admitted member, including a queue timeout, participates in the original all-terminal completion decision. Cancellation and host-control closure retain existing semantics.

## Regression boundaries

Cover default concurrent admission and FIFO/eight-slot claims, explicit no-wait rejection, original default/custom deadlines at full-capacity retries, queued expiry/cancellation without execution, native restart, hot/cold migration, archival/restore non-replay, observer/control boundaries and Goal membership/accounting. Actual CLI/shared MCP and Web rendering remain separate from native Windows and installed-host continuation acceptance.
