//! Read-only correlation of goal control and delivery evidence. No host probes.
use super::*;

pub(super) fn summarize(db: &Connection, record: &Wait, value: &Value) -> Result<Value> {
    let latest = db.query_row(
        "SELECT sequence,revision,at,status,detail FROM goal_wait_audits WHERE wait_id=? ORDER BY sequence DESC LIMIT 1",
        [&record.id], |r| Ok(json!({"sequence":r.get::<_,i64>(0)?,"revision":r.get::<_,i64>(1)?,"at":r.get::<_,i64>(2)?,"status":r.get::<_,String>(3)?,"detail":r.get::<_,String>(4)?})),
    ).optional()?.unwrap_or(Value::Null);
    let mut q = db.prepare("SELECT d.id,e.id,e.job_id,e.run_id,d.status,d.attempts,d.expires,d.next_at,d.failure_kind,d.last_error FROM deliveries d JOIN events e ON e.id=d.event_id JOIN targets t ON t.id=d.target_id JOIN goal_wait_jobs m ON m.job_id=e.job_id AND m.run_id=e.run_id WHERE m.wait_id=?1 AND t.session_key=?2 AND ((?3 IS NOT NULL AND d.id=?3) OR (?3 IS NULL AND e.kind<>'monitor_alert')) ORDER BY (d.status='cancelled'),d.rowid DESC LIMIT 1")?;
    let delivery = q.query_row(params![record.id,record.session_key,record.feedback_delivery], |r| Ok(json!({
        "delivery_id":r.get::<_,String>(0)?,"event_id":r.get::<_,String>(1)?,"job_id":r.get::<_,String>(2)?,"run_id":r.get::<_,String>(3)?,
        "status":r.get::<_,String>(4)?,"attempts":r.get::<_,u32>(5)?,"expires_at":r.get::<_,i64>(6)?,"next_attempt_at":r.get::<_,i64>(7)?,
        "failure_kind":r.get::<_,Option<String>>(8)?,"error":r.get::<_,Option<String>>(9)?,
    }))).optional()?.unwrap_or(Value::Null);
    let attempt = if let Some(id) = delivery["delivery_id"].as_str() {
        db.query_row("SELECT attempt,started,ended,outcome,error FROM delivery_attempts WHERE delivery_id=? ORDER BY attempt DESC LIMIT 1", [id], |r| Ok(json!({"attempt":r.get::<_,u32>(0)?,"started_at":r.get::<_,i64>(1)?,"ended_at":r.get::<_,Option<i64>>(2)?,"outcome":r.get::<_,Option<String>>(3)?,"error":r.get::<_,Option<String>>(4)?}))).optional()?.unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let next_check: i64 = db.query_row(
        "SELECT next_at FROM goal_waits WHERE id=?",
        [&record.id],
        |r| r.get(0),
    )?;
    Ok(
        json!({"phase":if record.feedback_delivery.is_some() {"feedback"} else {"completion"},
        "control_status":record.status,"next_action":value["continuation"]["action"],
        "last_control":latest,"delivery":delivery,"last_attempt":attempt,
        "next_check_at":if matches!(record.status.as_str(),"waiting"|"sealed") {Some(next_check)} else {None},
        "feedback_cycles_remaining":feedback::LIMIT.saturating_sub(record.feedback_count),
        "final_notification_accepted":value["notification_accepted"],
        "agent_execution":"unconfirmed","retry_native_mutation":false}),
    )
}
