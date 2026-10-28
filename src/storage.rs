use crate::{config::Config, core::*};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{Notify, Semaphore};
const DB_WORKERS: usize = 4;
struct CachedConnection {
    path: PathBuf,
    db: Connection,
}
#[derive(Clone)]
pub struct Store {
    pub config: Config,
    connections: Arc<Mutex<Vec<CachedConnection>>>,
    readonly: bool,
    pub(crate) web_info: Arc<Mutex<serde_json::Value>>,
    pub(crate) job_changes: Arc<Notify>,
    pub(crate) delivery_changes: Arc<Notify>,
    pub(crate) maintenance_stop: Arc<std::sync::atomic::AtomicBool>,
}
impl Store {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            readonly: false,
            web_info: Arc::new(Mutex::new(serde_json::json!({"enabled":false}))),
            connections: Arc::new(Mutex::new(Vec::new())),
            job_changes: Arc::new(Notify::new()),
            delivery_changes: Arc::new(Notify::new()),
            maintenance_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    pub fn readonly(config: Config) -> Self {
        let mut store = Self::new(config);
        store.readonly = true;
        store
    }
    pub async fn call<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        self.operation(f, false).await
    }
    /// Read-only work has reserved capacity even when writers wait on SQLite.
    pub async fn read<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        self.operation(f, true).await
    }
    async fn operation<T, F>(&self, f: F, read: bool) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        static READERS: OnceLock<Arc<Semaphore>> = OnceLock::new();
        static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
        let slots = if read { &READERS } else { &SLOTS };
        let permit = slots
            .get_or_init(|| Arc::new(Semaphore::new(if read { 2 } else { DB_WORKERS })))
            .clone()
            .acquire_owned()
            .await?;
        ensure!(!self.readonly || read, "offline inspection is read-only");
        let readonly = self.readonly;
        let path = self.config.root.join("relay.sqlite3");
        let connections = self.connections.clone();
        let job_changes = self.job_changes.clone();
        let delivery_changes = self.delivery_changes.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let metadata = std::fs::symlink_metadata(&path).ok();
            if let Some(m) = &metadata {
                ensure!(
                    m.is_file() && !crate::platform::is_link(m),
                    "database must be a regular file, not a symbolic link"
                );
            }
            let cached = connections
                .lock()
                .map_err(|_| anyhow::anyhow!("database pool poisoned"))?
                .pop()
                .filter(|cached| cached.path == path && metadata.is_some());
            let mut db = match cached {
                Some(cached) => cached.db,
                None => {
                    let db = if readonly {
                        Connection::open_with_flags(
                            &path,
                            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                        )?
                    } else {
                        Connection::open(&path)?
                    };
                    if readonly {
                        ensure!(
                            db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))?
                                == 9,
                            "offline schema is unsupported; use the matching runtime"
                        );
                    }
                    db.busy_timeout(std::time::Duration::from_secs(5))?;
                    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
                    db
                }
            };
            let before = db.total_changes();
            let mut result = f(&mut db);
            let changed = db.total_changes() != before;
            // Never hand an unfinished transaction to another operation. Failed
            // connections are dropped, preserving rollback-on-close semantics.
            if !db.is_autocommit() && result.is_ok() {
                result = Err(anyhow::anyhow!(
                    "database operation left an unfinished transaction"
                ));
            }
            if result.is_ok() {
                let mut pool = connections
                    .lock()
                    .map_err(|_| anyhow::anyhow!("database pool poisoned"))?;
                if pool.len() < DB_WORKERS {
                    pool.push(CachedConnection { path, db });
                }
            } else {
                drop(db);
            }
            if changed {
                job_changes.notify_one();
                delivery_changes.notify_one();
            }
            result
        })
        .await?
    }
    pub async fn init(&self) -> Result<()> {
        let version = self
            .read(|db| Ok(db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))?))
            .await?;
        // Older daemons allocate rowids independently of the archive high-water
        // marks. Never migrate underneath one or an older in-flight runner.
        let _migration_lock = if version < 9 {
            Some(
                self.config
                    .daemon_lock()
                    .context("stop the selected daemon before upgrading the data schema")?,
            )
        } else {
            None
        };
        self.call(|db| {
            let v: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
            ensure!(
                v <= 9,
                "database schema is newer than this executable; downgrade refused"
            );
            if v > 0 && v < 9 {
                let running:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE status IN ('launching','running'))",[],|r|r.get(0))?;
                ensure!(!running,"finish active runs with the previous runtime before upgrading the data schema");
            }
            db.pragma_update(None, "journal_mode", "WAL")?;
            if v == 0 {
                db.execute_batch(include_str!("schema.sql"))?;
            }
            if v < 2 {
                db.execute_batch(include_str!("delivery.sql"))?;
            }
            if v < 3 {
                db.execute_batch("BEGIN IMMEDIATE; ALTER TABLE sessions ADD COLUMN endpoint_identity TEXT; PRAGMA user_version=3; COMMIT;")?;
            }
            if v < 4 {
                db.execute_batch(
                    "BEGIN IMMEDIATE;
                     ALTER TABLE runs ADD COLUMN status TEXT NOT NULL DEFAULT 'queued';
                     ALTER TABLE runs ADD COLUMN started INTEGER;
                     UPDATE runs SET status=(SELECT status FROM jobs WHERE jobs.id=runs.job_id),
                         started=(SELECT at FROM events WHERE events.run_id=runs.id AND kind='started');
                     PRAGMA user_version=4;
                     COMMIT;",
                )?;
            }
            if v < 5 {
                db.execute_batch("BEGIN IMMEDIATE; ALTER TABLE deliveries ADD COLUMN failure_kind TEXT; PRAGMA user_version=5; COMMIT;")?;
            }
            if v < 6 {
                db.execute_batch("BEGIN IMMEDIATE;
                    CREATE TABLE target_recoveries(id TEXT PRIMARY KEY, at INTEGER NOT NULL,
                        from_target TEXT NOT NULL REFERENCES targets(id),
                        to_target TEXT NOT NULL REFERENCES targets(id),
                        session_key TEXT NOT NULL REFERENCES sessions(key), details TEXT NOT NULL);
                    PRAGMA user_version=6; COMMIT;")?;
            }
            if v < 7 {
                db.execute_batch("BEGIN IMMEDIATE;
                    CREATE INDEX IF NOT EXISTS jobs_status ON jobs(status);
                    CREATE INDEX IF NOT EXISTS sessions_lease ON sessions(connected,expires);
                    CREATE INDEX IF NOT EXISTS targets_session ON targets(session_key);
                    CREATE INDEX IF NOT EXISTS deliveries_target ON deliveries(target_id);
                    CREATE INDEX IF NOT EXISTS deliveries_lease ON deliveries(status,lease_until);
                    PRAGMA user_version=7; COMMIT;")?;
            }
            if v < 8 { db.execute_batch(include_str!("maintenance.sql"))?; }
            if v < 9 {
                db.pragma_update(None, "foreign_keys", false)?;
                db.execute_batch(include_str!("monitor.sql"))?;
                db.pragma_update(None, "foreign_keys", true)?;
                ensure!(!db.prepare("PRAGMA foreign_key_check")?.exists([])?, "migration foreign-key check failed");
            }
            Ok(())
        })
        .await
    }
    pub async fn start(&self, mut spec: StartSpec) -> Result<Job> {
        spec.validate()?;
        let raw = serde_json::to_string(&spec)?;
        self.call(move |db|{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if let Some(key)=&spec.idempotency_key
                && let Some((id,old))=tx.query_row("SELECT id,spec FROM jobs WHERE idem=?",[key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()? {
                    ensure!(old==raw,"idempotency key already used with a different specification"); return read_job(&tx,&id);
                }
            if let Some(key)=&spec.idempotency_key
                && let Some((id,hash))=tx.query_row("SELECT id,spec_hash FROM archive_jobs WHERE idem=?",[key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()? {
                ensure!(hash==crate::archive::hash(raw.as_bytes()),"idempotency key already used with a different specification");return read_job(&tx,&id);
            }
            let active:i64=tx.query_row("SELECT count(*) FROM jobs WHERE status IN ('queued','launching','running')",[],|r|r.get(0))?;
            ensure!(active<8,"active job quota (8) reached; wait or stop a job");
            let total:i64=tx.query_row("SELECT count(*) FROM jobs",[],|r|r.get(0))?;
            ensure!(total<10000,"job history quota (10000) reached; archive state before starting more jobs");
            let j=id(); let r=id(); let nonce=id(); let t=now();
            let sequence=next_sequence(&tx,"jobs")?;
            tx.execute("INSERT INTO jobs(rowid,id,run_id,spec,idem,status,created,updated) VALUES(?,?,?,?,?,'queued',?,?)",params![sequence,j,r,raw,spec.idempotency_key,t,t])?;
            tx.execute("INSERT INTO runs(id,job_id,attempt,nonce) VALUES(?,?,1,?)",params![r,j,nonce])?;
            if let Some(key)=&spec.session {crate::session::add_target(&tx,&j,key)?;}
            for rule in &spec.monitor_rules { crate::monitor::add_rule(&tx, &j, rule)?; }
            let result=read_job(&tx,&j)?; tx.commit()?; Ok(result)
        }).await
    }
    pub async fn status(&self, j: String) -> Result<Job> {
        validate_id(&j)?;
        self.read(move |db| read_job(db, &j)).await
    }
    pub async fn list(&self, after: i64, limit: u32) -> Result<Vec<Job>> {
        ensure!(
            (1..=100).contains(&limit) && after >= 0,
            "limit must be 1..100 and cursor nonnegative"
        );
        self.read(move |db| {
            let tx=db.transaction()?;
            let db=&tx;
            let mut s = db.prepare("SELECT id FROM (SELECT rowid AS sequence,id FROM jobs UNION ALL SELECT sequence,id FROM archive_jobs) WHERE sequence>? ORDER BY sequence LIMIT ?")?;
            let ids = s
                .query_map(params![after, limit], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids.iter().map(|j| read_job(db, j)).collect()
        })
        .await
    }
    pub async fn active(&self) -> Result<Vec<Job>> {
        self.read(|db| {
            let mut q=db.prepare("SELECT id FROM jobs WHERE status IN ('queued','launching','running') ORDER BY rowid")?;
            let ids=q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            ids.iter().map(|j|read_job(db,j)).collect()
        }).await
    }
    pub async fn cancel_requested(&self, j: String) -> Result<bool> {
        self.read(move |db| {
            Ok(db.query_row("SELECT cancel FROM jobs WHERE id=?", [j], |r| r.get(0))?)
        })
        .await
    }
    pub async fn stop(&self, j: String) -> Result<Job> {
        self.stop_with(j, 5, "manual".into()).await
    }
    pub async fn stop_with(&self, j: String, grace: u32, reason: String) -> Result<Job> {
        validate_id(&j)?;
        ensure!(grace <= 300, "grace_seconds must be 0..300");
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            crate::monitor::stop_tx(&tx, &j, grace, &reason)?;
            let job = read_job(&tx, &j)?;
            tx.commit()?;
            Ok(job)
        })
        .await
    }
    pub async fn events(&self, j: String, after: i64, limit: u32) -> Result<Vec<Event>> {
        validate_id(&j)?;
        ensure!(
            (1..=100).contains(&limit) && after >= 0,
            "invalid event limit/cursor"
        );
        self.read(move|db|{ let tx=db.transaction()?;let db=&tx; if let Some(snapshot)=crate::archive::load(db,&j)? { return crate::archive::events(&snapshot,after,limit); } read_job(db,&j)?; let mut q=db.prepare("SELECT seq,id,job_id,run_id,kind,at,payload FROM events WHERE job_id=? AND seq>? ORDER BY seq LIMIT ?")?;
            let rows=q.query_map(params![j,after,limit],|r|Ok(Event{seq:r.get(0)?,event_id:r.get(1)?,job_id:r.get(2)?,run_id:r.get(3)?,kind:r.get(4)?,at:r.get(5)?,payload:serde_json::from_str(&r.get::<_,String>(6)?).unwrap_or_default()}))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?) }).await
    }
    pub async fn finish(
        &self,
        j: String,
        state: &'static str,
        code: Option<i32>,
        signal: Option<i32>,
        detail: String,
    ) -> Result<()> {
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            finish_tx(&tx, &j, state, code, signal, &detail)?;
            tx.commit()?;
            Ok(())
        })
        .await
    }
}
pub fn read_job(db: &Connection, j: &str) -> Result<Job> {
    let row=db.query_row("SELECT j.id,j.run_id,j.status,j.spec,j.created,j.updated,j.cancel,r.runner_pid,r.runner_identity,r.workload_pid,r.workload_identity,r.exit_code,r.signal,r.detail,j.rowid,r.attempt,r.status,r.started,r.ended FROM jobs j JOIN runs r ON r.id=j.run_id WHERE j.id=?",[j],|r|{
        Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,bool>(6)?,r.get::<_,Option<u32>>(7)?,r.get::<_,Option<String>>(8)?,r.get::<_,Option<u32>>(9)?,r.get::<_,Option<String>>(10)?,r.get::<_,Option<i32>>(11)?,r.get::<_,Option<i32>>(12)?,r.get::<_,Option<String>>(13)?,r.get::<_,i64>(14)?,r.get::<_,u32>(15)?,r.get::<_,String>(16)?,r.get::<_,Option<i64>>(17)?,r.get::<_,Option<i64>>(18)?))
    }).optional()?;
    let Some((
        job_id,
        run_id,
        status,
        spec,
        created_at,
        updated_at,
        cancel_requested,
        runner_pid,
        runner_identity,
        workload_pid,
        workload_identity,
        exit_code,
        signal,
        detail,
        sequence,
        attempt,
        run_status,
        started_at,
        ended_at,
    )) = row
    else {
        return crate::archive::load(db, j)?
            .map(|s| s.job)
            .ok_or_else(|| anyhow::anyhow!("job not found"));
    };
    let spec: StartSpec = serde_json::from_str(&spec)?;
    let run = Run {
        run_id: run_id.clone(),
        job_id: job_id.clone(),
        attempt,
        status: run_status,
        started_at,
        ended_at,
        runner_pid,
        runner_identity: runner_identity.clone(),
        workload_pid,
        workload_identity: workload_identity.clone(),
        exit_code,
        signal,
    };
    Ok(Job {
        run,
        sequence,
        job_id,
        run_id,
        status,
        command: spec.command,
        cwd: spec.cwd,
        session: spec.session,
        created_at,
        updated_at,
        cancel_requested,
        runner_pid,
        runner_identity,
        workload_pid,
        workload_identity,
        exit_code,
        signal,
        detail,
    })
}
pub fn finish_tx(
    tx: &Connection,
    j: &str,
    state: &str,
    code: Option<i32>,
    signal: Option<i32>,
    detail: &str,
) -> Result<()> {
    let job = read_job(tx, j)?;
    if terminal(&job.status) {
        return Ok(());
    }
    ensure!(terminal(state), "invalid terminal state");
    let t = now();
    // Obsolete alert deliveries retain IDs, TTL and attempts; already accepted
    // or in-flight requests cannot be recalled from a host.
    tx.execute("UPDATE deliveries SET status='cancelled',last_error='monitor alert superseded by terminal result' WHERE status IN ('pending','retry_wait') AND event_id IN (SELECT id FROM events WHERE job_id=? AND kind='monitor_alert')",[j])?;
    tx.execute(
        "UPDATE jobs SET status=?,updated=? WHERE id=?",
        params![state, t, j],
    )?;
    tx.execute(
        "UPDATE runs SET exit_code=?,signal=?,detail=?,ended=?,status=? WHERE id=?",
        params![code, signal, detail, t, state, job.run_id],
    )?;
    insert_event(
        tx,
        j,
        &job.run_id,
        state,
        serde_json::json!({"detail":detail,"exit_code":code,"signal":signal}),
    )?;
    Ok(())
}
pub fn insert_event(
    tx: &Connection,
    j: &str,
    r: &str,
    kind: &str,
    payload: serde_json::Value,
) -> Result<()> {
    let payload = serde_json::to_string(&payload)?;
    ensure!(payload.len() <= 8192, "event payload too large");
    tx.execute(
        "INSERT INTO events(id,job_id,run_id,kind,at,payload) VALUES(?,?,?,?,?,?)",
        params![id(), j, r, kind, now(), payload],
    )?;
    if terminal(kind) {
        let event: String = tx.query_row(
            "SELECT id FROM events WHERE run_id=? AND kind=?",
            params![r, kind],
            |row| row.get(0),
        )?;
        let mut q = tx.prepare("SELECT id FROM targets WHERE job_id=? AND active=1")?;
        let targets = q
            .query_map([j], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        for target in targets {
            crate::session::enqueue(tx, &event, &target, now())?;
        }
    }
    Ok(())
}

/// Preserve cursors when the largest hot row is moved into cold storage.
pub(crate) fn next_sequence(db: &Connection, table: &str) -> Result<i64> {
    ensure!(
        ["jobs", "targets", "deliveries"].contains(&table),
        "unknown cursor sequence"
    );
    // max(rowid) also admits older fixtures/imports that used SQLite allocation.
    Ok(db.query_row(&format!("UPDATE sequence_highwater SET value=max(value,(SELECT coalesce(max(rowid),0) FROM {table}))+1 WHERE name=? RETURNING value"),[table],|r|r.get(0))?)
}
