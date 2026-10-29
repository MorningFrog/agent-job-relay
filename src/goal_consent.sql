BEGIN IMMEDIATE;
CREATE TABLE goal_consents(
 id TEXT PRIMARY KEY, session_key TEXT NOT NULL REFERENCES sessions(key),
 controller TEXT NOT NULL, expected TEXT NOT NULL, session_snapshot TEXT NOT NULL,
 cwd TEXT NOT NULL, label TEXT NOT NULL, job_limit INTEGER NOT NULL, used INTEGER NOT NULL DEFAULT 0,
 status TEXT NOT NULL CHECK(status IN ('pending','asking','approved','declined','revoked','expired','invalidated')),
 revision INTEGER NOT NULL DEFAULT 1, created INTEGER NOT NULL, updated INTEGER NOT NULL,
 expires INTEGER NOT NULL, question_token TEXT, source TEXT NOT NULL DEFAULT ''
);
CREATE INDEX goal_consent_session ON goal_consents(session_key,status,expires);
CREATE TABLE goal_consent_audits(
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, request_id TEXT NOT NULL REFERENCES goal_consents(id),
 revision INTEGER NOT NULL, at INTEGER NOT NULL, status TEXT NOT NULL, source TEXT NOT NULL
);
ALTER TABLE goal_waits ADD COLUMN consent_id TEXT REFERENCES goal_consents(id);
PRAGMA user_version=11;
COMMIT;
