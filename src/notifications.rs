//! Read-only notification diagnostics; host acceptance never proves Agent execution.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(crate) fn summarize(rows: Vec<(String, String, u64)>, problem: Value) -> Value {
    let mut counts = BTreeMap::<String, u64>::new();
    let mut completion = BTreeMap::<String, u64>::new();
    for (status, kind, count) in rows {
        *counts.entry(status.clone()).or_default() += count;
        if kind != "monitor_alert" {
            *completion.entry(status).or_default() += count;
        }
    }
    json!({"counts":counts,"completion":completion,"latest_problem":problem,
        "agent_execution":"unconfirmed"})
}

pub(crate) fn read(db: &Connection, job: &str) -> Result<Value> {
    let mut q = db.prepare("SELECT d.status,e.kind,count(*) FROM deliveries d JOIN targets t ON t.id=d.target_id JOIN events e ON e.id=d.event_id WHERE t.job_id=? GROUP BY d.status,e.kind")?;
    let rows = q
        .query_map([job], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? as u64))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let problem = db.query_row("SELECT d.id,d.status,d.failure_kind,d.last_error,d.expires,d.next_at FROM deliveries d JOIN targets t ON t.id=d.target_id WHERE t.job_id=? AND d.status IN ('retry_wait','dead_letter') ORDER BY d.rowid DESC LIMIT 1", [job], |r| Ok(json!({"delivery_id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"failure_kind":r.get::<_,Option<String>>(2)?,"error":r.get::<_,Option<String>>(3)?,"expires_at":r.get::<_,i64>(4)?,"next_attempt_at":r.get::<_,i64>(5)?}))).optional()?.unwrap_or(Value::Null);
    Ok(summarize(rows, problem))
}

pub(crate) fn context(detail: &mut Value) {
    let terminal = detail["job"]["status"]
        .as_str()
        .is_some_and(crate::core::terminal);
    let n = &detail["notifications"];
    let counts = if terminal {
        &n["completion"]
    } else {
        &n["counts"]
    };
    let has = |s: &str| counts[s].as_u64().unwrap_or(0) > 0;
    let state = if has("dead_letter") {
        "dead_letter"
    } else if has("retry_wait") {
        "retry_wait"
    } else if has("pending") || has("leased") {
        "pending"
    } else if has("accepted_by_host") {
        "accepted_by_host"
    } else if has("notification_recorded") {
        "inbox_only"
    } else if has("cancelled") {
        "cancelled"
    } else {
        "none"
    };
    detail["notifications"]["status"] = json!(state);
    detail["notifications"]["goal"] = crate::output::goal_summary(&detail["goal_wait"]);
    detail["notifications"]["recovery"] = json!({
        "requires_original_caller":true,
        "preview_args":["connect","--recover"],
        "apply_args":["connect","--recover","--apply"],
        "reason":"Use the recorded launcher/profile in the original chat to obtain a fresh caller binding and preview eligible recovery. Accepted/expired events are not recovered; inspect attempt history before an explicit retry. Goal waits retain their original binding and require explicit host goal handling."
    });
}
