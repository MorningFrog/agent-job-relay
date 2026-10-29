//! Required Desktop goal handoff. This owns host control, never job execution.
use crate::{
    core::*,
    goal_control::{Controller, Goal},
    session::AgentSession,
    storage::{Store, read_job},
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

const OPEN: &str = "'prepared','sealed','pausing','waiting','resuming'";
const DAY: i64 = 86_400_000;
const PAUSE_ACK_TIMEOUT: i64 = 300_000;

#[derive(Clone)]
struct Wait {
    id: String,
    session_key: String,
    controller: Controller,
    expected: Goal,
    paused: Option<Goal>,
    status: String,
    revision: i64,
    expires: i64,
    session_snapshot: String,
}
fn read(db: &Connection, id: &str) -> Result<Wait> {
    let (session_key, controller, expected, paused, status, revision, expires) = db.query_row(
        "SELECT session_key,controller,expected,paused,status,revision,expires FROM goal_waits WHERE id=?", [id],
        |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?)))?;
    let session_snapshot = db.query_row(
        "SELECT session_snapshot FROM goal_waits WHERE id=?",
        [id],
        |r| r.get(0),
    )?;
    Ok(Wait {
        id: id.into(),
        session_key,
        controller: serde_json::from_str(&controller)?,
        expected: serde_json::from_str(&expected)?,
        paused: paused.map(|v| serde_json::from_str(&v)).transpose()?,
        status,
        revision,
        expires,
        session_snapshot,
    })
}
fn audit(db: &Connection, id: &str, revision: i64, status: &str, detail: &str) -> Result<()> {
    db.execute(
        "INSERT INTO goal_wait_audits(wait_id,revision,at,status,detail) VALUES(?,?,?,?,?)",
        params![id, revision, now(), status, detail],
    )?;
    Ok(())
}
fn brief(db: &Connection, id: &str) -> Result<Value> {
    let mut value = db.query_row("SELECT id,session_key,status,revision,created,updated,expires,attempts,detail,notified FROM goal_waits WHERE id=?",[id],|r|Ok(json!({"goal_wait_id":r.get::<_,String>(0)?,"session_key":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"revision":r.get::<_,i64>(3)?,"created_at":r.get::<_,i64>(4)?,"updated_at":r.get::<_,i64>(5)?,"expires_at":r.get::<_,i64>(6)?,"attempts":r.get::<_,i64>(7)?,"detail":r.get::<_,String>(8)?,"notification_accepted":r.get::<_,bool>(9)?})))?;
    if value["expires_at"] == i64::MAX {
        value["expires_at"] = Value::Null;
    }
    let mut q =
        db.prepare("SELECT job_id,run_id FROM goal_wait_jobs WHERE wait_id=? ORDER BY rowid")?;
    value["jobs"] = json!(
        q.query_map([id], |r| Ok(
            json!({"job_id":r.get::<_,String>(0)?,"run_id":r.get::<_,String>(1)?})
        ))?
        .collect::<rusqlite::Result<Vec<_>>>()?
    );
    value["next_step"] = json!(match value["status"].as_str().unwrap_or("") {
        "prepared" => "Submit once with this binding; no goal has been paused yet.",
        "sealed" =>
            "Work is admitted through the explicit native-pause protocol. Only a user/system request can authorize this chat's native update_goal(status='paused'); plugin delegation alone cannot. After its acknowledgement, confirm with job_goal_wait(confirm_paused=true, expected_revision=this revision). If authority or native pause is unavailable, cancel this wait and report the host limitation once; do not ask for a special phrase, mark the goal blocked, poll or resubmit jobs.",
        "waiting" => "End this turn. Relay will handle completion without Agent polling.",
        "resumed" =>
            "Inspect the member results; host acceptance is not confirmed Agent execution.",
        "cancelled" | "suppressed" | "attention" =>
            "Inspect member results and use host goal controls explicitly. Commands were not replayed; Relay will not reactivate this wait.",
        _ =>
            "Work is admitted, but goal pause is not yet acknowledged. Inspect this wait before claiming automatic waiting.",
    });
    value["host_pause_required"] = json!(value["status"] == "sealed");
    Ok(value)
}
fn view(db: &Connection, id: &str) -> Result<Value> {
    let mut result = brief(db, id)?;
    let mut q=db.prepare("SELECT sequence,revision,at,status,detail FROM goal_wait_audits WHERE wait_id=? ORDER BY sequence DESC LIMIT 32")?;
    result["audits"]=json!(q.query_map([id],|r|Ok(json!({"sequence":r.get::<_,i64>(0)?,"revision":r.get::<_,i64>(1)?,"at":r.get::<_,i64>(2)?,"status":r.get::<_,String>(3)?,"detail":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?);
    Ok(result)
}
pub(crate) fn summary_for_job(db: &Connection, job: &str) -> Result<Option<Value>> {
    let id: Option<String> = db
        .query_row(
            "SELECT wait_id FROM goal_wait_jobs WHERE job_id=?",
            [job],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| brief(db, &id)).transpose()
}
pub(crate) fn check_retry(db: &Connection, job: &str, wait: Option<&str>) -> Result<()> {
    if let Some(wait) = wait {
        let old: Option<String> = db
            .query_row(
                "SELECT wait_id FROM goal_wait_jobs WHERE job_id=?",
                [job],
                |r| r.get(0),
            )
            .optional()?;
        ensure!(
            old.as_deref() == Some(wait),
            "idempotent job belongs to a different goal wait; its ownership was not moved"
        );
    }
    Ok(())
}
pub(crate) fn attach(
    db: &Connection,
    wait: &str,
    job: &str,
    run: &str,
    spec: &StartSpec,
) -> Result<()> {
    let record = read(db, wait)?;
    let consent: Option<String> = db.query_row(
        "SELECT consent_id FROM goal_waits WHERE id=?",
        [wait],
        |r| r.get(0),
    )?;
    if let Some(consent) = consent {
        crate::goal_consent::consume(
            db,
            &consent,
            &record.session_key,
            &record.expected,
            &record.controller,
            &spec.cwd,
        )?;
    }
    ensure!(
        record.controller.host_pause,
        "legacy goal binding cannot acknowledge an originating-host pause; prepare a new binding"
    );
    ensure!(
        record.status == "prepared" && record.expires > now(),
        "goal wait is sealed or expired; retry accepted keys unchanged and use a fresh wait only for unsubmitted work"
    );
    ensure!(
        spec.session.as_deref() == Some(&record.session_key),
        "goal wait belongs to a different caller binding"
    );
    ensure!(
        spec.idempotency_key.is_some(),
        "goal-wait submissions require a stable idempotency key"
    );
    ensure!(
        !spec.monitor_rules.iter().any(|r| r.notify_agent),
        "goal waiting groups terminal notifications; running rules may alert locally but cannot notify_agent"
    );
    let count: i64 = db.query_row(
        "SELECT count(*) FROM goal_wait_jobs WHERE wait_id=?",
        [wait],
        |r| r.get(0),
    )?;
    ensure!(count < 16, "goal wait member limit (16) reached");
    db.execute(
        "INSERT INTO goal_wait_jobs(job_id,run_id,wait_id) VALUES(?,?,?)",
        params![job, run, wait],
    )?;
    Ok(())
}

impl Store {
    pub async fn prepare_goal_wait(
        &self,
        session_key: String,
        controller: Controller,
        expected: Goal,
    ) -> Result<Value> {
        self.prepare_goal_wait_scoped(session_key, controller, expected, None)
            .await
    }
    pub async fn prepare_goal_wait_scoped(
        &self,
        session_key: String,
        controller: Controller,
        expected: Goal,
        consent_id: Option<String>,
    ) -> Result<Value> {
        validate_id(&session_key)?;
        ensure!(
            controller.host_pause,
            "goal waiting requires the originating-host pause protocol"
        );
        controller.validate_async().await?;
        ensure!(
            expected.thread_id == controller.thread_id && expected.status == "active",
            "goal wait requires the exact active goal"
        );
        let current = controller
            .read()
            .await?
            .context("goal was removed before preparation")?;
        ensure!(
            expected.same_identity(&current) && current.status == "active",
            "goal changed before preparation"
        );
        let s = self.session(session_key.clone()).await?;
        ensure!(
            s.connected
                && s.expires_at > now()
                && s.registration.adapter == crate::session::Adapter::DesktopExperimental
                && s.registration.session_id == controller.thread_id
                && s.registration.caller_thread_id.as_deref() == Some(&controller.thread_id),
            "goal waiting requires the actual Desktop caller binding"
        );
        crate::adapters::probe(&s).await?;
        let session_snapshot = serde_json::to_string(
            &json!({"registration":s.registration,"endpoint_identity":s.endpoint_identity}),
        )?;
        self.call(move|db|{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let t=now();
            if let Some(consent)=&consent_id {crate::goal_consent::check_scope(&tx,consent,&session_key,&current,&controller,None)?;}
            let expired={let mut q=tx.prepare("SELECT id FROM goal_waits WHERE status='prepared' AND expires<=? AND NOT EXISTS(SELECT 1 FROM goal_wait_jobs m WHERE m.wait_id=goal_waits.id)")?;q.query_map([t],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?};
            for id in expired { close_tx(&tx,&read(&tx,&id)?,"cancelled","unused binding expired")?; }
            let busy:Option<String>=tx.query_row(&format!("SELECT w.id FROM goal_waits w JOIN sessions a ON a.key=w.session_key JOIN sessions b ON b.key=? WHERE (w.status IN ({OPEN}) OR (w.status='resumed' AND w.notified=0)) AND a.agent=b.agent AND a.logical=b.logical AND a.host=b.host LIMIT 1"),[&session_key],|r|r.get(0)).optional()?;
            if let Some(id)=busy {
                let old=read(&tx,&id)?;
                let old_consent:Option<String>=tx.query_row("SELECT consent_id FROM goal_waits WHERE id=?",[&id],|r|r.get(0))?;
                if old.status=="prepared" && old.session_key==session_key && old_consent==consent_id && old.session_snapshot==session_snapshot && old.expected.same_identity(&current) && serde_json::to_value(&old.controller)?==serde_json::to_value(&controller)? {
                    let result=brief(&tx,&id)?;tx.commit()?;return Ok(result);
                }
                anyhow::bail!("this logical chat already has goal wait {id}; inspect or cancel that exact wait before creating another");
            }
            let count:i64=tx.query_row("SELECT count(*) FROM goal_waits",[],|r|r.get(0))?;
            ensure!(count<10000,"goal wait history quota (10000) reached");
            let id=id();
            let actual=crate::session::read_session(&tx,&session_key)?;
            ensure!(serde_json::to_string(&json!({"registration":actual.registration,"endpoint_identity":actual.endpoint_identity}))?==session_snapshot && actual.connected && actual.expires_at>t,"caller binding changed during goal preparation");
            tx.execute("INSERT INTO goal_waits(id,session_key,controller,expected,session_snapshot,status,created,updated,expires,next_at) VALUES(?,?,?,?,?,'prepared',?,?,?,?)",params![id,session_key,serde_json::to_string(&controller)?,serde_json::to_string(&current)?,session_snapshot,t,t,t+300000,t])?;
            tx.execute("UPDATE goal_waits SET consent_id=? WHERE id=?",params![consent_id,id])?;
            audit(&tx,&id,1,"prepared","current-terminal active goal requires handoff; goal unchanged")?;
            let result=brief(&tx,&id)?;tx.commit()?;Ok(result)
        }).await
    }
    pub async fn goal_wait(
        &self,
        id: String,
        cancel: bool,
        expected_revision: Option<i64>,
    ) -> Result<Value> {
        validate_id(&id)?;
        if !cancel {
            return self.read(move |db| view(db, &id)).await;
        }
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if cancel {
                let record = read(&tx, &id)?;
                ensure!(
                    expected_revision == Some(record.revision),
                    "goal wait revision changed; inspect before cancelling"
                );
                ensure!(
                    matches!(
                        record.status.as_str(),
                        "prepared" | "sealed" | "pausing" | "waiting" | "resuming" | "resumed"
                    ),
                    "goal wait is already closed"
                );
                close_tx(
                    &tx,
                    &record,
                    "cancelled",
                    "explicit wait cancellation; no further host controls scheduled; in-flight effects cannot be recalled",
                )?;
            }
            let result = view(&tx, &id)?;
            tx.commit()?;
            Ok(result)
        })
        .await
    }

    /// The submitting Agent must first use its own native goal tool, so the
    /// owning runtime and Desktop receive the pause event. A separate app-server
    /// can read/write the same DB without notifying either of them.
    pub async fn confirm_goal_pause(
        &self,
        id: String,
        expected_revision: Option<i64>,
    ) -> Result<Value> {
        validate_id(&id)?;
        let key = id.clone();
        let record = self.read(move |db| read(db, &key)).await?;
        ensure!(
            record.controller.host_pause,
            "legacy wait has no originating-host pause acknowledgement"
        );
        // Lost confirmation responses are safe to retry with the original revision.
        if record.status == "waiting" && expected_revision == Some(record.revision - 1) {
            return self.goal_wait(id, false, None).await;
        }
        ensure!(
            record.status == "sealed" && expected_revision == Some(record.revision),
            "goal wait phase or revision changed; inspect before confirming pause"
        );
        ensure!(
            record.expires > now(),
            "host pause acknowledgement expired; use host goal controls explicitly"
        );
        let session = self.session(record.session_key.clone()).await?;
        binding_matches(&record, &session)?;
        let paused = record
            .controller
            .read()
            .await?
            .context("goal removed before pause acknowledgement")?;
        ensure!(
            record.expected.same_identity(&paused) && paused.status == "paused",
            "originating-host pause is not visible for the original goal; do not claim waiting"
        );
        let expected = record.clone();
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let current = read(&tx, &expected.id)?;
            ensure!(current.status == "sealed" && current.revision == expected.revision && current.expires > now(), "goal wait changed before pause confirmation");
            binding_matches(&current, &crate::session::read_session(&tx, &current.session_key)?)?;
            tx.execute("UPDATE goal_waits SET status='waiting',revision=revision+1,paused=?,updated=?,next_at=?,expires=? WHERE id=? AND revision=?",params![serde_json::to_string(&paused)?,now(),now(),i64::MAX,current.id,current.revision])?;
            audit(&tx,&current.id,current.revision+1,"waiting","originating Agent acknowledged native host pause; original paused goal verified")?;
            let result = view(&tx, &current.id)?;
            tx.commit()?;
            Ok(result)
        }).await
    }
}

fn close_tx(db: &Connection, record: &Wait, status: &str, detail: &str) -> Result<bool> {
    let n=db.execute("UPDATE goal_waits SET status=?,revision=revision+1,updated=?,detail=? WHERE id=? AND revision=?",params![status,now(),detail,record.id,record.revision])?;
    if n == 0 {
        return Ok(false);
    }
    audit(db, &record.id, record.revision + 1, status, detail)?;
    // In-flight effects are not recalled. deliver_claim rechecks wait state.
    db.execute("UPDATE deliveries SET status='cancelled',failure_kind='goal_wait',last_error=?,token=NULL,lease_until=NULL WHERE target_id IN (SELECT t.id FROM targets t JOIN goal_wait_jobs m ON m.job_id=t.job_id WHERE m.wait_id=?) AND (status IN ('pending','retry_wait','dead_letter') OR (status='leased' AND lease_until<=?))",params![detail,record.id,now()])?;
    db.execute("UPDATE delivery_attempts SET ended=?,outcome='cancelled',error=? WHERE ended IS NULL AND delivery_id IN (SELECT d.id FROM deliveries d JOIN targets t ON t.id=d.target_id JOIN goal_wait_jobs m ON m.job_id=t.job_id WHERE m.wait_id=? AND d.status='cancelled' AND d.failure_kind='goal_wait')",params![now(),detail,record.id])?;
    Ok(true)
}
pub(crate) fn cancel_for_consent(db: &Connection, consent: &str, detail: &str) -> Result<()> {
    let ids = {
        let mut q=db.prepare("SELECT id FROM goal_waits WHERE consent_id=? AND (status IN ('prepared','sealed','pausing','waiting','resuming') OR (status='resumed' AND notified=0))")?;
        q.query_map([consent], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for id in ids {
        close_tx(db, &read(db, &id)?, "cancelled", detail)?;
    }
    Ok(())
}
pub(crate) fn cancel_job_tx(db: &Connection, job: &str) -> Result<()> {
    let id: Option<String> = db
        .query_row(
            "SELECT wait_id FROM goal_wait_jobs WHERE job_id=?",
            [job],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = id {
        let record = read(db, &id)?;
        if matches!(
            record.status.as_str(),
            "prepared" | "sealed" | "pausing" | "waiting" | "resuming" | "resumed"
        ) {
            close_tx(
                db,
                &record,
                "cancelled",
                "member stopped manually; automatic goal continuation cancelled",
            )?;
        }
    }
    Ok(())
}
pub(crate) fn restore_fence(db: &Connection) -> Result<()> {
    let mut q=db.prepare("SELECT id FROM goal_waits WHERE status IN ('prepared','sealed','pausing','waiting','resuming','resumed')")?;
    let ids = q
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(q);
    for id in ids {
        let record = read(db, &id)?;
        close_tx(
            db,
            &record,
            "cancelled",
            "backup restored; host control intents were not replayed",
        )?;
    }
    Ok(())
}
pub(crate) fn target_allowed(db: &Connection, job: &str, session: &str) -> Result<bool> {
    let original:Option<String>=db.query_row("SELECT w.session_key FROM goal_wait_jobs m JOIN goal_waits w ON w.id=m.wait_id WHERE m.job_id=?",[job],|r|r.get(0)).optional()?;
    Ok(original.is_none_or(|key| key == session))
}
async fn close(store: &Store, record: &Wait, state: &str, detail: &str) -> Result<()> {
    let record = record.clone();
    let state = state.to_string();
    let detail = detail.chars().take(1024).collect::<String>();
    store
        .call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            close_tx(&tx, &record, &state, &detail)?;
            tx.commit()?;
            Ok(())
        })
        .await
}
async fn phase(
    store: &Store,
    record: &Wait,
    status: &str,
    paused: Option<&Goal>,
) -> Result<Option<Wait>> {
    let record = record.clone();
    let status = status.to_string();
    let paused = paused.map(serde_json::to_string).transpose()?;
    store.call(move|db|{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n=tx.execute("UPDATE goal_waits SET status=?,revision=revision+1,paused=coalesce(?,paused),updated=?,next_at=? WHERE id=? AND revision=?",params![status,paused,now(),now(),record.id,record.revision])?;
        if n==0 {return Ok(None);}
        audit(&tx,&record.id,record.revision+1,&status,"host control phase")?;
        let record=read(&tx,&record.id)?;tx.commit()?;Ok(Some(record))
    }).await
}
pub async fn seal(store: &Store, id: &str) -> Result<Value> {
    validate_id(id)?;
    let key = id.to_string();
    store.call(move|db|{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let record=read(&tx,&key)?;
        let count:i64=tx.query_row("SELECT count(*) FROM goal_wait_jobs WHERE wait_id=?",[&key],|r|r.get(0))?;
        if record.status=="prepared" && count>0 {
            tx.execute("UPDATE goal_waits SET status='sealed',revision=revision+1,updated=?,expires=?,next_at=? WHERE id=?",params![now(),now()+PAUSE_ACK_TIMEOUT,now()+PAUSE_ACK_TIMEOUT,key])?;
            audit(&tx,&key,record.revision+1,"sealed","accepted member set frozen")?;
        }
        tx.commit()?;Ok(())
    }).await?;
    progress(store, id).await?;
    store.goal_wait(id.into(), false, None).await
}

async fn retry_read(store: &Store, record: &Wait, error: anyhow::Error) -> Result<()> {
    let id = record.id.clone();
    let revision = record.revision;
    let detail = format!("goal observation unavailable: {error:#}")
        .chars()
        .take(1024)
        .collect::<String>();
    store.call(move|db|{
        db.execute("UPDATE goal_waits SET next_at=?+min(300000,1000*(1<<min(attempts,8))),attempts=attempts+1,detail=? WHERE id=? AND revision=?",params![now(),detail,id,revision])?;Ok(())
    }).await
}
fn binding_matches(record: &Wait, session: &AgentSession) -> Result<()> {
    ensure!(
        serde_json::to_string(
            &json!({"registration":session.registration,"endpoint_identity":session.endpoint_identity})
        )? == record.session_snapshot,
        "original host registration changed; goal wait was not moved"
    );
    ensure!(
        session.connected
            && session.key == record.session_key
            && session.registration.adapter == crate::session::Adapter::DesktopExperimental
            && session.registration.session_id == record.controller.thread_id
            && session.registration.caller_thread_id.as_deref()
                == Some(&record.controller.thread_id),
        "originating session was invalidated or replaced"
    );
    let endpoint = session
        .registration
        .endpoint
        .as_ref()
        .context("original Desktop endpoint missing")?;
    ensure!(
        session.endpoint_identity.as_ref() == Some(&crate::platform::endpoint_identity(endpoint)?),
        "original Desktop endpoint changed; goal wait is not moved implicitly"
    );
    Ok(())
}
/// Recheck local cancellation/invalidation after slow controller startup/read,
/// immediately before transmission. The remaining external check/write race
/// requires conditional host RPCs and cannot be closed by a local transaction.
async fn mutation_allowed(store: &Store, record: &Wait) -> Result<bool> {
    let record = record.clone();
    store
        .read(move |db| {
            let current = read(db, &record.id)?;
            if current.revision != record.revision || current.status != record.status {
                return Ok(false);
            }
            let session = crate::session::read_session(db, &record.session_key)?;
            Ok(binding_matches(&record, &session).is_ok())
        })
        .await
}
pub async fn progress(store: &Store, id: &str) -> Result<()> {
    validate_id(id)?;
    // Cross-task/process fencing, without holding a SQLite transaction over I/O.
    let dir = store.config.root.join("goal-waits");
    crate::platform::secure_dir(&dir)?;
    let lock = crate::platform::private_file(&dir.join(format!("{id}.lock")))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(()),
        Err(e) => return Err(e.into()),
    }
    let key = id.to_string();
    let record = store.read(move |db| read(db, &key)).await?;
    if !record.controller.host_pause
        && matches!(
            record.status.as_str(),
            "prepared" | "sealed" | "pausing" | "waiting" | "resuming"
        )
    {
        return close(store, &record, "attention", "legacy storage-only pause did not acknowledge the owning host; use host goal controls explicitly").await;
    }
    match record.status.as_str() {
        "resumed" if record.expires <= now() => {
            return close(
                store,
                &record,
                "attention",
                "goal restored but notification expired; inspect results manually",
            )
            .await;
        }
        "pausing" | "resuming" => {
            return close(
                store,
                &record,
                "attention",
                "host mutation acknowledgement was lost; automatic control was not replayed",
            )
            .await;
        }
        "sealed" => {
            if record.expires <= now() {
                close(store, &record, "attention", "originating-host pause was not confirmed within five minutes; admitted commands were retained; use host goal controls explicitly").await?;
            }
        }
        "waiting" => {
            let key = record.id.clone();
            let jobs = store
                .read(move |db| {
                    let mut q = db.prepare(
                        "SELECT job_id,run_id FROM goal_wait_jobs WHERE wait_id=? ORDER BY rowid",
                    )?;
                    let ids = q
                        .query_map([key], |r| {
                            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                        })?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    ids.into_iter()
                        .map(|(id, run)| {
                            let j = read_job(db, &id)?;
                            ensure!(j.run_id == run, "goal member run changed");
                            Ok(j)
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .await?;
            if jobs.is_empty() || jobs.iter().any(|j| !terminal(&j.status)) {
                let id = record.id.clone();
                let revision = record.revision;
                store
                    .call(move |db| {
                        db.execute(
                            "UPDATE goal_waits SET next_at=? WHERE id=? AND revision=?",
                            params![now() + 1000, id, revision],
                        )?;
                        Ok(())
                    })
                    .await?;
                return Ok(());
            }
            let representative = jobs
                .iter()
                .max_by_key(|j| j.run.ended_at.unwrap_or(j.updated_at))
                .context("missing member")?;
            let expiry = representative
                .run
                .ended_at
                .unwrap_or(representative.updated_at)
                .saturating_add(DAY);
            if now() >= expiry {
                return close(
                    store,
                    &record,
                    "attention",
                    "completion delivery expired; use host goal controls explicitly",
                )
                .await;
            }
            let session = store.session(record.session_key.clone()).await?;
            if !session.connected
                || serde_json::to_string(
                    &json!({"registration":session.registration,"endpoint_identity":session.endpoint_identity}),
                )? != record.session_snapshot
            {
                return close(store,&record,"suppressed","originating host registration was changed or invalidated; goal waiting was not moved").await;
            }
            if let Err(e) = binding_matches(&record, &session) {
                return retry_read(store, &record, e).await;
            }
            if let Err(error) = record.controller.validate_async().await {
                return close(
                    store,
                    &record,
                    "attention",
                    &format!("selected goal controller changed: {error:#}"),
                )
                .await;
            }
            let paused = record
                .paused
                .as_ref()
                .context("missing acknowledged pause")?;
            let current = match record.controller.read().await {
                Ok(Some(goal)) if paused.unchanged_pause(&goal) => goal,
                Ok(_) => {
                    return close(
                        store,
                        &record,
                        "suppressed",
                        "goal removed, replaced, resumed or observably modified while waiting",
                    )
                    .await;
                }
                Err(e) => return retry_read(store, &record, e).await,
            };
            let Some(record) = phase(store, &record, "resuming", None).await? else {
                return Ok(());
            };
            match record
                .controller
                .set_status("active", &current, mutation_allowed(store, &record))
                .await
            {
                Ok(Some(active)) if current.same_identity(&active) && active.status == "active" => {
                    let job = representative.job_id.clone();
                    let record = record.clone();
                    store.call(move|db|{
                        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                        let n=tx.execute("UPDATE goal_waits SET status='resumed',revision=revision+1,updated=?,expires=?,detail='goal restored; host notification pending' WHERE id=? AND revision=?",params![now(),expiry,record.id,record.revision])?;
                        if n==1 {
                            audit(&tx,&record.id,record.revision+1,"resumed","status-only restoration acknowledged; accounting preserved")?;
                            tx.execute("UPDATE deliveries SET status='cancelled',failure_kind='goal_wait',last_error='combined into one goal-wait completion' WHERE status IN ('pending','retry_wait','dead_letter') AND target_id IN (SELECT t.id FROM targets t JOIN goal_wait_jobs m ON m.job_id=t.job_id WHERE m.wait_id=? AND (m.job_id<>? OR t.session_key<>?))",params![record.id,job,record.session_key])?;
                        }
                        tx.commit()?;Ok(())
                    }).await?;
                }
                _ => {
                    close(
                        store,
                        &record,
                        "attention",
                        "restore outcome uncertain or goal changed; no automatic replay",
                    )
                    .await?
                }
            }
        }
        _ => {}
    }
    Ok(())
}
pub async fn tick(store: &Store) -> Result<()> {
    let keys=store.read(|db|{let mut q=db.prepare("SELECT id FROM goal_waits WHERE (status IN ('sealed','pausing','waiting','resuming') AND next_at<=?1) OR (status='resumed' AND notified=0 AND expires<=?1) ORDER BY next_at,updated LIMIT 4")?;Ok(q.query_map([now()],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)}).await?;
    for key in keys {
        if let Err(e) = progress(store, &key).await {
            tracing::error!(error=%e,"goal wait reconciliation failed");
        }
    }
    Ok(())
}
pub(crate) async fn worker(store: Store) {
    loop {
        if let Err(error) = tick(&store).await {
            tracing::error!(%error,"goal wait worker failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
/// Called once under daemon exclusivity, before accepting new admissions.
pub(crate) async fn recover_admissions(store: &Store) -> Result<()> {
    store.call(|db| {
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut q=tx.prepare("SELECT id,revision FROM goal_waits WHERE status='prepared' AND EXISTS(SELECT 1 FROM goal_wait_jobs m WHERE m.wait_id=goal_waits.id)")?;
        let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;drop(q);
        for (id,revision) in rows {
            tx.execute("UPDATE goal_waits SET status='sealed',revision=revision+1,updated=?,next_at=?,expires=? WHERE id=?",params![now(),now()+PAUSE_ACK_TIMEOUT,now()+PAUSE_ACK_TIMEOUT,id])?;
            audit(&tx,&id,revision+1,"sealed","interrupted admission recovered; committed members retained without execution replay")?;
        }
        let ids = { let mut q=tx.prepare("SELECT id FROM goal_waits WHERE status IN ('prepared','sealed','pausing','waiting','resuming') OR (status='resumed' AND notified=0)")?;
            q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()? };
        for id in ids {
            let record=read(&tx,&id)?;
            if !record.controller.host_pause {
                close_tx(&tx,&record,"attention","legacy storage-only pause did not acknowledge the owning host; use host goal controls explicitly")?;
            }
        }
        tx.commit()?;Ok(())
    }).await
}

/// A second host-state check closes the common delete/pause-after-restore window.
pub(crate) async fn before_delivery(store: &Store, claim: &crate::delivery::Claim) -> Result<bool> {
    let job = claim.event["job_id"]
        .as_str()
        .context("missing event job")?
        .to_string();
    let record = store
        .read(move |db| {
            let id: Option<String> = db
                .query_row(
                    "SELECT wait_id FROM goal_wait_jobs WHERE job_id=?",
                    [job],
                    |r| r.get(0),
                )
                .optional()?;
            id.map(|id| read(db, &id)).transpose()
        })
        .await?;
    let Some(record) = record else {
        return Ok(true);
    };
    if record.status != "resumed" || record.session_key != claim.session.key {
        return Ok(false);
    }
    if !record.controller.host_pause {
        close(store, &record, "attention", "legacy storage-only pause cannot authorize completion notification; use host goal controls explicitly").await?;
        return Ok(false);
    }
    if serde_json::to_string(
        &json!({"registration":claim.session.registration,"endpoint_identity":claim.session.endpoint_identity}),
    )? != record.session_snapshot
    {
        close(
            store,
            &record,
            "suppressed",
            "original notification binding changed; goal wait was not moved",
        )
        .await?;
        return Ok(false);
    }
    binding_matches(&record, &claim.session)?;
    if let Err(error) = record.controller.validate_async().await {
        close(
            store,
            &record,
            "attention",
            &format!("selected goal controller changed: {error:#}"),
        )
        .await?;
        return Ok(false);
    }
    let paused = record.paused.as_ref().context("missing pause identity")?;
    match record.controller.read().await? {
        Some(current) if paused.same_identity(&current) && current.status == "active" => Ok(true),
        _ => {
            close(
                store,
                &record,
                "suppressed",
                "goal removed, paused or replaced before completion notification",
            )
            .await?;
            Ok(false)
        }
    }
}
pub(crate) fn claim_allowed(db: &Connection, claim: &crate::delivery::Claim) -> Result<bool> {
    Ok(!db.query_row("SELECT EXISTS(SELECT 1 FROM goal_wait_jobs m JOIN goal_waits w ON w.id=m.wait_id WHERE m.job_id=? AND (w.status<>'resumed' OR w.notified<>0 OR w.session_key<>?))",params![claim.event["job_id"].as_str(),claim.session.key],|r|r.get::<_,bool>(0))?)
}
