BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS jobs(
 id TEXT PRIMARY KEY, run_id TEXT NOT NULL UNIQUE, spec TEXT NOT NULL,
 idem TEXT UNIQUE, status TEXT NOT NULL CHECK(status IN ('queued','launching','running','completed','failed','cancelled','timeout','orphaned')),
 created INTEGER NOT NULL, updated INTEGER NOT NULL, cancel INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS runs(
 id TEXT PRIMARY KEY, job_id TEXT NOT NULL REFERENCES jobs(id), attempt INTEGER NOT NULL,
 nonce TEXT NOT NULL, runner_pid INTEGER, runner_identity TEXT, workload_pid INTEGER,
 workload_identity TEXT, exit_code INTEGER, signal INTEGER, detail TEXT, ended INTEGER
);
CREATE TABLE IF NOT EXISTS events(
 seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT NOT NULL UNIQUE,
 job_id TEXT NOT NULL REFERENCES jobs(id),run_id TEXT NOT NULL REFERENCES runs(id),
 kind TEXT NOT NULL,at INTEGER NOT NULL,payload TEXT NOT NULL, UNIQUE(run_id,kind)
);
CREATE INDEX IF NOT EXISTS event_job ON events(job_id,seq);
PRAGMA user_version=1;
COMMIT;
