BEGIN IMMEDIATE;
CREATE TABLE goal_waits(
 id TEXT PRIMARY KEY, session_key TEXT NOT NULL REFERENCES sessions(key),
 controller TEXT NOT NULL, expected TEXT NOT NULL, paused TEXT, session_snapshot TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('prepared','sealed','pausing','waiting','resuming','resumed','cancelled','suppressed','attention')),
 revision INTEGER NOT NULL DEFAULT 1, created INTEGER NOT NULL, updated INTEGER NOT NULL,
 expires INTEGER NOT NULL, next_at INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
 detail TEXT NOT NULL DEFAULT '', notified INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX goal_wait_due ON goal_waits(status,next_at);
CREATE INDEX goal_wait_session ON goal_waits(session_key,status);
-- Membership remains after hot job archival; restore never replays control intents.
CREATE TABLE goal_wait_jobs(
 job_id TEXT PRIMARY KEY, run_id TEXT NOT NULL, wait_id TEXT NOT NULL REFERENCES goal_waits(id)
);
CREATE INDEX goal_wait_members ON goal_wait_jobs(wait_id);
CREATE TABLE goal_wait_audits(
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, wait_id TEXT NOT NULL REFERENCES goal_waits(id),
 revision INTEGER NOT NULL, at INTEGER NOT NULL, status TEXT NOT NULL, detail TEXT NOT NULL
);
PRAGMA user_version=10;
COMMIT;
