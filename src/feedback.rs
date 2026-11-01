//! Read-only feedback receipts. Configured rules and registered host capabilities
//! are not evidence that output was produced, a delivery succeeded or an Agent ran.
use crate::{core::*, monitor::Rule, storage::Store};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

const QUIET_NOTICE_SECONDS: u64 = 600;

/// Describe captured evidence, never infer workload health from a quiet pipe.
/// Use the runner's sample time so an old snapshot cannot invent new silence.
fn visibility(job: &Job, observation: &Value, notifying: usize) -> Value {
    let mut value = json!({"status":"awaiting_sample","quiet_after_seconds":QUIET_NOTICE_SECONDS,
        "sampled_at":observation["sampled_at"],"advisories":[]});
    if job.status.is_terminal() {
        value["status"] = json!("terminal");
        return value;
    }
    if job.command.is_external() {
        value["status"] = json!("external");
        return value;
    }
    let Some(sampled_at) = observation["sampled_at"].as_i64() else {
        return value;
    };
    let (Some(elapsed), Some(stdout), Some(stderr)) = (
        observation["elapsed_seconds"].as_u64(),
        observation["stdout"]["total_bytes"].as_u64(),
        observation["stderr"]["total_bytes"].as_u64(),
    ) else {
        value["status"] = json!("unavailable");
        return value;
    };
    value["elapsed_seconds"] = json!(elapsed);
    value["output_bytes"] = json!(stdout.saturating_add(stderr));
    let last = ["stdout", "stderr"]
        .iter()
        .filter_map(|stream| observation[stream]["last_output_at"].as_i64())
        .max();
    let idle = match last {
        Some(last) if last <= sampled_at => {
            Some((sampled_at.saturating_sub(last) as u64 / 1000).min(elapsed))
        }
        None if stdout == 0 && stderr == 0 => Some(elapsed),
        _ => None,
    };
    value["output_idle_seconds"] = json!(idle);
    let age = now().saturating_sub(sampled_at);
    if !(0..=5000).contains(&age) {
        value["status"] = json!("stale");
        return value;
    }
    value["status"] = json!(match idle {
        Some(idle) if idle >= QUIET_NOTICE_SECONDS => "quiet",
        Some(_) => "observed",
        None => "unavailable",
    });
    let mut advisories = Vec::new();
    if idle.is_some_and(|idle| idle >= QUIET_NOTICE_SECONDS) {
        advisories.push(json!({"code":"quiet_output","message":
            "No captured output for at least 10 minutes at the runner sample. Check output routing, buffering or a real progress source; silence does not establish a hang. Keep the original run."}));
    }
    if elapsed >= QUIET_NOTICE_SECONDS && notifying == 0 {
        advisories.push(json!({"code":"no_mid_run_notification","message":
            "This long run has no enabled mid-run notifying rule. If completion-only was not intentional, select a bounded milestone or elapsed alert on the existing job with the original caller's authorization. Goal feedback also needs actual user pause/restoration consent. Do not restart work or poll."}));
    }
    value["advisories"] = json!(advisories);
    value
}

pub(crate) fn summarize(db: &Connection, detail: &Value) -> Result<Value> {
    let job: Job = serde_json::from_value(detail["job"].clone())?;
    let rules: Vec<Rule> = serde_json::from_value(detail["rules"].clone())?;
    let terminal = job.status.is_terminal();
    let goal_wait = detail["goal_wait"].is_object();
    let mut configured = 0;
    let mut notifying = 0;
    let mut remaining = 0;
    let mut summaries = Vec::new();
    for rule in rules.into_iter().filter(|r| !r.deleted) {
        let progress = matches!(
            rule.spec.condition,
            crate::monitor::Condition::Progress { .. }
        );
        let can_trigger = rule.enabled
            && !terminal
            && if progress {
                detail["observation"]["progress"]["alerts"]
                    .as_u64()
                    .unwrap_or(0)
                    < u64::from(crate::progress::MAX_ALERTS)
            } else {
                rule.triggered_revision != rule.revision
            };
        if rule.enabled {
            configured += 1;
            if rule.spec.notify_agent {
                notifying += 1;
                remaining += usize::from(can_trigger);
            }
        }
        let mut summary = json!({
            "rule_id":rule.id,"revision":rule.revision,
            "applied_revision":rule.applied_revision,"enabled":rule.enabled,
            "triggered_revision":rule.triggered_revision,
            "condition":rule.spec.condition,"action":rule.spec.action,
            "notify_agent":rule.spec.notify_agent,
            "application":if terminal {"terminal"} else if !rule.enabled {"disabled"}
                else if rule.applied_revision == rule.revision {"applied"} else {"pending"},
            "trigger_policy":if progress {"progress_cooldown"} else {"once_per_revision"},
            "can_trigger_again":can_trigger,
        });
        if let crate::monitor::Condition::Progress { source, .. } = &rule.spec.condition {
            summary["sample_observed"] =
                json!(detail["observation"]["progress"]["sources"][source].is_object());
        }
        summaries.push(summary);
    }

    // Follow recorded active targets, including explicit rebind/recovery. The
    // original job.session is not the current delivery destination after rebind.
    // Terminal cold snapshots need no live-target lookup or rehydration.
    let mut targets = Vec::new();
    if !terminal && !goal_wait {
        let mut q = db.prepare(
            "SELECT id,session_key FROM targets WHERE job_id=? AND active=1 ORDER BY rowid LIMIT 16",
        )?;
        let bindings = q
            .query_map([&job.job_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (target, key) in bindings {
            let session = crate::session::read_session(db, &key)?;
            let caps = crate::session::capabilities(&session.registration.adapter);
            targets.push(json!({
                "target_id":target,"session_key":key,"can_resume":caps.can_resume,
                "status":if !session.connected {"invalidated"}
                    else if session.expires_at <= now() {"lease_expired"} else {"configured"},
                "delivery":if caps.can_resume {"host"} else {"inbox"},
            }));
        }
    }
    let ready: Vec<_> = targets
        .iter()
        .filter(|t| t["status"] == "configured")
        .collect();
    let (status, reason) = if terminal {
        (
            "terminal",
            "The run has ended; rules cannot produce new mid-run feedback. Inspect retained events and deliveries.",
        )
    } else if goal_wait
        && matches!(
            detail["goal_wait"]["status"].as_str(),
            Some("cancelled" | "suppressed" | "attention")
        )
    {
        (
            "goal_wait_closed",
            "This goal wait is closed; local monitoring may continue but no further automatic goal control or notification is scheduled.",
        )
    } else if goal_wait
        && notifying > 0
        && detail["goal_wait"]["feedback_count"].as_u64().unwrap_or(0)
            >= detail["goal_wait"]["feedback_policy"]["max_cycles"]
                .as_u64()
                .unwrap_or(u64::MAX)
    {
        (
            "goal_feedback_exhausted",
            "This wait has reached its temporary-restoration budget; local evidence and final completion remain available.",
        )
    } else if goal_wait && notifying > 0 {
        (
            "goal_feedback",
            "Notifying rules use bounded temporary goal restoration. The Agent must report, acknowledge native re-pause and return this same group to waiting; final completion is separate.",
        )
    } else if goal_wait {
        (
            "goal_completion_only",
            "No notifying rule is enabled for this goal-wait member; Agent continuation is merged at member completion.",
        )
    } else if configured == 0 {
        (
            "not_configured",
            "No enabled feedback rule. Completion notification, if bound, is independent.",
        )
    } else if notifying == 0 {
        (
            "local_only",
            "Enabled rules record local evidence only; no mid-run Agent notification is selected.",
        )
    } else if remaining == 0 {
        (
            "exhausted",
            "Notifying one-shot revisions or the job-wide progress alert budget are exhausted. Explicit edits can rearm one-shot rules; they cannot reset the progress budget.",
        )
    } else if targets.is_empty() {
        (
            "unbound",
            "No active recorded notification target; no destination was inferred.",
        )
    } else if ready.is_empty() {
        (
            "binding_unavailable",
            "Recorded target registrations are invalidated or expired. Expired opted-in leases require verified delivery-time renewal; other bindings require explicit recovery.",
        )
    } else if !ready.iter().any(|t| t["can_resume"] == true) {
        (
            "inbox_only",
            "Rules can enqueue inbox events; the registered adapter cannot automatically resume an Agent.",
        )
    } else {
        (
            "configured",
            "Mid-run host notification is configured. Applied rules and matching output are still required; host availability, delivery and Agent execution are not confirmed by this receipt.",
        )
    };
    Ok(
        json!({"status":status,"reason":reason,"configured_rules":configured,
        "notifying_rules":notifying,"rules":summaries,"targets":targets,
        "visibility":visibility(&job,&detail["observation"],notifying),
        "can_confirm_resume":false}),
    )
}

impl Store {
    pub(crate) async fn feedback(&self, job: String) -> Result<Value> {
        self.read(move |db| {
            let tx = db.transaction()?;
            let mut detail = if let Some(snapshot) = crate::archive::load(&tx, &job)? {
                snapshot.inspection()?
            } else {
                let raw: Option<String> = tx
                    .query_row(
                        "SELECT data FROM runtime_observations WHERE job_id=?",
                        [&job],
                        |r| r.get(0),
                    )
                    .optional()?;
                json!({"job":crate::storage::read_job(&tx,&job)?,
                    "rules":crate::monitor::rules(&tx,&job)?,
                    "observation":raw.map(|v|serde_json::from_str::<Value>(&v)).transpose()?})
            };
            detail["goal_wait"] = json!(crate::goal_wait::summary_for_job(&tx, &job)?);
            summarize(&tx, &detail)
        })
        .await
    }
}
