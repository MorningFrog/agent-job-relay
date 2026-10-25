use crate::{config::Config, core::*, storage::Store};
use anyhow::{Context, Result};
use rusqlite::params;
use std::{process::Stdio, time::Duration};
pub async fn tick(store: &Store) -> Result<()> {
    let active = store.active().await?;
    for job in active {
        match job.status.as_str() {
            "queued" => {
                let j = job.job_id.clone();
                let nonce=store.call(move|db|{let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                    let changed=tx.execute("UPDATE jobs SET status='launching',updated=? WHERE id=? AND status='queued'",params![now(),j])?;
                    if changed==0 {return Ok(None);}
                    let n:String=tx.query_row("SELECT nonce FROM runs WHERE job_id=?",[&j],|r|r.get(0))?;tx.commit()?;Ok(Some(n)) }).await?;
                if let Some(nonce) = nonce {
                    let mut c = std::process::Command::new(std::env::current_exe()?);
                    c.arg("--state-dir")
                        .arg(&store.config.root)
                        .arg("runner")
                        .arg(&job.job_id)
                        .arg(nonce)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    crate::platform::detach(&mut c)?;
                    match tokio::process::Command::from(c).spawn() {
                        Ok(mut child) => {
                            tokio::spawn(async move {
                                if let Err(e) = child.wait().await {
                                    tracing::error!(error=%e,"runner reap failed");
                                }
                            });
                        }
                        Err(e) => {
                            store
                                .finish(
                                    job.job_id,
                                    "failed",
                                    None,
                                    None,
                                    format!("runner spawn failed: {e}"),
                                )
                                .await?
                        }
                    }
                }
            }
            "launching" | "running" => {
                if job.status == "launching" && now() - job.updated_at < 5000 {
                    continue;
                }
                let lock = store.config.run_lock(&job.run_id)?;
                match lock.try_lock() {
                    Ok(()) => {
                        // Hold the lock through finalization: a delayed runner cannot claim.
                        store.finish(job.job_id,"orphaned",None,None,"runner lock lost; execution outcome unknown; command was not replayed".into()).await?;
                    }
                    Err(std::fs::TryLockError::WouldBlock) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            _ => {}
        }
    }
    Ok(())
}
pub async fn serve(config: Config) -> Result<()> {
    crate::platform::require_runtime()?;
    config.prepare()?;
    let _lock = config.daemon_lock()?;
    let store = Store::new(config.clone());
    store.init().await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};
        let path = config.socket();
        anyhow::ensure!(
            path.as_os_str().len() < 104,
            "state path too long for Unix socket; use shorter --state-dir"
        );
        if path.exists() {
            let m = std::fs::symlink_metadata(&path)?;
            anyhow::ensure!(
                m.file_type().is_socket(),
                "refusing to replace a non-socket IPC path"
            );
            std::fs::remove_file(&path)?;
        }
        let listener = tokio::net::UnixListener::bind(&path).context("cannot bind relay socket")?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let delivery_task = tokio::spawn(crate::delivery::worker(store.clone()));
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _=tokio::signal::ctrl_c()=>break,
                _=tick.tick()=>{if let Err(e)=self::tick(&store).await {tracing::error!(error=%e,"reconciliation failed");}},
                r=listener.accept()=>{let (s,_)=r?;let permit=match slots.clone().try_acquire_owned(){Ok(p)=>p,Err(_)=>continue};
                    let service=store.clone();tasks.spawn(async move{let _permit=permit;if let Err(e)=crate::ipc::handle(s,service).await {tracing::debug!(error=%e,"IPC request rejected");}});},
                Some(r)=tasks.join_next()=>{if let Err(e)=r {tracing::error!(error=%e,"IPC worker panicked");}}
            }
        }
        delivery_task.abort();
        let _ = delivery_task.await;
        tasks.shutdown().await;
        std::fs::remove_file(path)?;
    }
    Ok(())
}
