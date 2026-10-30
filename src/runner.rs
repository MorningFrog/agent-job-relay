use crate::{config::Config, core::*, storage::Store};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
mod output;
mod supervisor;
pub const LOG_CAP: u64 = 16 * 1024 * 1024;
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
        tx.execute("UPDATE runs SET runner_pid=?,runner_identity=?,status='running',started=COALESCE(started,?) WHERE job_id=?",params![pid,identity,now(),key])?;
        let raw:String=tx.query_row("SELECT spec FROM jobs WHERE id=?",[key],|r|r.get(0))?;
        tx.commit()?;Ok(serde_json::from_str(&raw)?)
    }).await?;
    let outcome = supervisor::execute(&config, &store, &job, &spec).await;
    if let Err(e) = outcome {
        let (state, detail) = if spec.command.is_external() {
            (
                "orphaned",
                format!("external observer failed; external outcome unknown: {e:#}"),
            )
        } else {
            ("failed", format!("supervision error: {e:#}"))
        };
        store.finish(j, state, None, None, detail).await?;
    }
    drop(lock);
    Ok(())
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
        _ => anyhow::bail!("external execution cannot enter a Windows workload"),
    };
    let status = command
        .current_dir(spec.cwd)
        .envs(spec.env)
        .stdin(std::process::Stdio::null())
        .status()
        .context("cannot spawn Windows workload")?;
    std::process::exit(status.code().unwrap_or(1));
}
