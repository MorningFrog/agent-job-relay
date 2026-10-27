use crate::{config::Config, core::*};
use anyhow::{Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
#[derive(Clone)]
pub struct Store {
    pub config: Config,
}
impl Store {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    pub async fn call<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
        let permit = SLOTS
            .get_or_init(|| Arc::new(Semaphore::new(4)))
            .clone()
            .acquire_owned()
            .await?;
        let path = self.config.root.join("relay.sqlite3");
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if let Ok(m) = std::fs::symlink_metadata(&path) {
                ensure!(
                    m.is_file() && !crate::platform::is_link(&m),
                    "database must be a regular file, not a symbolic link"
                );
            }
            let mut db = Connection::open(path)?;
            db.busy_timeout(std::time::Duration::from_secs(5))?;
            db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
            f(&mut db)
        })
        .await?
    }
    pub async fn init(&self) -> Result<()> {
        self.call(|db| {
            let v: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
            ensure!(
                v <= 6,
                "database schema is newer than this executable; downgrade refused"
            );
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
            let active:i64=tx.query_row("SELECT count(*) FROM jobs WHERE status IN ('queued','launching','running')",[],|r|r.get(0))?;
            ensure!(active<8,"active job quota (8) reached; wait or stop a job");
            let total:i64=tx.query_row("SELECT count(*) FROM jobs",[],|r|r.get(0))?;
            ensure!(total<10000,"job history quota (10000) reached; archive state before starting more jobs");
            let j=id(); let r=id(); let nonce=id(); let t=now();
            tx.execute("INSERT INTO jobs(id,run_id,spec,idem,status,created,updated) VALUES(?,?,?,?,'queued',?,?)",params![j,r,raw,spec.idempotency_key,t,t])?;
            tx.execute("INSERT INTO runs(id,job_id,attempt,nonce) VALUES(?,?,1,?)",params![r,j,nonce])?;
            if let Some(key)=&spec.session {crate::session::add_target(&tx,&j,key)?;}
            let result=read_job(&tx,&j)?; tx.commit()?; Ok(result)
        }).await
    }
    pub async fn status(&self, j: String) -> Result<Job> {
        validate_id(&j)?;
        self.call(move |db| read_job(db, &j)).await
    }
    pub async fn list(&self, after: i64, limit: u32) -> Result<Vec<Job>> {
        ensure!(
            (1..=100).contains(&limit) && after >= 0,
            "limit must be 1..100 and cursor nonnegative"
        );
        self.call(move |db| {
            let mut s = db.prepare("SELECT id FROM jobs WHERE rowid>? ORDER BY rowid LIMIT ?")?;
            let ids = s
                .query_map(params![after, limit], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids.iter().map(|j| read_job(db, j)).collect()
        })
        .await
    }
    pub async fn active(&self) -> Result<Vec<Job>> {
        self.call(|db| {
            let mut q=db.prepare("SELECT id FROM jobs WHERE status IN ('queued','launching','running') ORDER BY rowid")?;
            let ids=q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            ids.iter().map(|j|read_job(db,j)).collect()
        }).await
    }
    pub async fn stop(&self, j: String) -> Result<Job> {
        validate_id(&j)?;
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let job = read_job(&tx, &j)?;
            if !terminal(&job.status) {
                tx.execute("UPDATE jobs SET cancel=1 WHERE id=?", [&j])?;
                if job.status == "queued" {
                    finish_tx(&tx, &j, "cancelled", None, None, "cancelled before launch")?;
                }
            }
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
        self.call(move|db|{ read_job(db,&j)?; let mut q=db.prepare("SELECT seq,id,job_id,run_id,kind,at,payload FROM events WHERE job_id=? AND seq>? ORDER BY seq LIMIT ?")?;
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
    let row=db.query_row("SELECT j.id,j.run_id,j.status,j.spec,j.created,j.updated,j.cancel,r.runner_pid,r.runner_identity,r.workload_pid,r.workload_identity,r.exit_code,r.signal,r.detail FROM jobs j JOIN runs r ON r.id=j.run_id WHERE j.id=?",[j],|r|{
        Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,bool>(6)?,r.get::<_,Option<u32>>(7)?,r.get::<_,Option<String>>(8)?,r.get::<_,Option<u32>>(9)?,r.get::<_,Option<String>>(10)?,r.get::<_,Option<i32>>(11)?,r.get::<_,Option<i32>>(12)?,r.get::<_,Option<String>>(13)?))
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
    )) = row
    else {
        bail!("job not found");
    };
    let spec: StartSpec = serde_json::from_str(&spec)?;
    let (attempt, run_status, started_at, ended_at) = db.query_row(
        "SELECT attempt,status,started,ended FROM runs WHERE id=?",
        [&run_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
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
    let sequence = db.query_row("SELECT rowid FROM jobs WHERE id=?", [j], |r| r.get(0))?;
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
