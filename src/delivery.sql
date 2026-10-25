BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS sessions(
 key TEXT PRIMARY KEY,agent TEXT NOT NULL,logical TEXT NOT NULL,instance TEXT NOT NULL,
 host TEXT NOT NULL,registration TEXT NOT NULL,expires INTEGER NOT NULL,connected INTEGER NOT NULL,
 UNIQUE(agent,logical,instance,host)
);
CREATE TABLE IF NOT EXISTS targets(
 id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES jobs(id),session_key TEXT NOT NULL REFERENCES sessions(key),
 active INTEGER NOT NULL,UNIQUE(job_id,session_key)
);
CREATE TABLE IF NOT EXISTS deliveries(
 id TEXT PRIMARY KEY,event_id TEXT NOT NULL REFERENCES events(id),target_id TEXT NOT NULL REFERENCES targets(id),
 status TEXT NOT NULL CHECK(status IN ('pending','leased','notification_recorded','accepted_by_host','retry_wait','dead_letter','cancelled')),
 attempts INTEGER NOT NULL,next_at INTEGER NOT NULL,expires INTEGER NOT NULL,
 token TEXT,lease_until INTEGER,last_error TEXT,UNIQUE(event_id,target_id)
);
CREATE TABLE IF NOT EXISTS delivery_attempts(
 delivery_id TEXT NOT NULL REFERENCES deliveries(id),attempt INTEGER NOT NULL,token TEXT NOT NULL,
 started INTEGER NOT NULL,ended INTEGER,outcome TEXT,error TEXT,PRIMARY KEY(delivery_id,attempt)
);
CREATE INDEX IF NOT EXISTS delivery_due ON deliveries(status,next_at);
PRAGMA user_version=2;
COMMIT;
