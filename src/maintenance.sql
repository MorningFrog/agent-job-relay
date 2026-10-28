BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS archive_jobs(
 sequence INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, run_id TEXT NOT NULL UNIQUE,
 idem TEXT UNIQUE, spec_hash TEXT NOT NULL, snapshot BLOB NOT NULL, archived_at INTEGER NOT NULL, cleanup_pending INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS archive_targets(id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES archive_jobs(id));
CREATE TABLE IF NOT EXISTS archive_deliveries(
 sequence INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, job_id TEXT NOT NULL REFERENCES archive_jobs(id),
 session_key TEXT NOT NULL REFERENCES sessions(key), event_id TEXT NOT NULL, kind TEXT NOT NULL, status TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS archive_delivery_job ON archive_deliveries(job_id,sequence);
CREATE INDEX IF NOT EXISTS archive_delivery_session ON archive_deliveries(session_key,sequence);
CREATE TABLE IF NOT EXISTS sequence_highwater(name TEXT PRIMARY KEY,value INTEGER NOT NULL);
INSERT OR IGNORE INTO sequence_highwater VALUES('jobs',(SELECT coalesce(max(rowid),0) FROM jobs));
INSERT OR IGNORE INTO sequence_highwater VALUES('targets',(SELECT coalesce(max(rowid),0) FROM targets));
INSERT OR IGNORE INTO sequence_highwater VALUES('deliveries',(SELECT coalesce(max(rowid),0) FROM deliveries));
CREATE TABLE IF NOT EXISTS maintenance_state(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS maintenance_backups(id TEXT PRIMARY KEY,at INTEGER NOT NULL,manifest_hash TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS jobs_age ON jobs(updated);
PRAGMA user_version=8;
COMMIT;
