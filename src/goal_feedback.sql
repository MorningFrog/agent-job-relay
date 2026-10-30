BEGIN IMMEDIATE;
ALTER TABLE goal_waits ADD COLUMN feedback_event TEXT;
ALTER TABLE goal_waits ADD COLUMN feedback_delivery TEXT;
ALTER TABLE goal_waits ADD COLUMN feedback_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE goal_waits ADD COLUMN feedback_last_at INTEGER NOT NULL DEFAULT 0;
PRAGMA user_version=12;
COMMIT;
