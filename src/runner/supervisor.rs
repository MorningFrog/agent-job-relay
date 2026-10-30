use super::output::Captures;
use crate::{
    config::Config,
    core::*,
    storage::{Store, insert_event},
};
use anyhow::{Context, Result};
use rusqlite::params;
#[cfg(not(windows))]
use std::process::Stdio;
use std::time::Duration;
pub(super) async fn execute(
    config: &Config,
    store: &Store,
    job: &Job,
    spec: &StartSpec,
) -> Result<()> {
    if spec.command.is_external() {
        return crate::remote::observe(config, store, job, spec).await;
    }
    if store.status(job.job_id.clone()).await?.cancel_requested {
        return store
            .finish(
                job.job_id.clone(),
                "cancelled",
                None,
                None,
                "cancelled before workload spawn".into(),
            )
            .await;
    }
    #[cfg(not(windows))]
    let (mut child, mut group_guard) = {
        let mut command = match &spec.command {
            CommandSpec::Exec { program, args } => {
                let mut c = std::process::Command::new(program);
                c.args(args);
                c
            }
            CommandSpec::Shell {
                interpreter,
                script,
            } => {
                let mut c = std::process::Command::new(interpreter);
                c.args(["-c", script]);
                c
            }
            _ => unreachable!("external execution uses the observer"),
        };
        command
            .current_dir(&spec.cwd)
            .envs(&spec.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::platform::workload(&mut command)?;
        let child = tokio::process::Command::from(command)
            .kill_on_drop(true)
            .spawn()
            .context("cannot spawn executable (check program and cwd)")?;
        let pid = child.id().context("missing workload PID")?;
        let group_guard = crate::platform::GroupGuard::new(pid);
        (child, group_guard)
    };
    #[cfg(windows)]
    let (mut child, mut group_guard) = crate::platform::spawn_workload(config, job)?;
    let pid = child.id().context("missing workload PID")?;
    let live = crate::monitor::Live::new();
    let (errors, mut output_errors) = tokio::sync::mpsc::channel(2);
    let mut captures = None;
    let mut forced = None;
    let mut capture_error = None;
    let mut stopping: Option<Stopping> = None;
    let mut phase = Phase::Running;
    // Every error after spawn passes through cleanup, even when SQLite cannot
    // accept final evidence. Output tasks are scoped and aborted on all exits.
    let outcome = async {
        publish_started(store, job, pid).await?;
        let (_, _, _, rules) = store.control(job.job_id.clone()).await?;
        live.lock().map_err(|_| anyhow::anyhow!("monitor state poisoned"))?.apply(rules);
        captures = Some(Captures::start(
            child.stdout.take().context("stdout not piped")?,
            child.stderr.take().context("stderr not piped")?,
            config.run_dir(&job.run_id), errors, live.clone(),
        ));
        let started = tokio::time::Instant::now();
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        let mut observation_at = tokio::time::Instant::now();
        let exit = loop {
            tokio::select! {
                biased;
                result=child.wait()=>{break result?;},
                Some(error)=output_errors.recv()=>{
                    capture_error=Some(error);forced=Some(JobState::Failed);
                    group_guard.signal(9)?;phase=Phase::Forced;
                    break child.wait().await?;
                },
                _=tick.tick()=>{
                    let (cancelled,grace,reason,rules)=store.control(job.job_id.clone()).await?;
                    live.lock().map_err(|_|anyhow::anyhow!("monitor state poisoned"))?.apply(rules);
                    persist_pending(store, job, &live, true).await?;
                    let timed=spec.timeout_seconds.is_some_and(|s|started.elapsed()>=Duration::from_secs(s));
                    if stopping.is_none() && (cancelled||timed) {
                        forced=Some(if cancelled {JobState::Cancelled} else {JobState::Timeout});
                        stopping=Some(Stopping {at: tokio::time::Instant::now(), grace: if timed {5}else{grace}, reason: if timed {"timeout".into()}else{reason}});
                        group_guard.signal(15)?;phase=Phase::Graceful;
                    }
                    if let Some(stop)=&mut stopping {
                        if cancelled {stop.grace=stop.grace.min(grace);}
                        if stop.at.elapsed()>=Duration::from_secs(stop.grace as u64) {
                            group_guard.signal(9)?;phase=Phase::Forced;
                            break child.wait().await?;
                        }
                    }
                    if tokio::time::Instant::now()>=observation_at {
                        let (snapshot,applied)=live.lock().map_err(|_|anyhow::anyhow!("monitor state poisoned"))?.snapshot(phase.as_str());
                        store.observe(job.job_id.clone(),snapshot,applied).await?;
                        observation_at=tokio::time::Instant::now()+Duration::from_secs(1);
                    }
                }
            }
        };
        Ok::<_, anyhow::Error>(exit)
    }.await;
    let cleaned = cleanup(&mut child, &mut group_guard).await;
    let exit = match outcome {
        Ok(exit) => {
            cleaned?;
            exit
        }
        Err(error) => {
            return Err(match cleaned {
                Ok(()) => error,
                Err(cleanup) => error.context(format!("workload cleanup failed: {cleanup:#}")),
            });
        }
    };
    let outputs = captures
        .as_mut()
        .context("missing output capture")?
        .drain()
        .await;
    persist_pending(store, job, &live, false).await?;
    let (mut snapshot, applied) = live
        .lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .snapshot(if forced.is_some() {
            phase.as_str()
        } else {
            "finished"
        });
    snapshot["monitor_health"] = serde_json::json!("finished");
    store.observe(job.job_id.clone(), snapshot, applied).await?;
    let (state, mut detail) = match outputs {
        Ok((a, b)) => (
            forced.unwrap_or(if exit.success() {
                JobState::Completed
            } else {
                JobState::Failed
            }),
            capture_error.unwrap_or_else(|| format!("stdout_truncated={a}; stderr_truncated={b}")),
        ),
        Err(e) => (
            JobState::Failed,
            format!("output persistence failed: {e:#}"),
        ),
    };
    if let Some(stop) = stopping {
        detail = format!(
            "{detail}; stop_reason={}; stop_method={}",
            stop.reason,
            phase.as_str()
        );
    }
    store
        .finish(
            job.job_id.clone(),
            state.as_str(),
            exit.code(),
            crate::platform::exit_signal(&exit),
            detail,
        )
        .await
}

#[derive(Clone, Copy)]
enum Phase {
    Running,
    Graceful,
    Forced,
}
impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Graceful => "graceful",
            Self::Forced => "forced",
        }
    }
}
struct Stopping {
    at: tokio::time::Instant,
    grace: u32,
    reason: String,
}

async fn cleanup(
    child: &mut tokio::process::Child,
    group: &mut crate::platform::GroupGuard,
) -> Result<()> {
    let gentle = group.signal(15);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let forced = group.signal(9);
    tokio::time::timeout(Duration::from_secs(2), child.wait())
        .await
        .context("workload exit deadline exceeded")??;
    // SIGKILL adoption can occur after the first scan. Repeat only inside the
    // runner's own child inventory, pinning each process instance before signal.
    for _ in 0..50 {
        if crate::platform::reap_descendants()? {
            gentle?;
            forced?;
            group.disarm();
            return Ok(());
        }
        crate::platform::terminate_adopted_descendants()?;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    anyhow::bail!("descendant cleanup deadline exceeded; no successful completion recorded")
}

async fn publish_started(store: &Store, job: &Job, pid: u32) -> Result<()> {
    let ident = crate::platform::identity(pid);
    let j = job.job_id.clone();
    let r = job.run_id.clone();
    store
        .call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "UPDATE runs SET workload_pid=?,workload_identity=? WHERE id=?",
                params![pid, ident, r],
            )?;
            insert_event(
                &tx,
                &j,
                &r,
                "started",
                serde_json::json!({"workload_pid":pid}),
            )?;
            tx.commit()?;
            Ok(())
        })
        .await?;
    Ok(())
}

async fn persist_pending(
    store: &Store,
    job: &Job,
    live: &crate::monitor::SharedLive,
    can_stop: bool,
) -> Result<()> {
    let progress = live
        .lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .take_progress();
    store
        .record_progress(job.job_id.clone(), progress, can_stop)
        .await?;
    let pending = live
        .lock()
        .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
        .pending();
    for (rule, evidence) in pending {
        store
            .trigger_rule(job.job_id.clone(), rule.clone(), evidence, can_stop)
            .await?;
        if can_stop {
            live.lock()
                .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
                .fired(&rule.id, rule.revision);
        }
    }
    Ok(())
}
