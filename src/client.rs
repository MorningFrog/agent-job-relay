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
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

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
    crate::platform::validate_endpoint(&endpoint)?;
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
        renew_on_delivery: true,
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
/// Explicitly start the selected daemon without a job, session or host identity.
pub async fn start_daemon(config: &Config) -> Result<Value> {
    ensure_daemon(config).await?;
    ipc::call(config, Request::Doctor).await
}

/// Start only our own per-state daemon, without a system service or PATH edit.
pub(crate) async fn ensure_daemon(config: &Config) -> Result<()> {
    ensure_daemon_from(
        config,
        &crate::platform::self_image()?,
        env!("CARGO_PKG_VERSION"),
    )
    .await
}
async fn ensure_daemon_from(config: &Config, image: &Path, version: &str) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(30), ensure_daemon_inner(config, image, version))
        .await
        .context("daemon readiness timed out; run foreground daemon with the same --state-dir for diagnostics")?
}
pub(crate) async fn start_installed_daemon(
    config: &Config,
    image: &Path,
    version: &str,
) -> Result<()> {
    // Start through the installed CLI so Windows' daemon-only CIM broker still
    // authenticates its own image, even when the updater is a different binary.
    let mut command = tokio::process::Command::new(image);
    command
        .arg("--state-dir")
        .arg(&config.root)
        .args(["daemon", "start"])
        .env_remove("CODEX_THREAD_ID")
        .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let status = tokio::time::timeout(Duration::from_secs(35), command.status())
        .await
        .context("installed daemon startup timed out")??;
    ensure!(
        status.success(),
        "installed daemon start failed ({status}); run the installed daemon start for diagnostics"
    );
    let info = ipc::call(config, Request::Doctor).await?;
    ensure!(
        !older_daemon(&info, version)?,
        "installed daemon did not load the requested version"
    );
    verify_runner_launch(&info)
}
async fn ensure_daemon_inner(config: &Config, image: &Path, version: &str) -> Result<()> {
    match ipc::call(config, Request::Doctor).await {
        Ok(info) if !older_daemon(&info, version)? => return verify_runner_launch(&info),
        Ok(_) => {}
        Err(e)
            if unavailable(&e)
                || e.downcast_ref::<std::io::Error>().is_some_and(|error| {
                    matches!(
                        error.kind(),
                        std::io::ErrorKind::UnexpectedEof
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::BrokenPipe
                    )
                }) =>
        {
            // A concurrent starter may close this optimistic read-only probe
            // while replacing the old daemon. Recheck under the startup lock;
            // never apply this retry policy to a mutation or pinned shutdown.
        }
        Err(e) => return Err(e),
    }
    config.prepare()?;
    crate::platform::validate_local_endpoint(&config.socket())?;
    let lock = crate::platform::private_file(&config.root.join("startup.lock"))?;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {
                tokio::time::sleep(Duration::from_millis(25)).await
            }
            Err(e) => return Err(e.into()),
        }
    }
    {
        // Another starter may have become ready while this lock was acquired.
        match ipc::doctor_peer(config).await {
            Ok((info, peer)) => {
                if !older_daemon(&info, version)? {
                    return verify_runner_launch(&info);
                }
                // Do not stop a useful old runtime when its runners prevent migration.
                upgrade_preflight(config)?;
                let peer =
                    peer.context("cannot pin the old daemon process; automatic upgrade refused")?;
                stop_daemon_peer(config, Some(&peer)).await?;
            }
            Err(e) if unavailable(&e) => {}
            Err(e) => return Err(e),
        }
        let mut command = std::process::Command::new(image);
        command
            .arg("--state-dir")
            .arg(&config.root)
            .arg("daemon")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        let guidance = format!(
            "cannot automatically start daemon; outside PowerShell fallback: {}",
            crate::platform::outside_start_command(config)?
        );
        #[cfg(not(windows))]
        let guidance = "cannot start independent daemon; run agent-job-relay --state-dir <same-path> daemon start in a terminal outside the host; use foreground daemon for diagnostics";
        crate::platform::launch_daemon(command, config)
            .await
            .context(guidance)?;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        match ipc::call(config, Request::Doctor).await {
            Ok(info) => {
                ensure!(
                    !older_daemon(&info, version)?,
                    "daemon upgrade did not load the requested version"
                );
                return verify_runner_launch(&info);
            }
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
fn older_daemon(info: &Value, version: &str) -> Result<bool> {
    let current = info["version"]
        .as_str()
        .context("daemon version is missing; automatic upgrade refused")?;
    Ok(crate::plugin::version(current)? < crate::plugin::version(version)?)
}
fn upgrade_preflight(config: &Config) -> Result<()> {
    let path = config.root.join("relay.sqlite3");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.is_file() && !crate::platform::is_link(&metadata),
        "database must be a regular file, not a symbolic link"
    );
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    ensure!(
        version <= 12,
        "database schema is newer than this executable; downgrade refused"
    );
    if version > 0 && version < 12 {
        let running: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM jobs WHERE status IN ('launching','running'))",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            !running,
            "finish active runs with the previous runtime before upgrading the data schema; old daemon was left running"
        );
    }
    Ok(())
}
pub(crate) fn verify_runner_launch(info: &Value) -> Result<()> {
    ensure!(
        info["daemon_executable_path_available"] != false
            || info["runner_launch_from_loaded_image"] == true,
        "existing legacy daemon runs from a removed executable and cannot launch runners. No job was submitted. Let its active jobs finish, then run daemon stop for the same state and submit again; no daemon was restarted automatically"
    );
    Ok(())
}

async fn connect_registration(
    config: &Config,
    registration: Registration,
    fingerprint: String,
) -> Result<Value> {
    let health = ipc::call(config, Request::Doctor).await?;
    verify_notification_runtime(&health)?;
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
fn verify_notification_runtime(info: &Value) -> Result<()> {
    ensure!(
        info["delivery_time_renewal"] == true && info["same_chat_recovery"] == true,
        "existing daemon predates notification recovery/renewal. No registration or job was changed. Let active jobs finish, explicitly stop/update the daemon for the same installed profile, then retry; no state directory was switched"
    );
    Ok(())
}
/// Explicitly opt into the experimental adapter using current host identity.
pub async fn connect(config: &Config) -> Result<Value> {
    let (registration, fingerprint) = desktop_registration()?;
    ensure_daemon(config).await?;
    connect_registration(config, registration, fingerprint).await
}
/// Capture workspace only from the real terminal, alongside its fresh identity.
pub async fn connect_context(
    config: &Config,
    cwd: PathBuf,
    recovery: Option<(bool, i64, u32)>,
) -> Result<Value> {
    if recovery.is_some() {
        return connect_context_inner(config, cwd, recovery, false).await;
    }
    connect_request(config, cwd, crate::goal_consent::Options::default()).await
}

pub async fn connect_request(
    config: &Config,
    cwd: PathBuf,
    options: crate::goal_consent::Options,
) -> Result<Value> {
    let cwd = crate::platform::normalized_path(cwd.canonicalize()?);
    ensure!(
        cwd.is_absolute() && cwd.is_dir(),
        "binding cwd must be a directory"
    );
    let goal =
        crate::goal_control::Controller::inspect(desktop_registration()?.0.session_id).await?;
    let mut value = connect(config).await?;
    value["goal_wait_policy"] = json!("required_for_active_desktop_goal");
    value["automatic_goal_pause_supported"] = json!(false);
    let key = value["session"]["key"]
        .as_str()
        .context("missing caller key")?
        .to_string();
    let goal_status = goal.as_ref().map(|(_, g)| g.status.as_str());
    value["goal_status"] = json!(goal_status);
    if goal_status != Some("active") {
        if ipc::call(config, Request::Doctor).await?["scoped_goal_consent"] == true {
            crate::storage::Store::new(config.clone())
                .clear_obsolete_goal_consents(key.clone())
                .await?;
        }
        if let Some(request_id) = options.request_id {
            value["consent"] = ipc::call(
                config,
                Request::GoalConsent {
                    request_id,
                    session_key: key.clone(),
                    decision: Default::default(),
                    expected_revision: None,
                },
            )
            .await?;
            value["status"] = json!("goal_not_active");
            value["next_step"] = if value["consent"].get("goal_status").is_some() {
                value["consent"]["next_step"].clone()
            } else {
                json!(
                    "This original request has no active goal. Inspect its retained Job/Run/wait IDs; do not submit remaining rounds or move its consent to another goal."
                )
            };
        } else if goal_status.is_some_and(|s| !matches!(s, "paused" | "complete" | "completed")) {
            value["status"] = json!("goal_not_active");
            value["next_step"] = json!(
                "The host goal is inactive. Do not submit work, create a replacement request, poll or automatically change its status. The user can resume the original goal through host controls, then reconnect using the retained request_id when available."
            );
        } else {
            value["binding"] = json!({"session_key":key,"cwd":cwd});
            value["status"] = json!("ready");
        }
        value["goal_state_verified"] =
            json!(std::env::var_os("CODEX_APP_RUNTIME_CLI_PATH").is_some());
        return Ok(value);
    }
    let (controller, expected) = goal.context("active goal missing")?;
    ensure!(
        ipc::call(config, Request::Doctor).await?["scoped_goal_consent"] == true,
        "selected daemon lacks scoped goal consent; update the same profile before submission"
    );
    let consent = ipc::call(
        config,
        Request::PrepareGoalConsent {
            session_key: key.clone(),
            controller: controller.clone(),
            expected: serde_json::to_value(&expected)?,
            cwd: cwd.clone(),
            request_id: options.request_id,
            label: options.label,
            job_limit: options.job_limit,
            user_authorized: options.user_authorized,
        },
    )
    .await?;
    value["status"] = json!(if consent["status"] == "approved" {
        "ready"
    } else if matches!(consent["status"].as_str(), Some("pending" | "asking")) {
        "needs_confirmation"
    } else {
        "request_closed"
    });
    value["next_step"] = consent["next_step"].clone();
    value["goal_state_verified"] = json!(true);
    if consent["status"] == "approved" {
        let wait = ipc::call(
            config,
            Request::PrepareGoalWait {
                session_key: key.clone(),
                controller,
                expected: serde_json::to_value(expected)?,
                consent_id: Some(
                    consent["request_id"]
                        .as_str()
                        .context("missing consent ID")?
                        .into(),
                ),
            },
        )
        .await?;
        value["binding"] = json!({"session_key":key,"cwd":cwd,"goal_wait_id":wait["goal_wait_id"]});
        value["goal_wait"] = wait;
    }
    value["consent"] = consent;
    Ok(value)
}

async fn connect_context_inner(
    config: &Config,
    cwd: PathBuf,
    recovery: Option<(bool, i64, u32)>,
    native_pause_requested: bool,
) -> Result<Value> {
    let cwd = crate::platform::normalized_path(cwd.canonicalize()?);
    ensure!(
        cwd.is_absolute() && cwd.is_dir(),
        "binding cwd must be a directory"
    );
    if let Some((_, after, limit)) = recovery {
        ensure!(
            after >= 0 && (1..=100).contains(&limit),
            "invalid recovery pagination"
        );
    }
    // Compatibility binding requires actual permission; ordinary requests use scoped preflight.
    let goal = if recovery.is_none() {
        crate::goal_control::Controller::detect(desktop_registration()?.0.session_id).await?
    } else {
        None
    };
    ensure!(
        goal.is_none() || native_pause_requested,
        "native goal handoff requires actual user permission; ordinary requests use scoped preflight"
    );
    let mut value = connect(config).await?;
    value["binding"] = json!({"session_key":value["session"]["key"],"cwd":cwd});
    if let Some((apply, after, limit)) = recovery {
        value["recovery"] = ipc::call(
            config,
            Request::RecoverSession {
                session_key: value["session"]["key"]
                    .as_str()
                    .context("missing binding key")?
                    .into(),
                apply,
                after,
                limit,
            },
        )
        .await?;
    } else if let Some((controller, expected)) = goal {
        ensure!(
            ipc::call(config, Request::Doctor).await?["goal_wait_host_pause"] == true,
            "this active goal requires native-host pause acknowledgement; update the selected runtime before submitting work"
        );
        let wait = ipc::call(
            config,
            Request::PrepareGoalWait {
                session_key: value["session"]["key"]
                    .as_str()
                    .context("missing caller key")?
                    .into(),
                controller,
                expected: serde_json::to_value(expected)?,
                consent_id: None,
            },
        )
        .await?;
        value["binding"]["goal_wait_id"] = wait["goal_wait_id"].clone();
        value["goal_wait"] = wait;
    }
    value["goal_wait_policy"] = json!("required_for_active_desktop_goal");
    value["automatic_goal_pause_supported"] = json!(false);
    value["goal_state_verified"] =
        json!(recovery.is_none() && std::env::var_os("CODEX_APP_RUNTIME_CLI_PATH").is_some());
    Ok(value)
}
pub fn brief_binding(value: Value) -> Value {
    if value["status"] == "goal_not_active" {
        let mut result = json!({"status":value["status"],"session_key":value["session"]["key"],"goal_status":value["goal_status"],"goal_state_verified":value["goal_state_verified"],"next_step":value["next_step"]});
        if let Some(consent) = value.get("consent") {
            result["consent"] = consent.clone();
        }
        return result;
    }
    if value.get("consent").is_some() {
        let mut result = json!({"status":value["status"],"session_key":value["session"]["key"],"consent":value["consent"],"goal_state_verified":value["goal_state_verified"],"next_step":value["next_step"]});
        if let Some(binding) = value.get("binding") {
            result["binding"] = binding.clone();
        }
        if let Some(wait) = value.get("goal_wait") {
            result["goal_wait"] = wait.clone();
            result["next_step"] = wait["next_step"].clone();
        }
        return result;
    }
    let mut brief = json!({"session_key":value["session"]["key"],"binding":value["binding"],"goal_wait_policy":value["goal_wait_policy"],"automatic_goal_pause_supported":value["automatic_goal_pause_supported"],"goal_state_verified":value["goal_state_verified"],"probe":"passed","can_resume":true,"can_confirm_resume":false,
        "next_step":"Call job_start with binding and spec.command. For requested mid-run feedback, select a real producer and initial spec.monitor_rules before submission; read returned feedback configuration, then end this turn without polling."});
    if let Some(wait) = value.get("goal_wait") {
        brief["goal_wait"] = wait.clone();
        brief["next_step"] = json!(
            "Explicit native-pause workflow: preserve this binding. Only if the user or system has requested this goal pause, admit work, use this chat's native update_goal(status='paused'), confirm with job_goal_wait, then end the turn. This is not automatic pause support; plugin instructions cannot override the host's lifecycle authority."
        );
    }
    if let Some(recovery) = value.get("recovery") {
        brief["recovery"] = crate::output::compact("recover_session", recovery.clone());
        brief["next_step"] = brief["recovery"]["next_step"].clone();
    }
    brief
}
pub async fn recover(config: &Config, apply: bool, after: i64, limit: u32) -> Result<Value> {
    let binding = connect(config).await?;
    let key = binding["session"]["key"]
        .as_str()
        .context("missing binding key")?
        .to_owned();
    ipc::call(
        config,
        Request::RecoverSession {
            session_key: key,
            apply,
            after,
            limit,
        },
    )
    .await
}
pub async fn connect_with_goal(config: &Config, cwd: PathBuf) -> Result<Value> {
    let value = connect_context_inner(config, cwd, None, true).await?;
    ensure!(
        value.get("goal_wait").is_some(),
        "goal waiting requires an active goal and the current Desktop runtime; no job was submitted"
    );
    Ok(value)
}
pub async fn start_waiting(
    config: &Config,
    mut specs: Vec<StartSpec>,
    batch: bool,
) -> Result<Value> {
    ensure!(
        specs.iter().all(|s| s.session.is_none()),
        "automatic goal waiting requires the current caller, not explicit sessions"
    );
    if batch {
        crate::batch::validate(&specs)?;
    } else {
        ensure!(specs.len() == 1, "one specification required");
        specs[0].validate()?;
    }
    let binding = connect_with_goal(config, std::env::current_dir()?).await?;
    let key = binding["session"]["key"]
        .as_str()
        .context("missing session")?;
    for spec in &mut specs {
        spec.session = Some(key.into());
    }
    let goal_wait_id = binding["goal_wait"]["goal_wait_id"]
        .as_str()
        .context("missing goal wait")?
        .into();
    call(
        config,
        if batch {
            Request::StartGoalBatch {
                specs,
                goal_wait_id,
            }
        } else {
            Request::StartGoal {
                spec: specs.remove(0),
                goal_wait_id,
            }
        },
    )
    .await
}
fn verify_external_runtime(info: &Value) -> Result<()> {
    ensure!(
        info["remote_execution"] == true && info["github_actions_observation"] == true,
        "selected daemon does not support external execution. No job was submitted. Finish active jobs, then explicitly stop/update this same profile and retry"
    );
    Ok(())
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
    if spec.command.is_external() {
        verify_external_runtime(&ipc::call(config, Request::Doctor).await?)?;
    }
    if let Some((registration, fingerprint)) = context {
        // The actual terminal selects the goal controller; the daemon/shared
        // MCP never infers it from its own inherited environment.
        let session = if std::env::var_os("CODEX_APP_RUNTIME_CLI_PATH").is_some() {
            connect_context(config, spec.cwd.clone(), None).await?
        } else {
            connect_registration(config, registration, fingerprint).await?
        };
        if matches!(
            session["status"].as_str(),
            Some("needs_confirmation" | "request_closed" | "goal_not_active")
        ) {
            return Ok(brief_binding(session));
        }
        spec.session = Some(
            session["session"]["key"]
                .as_str()
                .context("daemon returned no session key")?
                .into(),
        );
        if let Some(wait) = session["binding"]["goal_wait_id"].as_str() {
            let mut result = ipc::call(
                config,
                Request::StartGoal {
                    spec,
                    goal_wait_id: wait.into(),
                },
            )
            .await?;
            result["wake"]["automatic_binding"] = json!(true);
            result["wake"]["probe"] = json!("passed");
            return Ok(result);
        }
    }
    let mut result = ipc::call(config, Request::Start { spec }).await?;
    if wake_me {
        result["wake"]["automatic_binding"] = json!(true);
        result["wake"]["probe"] = json!("passed");
    }
    Ok(result)
}
pub async fn start_batch(
    config: &Config,
    mut specs: Vec<StartSpec>,
    wake_me: bool,
) -> Result<Value> {
    crate::batch::validate(&specs)?;
    ensure!(
        !wake_me || specs.iter().all(|spec| spec.session.is_none()),
        "choose automatic wake_me/--wake or explicit sessions, not both"
    );
    let context = if wake_me {
        Some(desktop_registration()?)
    } else {
        None
    };
    ensure_daemon(config).await?;
    let health = ipc::call(config, Request::Doctor).await?;
    if specs.iter().any(|s| s.command.is_external()) {
        verify_external_runtime(&health)?;
    }
    ensure!(
        health["batch_submission"] == true,
        "selected daemon does not support batch submission. No batch was submitted. Let active jobs finish, then explicitly stop/update the daemon for the same profile and retry"
    );
    if let Some((registration, fingerprint)) = context {
        let session = if std::env::var_os("CODEX_APP_RUNTIME_CLI_PATH").is_some() {
            connect_context(config, std::env::current_dir()?, None).await?
        } else {
            connect_registration(config, registration, fingerprint).await?
        };
        if matches!(
            session["status"].as_str(),
            Some("needs_confirmation" | "request_closed" | "goal_not_active")
        ) {
            return Ok(brief_binding(session));
        }
        let key = session["session"]["key"]
            .as_str()
            .context("daemon returned no session key")?;
        for spec in &mut specs {
            spec.session = Some(key.into());
        }
        if let Some(wait) = session["binding"]["goal_wait_id"].as_str() {
            return ipc::call(
                config,
                Request::StartGoalBatch {
                    specs,
                    goal_wait_id: wait.into(),
                },
            )
            .await;
        }
    }
    crate::batch::validate(&specs)?;
    let mut value = ipc::call(config, Request::StartBatch { specs }).await?;
    if wake_me {
        for item in value["results"]
            .as_array_mut()
            .context("missing batch results")?
        {
            if item["accepted"] == true {
                item["result"]["wake"]["automatic_binding"] = json!(true);
                item["result"]["wake"]["probe"] = json!("passed");
            }
        }
    }
    Ok(value)
}
pub async fn stop_daemon(config: &Config) -> Result<Value> {
    stop_daemon_peer(config, None).await
}
pub(crate) async fn stop_daemon_peer(config: &Config, peer: Option<&ipc::Peer>) -> Result<Value> {
    let response = match peer {
        Some(peer) => ipc::call_pinned(config, Request::Shutdown, peer).await,
        None => ipc::call(config, Request::Shutdown).await,
    };
    match response {
        Ok(mut v) => {
            wait_for_daemon_stop(config).await?;
            v["daemon"] = json!("stopped");
            Ok(v)
        }
        Err(e) if unavailable(&e) => {
            Ok(json!({"daemon":"stopped","already_stopped":true,"runners":"unaffected"}))
        }
        Err(e) => {
            if let Some(remote) = e.downcast_ref::<ipc::RemoteError>()
                && remote
                    .message
                    .starts_with("unknown variant `shutdown`, expected ")
                && let Some((pid, identity)) = &remote.shutdown_peer
            {
                crate::platform::stop_legacy_daemon(&config.root, *pid, identity)
                    .await
                    .context("legacy daemon rejected shutdown; verified graceful stop failed; no other process or state was selected")?;
                wait_for_daemon_stop(config).await?;
                return Ok(
                    json!({"daemon":"stopped","method":"legacy_sigint","runners":"continue independently","data":"preserved","state_dir":config.root}),
                );
            }
            Err(e.context("daemon stop failed; use this installation's recorded state with daemon stop --install-root <same-install-root>"))
        }
    }
}
async fn wait_for_daemon_stop(config: &Config) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true);
        crate::platform::secure_options(&mut options);
        let stopped = match options.open(config.root.join("daemon.lock")) {
            Ok(lock) => match lock.try_lock() {
                Ok(()) => true,
                Err(std::fs::TryLockError::WouldBlock) => false,
                Err(e) => return Err(e.into()),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => !config.socket().exists(),
            Err(e) => return Err(e.into()),
        };
        if stopped {
            return Ok(());
        }
        ensure!(
            tokio::time::Instant::now() < deadline,
            "shutdown requested but daemon lock is still held; installation was not changed; wait and retry for the same state"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
/// Shared high-level entry for CLI and MCP; submissions and explicit registrations auto-start a daemon.
pub async fn call(config: &Config, request: Request) -> Result<Value> {
    match request {
        Request::Start { spec } => start(config, spec, false).await,
        Request::StartBatch { specs } => start_batch(config, specs, false).await,
        request @ (Request::StartGoal { .. }
        | Request::StartGoalBatch { .. }
        | Request::PrepareGoalWait { .. }
        | Request::PrepareGoalConsent { .. }
        | Request::GoalConsent { .. }) => {
            ensure_daemon(config).await?;
            ensure!(
                ipc::call(config, Request::Doctor).await?["goal_wait_host_pause"] == true,
                "selected daemon lacks originating-host goal pause acknowledgement; no goal-aware work was submitted; update this same profile explicitly"
            );
            ipc::call(config, request).await
        }
        request @ (Request::GoalWait { cancel: true, .. }
        | Request::GoalWait {
            confirm_paused: true,
            ..
        }) => {
            ensure_daemon(config).await?;
            ipc::call(config, request).await
        }
        Request::GoalWait {
            goal_wait_id,
            cancel: false,
            confirm_paused: false,
            expected_revision,
        } => {
            match ipc::call(
                config,
                Request::GoalWait {
                    goal_wait_id: goal_wait_id.clone(),
                    cancel: false,
                    confirm_paused: false,
                    expected_revision,
                },
            )
            .await
            {
                Ok(value) => Ok(value),
                Err(error) if unavailable(&error) => {
                    crate::storage::Store::readonly(config.clone())
                        .goal_wait(goal_wait_id, false, None)
                        .await
                }
                Err(error) => Err(error),
            }
        }
        Request::Doctor => {
            let mut report = ipc::call(config, Request::Doctor).await?;
            report["desktop_context"] = context_summary();
            Ok(report)
        }
        Request::Shutdown => stop_daemon(config).await,
        Request::Inspect {
            job_id,
            before,
            limit,
        } => crate::monitor::inspect_client(config, job_id, before, limit).await,
        Request::Monitor {
            job_id,
            mutation: None,
        } => match ipc::call(
            config,
            Request::Monitor {
                job_id: job_id.clone(),
                mutation: None,
            },
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(e) if unavailable(&e) => {
                crate::storage::Store::readonly(config.clone())
                    .monitor(job_id, None)
                    .await
            }
            Err(e) => Err(e),
        },
        request @ (Request::Monitor {
            mutation: Some(_), ..
        }
        | Request::Stop { .. }) => {
            ensure_daemon(config).await?;
            ipc::call(config, request).await
        }
        request @ Request::RegisterSession { .. } => {
            ensure_daemon(config).await?;
            ipc::call(config, request).await
        }
        request => ipc::call(config, request).await,
    }
}

#[cfg(test)]
mod startup_tests {
    use super::verify_runner_launch;
    use serde_json::json;
    #[test]
    fn legacy_removed_image_is_rejected_but_loaded_image_capability_is_accepted() {
        let legacy = json!({"daemon_executable_path_available":false});
        assert!(
            verify_runner_launch(&legacy)
                .unwrap_err()
                .to_string()
                .contains("No job was submitted")
        );
        assert!(verify_runner_launch(&json!({"daemon_executable_path_available":false,"runner_launch_from_loaded_image":true})).is_ok());
        assert!(verify_runner_launch(&json!({"daemon_executable_path_available":true})).is_ok());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn interrupted_doctor_probe_waits_for_startup_lock_and_rechecks_the_peer() {
        use super::*;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::Builder::new()
            .prefix("startup-probe-")
            .tempdir_in(root)
            .unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let startup = crate::platform::private_file(&config.root.join("startup.lock")).unwrap();
        startup.try_lock().unwrap();
        let mut listener = crate::platform::Listener::bind(&config.socket()).unwrap();
        let waiting_config = config.clone();
        let mut waiting = tokio::spawn(async move {
            ensure_daemon_from(
                &waiting_config,
                &waiting_config.root.join("must-not-launch"),
                env!("CARGO_PKG_VERSION"),
            )
            .await
        });

        let mut probe = tokio::time::timeout(Duration::from_secs(2), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let request = ipc::read_frame(&mut probe, 65536).await.unwrap();
        assert_eq!(request["request"]["op"], "doctor");
        // An old daemon can close this already accepted read-only request when
        // another starter shuts it down while holding the startup lock.
        drop(probe);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut waiting)
                .await
                .is_err(),
            "an interrupted optimistic probe must wait for the existing starter"
        );
        drop(startup);

        let mut probe = tokio::time::timeout(Duration::from_secs(2), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let request = ipc::read_frame(&mut probe, 65536).await.unwrap();
        assert_eq!(request["request"]["op"], "doctor");
        ipc::write_frame(
            &mut probe,
            &json!({"protocol_version":1,"ok":true,"result":{"version":env!("CARGO_PKG_VERSION"),"runner_launch_from_loaded_image":true}}),
        )
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), waiting)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!config.root.join("relay.sqlite3").exists());
    }
}
