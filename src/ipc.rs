use crate::{config::Config, core::*, storage::Store};
#[cfg(unix)]
use anyhow::Context;
use anyhow::{Result, bail, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
#[derive(Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Start {
        spec: StartSpec,
    },
    Status {
        job_id: String,
    },
    List {
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    Stop {
        job_id: String,
    },
    Logs {
        job_id: String,
        #[serde(default = "default_stream")]
        stream: String,
        #[serde(default)]
        cursor: u64,
        #[serde(default = "default_bytes")]
        limit: u32,
    },
    Events {
        job_id: String,
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    Wait {
        job_id: String,
        #[serde(default)]
        after: i64,
        #[serde(default = "default_wait")]
        seconds: u32,
    },
    Doctor,
    Inbox {
        session_key: String,
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    RegisterSession {
        registration: crate::session::Registration,
    },
    Sessions {
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    InvalidateSession {
        session_key: String,
    },
    ProbeSession {
        session_key: String,
    },
    AddWakeTarget {
        job_id: String,
        session_key: String,
        #[serde(default)]
        replay: bool,
    },
    RebindTarget {
        target_id: String,
        session_key: String,
    },
    Deliveries {
        job_id: Option<String>,
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    RetryDelivery {
        delivery_id: String,
    },
    DeliveryAttempts {
        delivery_id: String,
        #[serde(default)]
        after: u32,
        #[serde(default = "default_limit")]
        limit: u32,
    },
}
fn default_limit() -> u32 {
    20
}
fn default_bytes() -> u32 {
    8192
}
fn default_stream() -> String {
    "stdout".into()
}
fn default_wait() -> u32 {
    10
}
pub async fn dispatch(store: &Store, req: Request) -> Result<Value> {
    match req {
        Request::Inbox{session_key,after,limit}=>store.inbox(session_key,after,limit).await,
        Request::RegisterSession{registration}=>{let s=store.register(registration).await?;Ok(json!({"capabilities":crate::session::capabilities(&s.registration.adapter),"session":s}))},
        Request::Sessions{after,limit}=>store.sessions(after,limit).await,
        Request::InvalidateSession{session_key}=>store.invalidate(session_key).await,
        Request::ProbeSession{session_key}=>{let s=store.session(session_key).await?;let probe=crate::adapters::probe(&s).await?;Ok(json!({"probe":probe,"lease_valid":s.connected&&s.expires_at>now(),"capabilities":crate::session::capabilities(&s.registration.adapter),"session":s}))},
        Request::AddWakeTarget{job_id,session_key,replay}=>store.target(job_id,session_key,replay).await,
        Request::RebindTarget{target_id,session_key}=>store.rebind(target_id,session_key).await,
        Request::Deliveries{job_id,after,limit}=>store.deliveries(job_id,after,limit).await,
        Request::RetryDelivery{delivery_id}=>store.retry_delivery(delivery_id).await,
        Request::DeliveryAttempts{delivery_id,after,limit}=>store.attempts(delivery_id,after,limit).await,
        Request::Start{spec}=>{
            let j=store.start(spec).await?;
            let wake=if let Some(key)=&j.session {let s=store.session(key.clone()).await?;json!({"session_key":key,"capabilities":crate::session::capabilities(&s.registration.adapter),"live_e2e_verified":false,"warning":"Inbox requires explicit inspection; Desktop auto-resume is experimental and unverified. Avoid polling loops."})}
                else {json!({"can_resume":false,"warning":"No wake target registered; use bounded job_wait for short jobs or inspect later. Do not poll in a loop."})};
            Ok(json!({"job":j,"wake":wake}))
        },
        Request::Status{job_id}=>Ok(serde_json::to_value(store.status(job_id).await?)?),
        Request::List{after,limit}=>{ let jobs=store.list(after,limit).await?; Ok(json!({"next_cursor":jobs.last().map_or(after,|j|j.sequence),"jobs":jobs})) },
        Request::Stop{job_id}=>Ok(serde_json::to_value(store.stop(job_id).await?)?),
        Request::Events{job_id,after,limit}=>{let e=store.events(job_id,after,limit).await?;Ok(json!({"next_cursor":e.last().map_or(after,|e|e.seq),"events":e,"untrusted":true}))},
        Request::Wait{job_id,after,seconds}=>{ensure!((1..=30).contains(&seconds),"wait must be 1..30 seconds");let deadline=tokio::time::Instant::now()+Duration::from_secs(seconds as u64);
            loop {let e=store.events(job_id.clone(),after,100).await?;let j=store.status(job_id.clone()).await?;
                if !e.is_empty()||terminal(&j.status)||tokio::time::Instant::now()>=deadline{return Ok(json!({"job":j,"events":e,"next_cursor":e.last().map_or(after,|e|e.seq)}));}
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        },
        Request::Logs{job_id,stream,cursor,limit}=>{
            ensure!((1..=65536).contains(&limit),"log limit must be 1..65536 bytes");ensure!(stream=="stdout"||stream=="stderr","stream must be stdout or stderr");
            let j=store.status(job_id).await?; let path=store.config.run_dir(&j.run_id).join(format!("{stream}.log"));
            use tokio::io::{AsyncReadExt,AsyncSeekExt};
            let mut data=vec![0;limit as usize]; let n=match tokio::fs::File::open(path).await {
                Ok(mut f)=>{ensure!(cursor<=f.metadata().await?.len(),"cursor is beyond end of log");f.seek(std::io::SeekFrom::Start(cursor)).await?;f.read(&mut data).await?},
                Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{ensure!(cursor==0,"log not created yet; cursor must be zero");0},
                Err(e)=>return Err(e.into())
            };data.truncate(n);Ok(json!({"text":String::from_utf8_lossy(&data),"bytes":n,"next_cursor":cursor+n as u64,"untrusted":true,"encoding":"utf8_lossy","cap_bytes":crate::runner::LOG_CAP}))
        },
        Request::Doctor=>store.call(|db|{let integrity:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;
            let active:i64=db.query_row("SELECT count(*) FROM jobs WHERE status IN ('queued','launching','running')",[],|r|r.get(0))?;
            Ok(json!({"daemon":"reachable","store":integrity,"active_jobs":active,"sessions":db.query_row("SELECT count(*) FROM sessions WHERE connected=1 AND expires>?",[now()],|r|r.get::<_,i64>(0))?,"dead_letters":db.query_row("SELECT count(*) FROM deliveries WHERE status='dead_letter'",[],|r|r.get::<_,i64>(0))?,"runtime":std::env::consts::OS,"auto_resume":"not verified","next_step":"Register an explicit session; Desktop requires opt-in experimental binding. No service is installed."})) }).await
    }
}
pub async fn call(config: &Config, request: Request) -> Result<Value> {
    crate::platform::require_runtime()?;
    #[cfg(unix)]
    {
        tokio::time::timeout(Duration::from_secs(40), async {
            let mut stream = tokio::net::UnixStream::connect(config.socket())
                .await
                .context(
                    "daemon unavailable; run agent-job-relay --state-dir <same-path> daemon",
                )?;
            write_frame(&mut stream, &serde_json::to_value(request)?).await?;
            let response = read_frame(&mut stream, 1048576).await?;
            if response["ok"] == true {
                Ok(response["result"].clone())
            } else {
                bail!(
                    "{}",
                    response["error"]
                        .as_str()
                        .unwrap_or("invalid daemon response")
                )
            }
        })
        .await
        .context("daemon request timed out (40s)")?
    }
    #[cfg(not(unix))]
    {
        let _ = (config, request);
        bail!("native IPC unavailable")
    }
}
#[cfg(unix)]
pub async fn handle(mut stream: tokio::net::UnixStream, store: Store) -> Result<()> {
    let cred = stream.peer_cred()?;
    ensure!(
        cred.uid() == unsafe { libc::geteuid() },
        "IPC peer owner mismatch"
    );
    let raw =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut stream, 131072)).await??;
    let result = match serde_json::from_value::<Request>(raw) {
        Ok(req) => dispatch(&store, req).await,
        Err(e) => Err(e.into()),
    };
    let envelope = match result {
        Ok(v) => json!({"ok":true,"result":v}),
        Err(e) => json!({"ok":false,"error":format!("{e:#}")}),
    };
    tokio::time::timeout(Duration::from_secs(5), write_frame(&mut stream, &envelope)).await??;
    Ok(())
}
pub async fn read_frame<S: tokio::io::AsyncRead + Unpin>(s: &mut S, max: usize) -> Result<Value> {
    use tokio::io::AsyncReadExt;
    let n = s.read_u32_le().await? as usize;
    ensure!(n > 0 && n <= max, "invalid frame size");
    let mut b = vec![0; n];
    s.read_exact(&mut b).await?;
    Ok(serde_json::from_slice(&b)?)
}
pub async fn write_frame<S: tokio::io::AsyncWrite + Unpin>(s: &mut S, v: &Value) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let b = serde_json::to_vec(v)?;
    ensure!(b.len() <= 1048576, "response exceeds frame limit");
    s.write_u32_le(b.len() as u32).await?;
    s.write_all(&b).await?;
    s.flush().await?;
    Ok(())
}
