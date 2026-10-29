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
    let value = api(spec, cwd, path).await?;
    let mut sample = workflow(spec, &value)?;
    // Optional detail must never hide authoritative run completion or imply failure.
    match jobs(spec, cwd).await {
        Ok(progress) => sample["jobs"] = progress,
        Err(error) => sample["jobs_error"] = json!(format!("{error:#}")),
    }
    Ok(sample)
}

async fn api(spec: &ActionsSpec, cwd: &std::path::Path, path: String) -> Result<Value> {
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
        anyhow::bail!("GitHub observation unavailable: {kind}");
    }
    serde_json::from_slice(&output).context("invalid GitHub API response")
}

fn workflow(spec: &ActionsSpec, value: &Value) -> Result<Value> {
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
    let url = format!("{expected_url}/attempts/{}", spec.attempt);
    Ok(
        json!({"repository":repo,"run_id":spec.run_id,"attempt":spec.attempt,"hostname":spec.hostname,"status":status,"conclusion":conclusion,"url":url,"sampled_at":crate::core::now(),"can_cancel":false,"mode":"observe_only"}),
    )
}

async fn jobs(spec: &ActionsSpec, cwd: &std::path::Path) -> Result<Value> {
    // One bounded page; large matrices remain accessible through the attempt link.
    let value = api(
        spec,
        cwd,
        format!(
            "repos/{}/actions/runs/{}/attempts/{}/jobs?per_page=20",
            spec.repository, spec.run_id, spec.attempt
        ),
    )
    .await?;
    summarize_jobs(spec, &value)
}

fn summarize_jobs(spec: &ActionsSpec, value: &Value) -> Result<Value> {
    let jobs = value["jobs"].as_array().context("missing workflow jobs")?;
    ensure!(jobs.len() <= 20, "workflow jobs page exceeds limit");
    let total = value["total_count"]
        .as_u64()
        .context("missing workflow jobs count")?;
    ensure!(total >= jobs.len() as u64, "invalid workflow jobs count");
    let mut items = Vec::new();
    for job in jobs {
        ensure!(
            job["run_id"].as_u64() == Some(spec.run_id)
                && job["run_attempt"].as_u64() == Some(spec.attempt as u64),
            "workflow job run/attempt identity mismatch"
        );
        let steps = job["steps"]
            .as_array()
            .context("missing workflow job steps")?;
        let current = steps.iter().find(|step| step["status"] == "in_progress");
        let bounded = |value: &Value| {
            value
                .as_str()
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>()
        };
        items.push(json!({
            "id":job["id"].as_u64().context("missing workflow job identity")?,
            "name":bounded(&job["name"]),
            "status":bounded(&job["status"]),
            "conclusion":job["conclusion"].as_str().map(|_| bounded(&job["conclusion"])),
            "completed_steps":steps.iter().filter(|step| step["status"] == "completed").count(),
            "total_steps":steps.len(),
            "current_step":current.map(|step| bounded(&step["name"]))
        }));
    }
    Ok(json!({"items":items,"total_count":total,"truncated":total > jobs.len() as u64}))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_details_fence_attempt_identity_and_bound_retained_names_and_pages() {
        let spec = ActionsSpec {
            repository: "owner/repo".into(),
            run_id: 123,
            attempt: 2,
            hostname: "github.com".into(),
            gh_program: "gh".into(),
        };
        let job = json!({"id":456,"run_id":123,"run_attempt":2,"name":"步".repeat(200),"status":"in_progress","conclusion":null,"steps":[{"name":"编".repeat(200),"status":"in_progress"}]});
        let mut page = json!({"total_count":21,"jobs":vec![job;20]});
        let result = summarize_jobs(&spec, &page).unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 20);
        assert_eq!(
            result["items"][0]["name"].as_str().unwrap().chars().count(),
            160
        );
        assert_eq!(
            result["items"][0]["current_step"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            160
        );
        assert_eq!(result["truncated"], true);
        page["jobs"][0]["run_attempt"] = json!(3);
        assert!(
            summarize_jobs(&spec, &page)
                .unwrap_err()
                .to_string()
                .contains("identity mismatch")
        );
        page["jobs"][0]["run_attempt"] = json!(2);
        page["jobs"][0]["run_id"] = json!(124);
        assert!(
            summarize_jobs(&spec, &page)
                .unwrap_err()
                .to_string()
                .contains("identity mismatch")
        );
        page["jobs"][0]["run_id"] = json!(123);
        let extra = page["jobs"][0].clone();
        page["jobs"].as_array_mut().unwrap().push(extra);
        assert!(
            summarize_jobs(&spec, &page)
                .unwrap_err()
                .to_string()
                .contains("page exceeds limit")
        );
    }
}
