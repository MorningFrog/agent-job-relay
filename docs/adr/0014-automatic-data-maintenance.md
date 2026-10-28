# ADR 0014: automatic cold history and recoverable local backups

## Decision

Keep one Rust runtime and one SQLite transaction domain. Schema v8 adds compressed cold-job snapshots, indexed delivery/target catalogs, cursor high-water marks and maintenance/backup journals. It follows v7's runtime lookup indexes. Automatically archive terminal history by age/count/log pressure while excluding unresolved deliveries and unexpired recoverable dead letters. Keep historical session identities; quota only live registrations.

Archive metadata and hot-row removal commit atomically after source-snapshot revalidation. Publish/fsync compressed logs first, verify them before deleting raw files and persist cleanup work. Retain archives indefinitely, including original specifications, IDs, attempts, TTLs and audits. Idempotency checks span hot and cold data. Explicit delivery/target mutations restore terminal relational history in their transaction and cannot replay execution. High-water allocation prevents rowid pagination reuse after the largest hot row is archived; non-cursor rowids need not survive rehydration.

Use SQLite's online backup API with a pinned read snapshot and private bounded log copies. Publish a checksummed manifest last; journal the exact staging identity/paths before file creation. Sealed candidates finish forward unchanged; unsealed owned staging can be rebuilt after inventory validation. Never scan for or delete guessed backup directories. Rotate only recorded, verified completed backups. Restore into a new profile, invalidate registrations, fence claims and classify old active intents as orphaned before publishing it. Backups on the same disk are local recovery points, not independent disaster recovery.

Reserve two read workers independently of four general database workers. Reconcile in a supervised daemon task so SQL waits do not stop IPC acceptance. Keep writes short and cancellation reads minimal. Move full SQLite integrity checking to explicit maintenance rather than every Doctor request. Automatic maintenance and backup work join orderly shutdown, with cancellation checks between backup steps/files; runners remain independent.

## Compatibility and consequences

The CLI adds `data` management without adding MCP tools or changing IPC v1. Cold records are transparently read through existing bounded job/event/delivery/inbox/log interfaces. Compression bounds each snapshot and log read; repeated seeks into cold gzip logs require bounded decompression. Freed SQLite pages are reused without an automatic blocking VACUUM. Total preserved history, historical registrations/inbox files and backups still require disk space; no silent history purge or exactly-once delivery is introduced.

Migration requires exclusive daemon ownership and no old launching/running jobs. This prevents older rowid allocators from writing through an archive migration. Downgrades remain refused. Data policy/state belong to the selected profile and survive relocation/update. Real Windows maintenance, physical power-loss/ENOSPC and long-duration qualification need their own evidence; fake/injected tests are distinct.

## Regression rationale

Behavioral tests cover automatic daemon operation, all retention triggers, hot/cold pagination and idempotency, binary log cursors, protected claims/recovery, explicit retry rehydration, migration fencing, archive rollback/post-commit cleanup, pinned-WAL backup/restore without execution replay, sealed/unsealed backup recovery, ownership/foreign-file refusal and rolling rotation. Runtime tests verify readable diagnostics and shutdown acceptance during a real SQLite writer wait; reserved-read tests saturate all general database workers without blocking status.
