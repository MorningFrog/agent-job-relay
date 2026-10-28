use super::transport::exchange_at;
use super::*;
use crate::{config::Config, monitor::Live, storage::Store};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    time::Duration,
};
type Outcome = (&'static str, Option<i32>, Option<i32>, String);

#[derive(Deserialize)]
struct Sample {
    run: RunRef,
    status: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    detail: Option<String>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    events: Vec<Event>,
    next_event: i64,
    observation: Value,
}
fn request(spec: &RemoteSpec, operation: Operation) -> BridgeRequest {
    BridgeRequest {
        protocol_version: 1,
        target_id: spec.target_id.clone(),
        request_id: id(),
        operation,
    }
}
async fn resolve(
    spec: &RemoteSpec,
    job: &Job,
    cancelled: bool,
    cwd: &std::path::Path,
) -> Result<Option<RunRef>> {
    let operation = match &spec.intent {
        Intent::Attach { run, .. } => return Ok(Some(run.clone())),
        Intent::Launch { spec } if !cancelled => Operation::Launch {
            origin_job_id: job.job_id.clone(),
            origin_run_id: job.run_id.clone(),
            spec: spec.clone(),
        },
        Intent::Launch { .. } => Operation::Lookup {
            origin_job_id: job.job_id.clone(),
            origin_run_id: job.run_id.clone(),
        },
    };
    let result = exchange_at(&spec.transport, &request(spec, operation), Some(cwd)).await?;
    ensure!(
        result["origin_job_id"] == job.job_id && result["origin_run_id"] == job.run_id,
        "remote origin identity mismatch"
    );
    let run: Option<RunRef> = serde_json::from_value(result["run"].clone())?;
    if let Some(run) = &run {
        run.validate()?;
    }
    Ok(run)
}

/// Independent local observer, restartable only for pinned external intents.
/// A transport error never becomes a remote command's failure or cancellation.
pub async fn observe(config: &Config, store: &Store, job: &Job, spec: &StartSpec) -> Result<()> {
    let previous = store.inspect(Some(job.job_id.clone()), None, 1).await?;
    let mut external = previous["observation"]["external"].clone();
    if !external.is_object() {
        external = json!({"mode":if matches!(spec.command, CommandSpec::Remote { .. }) {"remote_runtime"}else{"github_actions"},"origin_job_id":job.job_id,"origin_run_id":job.run_id,"connection":"unobserved"});
    }
    let mut run: Option<RunRef> = external
        .get("run")
        .filter(|v| !v.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    if run.is_none()
        && let CommandSpec::Remote { remote } = &spec.command
        && let Intent::Attach { run: attached, .. } = &remote.intent
    {
        run = Some(attached.clone());
        external["run"] = json!(run);
    }
    if let Some(run) = &run {
        run.validate()?;
    }
    let mut after_event = external["next_event"].as_i64().unwrap_or(0);
    let mut logs = Vec::new();
    let live = Live::new();
    // Resume already-applied matchers against retained bytes if the observer
    // died between persisting output and committing a rule trigger.
    let persisted_rules: Vec<crate::monitor::Rule> =
        serde_json::from_value(previous["rules"].clone())?;
    live.lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .apply(
            persisted_rules
                .into_iter()
                .filter(|r| r.applied_revision == r.revision)
                .collect(),
        );
    for (stream, name) in ["stdout", "stderr"].iter().enumerate() {
        let mut file = crate::platform::private_file(
            &config.run_dir(&job.run_id).join(format!("{name}.log")),
        )?;
        let len = file.metadata()?.len();
        ensure!(
            len <= crate::runner::LOG_CAP,
            "observer log exceeds capacity"
        );
        file.seek(SeekFrom::Start(
            len.saturating_sub(crate::monitor::TAIL_BYTES as u64),
        ))?;
        let mut tail = Vec::new();
        file.read_to_end(&mut tail)?;
        if !tail.is_empty() {
            live.lock()
                .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
                .restore_output(stream, len, &tail);
        }
        file.seek(SeekFrom::End(0))?;
        logs.push(file);
    }
    let mut failures = 0u32;
    let mut next = tokio::time::Instant::now();
    let mut last_stop: Option<(RunRef, u32)> = None;
    let mut observed_cancel = None;
    let mut last_publish = tokio::time::Instant::now();
    let began = tokio::time::Instant::now();
    let started_at = store
        .status(job.job_id.clone())
        .await?
        .run
        .started_at
        .unwrap_or(job.created_at);
    let elapsed_before = Duration::from_millis(now().saturating_sub(started_at).max(0) as u64);
    live.lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .resume_elapsed(elapsed_before);
    loop {
        let (cancelled, grace, _, rules) = store.control(job.job_id.clone()).await?;
        live.lock()
            .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
            .apply(rules);
        let timed_out = spec
            .timeout_seconds
            .is_some_and(|s| elapsed_before + began.elapsed() >= Duration::from_secs(s));
        if timed_out {
            external["connection"] = json!("observer_timeout");
            external["evidence_stale"] = json!(true);
            publish(store, job, &live, &external, "finished").await?;
            return store.finish(job.job_id.clone(), "orphaned", None, None, "observation deadline reached; external outcome unknown; no external stop or launch replay".into()).await;
        }
        let can_stop = match &spec.command {
            CommandSpec::Remote { remote } => remote.can_stop(),
            _ => false,
        };
        if cancelled && (!can_stop || (run.is_none() && external["launch_attempted"] != true)) {
            external["connection"] = json!("observation_stopped");
            publish(store, job, &live, &external, "finished").await?;
            return store.finish(job.job_id.clone(), "cancelled", None, None, "observation stopped; external execution was not cancelled; no external outcome inferred".into()).await;
        }
        if cancelled && observed_cancel != Some(grace) {
            observed_cancel = Some(grace);
            next = tokio::time::Instant::now();
        }
        if tokio::time::Instant::now() >= next {
            let mut catch_up = false;
            let result: Result<Option<Outcome>> = async {
                match &spec.command {
                    CommandSpec::Remote { remote } => {
                        external["target_id"] = json!(remote.target_id);
                        external["can_stop"] = json!(can_stop);
                        if run.is_none() {
                            // Publish uncertainty before sending any execution intent.
                            external["launch_attempted"] = json!(true);
                            publish(store, job, &live, &external, "running").await?;
                            run = resolve(remote, job, cancelled, &spec.cwd).await?;
                            external["run"] = json!(run);
                            publish(store, job, &live, &external, "running").await?;
                        }
                        let target = run.clone().context("launch acknowledgement unknown; cancellation will only look up the original intent")?;
                        if cancelled && last_stop.as_ref().is_none_or(|(r, old)| r != &target || grace < *old) {
                            let result = exchange_at(&remote.transport, &request(remote, Operation::Stop { run: target.clone(), grace_seconds: grace }), Some(&spec.cwd)).await?;
                            ensure!(result["run"] == json!(target), "remote stop identity mismatch");
                            last_stop = Some((target.clone(), grace));
                            external["stop"] = json!({"state":"requested","grace_seconds":grace,"at":now(),"terminal_observed":false});
                        }
                        let cursors = [logs[0].metadata()?.len(), logs[1].metadata()?.len()];
                        let value = exchange_at(&remote.transport, &request(remote, Operation::Sample { run: target.clone(), cursors, after_event }), Some(&spec.cwd)).await?;
                        let sample: Sample = serde_json::from_value(value).map_err(|_|anyhow::anyhow!("malformed remote sample"))?;
                        ensure!(sample.run == target, "remote sample execution identity mismatch");
                        ensure!(["queued", "launching", "running", "completed", "failed", "cancelled", "timeout", "orphaned"].contains(&sample.status.as_str()), "invalid remote state");
                        ensure!(sample.stdout.len() <= super::protocol::LOG_PAGE as usize && sample.stderr.len() <= super::protocol::LOG_PAGE as usize && sample.events.len() <= 20, "oversized remote sample");
                        let mut cursor = after_event;
                        for event in &sample.events {
                            ensure!(event.job_id == target.job_id && event.run_id == target.run_id && event.seq > cursor, "remote event identity/cursor mismatch");
                            cursor = event.seq;
                        }
                        ensure!(sample.next_event == cursor, "remote event cursor mismatch");
                        let drained = sample.stdout.is_empty() && sample.stderr.is_empty();
                        catch_up = sample.stdout.len() == super::protocol::LOG_PAGE as usize || sample.stderr.len() == super::protocol::LOG_PAGE as usize || sample.events.len() == 20 || (terminal(&sample.status) && !drained);
                        for (i, bytes) in [&sample.stdout, &sample.stderr].iter().enumerate() {
                            ensure!(cursors[i] + bytes.len() as u64 <= crate::runner::LOG_CAP, "remote log exceeds capacity");
                            // File length is the committed cursor. Recovery truncates no data
                            // and never advances a separate cursor ahead of persisted bytes.
                            logs[i].write_all(bytes)?;
                            logs[i].sync_data()?;
                            if !bytes.is_empty() { live.lock().map_err(|_| anyhow::anyhow!("monitor state poisoned"))?.output(i, bytes); }
                        }
                        after_event = sample.next_event;
                        external["next_event"] = json!(after_event);
                        external["cursors"] = json!([logs[0].metadata()?.len(), logs[1].metadata()?.len()]);
                        external["status"] = json!(sample.status);
                        external["observation"] = sample.observation;
                        if !sample.events.is_empty() { external["recent_events"] = json!(sample.events); }
                        if terminal(&sample.status) && drained {
                            // Drain event pages as well before sealing the local evidence.
                            if sample.events.len() == 20 { return Ok(None); }
                            let state = match sample.status.as_str() { "completed" => "completed", "cancelled" => "cancelled", "timeout" => "timeout", "orphaned" => "orphaned", _ => "failed" };
                            external["stop"]["terminal_observed"] = json!(cancelled);
                            return Ok(Some((state, sample.exit_code, sample.signal, format!("remote terminal observed; detail={}", sample.detail.unwrap_or_default()))));
                        }
                        Ok(None)
                    }
                    CommandSpec::GithubActions { workflow } => {
                        let sample = super::actions::sample(workflow, &spec.cwd).await?;
                        let state = super::actions::terminal_state(&sample);
                        let detail = format!("workflow attempt observed: {} ({})", sample["conclusion"].as_str().unwrap_or("pending"), sample["url"].as_str().unwrap_or(""));
                        external["workflow"] = sample;
                        Ok(state.map(|state| (state, None, None, detail)))
                    }
                    _ => unreachable!(),
                }
            }.await;
            let mut finish = None;
            match result {
                Ok(outcome) => {
                    failures = 0;
                    external["connection"] = json!("connected");
                    external["last_success_at"] = json!(now());
                    external["last_error"] = Value::Null;
                    finish = outcome;
                }
                Err(error) => {
                    failures = failures.saturating_add(1);
                    external["connection"] = json!("disconnected");
                    // Our fixed transport/protocol errors contain no raw remote stderr.
                    external["last_error"] =
                        json!(error.to_string().chars().take(512).collect::<String>());
                }
            }
            external["consecutive_failures"] = json!(failures);
            external["evidence_stale"] = json!(failures > 0);
            let phase = if finish.is_some() {
                "finished"
            } else if cancelled {
                "remote_stop_pending"
            } else {
                "running"
            };
            publish(store, job, &live, &external, phase).await?;
            last_publish = tokio::time::Instant::now();
            if let Some((state, code, signal, mut detail)) = finish {
                let mut end = detail.len().min(4096);
                while !detail.is_char_boundary(end) {
                    end -= 1;
                }
                detail.truncate(end);
                return store
                    .finish(job.job_id.clone(), state, code, signal, detail)
                    .await;
            }
            let seconds = if failures == 0 {
                if matches!(spec.command, CommandSpec::GithubActions { .. }) {
                    5
                } else {
                    1
                }
            } else {
                (1u64 << failures.min(6)).min(60)
            };
            next = tokio::time::Instant::now()
                + if failures == 0 && catch_up {
                    Duration::ZERO
                } else {
                    Duration::from_secs(seconds)
                };
        }
        let pending = live
            .lock()
            .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
            .pending();
        for (rule, evidence) in pending {
            store
                .trigger_rule(job.job_id.clone(), rule.clone(), evidence, true)
                .await?;
            live.lock()
                .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
                .fired(&rule.id, rule.revision);
        }
        if last_publish.elapsed() >= Duration::from_secs(1) {
            publish(
                store,
                job,
                &live,
                &external,
                if cancelled {
                    "remote_stop_pending"
                } else {
                    "running"
                },
            )
            .await?;
            last_publish = tokio::time::Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
async fn publish(
    store: &Store,
    job: &Job,
    live: &crate::monitor::SharedLive,
    external: &Value,
    phase: &str,
) -> Result<()> {
    let (mut value, applied) = live
        .lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .snapshot(phase);
    value["external"] = external.clone();
    value["monitor_health"] = json!(if phase == "finished" {
        "finished"
    } else if external["evidence_stale"] == true {
        "degraded"
    } else {
        "running"
    });
    store.observe(job.job_id.clone(), value, applied).await
}
