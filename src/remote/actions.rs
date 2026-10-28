use super::{ActionsSpec, transport};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub(crate) async fn sample(spec: &ActionsSpec, cwd: &std::path::Path) -> Result<Value> {
    spec.validate()?;
    // The attempt endpoint cannot silently select a newer rerun.
    let path = format!(
        "repos/{}/actions/runs/{}/attempts/{}",
        spec.repository, spec.run_id, spec.attempt
    );
    let args = vec![
        "api".into(),
        "--hostname".into(),
        spec.hostname.clone(),
        "--method".into(),
        "GET".into(),
        "-H".into(),
        "Accept: application/vnd.github+json".into(),
        path,
    ];
    let (exit, output, errors) =
        transport::process(&spec.gh_program, &args, &[], Some(cwd)).await?;
    if !exit.success() {
        let errors = String::from_utf8_lossy(&errors).to_ascii_lowercase();
        let kind = if errors.contains("rate limit") || errors.contains("429") {
            "rate_limited"
        } else if errors.contains("401") || errors.contains("403") || errors.contains("auth login")
        {
            "authentication_or_permission"
        } else if errors.contains("404") {
            "not_found_or_not_authorized"
        } else {
            "transport_unavailable"
        };
        anyhow::bail!("GitHub observation unavailable: {kind}; workflow result remains unknown");
    }
    let value: Value = serde_json::from_slice(&output).context("invalid GitHub API response")?;
    ensure!(
        value["id"].as_u64() == Some(spec.run_id)
            && value["run_attempt"].as_u64() == Some(spec.attempt as u64),
        "workflow run/attempt identity mismatch"
    );
    let repo = value["repository"]["full_name"]
        .as_str()
        .context("missing workflow repository identity")?;
    ensure!(
        repo.eq_ignore_ascii_case(&spec.repository),
        "workflow repository identity mismatch"
    );
    let expected_url = format!(
        "https://{}/{}/actions/runs/{}",
        spec.hostname, repo, spec.run_id
    );
    let url = value["html_url"].as_str().context("missing workflow URL")?;
    ensure!(
        url == expected_url || url == format!("{expected_url}/attempts/{}", spec.attempt),
        "workflow URL identity mismatch"
    );
    let status = value["status"]
        .as_str()
        .context("missing workflow status")?;
    ensure!(
        [
            "queued",
            "in_progress",
            "waiting",
            "pending",
            "requested",
            "completed"
        ]
        .contains(&status),
        "unsupported workflow status"
    );
    let conclusion = value["conclusion"].as_str();
    if status == "completed" {
        ensure!(
            conclusion.is_some_and(|c| [
                "success",
                "failure",
                "cancelled",
                "timed_out",
                "action_required",
                "neutral",
                "skipped",
                "stale",
                "startup_failure"
            ]
            .contains(&c)),
            "missing/unsupported final workflow conclusion"
        );
    }
    Ok(
        json!({"repository":repo,"run_id":spec.run_id,"attempt":spec.attempt,"hostname":spec.hostname,"status":status,"conclusion":conclusion,"url":url,"sampled_at":crate::core::now(),"can_cancel":false,"mode":"observe_only"}),
    )
}

pub(crate) fn terminal_state(value: &Value) -> Option<&'static str> {
    if value["status"] != "completed" {
        return None;
    }
    Some(match value["conclusion"].as_str().unwrap_or("") {
        "success" | "neutral" | "skipped" => "completed",
        "cancelled" => "cancelled",
        "timed_out" => "timeout",
        _ => "failed",
    })
}
