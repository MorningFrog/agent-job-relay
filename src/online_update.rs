//! Idle-profile fencing for npm maintenance. The npm launcher runs a private
//! copy of this executable so Windows never replaces its own loaded image.
use crate::{client, config::Config, deployment, ipc, platform};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::json;
use std::{io::Write, path::PathBuf, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

pub fn validate_version(value: &str) -> Result<()> {
    crate::plugin::version(value)?;
    ensure!(
        value
            .split('.')
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'))),
        "version must use canonical x.y.z notation"
    );
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeFinish {
    executable: PathBuf,
    version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemovalFinish {
    release_only: bool,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Finish {
    Runtime(RuntimeFinish),
    Removal(RemovalFinish),
}
#[derive(Clone, Copy)]
pub enum GuardMode {
    Update { allow_active: bool },
    Remove { force: bool },
}

async fn check_jobs(config: &Config, mode: GuardMode) -> Result<usize> {
    if deployment::active_job_count(&config.root)? == 0 {
        return Ok(0);
    }
    match mode {
        GuardMode::Update { allow_active: true } | GuardMode::Remove { force: true } => {
            // Read only: an active profile must already use this runtime's
            // schema and decodable job contract; maintenance never migrates it.
            Ok(crate::storage::Store::readonly(config.clone())
                .active()
                .await?
                .len())
        }
        _ => {
            deployment::require_idle_jobs(&config.root)
                .context("active tasks block removal; use uninstall --force to stop them first")?;
            Ok(0)
        }
    }
}

async fn stop_jobs(config: &Config) -> Result<(Vec<std::fs::File>, Vec<String>)> {
    if !config.root.join("relay.sqlite3").exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let store = crate::storage::Store::new(config.clone());
    let jobs = store.active().await?;
    for job in &jobs {
        store.stop(job.job_id.clone()).await?;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut locks = Vec::new();
    for job in &jobs {
        loop {
            let current = store.status(job.job_id.clone()).await?;
            ensure!(
                current.run_id == job.run_id,
                "run changed during forced removal"
            );
            let runner_alive = current
                .run
                .runner_pid
                .zip(current.run.runner_identity.as_ref())
                .is_some_and(|(pid, identity)| platform::identity(pid).as_ref() == Some(identity));
            if crate::core::terminal(&current.status) && !runner_alive {
                let lock = config.run_lock(&job.run_id)?;
                match lock.try_lock() {
                    Ok(()) => {
                        locks.push(lock);
                        break;
                    }
                    Err(std::fs::TryLockError::WouldBlock) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "task stop remains unconfirmed; runtime retained; inspect the same Job/Run and retry --force after recovery"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    deployment::require_idle_jobs(&config.root)?;
    Ok((locks, jobs.into_iter().map(|job| job.run_id).collect()))
}

pub async fn guard(
    installation: PathBuf,
    mut profiles: Vec<PathBuf>,
    target: &str,
    apply: bool,
    mode: GuardMode,
    original_version: Option<&str>,
) -> Result<()> {
    validate_version(target)?;
    let original_version = original_version.unwrap_or(env!("CARGO_PKG_VERSION"));
    validate_version(original_version)?;
    let mut minimum_restart = crate::plugin::version(original_version)?;
    ensure!(
        minimum_restart <= crate::plugin::version(target)?,
        "runtime downgrade refused: original runtime is newer than the target"
    );
    ensure!(
        !matches!(mode, GuardMode::Update { allow_active: true }) || cfg!(target_os = "linux"),
        "active-run package replacement is supported on Linux only; finish tasks before updating on this platform"
    );
    ensure!(
        crate::plugin::version(target)? >= crate::plugin::version(env!("CARGO_PKG_VERSION"))?,
        "runtime downgrade refused"
    );
    profiles.sort();
    profiles.dedup();
    ensure!(
        !profiles.is_empty() && profiles.len() <= 8,
        "select 1..8 profiles"
    );
    let mut locks = Vec::new();
    let mut peers = Vec::new();
    let mut active_jobs = 0;
    let mut stopped_runs = Vec::new();
    deployment::checked_path(&installation)?;
    let _installation_lock = if apply {
        let lock = platform::private_file(
            &installation
                .parent()
                .context("npm package parent missing")?
                .join(".agent-job-relay-npm-update.lock"),
        )?;
        lock.try_lock()
            .context("another npm update is in progress")?;
        Some(lock)
    } else {
        None
    };
    // Validate every profile before stopping any daemon. Startup locks also
    // serialize concurrent updates, admission and schema migration.
    for root in &profiles {
        ensure!(root.is_absolute(), "profile must be absolute");
        deployment::checked_path(root)?;
        if apply || root.exists() {
            platform::secure_dir(root)?;
        } else {
            continue;
        }
        if apply {
            let lock = platform::private_file(&root.join("startup.lock"))?;
            lock.try_lock()
                .context("another startup/update is in progress")?;
            locks.push(lock);
        }
        let config = Config { root: root.clone() };
        active_jobs += check_jobs(&config, mode).await?;
        match ipc::doctor_peer(&config).await {
            Ok((health, peer)) => {
                let version = health["version"]
                    .as_str()
                    .context("daemon version missing")?;
                ensure!(
                    crate::plugin::version(version)? <= crate::plugin::version(target)?,
                    "running daemon is newer than the update; downgrade refused"
                );
                minimum_restart = minimum_restart.max(crate::plugin::version(version)?);
                peers.push((config, peer.context("cannot pin the original daemon")?));
            }
            Err(error) if client::unavailable(&error) => {}
            Err(error) => return Err(error),
        }
    }
    if apply {
        for (config, peer) in &peers {
            client::stop_daemon_peer(config, Some(peer)).await?;
        }
        for root in &profiles {
            if root.exists() {
                locks.push(Config { root: root.clone() }.daemon_lock()?);
                let config = Config { root: root.clone() };
                if matches!(mode, GuardMode::Remove { force: true }) {
                    let (run_locks, runs) = stop_jobs(&config).await?;
                    locks.extend(run_locks);
                    stopped_runs.extend(runs);
                } else {
                    check_jobs(&config, mode).await.context("work changed during preflight; runtime retained; inspect the same profile before retrying")?;
                }
            }
        }
    }
    println!(
        "{}",
        json!({"ready":true,"profiles":profiles,"online_profiles":peers.iter().map(|(c,_)|&c.root).collect::<Vec<_>>(),"apply":apply,"active_jobs":active_jobs,"stopped_runs":stopped_runs})
    );
    std::io::stdout().flush()?;
    if !apply {
        return Ok(());
    }
    // A lost controller never authorizes replacement, rollback or replay.
    let mut line = String::new();
    let mut input = BufReader::new(tokio::io::stdin()).take(16 * 1024);
    let count = tokio::time::timeout(Duration::from_secs(600), input.read_line(&mut line))
        .await
        .context("update controller timed out; profiles remain stopped; restart explicitly")??;
    ensure!(
        count > 0 && line.ends_with('\n'),
        "update controller disconnected; profiles remain stopped; restart explicitly"
    );
    let finish: Finish = serde_json::from_str(&line)?;
    match (mode, finish) {
        (GuardMode::Update { .. }, Finish::Runtime(finish)) => {
            validate_version(&finish.version)?;
            ensure!(
                finish.executable.is_absolute(),
                "runtime executable must be absolute"
            );
            ensure!(
                crate::plugin::version(&finish.version)? >= minimum_restart,
                "runtime rollback refused"
            );
            drop(locks);
            for (config, _) in peers {
                client::start_installed_daemon(&config, &finish.executable, &finish.version)
                    .await?;
            }
        }
        (GuardMode::Remove { .. }, Finish::Removal(RemovalFinish { release_only: true })) => {
            drop(locks);
        }
        _ => anyhow::bail!(
            "maintenance completion does not match the selected operation; no startup was replayed"
        ),
    }
    println!("{}", json!({"finished":true}));
    std::io::stdout().flush()?;
    // Hold installation ownership until the controller records the outcome.
    // A lost acknowledgement never repeats daemon startup.
    line.clear();
    tokio::time::timeout(Duration::from_secs(30), input.read_line(&mut line))
        .await
        .context("update outcome acknowledgement timed out")??;
    ensure!(
        line == "release\n",
        "update outcome acknowledgement missing; inspect the recorded stage"
    );
    Ok(())
}
