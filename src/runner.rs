use crate::{
    config::Config,
    core::*,
    storage::{Store, insert_event},
};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
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
pub async fn run(config: Config, j: String, nonce: String) -> Result<()> {
    crate::platform::require_runtime()?;
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
        tx.execute("UPDATE runs SET runner_pid=?,runner_identity=? WHERE job_id=?",params![pid,identity,key])?;
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
    let mut child = Command::from(command)
        .kill_on_drop(true)
        .spawn()
        .context("cannot spawn executable (check program and cwd)")?;
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
        crate::platform::signal_group(pid, 9)?;
        child.wait().await?;
        return Err(e);
    }
    let stdout = child.stdout.take().context("stdout not piped")?;
    let stderr = child.stderr.take().context("stderr not piped")?;
    let out = tokio::spawn(capture(
        stdout,
        config.run_dir(&job.run_id).join("stdout.log"),
    ));
    let err = tokio::spawn(capture(
        stderr,
        config.run_dir(&job.run_id).join("stderr.log"),
    ));
    let started = tokio::time::Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let mut forced = None;
    let exit = loop {
        tokio::select! {
            result=child.wait()=>{break result?;},
            _=tick.tick()=>{
                let state=store.status(job.job_id.clone()).await?;
                let timed=spec.timeout_seconds.is_some_and(|s|started.elapsed()>=Duration::from_secs(s));
                if state.cancel_requested||timed {
                    forced=Some(if state.cancel_requested {"cancelled"} else {"timeout"});
                    crate::platform::signal_group(pid,15)?;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    crate::platform::signal_group(pid,9)?;
                    break child.wait().await?;
                }
            }
        }
    };
    // Workloads are non-daemonizing: collect descendants that kept pipes open.
    crate::platform::signal_group(pid, 15)?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    crate::platform::signal_group(pid, 9)?;
    let outputs = tokio::time::timeout(Duration::from_secs(2), async {
        Ok::<_, anyhow::Error>((out.await??, err.await??))
    })
    .await;
    let (state, detail) = match outputs {
        Ok(Ok((a, b))) => (
            forced.unwrap_or(if exit.success() {
                "completed"
            } else {
                "failed"
            }),
            format!("stdout_truncated={a}; stderr_truncated={b}"),
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
