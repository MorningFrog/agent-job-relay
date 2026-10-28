use crate::{config::Config, core::*, storage::Store};
#[cfg(any(unix, windows))]
use anyhow::Context;
#[cfg(not(any(unix, windows)))]
use anyhow::bail;
use anyhow::{Result, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

/// Preserve the authenticated peer when an older daemon rejects shutdown.
/// This is not inferred from a PID file or a process-name search.
#[derive(Debug)]
pub(crate) struct RemoteError {
    pub message: String,
    pub shutdown_peer: Option<(u32, String)>,
}
impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RemoteError {}
#[derive(Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Start {
        spec: StartSpec,
    },
    StartBatch {
        #[schemars(length(min = 1, max = 16))]
        specs: Vec<StartSpec>,
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
        #[serde(default = "crate::monitor::default_grace")]
        grace_seconds: u32,
        #[serde(default)]
        wait_seconds: u32,
    },
    Inspect {
        #[serde(default)]
        job_id: Option<String>,
        #[serde(default)]
        before: Option<i64>,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    Monitor {
        job_id: String,
        #[serde(default)]
        mutation: Option<crate::monitor::Mutation>,
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
    UiInfo,
    Shutdown,
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
    RecoverSession {
        session_key: String,
        #[serde(default)]
        apply: bool,
        #[serde(default)]
        after: i64,
        #[serde(default = "default_limit")]
        limit: u32,
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
        Request::Shutdown => {
            Ok(json!({"daemon":"stopping","runners":"continue independently","data":"preserved"}))
        }
        Request::Inbox {
            session_key,
            after,
            limit,
        } => store.inbox(session_key, after, limit).await,
        Request::RegisterSession { registration } => {
            let s = store.register(registration).await?;
            Ok(
                json!({"capabilities":crate::session::capabilities(&s.registration.adapter),"session":s}),
            )
        }
        Request::Sessions { after, limit } => store.sessions(after, limit).await,
        Request::InvalidateSession { session_key } => store.invalidate(session_key).await,
        Request::ProbeSession { session_key } => {
            let s = store.session(session_key).await?;
            let probe = crate::adapters::probe(&s).await?;
            Ok(
                json!({"probe":probe,"lease_valid":s.connected&&s.expires_at>now(),"capabilities":crate::session::capabilities(&s.registration.adapter),"session":s}),
            )
        }
        Request::RecoverSession {
            session_key,
            apply,
            after,
            limit,
        } => {
            let s = store.session(session_key).await?;
            crate::adapters::probe(&s).await?;
            store.recover_session(s, apply, after, limit).await
        }
        Request::AddWakeTarget {
            job_id,
            session_key,
            replay,
        } => store.target(job_id, session_key, replay).await,
        Request::RebindTarget {
            target_id,
            session_key,
        } => store.rebind(target_id, session_key).await,
        Request::Deliveries {
            job_id,
            after,
            limit,
        } => store.deliveries(job_id, after, limit).await,
        Request::RetryDelivery { delivery_id } => store.retry_delivery(delivery_id).await,
        Request::DeliveryAttempts {
            delivery_id,
            after,
            limit,
        } => store.attempts(delivery_id, after, limit).await,
        Request::Start { spec } => {
            let j = store.start(spec).await?;
            Ok(start_result(store, j).await)
        }
        Request::StartBatch { specs } => crate::batch::submit(store, specs).await,
        Request::Status { job_id } => Ok(serde_json::to_value(store.status(job_id).await?)?),
        Request::List { after, limit } => {
            let jobs = store.list(after, limit).await?;
            Ok(json!({"next_cursor":jobs.last().map_or(after,|j|j.sequence),"jobs":jobs}))
        }
        Request::Inspect {
            job_id,
            before,
            limit,
        } => store.inspect(job_id, before, limit).await,
        Request::Monitor { job_id, mutation } => store.monitor(job_id, mutation).await,
        Request::Stop {
            job_id,
            grace_seconds,
            wait_seconds,
        } => {
            ensure!(wait_seconds <= 30, "wait_seconds must be 0..30");
            let mut job = store
                .stop_with(job_id.clone(), grace_seconds, "manual".into())
                .await?;
            let until = tokio::time::Instant::now() + Duration::from_secs(wait_seconds as u64);
            while !terminal(&job.status) && tokio::time::Instant::now() < until {
                tokio::time::sleep(Duration::from_millis(100)).await;
                job = store.status(job_id.clone()).await?;
            }
            let mut value = serde_json::to_value(&job)?;
            value["stop_outcome"] = json!(if terminal(&job.status) {
                "terminal_observed"
            } else {
                "requested"
            });
            Ok(value)
        }
        Request::Events {
            job_id,
            after,
            limit,
        } => {
            let e = store.events(job_id, after, limit).await?;
            Ok(json!({"next_cursor":e.last().map_or(after,|e|e.seq),"events":e,"untrusted":true}))
        }
        Request::Wait {
            job_id,
            after,
            seconds,
        } => {
            ensure!((1..=30).contains(&seconds), "wait must be 1..30 seconds");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds as u64);
            loop {
                let e = store.events(job_id.clone(), after, 100).await?;
                let j = store.status(job_id.clone()).await?;
                if !e.is_empty() || terminal(&j.status) || tokio::time::Instant::now() >= deadline {
                    return Ok(
                        json!({"job":j,"events":e,"next_cursor":e.last().map_or(after,|e|e.seq)}),
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        Request::Logs {
            job_id,
            stream,
            cursor,
            limit,
        } => {
            ensure!(
                (1..=65536).contains(&limit),
                "log limit must be 1..65536 bytes"
            );
            ensure!(
                stream == "stdout" || stream == "stderr",
                "stream must be stdout or stderr"
            );
            let j = store.status(job_id).await?;
            crate::archive::logs(store.config.clone(), j.run_id, stream, cursor, limit).await
        }
        Request::UiInfo => Ok(store
            .web_info
            .lock()
            .map_err(|_| anyhow::anyhow!("UI state poisoned"))?
            .clone()),
        Request::Doctor => {
            let mut value=store.read(|db|{let _:i64=db.query_row("PRAGMA schema_version",[],|r|r.get(0))?;
            let integrity="ok";
            let active:i64=db.query_row("SELECT count(*) FROM jobs WHERE status IN ('queued','launching','running')",[],|r|r.get(0))?;
            Ok(json!({"daemon":"reachable","runner_launch_from_loaded_image":cfg!(target_os="linux")&&crate::platform::self_image().is_ok(),"protocol_version":1,"version":env!("CARGO_PKG_VERSION"),"store":integrity,"capacity":crate::maintenance::capacity(db)?,"store_check":"connection and schema; use data check for full integrity","active_jobs":active,"orphaned_jobs":db.query_row("SELECT count(*) FROM jobs WHERE status='orphaned'",[],|r|r.get::<_,i64>(0))?,"delivery_recovery":true,"same_chat_recovery":true,"plugin_bridge":true,"delivery_time_renewal":true,"rebind_required":db.query_row("SELECT count(*) FROM deliveries WHERE status='retry_wait' AND failure_kind='rebind_required'",[],|r|r.get::<_,i64>(0))?,"retrying_deliveries":db.query_row("SELECT count(*) FROM deliveries WHERE status='retry_wait'",[],|r|r.get::<_,i64>(0))?,"sessions":db.query_row("SELECT count(*) FROM sessions WHERE connected=1 AND expires>?",[now()],|r|r.get::<_,i64>(0))?,"dead_letters":db.query_row("SELECT count(*) FROM deliveries WHERE status='dead_letter'",[],|r|r.get::<_,i64>(0))?,"runtime":std::env::consts::OS,"daemon_launch_method":std::env::var("AJR_DAEMON_LAUNCH_METHOD").unwrap_or_else(|_| "foreground".into()),"auto_resume":"not verified","next_step":"Use start --wake or job_start with wake_me=true from the current Desktop host context; no service is installed."})) }).await?;
            value["runtime_monitoring"] = json!(true);
            value["batch_submission"] = json!(true);
            value["remote_execution"] = json!(true);
            value["github_actions_observation"] = json!(true);
            let mut ui = store
                .web_info
                .lock()
                .map_err(|_| anyhow::anyhow!("UI state poisoned"))?
                .clone();
            if let Some(object) = ui.as_object_mut() {
                object.remove("access_url");
            }
            value["web_ui"] = ui;
            Ok(value)
        }
    }
}
/// Once admitted, a job must remain visible even if wake diagnostics fail.
pub(crate) async fn start_result(store: &Store, job: Job) -> Value {
    let wake = if let Some(key) = &job.session {
        match store.session(key.clone()).await {
            Ok(session) => {
                json!({"session_key":key,"capabilities":crate::session::capabilities(&session.registration.adapter),"live_e2e_verified":false,"warning":"Inbox requires explicit inspection; Desktop wake is experimental. Host acceptance does not confirm Agent execution. Avoid polling loops."})
            }
            Err(error) => {
                json!({"session_key":key,"can_resume":false,"warning":format!("job admitted; wake diagnostics unavailable: {error:#}")})
            }
        }
    } else {
        json!({"can_resume":false,"warning":"No wake target registered; use bounded job_wait for short jobs or inspect later. Do not poll in a loop."})
    };
    json!({"job":job,"wake":wake})
}
pub async fn call(config: &Config, request: Request) -> Result<Value> {
    crate::platform::require_runtime()?;
    #[cfg(any(unix, windows))]
    {
        let is_doctor = matches!(&request, Request::Doctor);
        let is_shutdown = matches!(&request, Request::Shutdown);
        tokio::time::timeout(Duration::from_secs(40), async {
            let mut stream = crate::platform::connect_host(&config.socket())
                .await
                .context(
                    "daemon unavailable; start/job_start auto-starts it, or run agent-job-relay --state-dir <same-path> daemon",
                )?;
            let shutdown_peer = if is_shutdown {
                crate::platform::peer_pid(&stream)?
                    .and_then(|pid| crate::platform::identity(pid).map(|identity| (pid, identity)))
            } else { None };
            let image_path_available = if is_doctor {
                crate::platform::peer_pid(&stream).ok().flatten()
                    .and_then(crate::platform::process_image_path_available)
            } else { None };
            write_frame(
                &mut stream,
                &json!({"protocol_version":1,"request":request}),
            )
            .await?;
            let response = read_frame(&mut stream, 1048576).await?;
            ensure!(
                response["protocol_version"] == 1,
                "unsupported daemon protocol; restart daemon with this executable"
            );
            if response["ok"] == true {
                let mut result = response["result"].clone();
                if let Some(available) = image_path_available {
                    ensure!(result.is_object(), "malformed doctor result");
                    result["daemon_executable_path_available"] = json!(available);
                }
                Ok(result)
            } else {
                Err(RemoteError {
                    message: response["error"].as_str().unwrap_or("invalid daemon response").into(),
                    shutdown_peer,
                }.into())
            }
        })
        .await
        .context("daemon request timed out (40s)")?
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (config, request);
        bail!("native IPC unavailable")
    }
}
#[cfg(any(unix, windows))]
pub async fn handle(mut stream: crate::platform::ServerStream, store: Store) -> Result<bool> {
    crate::platform::verify_peer(&stream)?;
    let raw =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut stream, 131072)).await??;
    let mut shutdown = false;
    let result = if raw["protocol_version"] != 1 {
        Err(anyhow::anyhow!(
            "unsupported IPC protocol_version; expected 1"
        ))
    } else {
        match serde_json::from_value::<Request>(raw["request"].clone()) {
            Ok(req) => {
                shutdown = matches!(req, Request::Shutdown);
                dispatch(&store, req).await
            }
            Err(e) => Err(e.into()),
        }
    };
    let envelope = match result {
        Ok(v) => json!({"protocol_version":1,"ok":true,"result":v}),
        Err(e) => json!({"protocol_version":1,"ok":false,"error":format!("{e:#}")}),
    };
    tokio::time::timeout(Duration::from_secs(5), write_frame(&mut stream, &envelope)).await??;
    Ok(shutdown)
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
