//! Reconnect to recorded handoffs without admitting work or replaying controls.
use super::*;

pub(super) fn describe(value: &mut Value) {
    let state = value["status"].as_str().unwrap_or("");
    let expired = value["expires_at"].as_i64().is_some_and(|t| t <= now())
        && !(state == "resumed" && value["notification_accepted"] == true);
    let action = if expired {
        "host_control"
    } else {
        match state {
            "prepared" => "submit",
            "sealed" => "native_pause",
            "waiting" => "wait",
            "resumed" if value["feedback"].is_object() => "report_then_pause",
            "resumed" if value["notification_accepted"] == true => "inspect_results",
            "resumed" => "await_notification",
            _ => "host_control",
        }
    };
    value["continuation"] = json!({
        "action":action,
        "requires_original_chat":true,
        "can_admit":action=="submit",
        "retry_native_mutation":false,
        "confirmation_revision":if matches!(action,"native_pause"|"report_then_pause") {value["revision"].clone()} else {Value::Null},
    });
    if expired {
        value["next_step"] = json!(
            "This handoff deadline has expired. Inspect the original member results and use host goal controls explicitly. Do not repeat native pause, restore or submission to recover it."
        );
    }
}

pub(crate) fn for_request(db: &Connection, request: &str) -> Result<Vec<Value>> {
    let mut q =
        db.prepare("SELECT id FROM goal_waits WHERE consent_id=? ORDER BY rowid DESC LIMIT 16")?;
    q.query_map([request], |r| r.get::<_, String>(0))?
        .map(|id| brief(db, &id?))
        .collect()
}

impl Store {
    /// A bounded local read, using the freshly registered caller. Never adopts
    /// another request's consent or changes the recorded host binding.
    pub(crate) async fn pending_goal_handoff(
        &self,
        session_key: String,
        observed: Option<(Controller, Goal)>,
    ) -> Result<Option<Value>> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            let key = session_key.clone();
            let observed = observed.clone();
            let result = self.read(move |db| {
                let tx = db.transaction()?;
                let id: Option<String> = tx.query_row(
                    "SELECT id FROM goal_waits WHERE session_key=?1 AND (status IN ('prepared','sealed','pausing','waiting','resuming') OR (status='resumed' AND notified=0)) AND (status<>'prepared' OR expires>?2) ORDER BY rowid LIMIT 1",
                    params![key,now()], |r| r.get(0),
                ).optional()?;
                let Some(id) = id else { return Ok(None) };
                let record = read(&tx, &id)?;
                let session = crate::session::read_session(&tx, &key)?;
                let mut value = brief(&tx, &id)?;
                let same_goal = match observed.as_ref() {
                    Some((controller, goal)) => record.expected.same_identity(goal)
                        && serde_json::to_value(controller)? == serde_json::to_value(&record.controller)?,
                    None => false,
                };
                let same_binding = serde_json::to_string(&json!({"registration":session.registration,"endpoint_identity":session.endpoint_identity}))? == record.session_snapshot
                    && session.connected;
                let status = observed.as_ref().map(|(_, goal)| goal.status.as_str());
                value["goal_status"] = json!(status);
                if !same_goal || !same_binding || !matches!(status, Some("active" | "paused")) {
                    value["continuation"]["action"] = json!("host_control");
                    value["continuation"]["can_admit"] = json!(false);
                    value["continuation"]["confirmation_revision"] = Value::Null;
                    value["next_step"] = json!("The original host goal or binding is unavailable, changed or inactive. Retain these request/Job/Run/wait IDs, inspect results and use the original host controls explicitly. Do not submit, re-prompt, overwrite a blocker or replay a native mutation.");
                } else if status == Some("paused")
                    && matches!(value["continuation"]["action"].as_str(), Some("native_pause" | "report_then_pause"))
                {
                    value["continuation"]["action"] = json!("confirm_pause");
                    value["next_step"] = json!("The original goal is already paused. Only if this chat received its native pause success acknowledgement, confirm that existing pause with job_goal_wait(confirm_paused=true, expected_revision=this revision). Do not repeat native pause. If its outcome is uncertain, inspect results and use host controls explicitly instead.");
                }
                // The host can start handling a final message before its ACK
                // reaches our transaction. Settle only this local live claim;
                // do not probe/poll the host or mark it accepted ourselves.
                let in_flight = value["continuation"]["action"] == "await_notification"
                    && tx.query_row("SELECT EXISTS(SELECT 1 FROM deliveries d JOIN targets t ON t.id=d.target_id JOIN goal_wait_jobs m ON m.job_id=t.job_id WHERE m.wait_id=? AND d.status='leased' AND d.lease_until>? AND d.expires>?)", params![id,now(),now()], |r| r.get::<_,bool>(0))?;
                Ok(Some((value,in_flight)))
            }).await?;
            match result {
                Some((_, true)) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Some((value, _)) => return Ok(Some(value)),
                None => return Ok(None),
            }
        }
    }
}
