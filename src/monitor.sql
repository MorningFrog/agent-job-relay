BEGIN IMMEDIATE;
CREATE TEMP TABLE event_sequence AS SELECT seq FROM sqlite_sequence WHERE name='events';
CREATE TABLE events_new(
 seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT NOT NULL UNIQUE,
 job_id TEXT NOT NULL REFERENCES jobs(id),run_id TEXT NOT NULL REFERENCES runs(id),
 kind TEXT NOT NULL,at INTEGER NOT NULL,payload TEXT NOT NULL
);
INSERT INTO events_new SELECT * FROM events;
DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
UPDATE sqlite_sequence SET seq=max(seq,coalesce((SELECT seq FROM event_sequence),0)) WHERE name='events';
INSERT INTO sqlite_sequence(name,seq) SELECT 'events',seq FROM event_sequence WHERE NOT EXISTS(SELECT 1 FROM sqlite_sequence WHERE name='events');
DROP TABLE event_sequence;
CREATE INDEX event_job ON events(job_id,seq);
CREATE INDEX event_kind_sequence ON events(kind,seq DESC);
CREATE UNIQUE INDEX event_lifecycle ON events(run_id,kind) WHERE kind IN ('started','completed','failed','cancelled','timeout','orphaned');
CREATE TABLE monitor_rules(
 id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES jobs(id),revision INTEGER NOT NULL,
 enabled INTEGER NOT NULL,deleted INTEGER NOT NULL DEFAULT 0,spec TEXT NOT NULL,
 applied_revision INTEGER NOT NULL DEFAULT 0,applied_at INTEGER,
 triggered_revision INTEGER NOT NULL DEFAULT 0,triggered_at INTEGER
);
CREATE INDEX monitor_job ON monitor_rules(job_id);
CREATE TABLE monitor_changes(
 id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES jobs(id),rule_id TEXT NOT NULL,
 revision INTEGER NOT NULL,at INTEGER NOT NULL,details TEXT NOT NULL
);
CREATE TABLE runtime_observations(job_id TEXT PRIMARY KEY REFERENCES jobs(id),data TEXT NOT NULL);
CREATE TABLE stop_requests(job_id TEXT PRIMARY KEY REFERENCES jobs(id),grace_seconds INTEGER NOT NULL,reason TEXT NOT NULL,at INTEGER NOT NULL);
PRAGMA user_version=9;
COMMIT;
