use crate::{
    config::Config,
    core::*,
    storage::{Store, insert_event},
};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
#[cfg(not(windows))]
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
pub const LOG_CAP: u64 = 16 * 1024 * 1024;
async fn capture(mut source: impl AsyncRead + Unpin, path: std::path::PathBuf) -> Result<bool> {
    let file = crate::platform::private_file(&path)?;
    let mut file = tokio::fs::File::from_std(file);
    let mut buf = [0u8; 8192];
    let mut written = 0;
    let mut truncated = false;
    loop {
        let n = source.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let keep = (LOG_CAP - written).min(n as u64) as usize;
        if keep > 0 {
            file.write_all(&buf[..keep]).await?;
            written += keep as u64;
        }
        truncated |= keep < n;
    }
    file.sync_all().await?;
    Ok(truncated)
}
async fn capture_checked(
    source: impl AsyncRead + Unpin,
    path: std::path::PathBuf,
    errors: tokio::sync::mpsc::Sender<String>,
) -> Result<bool> {
    let result = capture(source, path).await;
    if let Err(e) = &result {
        let _ = errors
            .send(format!("output persistence failed: {e:#}"))
            .await;
    }
    result
}
pub async fn run(config: Config, j: String, nonce: String) -> Result<()> {
    crate::platform::require_runtime()?;
    crate::platform::runner_initialize()?;
    let store = Store::new(config.clone());
    let job = store.status(j.clone()).await?;
    let lock = config.run_lock(&job.run_id)?;
    lock.try_lock().context("another runner owns this run")?;
    let key = j.clone();
    let pid = std::process::id();
    let identity =
        crate::platform::identity(pid).context("cannot verify runner process identity")?;
    let spec:StartSpec=store.call(move|db|{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let ok=tx.execute("UPDATE jobs SET status='running',updated=? WHERE id=? AND status='launching' AND run_id IN (SELECT id FROM runs WHERE nonce=?)",params![now(),key,nonce])?;
        ensure!(ok==1,"launch claim is stale; command was not executed");
        tx.execute("UPDATE runs SET runner_pid=?,runner_identity=?,status='running',started=? WHERE job_id=?",params![pid,identity,now(),key])?;
        let raw:String=tx.query_row("SELECT spec FROM jobs WHERE id=?",[key],|r|r.get(0))?;
        tx.commit()?;Ok(serde_json::from_str(&raw)?)
    }).await?;
    let outcome = execute(&config, &store, &job, &spec).await;
    if let Err(e) = outcome {
        store
            .finish(j, "failed", None, None, format!("supervision error: {e:#}"))
            .await?;
    }
    drop(lock);
    Ok(())
}
async fn execute(config: &Config, store: &Store, job: &Job, spec: &StartSpec) -> Result<()> {
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
    let ident = crate::platform::identity(pid);
    let j = job.job_id.clone();
    let r = job.run_id.clone();
    // If publication fails the spawned process must be killed before returning.
    if let Err(e) = store
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
        .await
    {
        group_guard.signal(9)?;
        child.wait().await?;
        return Err(e);
    }
    let stdout = child.stdout.take().context("stdout not piped")?;
    let stderr = child.stderr.take().context("stderr not piped")?;
    let (errors, mut output_errors) = tokio::sync::mpsc::channel(2);
    let mut out = tokio::spawn(capture_checked(
        stdout,
        config.run_dir(&job.run_id).join("stdout.log"),
        errors.clone(),
    ));
    let mut err = tokio::spawn(capture_checked(
        stderr,
        config.run_dir(&job.run_id).join("stderr.log"),
        errors,
    ));
    let started = tokio::time::Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let mut forced = None;
    let mut capture_error = None;
    let exit = loop {
        tokio::select! {
            result=child.wait()=>{break result?;},
            Some(error)=output_errors.recv()=>{
                capture_error=Some(error);forced=Some("failed");
                group_guard.signal(15)?;
                tokio::time::sleep(Duration::from_millis(500)).await;
                group_guard.signal(9)?;
                break child.wait().await?;
            },
            _=tick.tick()=>{
                let cancelled=store.cancel_requested(job.job_id.clone()).await?;
                let timed=spec.timeout_seconds.is_some_and(|s|started.elapsed()>=Duration::from_secs(s));
                if cancelled||timed {
                    forced=Some(if cancelled {"cancelled"} else {"timeout"});
                    group_guard.signal(15)?;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    group_guard.signal(9)?;
                    break child.wait().await?;
                }
            }
        }
    };
    // Workloads are non-daemonizing: collect descendants that kept pipes open.
    group_guard.signal(15)?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    group_guard.signal(9)?;
    group_guard.disarm();
    let outputs = tokio::time::timeout(Duration::from_secs(2), async {
        Ok::<_, anyhow::Error>(((&mut out).await??, (&mut err).await??))
    })
    .await;
    if !matches!(&outputs, Ok(Ok(_))) {
        out.abort();
        err.abort();
    }
    for _ in 0..50 {
        if crate::platform::reap_descendants()? {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (state, detail) = match outputs {
        Ok(Ok((a, b))) => (
            forced.unwrap_or(if exit.success() {
                "completed"
            } else {
                "failed"
            }),
            capture_error.unwrap_or_else(|| format!("stdout_truncated={a}; stderr_truncated={b}")),
        ),
        Ok(Err(e)) => ("failed", format!("output persistence failed: {e}")),
        Err(_) => (
            "failed",
            "output drain deadline exceeded; descendant may have escaped process group".into(),
        ),
    };
    store
        .finish(
            job.job_id.clone(),
            state,
            exit.code(),
            crate::platform::exit_signal(&exit),
            detail,
        )
        .await
}

#[cfg(windows)]
pub async fn windows_workload(config: Config, job: String, name: String) -> Result<()> {
    crate::platform::join_workload(&name)?;
    crate::core::validate_id(&job)?;
    let store = Store::new(config);
    let spec: StartSpec = store
        .call(move |db| {
            let raw: String = db.query_row(
                "SELECT spec FROM jobs WHERE id=? AND status='running'",
                [job],
                |r| r.get(0),
            )?;
            Ok(serde_json::from_str(&raw)?)
        })
        .await?;
    let mut command = match spec.command {
        CommandSpec::Exec { program, args } => {
            let mut c = std::process::Command::new(program);
            c.args(args);
            c
        }
        CommandSpec::Shell {
            interpreter,
            script,
        } => crate::platform::shell_command(&interpreter, &script),
    };
    let status = command
        .current_dir(spec.cwd)
        .envs(spec.env)
        .stdin(std::process::Stdio::null())
        .status()
        .context("cannot spawn Windows workload")?;
    std::process::exit(status.code().unwrap_or(1));
}
