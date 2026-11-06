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
        "desktop_preview_args":["connect","--recover"],
        "desktop_apply_args":["connect","--recover","--apply"],
        "opencode_tool":"relay_recover",
        "inbox":"inspect retained results; no automatic continuation",
        "reason":"Use the original host caller binding to preview eligible recovery: Desktop uses its recorded launcher/profile connect --recover; OpenCode uses relay_recover. Inbox-only hosts inspect retained results. Accepted/expired events are not recovered; inspect attempt history before an explicit retry. Goal waits retain their original binding and require explicit host goal handling."
    });
    detail["notifications"]["recovery"]["plan"] = plan(detail);
}

fn plan(detail: &Value) -> Value {
    let n = &detail["notifications"];
    let goal = &n["goal"];
    let problem = &n["latest_problem"];
    let action = goal["continuation"]["action"].as_str();
    let (action, reason) = if action == Some("await_notification")
        && (goal["diagnostics"]["delivery"]["status"] == "dead_letter"
            || goal["diagnostics"]["delivery"]["failure_kind"] == "expired")
    {
        (
            "host_control",
            "The original goal was restored, but its final notification stopped retrying or expired. Inspect the correlated delivery and attempt, then use original host controls explicitly; do not restore again or rebind this goal wait.",
        )
    } else if let Some(action) = action {
        (
            action,
            match action {
                "submit" | "reconnect_request" => {
                    "Continue the original approved request from its originating chat."
                }
                "native_pause" => {
                    "Finish the authorized native pause in the originating chat, then acknowledge its observed revision."
                }
                "confirm_pause" => {
                    "Confirm only an existing native pause whose success was acknowledged in the originating chat. An uncertain pause must not be repeated."
                }
                "report_then_pause" => {
                    "Report the requested intermediate evidence, then finish native re-pause and revision confirmation on this same wait."
                }
                "wait" | "await_notification" => {
                    "The existing handoff or final delivery is in progress. Yield; no Agent polling or new submission is needed."
                }
                "inspect_results" => {
                    "Inspect the original results and continue through the original request if it has remaining admission budget."
                }
                _ => {
                    "Inspect the original results and use the original host's explicit goal controls. Do not overwrite blockers or repeat uncertain pause/restore operations."
                }
            },
        )
    } else if problem["expires_at"]
        .as_i64()
        .is_some_and(|at| at <= crate::core::now())
        || problem["failure_kind"] == "expired"
    {
        (
            "inspect_expired",
            "Inspect retained results and attempt history. Automatic recovery excludes expired events; an explicit retry changes the expiry and needs a deliberate decision.",
        )
    } else if problem["failure_kind"] == "rebind_required" {
        (
            "recover_binding",
            "From the original chat, preview recovery using a fresh caller binding, review eligible deliveries, then explicitly apply. Preserve original delivery IDs and expiry.",
        )
    } else if problem["failure_kind"] == "permanent" || n["status"] == "dead_letter" {
        (
            "inspect_failure",
            "Inspect the failed attempt and correct its host/configuration cause before deciding on an explicit delivery retry.",
        )
    } else {
        match n["status"].as_str() {
            Some("pending" | "retry_wait") => (
                "wait_delivery",
                "Delivery is pending or scheduled for bounded automatic retry. Yield and retain the original task; no resubmission is needed.",
            ),
            Some("accepted_by_host") => (
                "inspect_results",
                "The host accepted this notification. Inspect results if handling that event; Agent execution remains unconfirmed and automatic recovery excludes accepted events.",
            ),
            Some("inbox_only") => (
                "inspect_inbox",
                "Inspect retained results or the originating session inbox. This adapter does not automatically continue an Agent.",
            ),
            Some("cancelled") => (
                "inspect_results",
                "Automatic notification is closed. Retained results remain available.",
            ),
            _ if detail["feedback"]["status"] == "binding_unavailable" => (
                "recover_binding",
                "Preview notification recovery from the original chat with a fresh caller binding.",
            ),
            _ => (
                "inspect_results",
                "Inspect the original task and its configured notification capability.",
            ),
        }
    };
    json!({"action":action,"reason":reason,"requires_original_caller":true,
        "job_id":detail["job"]["job_id"],"run_id":detail["job"]["run_id"],
        "delivery_id":problem["delivery_id"],"expires_at":problem["expires_at"],
        "next_attempt_at":problem["next_attempt_at"],"goal_wait_id":goal["goal_wait_id"],
        "request_id":goal["request_id"],"expected_revision":goal["revision"],
        "confirmation_revision":goal["continuation"]["confirmation_revision"],
        "automatic_mutation":false,"replay_execution":false})
}

pub(crate) fn profile_commands(detail: &mut Value, config: &crate::config::Config) {
    detail["notifications"]["recovery"]["commands_available"] = json!(false);
    if let Ok(program) = std::env::current_exe()
        && std::fs::symlink_metadata(&program)
            .is_ok_and(|m| m.is_file() && !crate::platform::is_link(&m))
    {
        detail["notifications"]["recovery"]["commands_available"] = json!(true);
        for (name, apply) in [("desktop_preview", false), ("desktop_apply", true)] {
            let mut args = vec![
                "--state-dir".to_owned(),
                config.root.to_string_lossy().into_owned(),
                "connect".into(),
                "--recover".into(),
            ];
            if apply {
                args.push("--apply".into());
            }
            detail["notifications"]["recovery"][name] = json!({"program":program,"args":args});
        }
        detail["notifications"]["recovery"]["shell"] =
            json!(if cfg!(windows) { "powershell" } else { "posix" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_selects_goal_fences_and_expiry_without_mutation() {
        for action in [
            "native_pause",
            "confirm_pause",
            "report_then_pause",
            "wait",
            "await_notification",
            "host_control",
        ] {
            let detail = json!({"job":{"job_id":"job","run_id":"run"},"notifications":{
                "status":"retry_wait","goal":{"goal_wait_id":"wait","request_id":"request","revision":7,
                    "continuation":{"action":action,"confirmation_revision":7}},
                "latest_problem":{"failure_kind":"rebind_required"}}});
            let original = detail.clone();
            let value = plan(&detail);
            assert_eq!(value["action"], action);
            assert_eq!(value["expected_revision"], 7);
            assert_eq!(value["confirmation_revision"], 7);
            assert_eq!(value["request_id"], "request");
            assert_eq!(value["replay_execution"], false);
            assert_eq!(detail, original);
        }
        let mut detail = json!({"notifications":{"status":"retry_wait","latest_problem":{"failure_kind":"rebind_required","expires_at":1}}});
        assert_eq!(plan(&detail)["action"], "inspect_expired");
        detail["notifications"]["latest_problem"] = json!({"failure_kind":"permanent"});
        assert_eq!(plan(&detail)["action"], "inspect_failure");
        detail["notifications"] = json!({"status":"inbox_only"});
        assert_eq!(plan(&detail)["action"], "inspect_inbox");
        detail["notifications"] = json!({"goal":{"continuation":{"action":"await_notification"},"diagnostics":{"delivery":{"status":"dead_letter"}}}});
        assert_eq!(plan(&detail)["action"], "host_control");
        detail["notifications"]["goal"]["continuation"]["action"] = json!("report_then_pause");
        assert_eq!(plan(&detail)["action"], "report_then_pause");
    }
}
