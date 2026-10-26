//! Client-side convenience, using only the calling host's inherited context.
//! The daemon must never guess which session belongs to a CLI or MCP caller.
use crate::{
    config::Config,
    core::StartSpec,
    ipc::{self, Request},
    session::{Adapter, Registration},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio, time::Duration};

fn desktop_registration() -> Result<(Registration, String)> {
    crate::platform::require_runtime()?;
    let thread=std::env::var("CODEX_THREAD_ID").ok().filter(|s|!s.trim().is_empty()).context(
        "automatic wake needs CODEX_THREAD_ID from the current Desktop session; run start --wake inside that session's terminal tool, or use an explicitly registered --session. No session was guessed")?;
    let endpoint=std::env::var_os("CODEX_APP_TOOLS_PIPE_PATH").filter(|s|!s.is_empty()).map(PathBuf::from).context(
        "automatic wake needs CODEX_APP_TOOLS_PIPE_PATH from the current Desktop host; use its terminal tool or explicitly register a supported endpoint. Inbox cannot auto-resume")?;
    ensure!(
        thread.len() <= 256 && !thread.contains('\0'),
        "invalid inherited Desktop thread identity"
    );
    ensure!(
        endpoint.is_absolute() && endpoint.as_os_str().len() < 104,
        "inherited Desktop endpoint must be an absolute Unix socket path shorter than 104 bytes"
    );
    let fingerprint=crate::platform::endpoint_identity(&endpoint).context("inherited Desktop endpoint is unavailable or not owned; reconnect the client to the current host")?;
    let registration = Registration {
        agent_kind: "codex_desktop".into(),
        session_id: thread.clone(),
        caller_thread_id: Some(thread),
        host_id: "local-desktop".into(),
        instance_id: format!("socket:{fingerprint}"),
        adapter: Adapter::DesktopExperimental,
        endpoint: Some(endpoint),
        lease_seconds: 86400,
        // --wake / wake_me=true is explicit opt-in, described as experimental.
        experimental: true,
    };
    Ok((registration, fingerprint))
}

pub fn context_summary() -> Value {
    match desktop_registration() {
        Ok(_) => {
            json!({"available":true,"source":"current process inherited host context","next_step":"Use start --wake or job_start with wake_me=true (experimental Desktop wake)"})
        }
        Err(e) => json!({"available":false,"reason":format!("{e:#}"),"can_resume":false}),
    }
}
pub(crate) fn unavailable(error: &anyhow::Error) -> bool {
    error.downcast_ref::<std::io::Error>().is_some_and(|e| {
        matches!(
            e.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
        )
    })
}
/// Start only our own per-state daemon, without a system service or PATH edit.
async fn ensure_daemon(config: &Config) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(8), ensure_daemon_inner(config))
        .await
        .context("daemon readiness timed out (8s); run foreground daemon with the same --state-dir for diagnostics")?
}
async fn ensure_daemon_inner(config: &Config) -> Result<()> {
    match ipc::call(config, Request::Doctor).await {
        Ok(_) => return Ok(()),
        Err(e) if unavailable(&e) => {}
        Err(e) => return Err(e),
    }
    config.prepare()?;
    ensure!(
        config.socket().as_os_str().len() < 104,
        "state path too long for Unix socket; use a shorter --state-dir"
    );
    let lock = crate::platform::private_file(&config.root.join("startup.lock"))?;
    let owns_startup = match lock.try_lock() {
        Ok(()) => true,
        Err(std::fs::TryLockError::WouldBlock) => false,
        Err(e) => return Err(e.into()),
    };
    if owns_startup {
        // Another starter may have become ready while this lock was acquired.
        match ipc::call(config, Request::Doctor).await {
            Ok(_) => return Ok(()),
            Err(e) if unavailable(&e) => {}
            Err(e) => return Err(e),
        }
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command
            .arg("--state-dir")
            .arg(&config.root)
            .arg("daemon")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::platform::detach(&mut command)?;
        let mut child=tokio::process::Command::from(command).kill_on_drop(false).spawn().context("cannot auto-start daemon; run agent-job-relay daemon in a terminal for diagnostics")?;
        tokio::spawn(async move {
            if let Err(e) = child.wait().await {
                tracing::error!(error=%e,"auto-started daemon reap failed");
            }
        });
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        match ipc::call(config, Request::Doctor).await {
            Ok(_) => return Ok(()),
            Err(e) if unavailable(&e) => {}
            Err(e) => return Err(e),
        }
        ensure!(
            tokio::time::Instant::now() < deadline,
            "daemon did not become ready; run agent-job-relay --state-dir '{}' daemon in a terminal for diagnostics",
            config.root.display()
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn connect_registration(
    config: &Config,
    registration: Registration,
    fingerprint: String,
) -> Result<Value> {
    let result = ipc::call(config, Request::RegisterSession { registration }).await?;
    ensure!(
        result["session"]["endpoint_identity"].as_str() == Some(&fingerprint),
        "Desktop endpoint changed during registration; retry from the current host context"
    );
    let key = result["session"]["key"]
        .as_str()
        .context("daemon returned no session key")?
        .to_string();
    let mut result = result;
    let probe = ipc::call(config, Request::ProbeSession { session_key: key }).await?;
    result["probe"] = probe["probe"].clone();
    result["connection_source"] = json!("current process inherited host context");
    Ok(result)
}
/// Explicitly opt into the experimental adapter using current host identity.
pub async fn connect(config: &Config) -> Result<Value> {
    let (registration, fingerprint) = desktop_registration()?;
    ensure_daemon(config).await?;
    connect_registration(config, registration, fingerprint).await
}
pub async fn start(config: &Config, mut spec: StartSpec, wake_me: bool) -> Result<Value> {
    ensure!(
        !wake_me || spec.session.is_none(),
        "choose automatic wake_me/--wake or explicit spec.session/--session, not both"
    );
    spec.validate()?;
    // Missing identity fails before starting a daemon or persisting any job.
    let context = if wake_me {
        Some(desktop_registration()?)
    } else {
        None
    };
    ensure_daemon(config).await?;
    if let Some((registration, fingerprint)) = context {
        let session = connect_registration(config, registration, fingerprint).await?;
        spec.session = Some(
            session["session"]["key"]
                .as_str()
                .context("daemon returned no session key")?
                .into(),
        );
    }
    let mut result = ipc::call(config, Request::Start { spec }).await?;
    if wake_me {
        result["wake"]["automatic_binding"] = json!(true);
        result["wake"]["probe"] = json!("passed");
    }
    Ok(result)
}
pub async fn stop_daemon(config: &Config) -> Result<Value> {
    match ipc::call(config, Request::Shutdown).await {
        Ok(v) => Ok(v),
        Err(e) if unavailable(&e) => {
            Ok(json!({"daemon":"stopped","already_stopped":true,"runners":"unaffected"}))
        }
        Err(e) => Err(e),
    }
}
/// Shared high-level entry for CLI and MCP; only submissions auto-start a daemon.
pub async fn call(config: &Config, request: Request) -> Result<Value> {
    match request {
        Request::Start { spec } => start(config, spec, false).await,
        Request::Doctor => {
            let mut report = ipc::call(config, Request::Doctor).await?;
            report["desktop_context"] = context_summary();
            Ok(report)
        }
        Request::Shutdown => stop_daemon(config).await,
        request => ipc::call(config, request).await,
    }
}
