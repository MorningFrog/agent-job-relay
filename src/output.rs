//! Compact Agent-facing views; durable records and raw IPC remain unchanged.
use serde_json::{Value, json};

fn fields(value: &Value, names: &[&str]) -> Value {
    let mut result = serde_json::Map::new();
    for name in names {
        if let Some(value) = value.get(*name) {
            result.insert((*name).into(), value.clone());
        }
    }
    Value::Object(result)
}

fn job(value: &Value) -> Value {
    fields(
        value,
        &[
            "job_id",
            "run_id",
            "status",
            "exit_code",
            "signal",
            "cancel_requested",
            "detail",
            "stop_outcome",
            "goal_wait",
        ],
    )
}

pub fn compact(op: &str, value: Value) -> Value {
    match op {
        "start_batch" => {
            let mut result = value;
            if let Some(items) = result["results"].as_array_mut() {
                for item in items {
                    if item["accepted"] == true {
                        item["result"] = compact("start", item["result"].take());
                    }
                }
            }
            result
        }
        "start" => {
            let capabilities = &value["wake"]["capabilities"];
            let resume = capabilities["can_resume"].as_bool().unwrap_or(false);
            let mut wake = fields(
                &value["wake"],
                &["session_key", "automatic_binding", "probe"],
            );
            wake["can_resume"] = json!(resume);
            wake["can_confirm_resume"] = json!(false);
            let mut result = json!({"job":job(&value["job"]),"wake":wake,"next_step":if resume {
                "End this turn; inspect job_status when the completion notification arrives."
            } else {
                "Use job_wait once for a short job, or inspect job_status later; no automatic continuation is bound."
            }});
            if let Some(wait) = value.get("goal_wait") {
                result["goal_wait"] = wait.clone();
                result["next_step"] = wait["next_step"].clone();
            }
            result
        }
        "status" | "stop" => {
            let mut result = job(&value);
            result["next_step"] = json!(match value["status"].as_str().unwrap_or("") {
                "orphaned" =>
                    "Inspect job_events and external side effects before deliberately submitting again.",
                "completed" | "failed" | "cancelled" | "timeout" =>
                    "Read bounded job_logs for output; job_events has execution details.",
                _ if value["cancel_requested"] == true =>
                    "Cancellation requested; inspect job_status later.",
                _ =>
                    "Wait for the bound notification, or inspect job_status later; avoid polling loops.",
            });
            result
        }
        "list" | "wait" => {
            let mut result = value;
            if let Some(jobs) = result.get_mut("jobs").and_then(Value::as_array_mut) {
                for item in jobs {
                    *item = job(item);
                }
            }
            if let Some(item) = result.get_mut("job") {
                *item = job(item);
            }
            result
        }
        "recover_session" => {
            let mut result = fields(&value, &["session_key", "apply", "targets", "next_cursor"]);
            let targets = value["targets"].as_array();
            let count = targets.map_or(0, Vec::len);
            let busy = targets.map_or(0, |rows| {
                rows.iter().filter(|r| r["in_flight"] == true).count()
            });
            let applied = targets.map_or(0, |rows| {
                rows.iter().filter(|r| r["applied"] == true).count()
            });
            result["summary"] = json!({"on_page":count,"in_flight":busy,"applied":applied});
            result["next_step"] = json!(if count == 0 {
                "Recovery scan complete; commands were not rerun."
            } else if value["apply"] == true {
                "Continue with after=next_cursor; retry skipped in-flight targets later."
            } else {
                "Use apply=true to restore this page, or after=next_cursor to preview the next page."
            });
            result
        }
        "doctor" => {
            let mut result = fields(
                &value,
                &[
                    "daemon",
                    "web_ui",
                    "runtime_monitoring",
                    "remote_execution",
                    "github_actions_observation",
                    "goal_waiting",
                    "goal_waits",
                    "management",
                    "store",
                    "store_check",
                    "capacity",
                    "version",
                    "active_jobs",
                    "orphaned_jobs",
                    "sessions",
                    "dead_letters",
                    "rebind_required",
                    "retrying_deliveries",
                    "runner_readiness",
                    "automatic_startup",
                    "mcp_host_connection",
                    "binding_mode",
                    "desktop_context",
                ],
            );
            if value.get("capacity").is_some() {
                result["capacity"] = fields(
                    &value["capacity"],
                    &[
                        "hot_jobs",
                        "hot_job_limit",
                        "archived_jobs",
                        "active_sessions",
                        "active_session_limit",
                        "last_backup_at",
                        "last_error",
                        "warning",
                    ],
                );
            }
            result["next_step"] = json!(
                if value["runner_readiness"]["status"] == "restart_required" {
                    "Let active jobs finish, then explicitly stop/update the same installed profile."
                } else if value["rebind_required"].as_u64().unwrap_or(0) > 0 {
                    "Preview recovery with connect --recover in this chat's terminal; --apply restores eligible notifications."
                } else if value["dead_letters"].as_u64().unwrap_or(0) > 0 {
                    "Inspect job_deliveries and job_delivery_attempts before choosing recovery or explicit retry."
                } else if value["daemon"] == "stopped" {
                    "Submit a job or run connect; the daemon starts on demand."
                } else {
                    "Ready; bind from the actual host caller before a wake-enabled submission."
                }
            );
            result
        }
        _ => value,
    }
}

pub fn installation(value: Value) -> Value {
    let mut result = fields(&value, &["mode", "version", "data", "next_step"]);
    result["management"] = value
        .get("management")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    result["install"] = fields(
        &value["install"],
        &["changed", "root", "state_dir", "backup"],
    );
    if let Some(marketplace) = value.get("marketplace") {
        result["marketplace"] = fields(marketplace, &["changed", "config_path", "backup"]);
    }
    if let Some(caches) = value.get("caches").and_then(Value::as_array) {
        result["caches"] = Value::Array(
            caches
                .iter()
                .map(|c| fields(c, &["path", "changed", "backup"]))
                .collect(),
        );
    }
    result
}
