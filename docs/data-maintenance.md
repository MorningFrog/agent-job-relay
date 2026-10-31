# Automatic data maintenance

[Documentation index](README.md) · [Command reference](commands.md) · [Updating](updating.md)

Source 0.1.17 uses schema v12 for durable goal-feedback identities/counters; earlier 0.1.12–0.1.16 runtimes use v11, 0.1.11 uses v9 and 0.1.9 uses v6. Finish existing workloads before upgrading. Current-source runtime entries hand over an older daemon automatically after migration checks; versions before 0.1.12 need an explicit same-profile stop. Migration refuses an old daemon lock or launching/running jobs; older executables refuse the new schema. Keep the same state profile and never downgrade a used database.

Examples use the online ChatGPT/Codex profile. Substitute the appropriate [command prefix](commands.md#choose-the-entry-point-and-profile) for OpenCode or a native offline installation; keep the original profile for all operations.

## Default operation

While the daemon runs, maintenance checks once a minute. It archives eligible completed history older than 30 days, or earlier when hot history exceeds 1,000 jobs or raw hot logs exceed 256 MiB. Each batch handles at most 32 jobs; a full batch schedules another check after one second. Active jobs, unresolved deliveries and unexpired dead letters remain protected. These are soft working-set budgets: protected work can exceed them. The submission guard remains 10,000 hot jobs; archived history does not consume it. Session registration limits 1,000 currently connected, unexpired sessions; expired historical registrations retain their identities without consuming that quota.

Archiving is automatic and preserves history indefinitely. Cold relational records become compressed SQLite snapshots; logs become private gzip files beside their original run. Status, list, events, deliveries, attempts, inbox and byte-cursor log reads include archived data. Stable IDs, pagination cursors, idempotency keys, expiry, attempt history and recovery audits survive. An explicit retry or target change restores the required relational history transactionally; it never restarts the command. Archives reuse freed database pages but do not promise that the database file shrinks immediately. Archived logs, session metadata and historical inbox files are retained, so total disk usage still grows; automatic archiving is not destructive purge.

The daemon creates a complete local database/log backup on the first maintenance pass with jobs, then every 24 hours, and retains the latest three verified backups. It needs no system scheduler or service. With the daemon stopped, maintenance pauses and resumes on its next start. Backups live under the selected profile's `backups/` directory; they do not protect against loss of that entire disk. Copy a completed backup directory elsewhere when independent storage is needed.

## Inspect capacity and change budgets

```bash
agent-job-relay codex data status
agent-job-relay codex data policy
agent-job-relay codex data policy --hot-jobs 500 --hot-log-mib 128
agent-job-relay codex data policy --hot-jobs 500 --hot-log-mib 128 --apply
```

Expected: status reports hot/archived job counts, session quota, database/reusable bytes, available disk space at the last scan, the last scan, recent backup IDs and maintenance errors. Policy changes are previews until `--apply`; the daemon reloads them on its next check. Other options are `--archive-after-days`, `--backup-hours` and `--backup-keep`. Zero archive days archives eligible terminal history at the next check. The documented defaults require no configuration.

To preview or immediately run a maintenance batch:

```bash
agent-job-relay codex data maintain
agent-job-relay codex data maintain --apply
```

Expected: preview reports eligible archival work without compressing/deleting logs or making a backup. Apply also creates a due backup and rotates verified backups. A running maintenance operation holds the profile's lock; another invocation reports that it is busy. File errors and protected notifications can defer archiving. Inspect `data status` or `job_doctor` if the hot quota is near its limit.

## Back up and restore

```bash
agent-job-relay codex data backup
```

Expected: the command prints a completed backup ID and directory. Each backup contains a SQLite online snapshot, bounded stdout/stderr files and a SHA-256 manifest. Completed-job output is immutable; active output is a bounded prefix and may extend beyond the database snapshot time. External workload files, inherited environment, host configuration, endpoint/lock files and raw inbox receipt files are not copied. Durable notification/event records are included in SQLite. Command specifications and logs can contain secrets; backup files retain the same private ownership requirements as state files.

For restore, replace both placeholders below with absolute paths. The destination must not exist and its parent must already exist:

```bash
agent-job-relay codex data restore --from "<backup-directory>" --to "<new-state-directory>"
agent-job-relay --state-dir "<new-state-directory>" data check
```

Expected: checksums, file inventory, schema, SQLite integrity and foreign keys pass before the new profile is published. Existing state is never overwritten. Queued/launching/running jobs become `orphaned`; commands are not replayed. Sessions are invalidated and outstanding claims are fenced without changing their IDs, expiry or attempt history. Accepted notifications are not resent. Source v12 restores verified v8–v12 backups forward, invalidates consent and cancels restored goal-control intents and grouped notifications, and retains membership/audits without touching the old host goal. Goal waits are not moved by ordinary notification recovery. Explicitly bind the actual current caller and recover eligible notifications if needed. Audit external side effects before deliberately submitting work again.

## Interrupted operations and diagnostics

The archive transaction rechecks its source snapshot before moving relational records. Compression is durable before that commit; raw logs are removed only after verifying the compressed copy. A persisted cleanup marker completes an interrupted post-commit unlink on a later pass. Failure before commit leaves hot history usable.

Backups publish a manifest last and record their exact staging identity and planned paths. Restart adopts unchanged sealed bytes or rebuilds only the recorded, unsealed staging directory. Unsealed staging is disposable and is not a restore point. Unknown files/links stop automatic recovery; completed backups with changed, missing or foreign files are never automatically removed. Rotation touches only recorded backups, never unrelated directories. These deterministic recovery tests do not establish physical power-loss or ENOSPC qualification.

`doctor`/`job_doctor` perform lightweight connection/schema checks. Run `data check` for explicit SQLite integrity/foreign-key validation. Maintenance failure is visible in capacity diagnostics and does not silently delete history or restart workloads. See [testing](testing.md), [limitations](limitations.md) and [architecture](architecture.md).

## Recent log segments

Runtime 0.1.18+ native runs additionally retain four checksummed recent 1-MiB output segments per stream after the existing 16-MiB prefix. Hot-log budgets count their bounded files. Archival seals `.recent.gz` and verifies it byte for byte before removing `.recent`; absolute offsets and eviction gaps remain readable after archival. Backups include these exact managed names in their existing checksum inventory. Only recent-ring copies take a shared file lock at a stable boundary; append-only prefixes retain bounded non-locking copies so Windows workload output remains writable. This is a per-file snapshot, not an atomic snapshot of all running output. Pending unsealed capture bytes and files outside managed log storage are excluded. Recovery still uses only recorded run/staging paths, rejects foreign files and never replays restored execution. Older prefix-only backups remain supported. See [log reading and export](runtime-monitoring.md#read-and-export-retained-logs).
