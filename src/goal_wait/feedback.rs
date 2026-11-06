//! Bounded temporary goal restoration. Re-pause always belongs to the owning
//! Agent's native host tool; this module never pauses goals through storage.
use super::*;

pub(super) const COOLDOWN: i64 = 60_000;
pub(super) const LIMIT: u32 = 32;

pub(super) fn context(db: &Connection, delivery: &str) -> Result<Value> {
    let marker = format!("coalesced into goal feedback {delivery}");
    let count: u32 = db.query_row("SELECT count(*) FROM deliveries WHERE status='cancelled' AND failure_kind='goal_feedback' AND last_error=?", [&marker], |r| r.get(0))?;
    // Explicit stop-rule evidence sorts first. It remains data, never origin
    // control authority; arbitrary output is not assigned semantic severity.
    let mut q = db.prepare("SELECT e.id,e.job_id,e.run_id,e.seq,e.at,e.payload,d.id FROM deliveries d JOIN events e ON e.id=d.event_id WHERE d.status='cancelled' AND d.failure_kind='goal_feedback' AND d.last_error=? ORDER BY (json_extract(e.payload,'$.rule.action')='stop' OR json_extract(e.payload,'$.evidence.source_rule.action')='stop') DESC,e.seq DESC LIMIT 4")?;
    let items = q.query_map([marker], |r| {
        let payload: String = r.get(5)?;
        // Excerpts are explicitly incomplete JSON text, not executable fields.
        let excerpt: String = payload.chars().take(512).collect();
        Ok(json!({"event_id":r.get::<_,String>(0)?,"job_id":r.get::<_,String>(1)?,"run_id":r.get::<_,String>(2)?,"sequence":r.get::<_,i64>(3)?,"at":r.get::<_,i64>(4)?,"delivery_id":r.get::<_,String>(6)?,"evidence_excerpt":excerpt,"truncated":payload.chars().count()>512}))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"coalesced_count":count,"items":items,"truncated":count as usize>items.len(),"untrusted_output":true}),
    )
}

#[derive(Clone)]
pub(super) struct Selection {
    pub event: String,
    pub delivery: String,
    sequence: i64,
}

pub(super) fn select(db: &Connection, record: &Wait) -> Result<Option<Selection>> {
    if record.feedback_count >= LIMIT || now().saturating_sub(record.feedback_last_at) < COOLDOWN {
        return Ok(None);
    }
    Ok(db.query_row(
        "SELECT e.id,d.id,e.seq FROM deliveries d JOIN events e ON e.id=d.event_id JOIN targets t ON t.id=d.target_id JOIN goal_wait_jobs m ON m.job_id=e.job_id JOIN jobs j ON j.id=m.job_id WHERE m.wait_id=?1 AND m.run_id=e.run_id AND j.run_id=m.run_id AND j.status NOT IN ('completed','failed','cancelled','timeout','orphaned') AND e.kind='monitor_alert' AND t.active=1 AND t.session_key=?2 AND d.expires>?3 AND d.status IN ('pending','retry_wait') AND d.next_at<=?3 ORDER BY e.seq DESC,d.rowid LIMIT 1",
        params![record.id,record.session_key,now()], |r| Ok(Selection { event:r.get(0)?,delivery:r.get(1)?,sequence:r.get(2)? }),
    ).optional()?)
}

pub(super) async fn begin(
    store: &Store,
    record: &Wait,
    selection: &Selection,
) -> Result<Option<Wait>> {
    let record = record.clone();
    let selection = selection.clone();
    store.call(move|db| {
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current=read(&tx,&record.id)?;
        if current.revision!=record.revision || current.status!="waiting" {
            return Ok(None);
        }
        let Some(candidate)=select(&tx,&current)? else {return Ok(None)};
        if candidate.delivery!=selection.delivery { return Ok(None) }
        let deadline=now()+PAUSE_ACK_TIMEOUT;
        tx.execute("UPDATE goal_waits SET status='resuming',revision=revision+1,updated=?,expires=?,next_at=?,feedback_event=?,feedback_delivery=?,feedback_count=feedback_count+1,feedback_last_at=? WHERE id=? AND revision=?",params![now(),deadline,now(),selection.event,selection.delivery,now(),record.id,record.revision])?;
        audit(&tx,&record.id,record.revision+1,"resuming",&format!("temporary feedback cycle {}; event {}; delivery {}",record.feedback_count+1,selection.event,selection.delivery))?;
        // Coalesce older unsent alerts, preserving all IDs/events/attempt history.
        tx.execute("UPDATE deliveries SET status='cancelled',last_error=?4,failure_kind='goal_feedback' WHERE status IN ('pending','retry_wait','dead_letter') AND id<>?1 AND event_id IN (SELECT e.id FROM events e JOIN goal_wait_jobs m ON m.job_id=e.job_id AND m.run_id=e.run_id WHERE m.wait_id=?2 AND e.kind='monitor_alert' AND e.seq<=?3)",params![selection.delivery,record.id,selection.sequence,format!("coalesced into goal feedback {}",selection.delivery)])?;
        let result=read(&tx,&record.id)?;
        tx.commit()?;
        Ok(Some(result))
    }).await
}

/// Before transmitting an intermediate alert, all-member completion takes
/// priority. An already transmitted/accepted effect is never recalled.
pub(super) async fn promote_terminal(
    store: &Store,
    record: &Wait,
    claim: &crate::delivery::Claim,
) -> Result<bool> {
    let record = record.clone();
    let claim = claim.clone();
    store.call(move|db| {
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current=read(&tx,&record.id)?;
        if current.status!="resumed" || current.revision!=record.revision || current.feedback_delivery.as_deref()!=Some(&claim.delivery.id) {return Ok(false)}
        let mut q=tx.prepare("SELECT job_id,run_id FROM goal_wait_jobs WHERE wait_id=?")?;
        let ids=q.query_map([&record.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        let mut jobs=Vec::new();
        for (id,run) in ids {
            let j=read_job(&tx,&id)?;
            ensure!(j.run_id==run,"goal member run changed");
            if !j.status.is_terminal() {return Ok(false)}
            jobs.push(j);
        }
        let representative=jobs.iter().max_by_key(|j|j.run.ended_at.unwrap_or(j.updated_at)).context("missing feedback members")?;
        let expiry=representative.run.ended_at.unwrap_or(representative.updated_at).saturating_add(DAY);
        let changed=tx.execute("UPDATE deliveries SET status='cancelled',token=NULL,lease_until=NULL,last_error='all members completed before feedback transmission',failure_kind='superseded' WHERE id=? AND status='leased' AND token=? AND lease_until>?",params![claim.delivery.id,claim.token,now()])?;
        if changed!=1 {return Ok(false)}
        tx.execute("UPDATE delivery_attempts SET ended=?,outcome='superseded',error='all members completed before feedback transmission' WHERE delivery_id=? AND token=? AND ended IS NULL",params![now(),claim.delivery.id,claim.token])?;
        tx.execute("UPDATE goal_waits SET revision=revision+1,updated=?,expires=?,feedback_event=NULL,feedback_delivery=NULL,detail='final completion superseded unsent feedback' WHERE id=? AND revision=?",params![now(),expiry,record.id,record.revision])?;
        merge_terminal(&tx,&record.id,&representative.job_id,&record.session_key)?;
        audit(&tx,&record.id,record.revision+1,"resumed","final completion superseded unsent feedback; goal already restored")?;
        tx.commit()?;
        Ok(true)
    }).await
}

pub(super) fn merge_terminal(db: &Connection, wait: &str, job: &str, session: &str) -> Result<()> {
    db.execute("UPDATE deliveries SET status='cancelled',failure_kind='goal_wait',last_error='combined into one goal-wait completion' WHERE status IN ('pending','retry_wait','dead_letter') AND target_id IN (SELECT t.id FROM targets t JOIN goal_wait_jobs m ON m.job_id=t.job_id WHERE m.wait_id=? AND (m.job_id<>? OR t.session_key<>?))",params![wait,job,session])?;
    Ok(())
}
