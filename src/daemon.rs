use crate::{config::Config, core::*, storage::Store};
#[cfg(any(unix, windows))]
use anyhow::Context;
use anyhow::Result;
use rusqlite::params;
use std::process::Stdio;
#[cfg(any(unix, windows))]
use std::time::Duration;
pub async fn tick(store: &Store) -> Result<()> {
    let active = store.active().await?;
    for job in active {
        match job.status.as_str() {
            "queued" => {
                let j = job.job_id.clone();
                let nonce=store.call(move|db|{let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                    let changed=tx.execute("UPDATE jobs SET status='launching',updated=? WHERE id=? AND status='queued'",params![now(),j])?;
                    if changed==0 {return Ok(None);}
                    tx.execute("UPDATE runs SET status='launching' WHERE job_id=?",[&j])?;
                    let n:String=tx.query_row("SELECT nonce FROM runs WHERE job_id=?",[&j],|r|r.get(0))?;tx.commit()?;Ok(Some(n)) }).await?;
                if let Some(nonce) = nonce {
                    let mut c = std::process::Command::new(crate::platform::self_image()?);
                    c.arg("--state-dir")
                        .arg(&store.config.root)
                        .arg("runner")
                        .arg(&job.job_id)
                        .arg(nonce)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    match crate::platform::spawn_background(c) {
                        Ok(()) => {}
                        Err(e) => {
                            store
                                .finish(
                                    job.job_id,
                                    "failed",
                                    None,
                                    None,
                                    format!("runner spawn failed: {e:#}"),
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
    #[cfg(any(unix, windows))]
    {
        let path = config.socket();
        let mut listener =
            crate::platform::Listener::bind(&path).context("cannot bind private relay endpoint")?;
        // Keep the receiver alive while reconciliation awaits database work.
        // Recreating ctrl_c() inside select loses signals in that interval.
        let mut interrupt = crate::platform::interrupt_signal()?;
        let delivery_task = tokio::spawn(crate::delivery::worker(store.clone()));
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _=interrupt.recv()=>break,
                _=tick.tick()=>{if let Err(e)=self::tick(&store).await {tracing::error!(error=%e,"reconciliation failed");}},
                r=listener.accept()=>{let s=r?;let permit=match slots.clone().try_acquire_owned(){Ok(p)=>p,Err(_)=>continue};
                    let service=store.clone();tasks.spawn(async move{let _permit=permit;match crate::ipc::handle(s,service).await {Ok(shutdown)=>shutdown,Err(e)=>{tracing::debug!(error=%e,"IPC request rejected");false}}});},
                Some(r)=tasks.join_next()=>{match r {Ok(true)=>break,Ok(false)=>{},Err(e)=>tracing::error!(error=%e,"IPC worker panicked")}}
            }
        }
        delivery_task.abort();
        let _ = delivery_task.await;
        tasks.shutdown().await;
        crate::platform::cleanup_endpoint(&path)?;
    }
    Ok(())
}
