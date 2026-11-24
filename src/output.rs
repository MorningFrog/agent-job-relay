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
            "title",
            "run_id",
            "status",
            "queue",
            "exit_code",
            "signal",
            "cancel_requested",
            "detail",
            "stop_outcome",
            "goal_wait",
        ],
    )
}

pub(crate) fn consent_summary(value: &Value) -> Value {
    let mut result = fields(
        value,
        &[
            "request_id",
            "session_key",
            "status",
            "revision",
            "goal_status",
            "job_limit",
            "jobs_admitted",
            "jobs_remaining",
            "expires_at",
            "question_pending",
            "ask_user_now",
            "continuation",
            "next_step",
        ],
    );
    if value["status"] == "pending" || value["ask_user_now"] == true {
        result["question"] = value["question"].clone();
        result["choices"] = value["choices"].clone();
    }
    result
}

pub(crate) fn handoff(value: &Value) -> Value {
    let mut result = fields(
        value,
        &[
            "goal_wait_id",
            "request_id",
            "jobs",
            "status",
            "revision",
            "host_pause_required",
            "expires_at",
            "feedback_count",
            "results",
            "notification_accepted",
            "continuation",
            "next_step",
        ],
    );
    if value["feedback"].is_object() {
        result["feedback"] = value["feedback"].clone();
    }
    if matches!(
        value["status"].as_str(),
        Some("attention" | "suppressed" | "cancelled")
    ) {
        result["detail"] = value["detail"].clone();
    }
    if result["continuation"].is_null() {
        crate::goal_wait::set_action(&mut result, "host_control");
    }
    result
}

/// Keep goal control identity/revision without copying host snapshots or audits.
pub(crate) fn goal_summary(value: &Value) -> Value {
    fields(
        value,
        &[
            "goal_wait_id",
            "jobs",
            "status",
            "revision",
            "host_pause_required",
            "detail",
            "expires_at",
            "feedback",
            "request_id",
            "continuation",
            "next_step",
            "notification_accepted",
            "feedback_policy",
            "diagnostics",
        ],
    )
}
fn excerpt(text: &str, limit: usize) -> &str {
    let mut start = text.len().saturating_sub(limit);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}
pub(crate) fn terminal_results(db: &rusqlite::Connection, members: &[Value]) -> Vec<Value> {
    members
        .iter()
        .map(|member| {
            let result = (|| {
                let job_id = member["job_id"].as_str()?;
                let run_id = member["run_id"].as_str()?;
                let job = crate::storage::read_job(db, job_id).ok()?;
                terminal_run(&job, run_id)
            })();
            // Retain the original pin even when the summary is unavailable.
            result.unwrap_or_else(|| member.clone())
        })
        .collect()
}

/// Project only verified, typed run metadata into host notifications.
fn terminal_run(job: &crate::core::Job, run_id: &str) -> Option<Value> {
    if job.run.run_id != run_id || !job.run.status.is_terminal() {
        return None;
    }
    let mut result = json!({"job_id":job.job_id,"run_id":run_id,"status":job.run.status});
    use crate::core::CommandSpec;
    if matches!(
        &job.command,
        CommandSpec::GithubActions { .. } | CommandSpec::RetiredObservation { .. }
    ) || matches!(&job.command, CommandSpec::Remote { remote } if !remote.can_stop())
    {
        result["read_only_observation"] = json!(true);
    }
    if let Some(exit) = job.run.exit_code {
        result["exit_code"] = json!(exit);
    }
    if let Some(signal) = job.run.signal {
        result["signal"] = json!(signal);
    }
    if let Some((start, end)) = job.run.started_at.zip(job.run.ended_at) {
        result["duration_seconds"] = json!(end.saturating_sub(start).max(0) / 1000);
    }
    Some(result)
}

fn inspection(value: Value) -> Value {
    if value.get("job").is_none() {
        return value;
    }
    let record = &value["job"];
    let events = value["events"].as_array();
    let terminal = record["status"].as_str().is_some_and(crate::core::terminal);
    let outcome_event = events
        .and_then(|events| {
            events.iter().find(|event| {
                event["run_id"] == record["run_id"] && event["kind"] == record["status"]
            })
        })
        .filter(|_| terminal);
    let mut notifications = fields(
        &value["notifications"],
        &[
            "status",
            "counts",
            "completion",
            "agent_execution",
            "latest_problem",
        ],
    );
    notifications["recovery"]["plan"] = fields(
        &value["notifications"]["recovery"]["plan"],
        &[
            "action",
            "job_id",
            "run_id",
            "delivery_id",
            "goal_wait_id",
            "request_id",
            "expected_revision",
            "confirmation_revision",
            "expires_at",
            "next_attempt_at",
        ],
    );
    let mut result = json!({"job":job(record), "sampled_at":value["sampled_at"],
        "notifications":notifications, "goal_wait":goal_summary(&value["goal_wait"]),
        "feedback":fields(&value["feedback"], &["status","configured_rules","notifying_rules","visibility","activity"]),
        "stop":value["stop"], "untrusted_output":true});
    result["job"]["run"] = fields(&record["run"], &["started_at", "ended_at", "attempt"]);
    if let Some(detail) = record["detail"].as_str() {
        result["job"]["detail"] = json!(excerpt(detail, 768));
    }
    result["elapsed_seconds"] = match (
        record["run"]["started_at"].as_i64(),
        record["run"]["ended_at"].as_i64(),
    ) {
        (Some(start), Some(end)) => json!(end.saturating_sub(start).max(0) / 1000),
        _ => value["observation"]["elapsed_seconds"].clone(),
    };
    if let Some(event) = outcome_event {
        result["terminal_event"] = fields(event, &["seq", "event_id", "at"]);
    }
    if let Some(feedback) = value["observation"].get("remote_feedback") {
        result["remote_feedback"] = fields(feedback, &["alerts", "suppressed", "coalesced"]);
        result["remote_feedback"]["milestones"] =
            json!(feedback["milestones"].as_array().map(|m| {
                m.iter()
                    .rev()
                    .take(2)
                    .map(|item| {
                        let mut brief = fields(item, &["rule_id", "revision", "at"]);
                        brief["evidence"]["remote"] = item["evidence"]["remote"].clone();
                        brief["evidence"]["source_excerpt"] = json!(excerpt(
                            &item["evidence"]["source_evidence"].to_string(),
                            512
                        ));
                        brief
                    })
                    .collect::<Vec<_>>()
            }));
    }
    for stream in ["stdout", "stderr"] {
        let source = &value["observation"][stream];
        let text = source["text"].as_str().unwrap_or("");
        result["output"][stream] = json!({"text":excerpt(text,2048),"excerpt_truncated":text.len()>2048,
            "total_bytes":source["total_bytes"],"retained_from":source["retained_from"],
            "last_output_at":source["last_output_at"]});
    }
    let progress = value["observation"]
        .get("progress")
        .or_else(|| value["observation"]["external"]["observation"].get("progress"))
        .filter(|progress| progress.is_object())
        .or_else(|| {
            outcome_event
                .and_then(|event| event["payload"].get("progress"))
                .filter(|p| p.is_object())
        });
    if let Some(progress) = progress {
        let mut brief = fields(
            progress,
            &[
                "accepted",
                "alerts",
                "suppressed",
                "dropped",
                "rejected",
                "deferred",
                "coalesced",
            ],
        );
        if let Some(sources) = progress["sources"].as_object() {
            for (name, sample) in sources.iter().take(8) {
                brief["sources"][name] = fields(
                    sample,
                    &["stage", "step", "total", "metrics", "observed_run"],
                );
            }
        }
        brief["milestones"] = json!(
            progress["milestones"]
                .as_array()
                .map(|a| a.iter().rev().take(3).collect::<Vec<_>>())
        );
        result["progress"] = brief;
    }
    result["external"] = fields(
        &value["observation"]["external"],
        &[
            "mode",
            "connection",
            "last_success_at",
            "evidence_stale",
            "status",
            "last_error",
            "run",
            "can_stop",
        ],
    );
    for (kind, list) in [("workflow", "jobs"), ("pipeline", "workflows")] {
        if let Some(observed) = value["observation"]["external"].get(kind) {
            let mut brief = fields(
                observed,
                &[
                    "server_url",
                    "repository",
                    "repo_id",
                    "run_id",
                    "attempt",
                    "pipeline_id",
                    "pipeline_number",
                    "commit",
                    "status",
                    "conclusion",
                    "url",
                    "sampled_at",
                ],
            );
            if let Some(items) = observed[list]["items"].as_array() {
                let noteworthy = |item: &Value| {
                    matches!(
                        item["status"].as_str(),
                        Some("running" | "in_progress" | "failure" | "error" | "declined")
                    ) || matches!(
                        item["conclusion"].as_str(),
                        Some("failure" | "timed_out" | "action_required")
                    )
                };
                let highlights: Vec<_> = items
                    .iter()
                    .filter(|item| noteworthy(item))
                    .chain(items.iter().filter(|item| !noteworthy(item)))
                    .take(3)
                    .collect();
                brief[list] = json!({"total_count":observed[list]["total_count"],"highlights":highlights,
                    "truncated":items.len()>highlights.len() || observed[list]["truncated"]==true});
            }
            result["external"][kind] = brief;
        }
    }
    result["events_after"] = json!(
        events
            .and_then(|a| a.iter().filter_map(|e| e["seq"].as_i64()).max())
            .unwrap_or(0)
    );
    result["recent_events_after"] = json!(
        events
            .and_then(|a| a.iter().filter_map(|e| e["seq"].as_i64()).min())
            .unwrap_or(1)
            .saturating_sub(1)
    );
    result["recent_events"] = json!(events.map(|a| {
        a.iter()
            .take(3)
            .map(|e| {
                let mut event = fields(e, &["seq", "event_id", "kind", "at"]);
                if e["kind"] == "monitor_alert" {
                    let payload = e["payload"].to_string();
                    let mut end = payload.len().min(768);
                    while !payload.is_char_boundary(end) {
                        end -= 1;
                    }
                    event["payload_excerpt"] = json!(&payload[..end]);
                    event["excerpt_truncated"] = json!(end < payload.len());
                }
                event
            })
            .collect::<Vec<_>>()
    }));
    result["deliveries_after"] = value["deliveries"]["next_cursor"].clone();
    result["next_step"] = json!(if terminal {
        "Summarize this outcome; use job_logs, job_events or full=true only for missing evidence."
    } else {
        "Report requested evidence and follow any Goal handoff. Avoid polling."
    });
    result
}

fn admission(value: &Value, wait: &Value) -> Value {
    let mut wake = fields(
        &value["wake"],
        &["session_key", "automatic_binding", "probe"],
    );
    let priority = |target: &Value| match target["status"].as_str() {
        Some("configured") if target["can_resume"] == true => 3,
        Some("lease_expired") if target["renew_on_delivery"] == true => 2,
        Some("configured") => 1,
        _ => 0,
    };
    let target = value["feedback"]["targets"].as_array().and_then(|targets| {
        targets
            .iter()
            .filter(|target| priority(target) > 0)
            .max_by_key(|target| priority(target))
    });
    let mut mode = if let Some(target) = target {
        wake["session_key"] = target["session_key"].clone();
        if target["status"] == "lease_expired" {
            wake["target_status"] = json!("renewal_required");
        }
        target["delivery"].as_str().unwrap_or("unavailable")
    } else if (value["feedback"]["targets"].is_null() || value["feedback"]["status"] == "terminal")
        && (value["wake"]["target_status"] == "configured"
            || value["wake"]["target_status"].is_null())
    {
        let caps = &value["wake"]["capabilities"];
        if caps["can_resume"] == true {
            "host"
        } else if caps["can_notify"] == true {
            "inbox"
        } else {
            "unavailable"
        }
    } else {
        "unavailable"
    };
    if wait.is_object() {
        mode = if matches!(
            wait["status"].as_str(),
            Some("attention" | "cancelled" | "suppressed")
        ) {
            "unavailable"
        } else {
            "goal"
        };
    }
    wake["mode"] = json!(mode);
    wake["can_resume"] = json!(matches!(mode, "host" | "goal"));
    wake["can_confirm_resume"] = json!(false);
    if let Some(status) = value["wake"]
        .get("target_status")
        .filter(|s| target.is_none() && **s != "configured")
    {
        wake["target_status"] = status.clone();
    }

    let mut feedback = fields(
        &value["feedback"],
        &["status", "configured_rules", "notifying_rules"],
    );
    if let Some(rules) = value["feedback"]["rules"]
        .as_array()
        .filter(|rules| !rules.is_empty())
    {
        feedback["rules"] = json!(
            rules
                .iter()
                .map(|rule| fields(
                    rule,
                    &[
                        "rule_id",
                        "revision",
                        "condition",
                        "action",
                        "enabled",
                        "notify_agent",
                        "application",
                        "sample_observed",
                        "pending_stage",
                    ]
                ))
                .collect::<Vec<_>>()
        );
    }
    if let Some(advisories) = value["feedback"]["visibility"]["advisories"]
        .as_array()
        .filter(|advisories| !advisories.is_empty())
    {
        feedback["visibility"]["advisories"] = json!(advisories);
    }
    let mut result = json!({"accepted":true,"job":fields(&value["job"], &["job_id","run_id","title","status","queue","exit_code","signal"]),
        "wake":wake,"feedback":feedback});
    if let Some(storage) = value.get("storage").filter(|v| v["status"] != "ok") {
        result["storage"] = fields(storage, &["status", "available_bytes", "new_jobs_allowed"]);
    }
    result
}

fn admission_action(value: &Value) -> (&'static str, &'static str) {
    if value["feedback"]["status"] == "terminal" {
        return (
            "report_results",
            "Summarize the original outcome. Use job_inspect if more evidence is needed.",
        );
    }
    match value["wake"]["mode"].as_str() {
        Some("host") if value["wake"]["target_status"] == "renewal_required" => (
            "yield_for_notification",
            "End this turn. Relay verifies the original endpoint for lease renewal when delivery is due.",
        ),
        Some("host") => (
            "yield_for_notification",
            "End this turn and wait for the original chat's notification.",
        ),
        Some("inbox") => (
            "share_inspection",
            "Give the Job ID and job_inspect or job_inbox; the inbox requires an explicit read.",
        ),
        _ => (
            "share_inspection",
            "Give the Job ID and job_inspect; this run has no available automatic continuation.",
        ),
    }
}

pub fn compact(op: &str, value: Value) -> Value {
    if value.get("job").is_none()
        && matches!(
            value["status"].as_str(),
            Some("needs_confirmation" | "request_closed" | "goal_not_active" | "goal_wait_pending")
        )
    {
        return value;
    }
    match op {
        "inspect" => inspection(value),
        "goal_wait" => handoff(&value),
        "start_batch" => {
            let mut result = value;
            let mut needs_inspection = false;
            let wait = result["goal_wait"].clone();
            if let Some(items) = result["results"].as_array_mut() {
                for item in items {
                    if item["accepted"] == true {
                        let receipt = admission(&item["result"], &wait);
                        needs_inspection |= receipt["wake"]["mode"] != "host"
                            && receipt["feedback"]["status"] != "terminal";
                        item["result"] = receipt;
                    }
                }
            }
            if let Some(wait) = result.get_mut("goal_wait") {
                *wait = handoff(wait);
                result.as_object_mut().unwrap().remove("next_step");
            } else {
                let (action, next) = if result["accepted"] == 0 {
                    ("review_rejections", "Review indexed rejections.")
                } else if needs_inspection {
                    (
                        "share_inspection",
                        "Give Job IDs and job_inspect for items without automatic continuation, then end this turn.",
                    )
                } else {
                    (
                        "yield",
                        "Use any terminal evidence and end this turn. Await notifications for remaining host-bound runs.",
                    )
                };
                result["continuation"] = json!({"action":action,"actor":"agent"});
                result["next_step"] = json!(next);
            }
            result
        }
        "start" => {
            let mut result = admission(&value, &value["goal_wait"]);
            if let Some(wait) = value.get("goal_wait") {
                result["goal_wait"] = handoff(wait);
            } else {
                let (action, next) = admission_action(&result);
                result["continuation"] = json!({"action":action,"actor":"agent"});
                result["next_step"] = json!(next);
            }
            result
        }
        "status" | "stop" => {
            let mut result = job(&value);
            if let Some(wait) = value.get("goal_wait").filter(|wait| wait.is_object()) {
                result["goal_wait"] = handoff(wait);
                return result;
            }
            result["next_step"] = json!(match value["status"].as_str().unwrap_or("") {
                "orphaned" =>
                    "Inspect job_events and external side effects before deliberately submitting again.",
                "completed" | "failed" | "cancelled" | "timeout" =>
                    "Use job_inspect if the outcome needs more evidence.",
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
                    "structured_progress_v1",
                    "remote_execution",
                    "remote_alert_feedback",
                    "github_actions_observation",
                    "goal_waiting",
                    "goal_wait_host_pause",
                    "goal_wait_feedback",
                    "goal_wait_recovery",
                    "goal_wait_policy",
                    "automatic_goal_pause_supported",
                    "goal_pause_authorization_policy",
                    "scoped_goal_consent",
                    "persistent_goal_consent",
                    "native_goal_consent_form",
                    "automatic_goal_pause_reason",
                    "managed_mcp_updates_supported",
                    "runtime_workflow_sha256",
                    "runtime_topics_sha256",
                    "goal_waits",
                    "management",
                    "store",
                    "store_check",
                    "capacity",
                    "version",
                    "active_jobs",
                    "bounded_queue",
                    "queued_jobs",
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
        &["changed", "root", "state_dir", "backup", "daemon_restarted"],
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
