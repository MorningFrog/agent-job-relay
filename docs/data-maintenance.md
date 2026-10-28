# Automatic data maintenance

Version 0.1.10 and newer use schema v9; 0.1.9 uses schema v6. Finish existing workloads and explicitly stop the selected old daemon before upgrading. Migration refuses an old daemon lock or launching/running jobs; older executables refuse the new schema. Keep the same state profile and never downgrade a used database.

## Default operation

While the daemon runs, maintenance checks once a minute. It archives eligible completed history older than 30 days, or earlier when hot history exceeds 1,000 jobs or raw hot logs exceed 256 MiB. Each batch handles at most 32 jobs; a full batch schedules another check after one second. Active jobs, unresolved deliveries and unexpired dead letters remain protected. These are soft working-set budgets: protected work can exceed them. The submission guard remains 10,000 hot jobs; archived history does not consume it. Session registration limits 1,000 currently connected, unexpired sessions; expired historical registrations retain their identities without consuming that quota.

Archiving is automatic and preserves history indefinitely. Cold relational records become compressed SQLite snapshots; logs become private gzip files beside their original run. Status, list, events, deliveries, attempts, inbox and byte-cursor log reads include archived data. Stable IDs, pagination cursors, idempotency keys, expiry, attempt history and recovery audits survive. An explicit retry or target change restores the required relational history transactionally; it never restarts the command. Archives reuse freed database pages but do not promise that the database file shrinks immediately. Archived logs, session metadata and historical inbox files are retained, so total disk usage still grows; automatic archiving is not destructive purge.

The daemon creates a complete local database/log backup on the first maintenance pass with jobs, then every 24 hours, and retains the latest three verified backups. It needs no system scheduler or service. With the daemon stopped, maintenance pauses and resumes on its next start. Backups live under the selected profile's `backups/` directory; they do not protect against loss of that entire disk. Copy a completed backup directory elsewhere when independent storage is needed.

## Inspect capacity and change budgets

```bash
agent-job-relay data status
agent-job-relay data policy
agent-job-relay data policy --hot-jobs 500 --hot-log-mib 128
agent-job-relay data policy --hot-jobs 500 --hot-log-mib 128 --apply
```

Expected: status reports hot/archived job counts, session quota, database/reusable bytes, available disk space at the last scan, the last scan, recent backup IDs and maintenance errors. Policy changes are previews until `--apply`; the daemon reloads them on its next check. Other options are `--archive-after-days`, `--backup-hours` and `--backup-keep`. Zero archive days archives eligible terminal history at the next check. The documented defaults require no configuration.

To preview or immediately run a maintenance batch:

```bash
agent-job-relay data maintain
agent-job-relay data maintain --apply
```

Expected: preview reports eligible archival work without compressing/deleting logs or making a backup. Apply also creates a due backup and rotates verified backups. A running maintenance operation holds the profile's lock; another invocation reports that it is busy. File errors and protected notifications can defer archiving. Inspect `data status` or `job_doctor` if the hot quota is near its limit.

## Back up and restore

```bash
agent-job-relay data backup
```

Expected: the command prints a completed backup ID and directory. Each backup contains a SQLite online snapshot, bounded stdout/stderr files and a SHA-256 manifest. Completed-job output is immutable; active output is a bounded prefix and may extend beyond the database snapshot time. External workload files, inherited environment, host configuration, endpoint/lock files and raw inbox receipt files are not copied. Durable notification/event records are included in SQLite. Command specifications and logs can contain secrets; backup files retain the same private ownership requirements as state files.

For restore, replace both placeholders below with absolute paths. The destination must not exist and its parent must already exist:

```bash
agent-job-relay data restore --from "<backup-directory>" --to "<new-state-directory>"
agent-job-relay --state-dir "<new-state-directory>" data check
```

Expected: checksums, file inventory, schema, SQLite integrity and foreign keys pass before the new profile is published. Existing state is never overwritten. Queued/launching/running jobs become `orphaned`; commands are not replayed. Sessions are invalidated and outstanding claims are fenced without changing their IDs, expiry or attempt history. Accepted notifications are not resent. Explicitly bind the actual current caller and recover eligible notifications if needed. Audit external side effects before deliberately submitting work again.

## Interrupted operations and diagnostics

The archive transaction rechecks its source snapshot before moving relational records. Compression is durable before that commit; raw logs are removed only after verifying the compressed copy. A persisted cleanup marker completes an interrupted post-commit unlink on a later pass. Failure before commit leaves hot history usable.

Backups publish a manifest last and record their exact staging identity and planned paths. Restart adopts unchanged sealed bytes or rebuilds only the recorded, unsealed staging directory. Unsealed staging is disposable and is not a restore point. Unknown files/links stop automatic recovery; completed backups with changed, missing or foreign files are never automatically removed. Rotation touches only recorded backups, never unrelated directories. These deterministic recovery tests do not establish physical power-loss or ENOSPC qualification.

`doctor`/`job_doctor` perform lightweight connection/schema checks. Run `data check` for explicit SQLite integrity/foreign-key validation. Maintenance failure is visible in capacity diagnostics and does not silently delete history or restart workloads. See [testing](testing.md), [limitations](limitations.md) and [architecture](architecture.md).
