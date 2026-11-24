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
    set_action(value, action);
    if expired {
        value["next_step"] =
            json!("Handoff expired. Retain the original runs and use the original host controls.");
    }
}

pub(crate) fn set_action(value: &mut Value, action: &str) {
    let (actor, next_step) = match action {
        "submit" => ("agent", "Submit with this original binding."),
        "native_pause" => (
            "original_chat",
            "Native-pause this Goal, then confirm success with job_goal_wait(confirm_paused=true, expected_revision=confirmation_revision). If pause is uncertain, preserve this handoff.",
        ),
        "confirm_pause" => (
            "agent",
            "Confirm the original native success with job_goal_wait(confirm_paused=true, expected_revision=confirmation_revision). If success is uncertain, preserve the handoff and use host controls.",
        ),
        "report_then_pause" => (
            "original_chat",
            "Report this feedback, then native-pause and confirm this same wait/revision even if jobs finished. If pause already succeeded, only confirm it. Preserve uncertain handoffs.",
        ),
        "wait" => (
            "relay",
            "End this turn. Relay waits for the original host turn to finish before restoring the Goal.",
        ),
        "await_notification" => (
            "relay",
            "Yield while Relay settles final delivery. Use any exact-run evidence already received.",
        ),
        "inspect_results" => (
            "agent",
            "Summarize terminal member evidence. Inspect only original jobs whose evidence is needed.",
        ),
        "reconnect_request" => (
            "agent",
            "Reconnect once with the original request_id and workspace.",
        ),
        _ => (
            "original_chat",
            "Retain the original IDs and use the original host controls. Preserve any uncertain handoff.",
        ),
    };
    value["continuation"] = json!({
        "action":action,
        "actor":actor,
        "requires_original_chat":true,
        "can_admit":action=="submit",
        "retry_native_mutation":false,
        "confirmation_revision":if matches!(action,"native_pause"|"confirm_pause"|"report_then_pause") {value["revision"].clone()} else {Value::Null},
    });
    value["next_step"] = json!(next_step);
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
                    set_action(&mut value, "host_control");
                } else if status == Some("paused")
                    && matches!(value["continuation"]["action"].as_str(), Some("native_pause" | "report_then_pause"))
                {
                    set_action(&mut value, "confirm_pause");
                }
                // The host can start handling a final message before its ACK
                // reaches our transaction. Settle only this local live claim;
                // do not probe/poll the host or mark it accepted ourselves.
                value["diagnostics"]["next_action"] = value["continuation"]["action"].clone();
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
