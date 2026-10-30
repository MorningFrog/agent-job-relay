//! Idle-profile fencing for npm's self-update. The npm launcher runs a private
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
struct Finish {
    executable: PathBuf,
    version: String,
}

pub async fn guard(
    installation: PathBuf,
    mut profiles: Vec<PathBuf>,
    target: &str,
    apply: bool,
) -> Result<()> {
    validate_version(target)?;
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
        deployment::require_idle_jobs(root)?;
        let config = Config { root: root.clone() };
        match ipc::doctor_peer(&config).await {
            Ok((health, peer)) => {
                let version = health["version"]
                    .as_str()
                    .context("daemon version missing")?;
                ensure!(
                    crate::plugin::version(version)? <= crate::plugin::version(target)?,
                    "running daemon is newer than the update; downgrade refused"
                );
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
                deployment::require_idle_jobs(root).context("work was admitted during preflight; files preserved; finish the same profile's jobs and retry")?;
            }
        }
    }
    println!(
        "{}",
        json!({"ready":true,"profiles":profiles,"online_profiles":peers.iter().map(|(c,_)|&c.root).collect::<Vec<_>>(),"apply":apply})
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
    validate_version(&finish.version)?;
    ensure!(
        finish.executable.is_absolute(),
        "runtime executable must be absolute"
    );
    // The controller supplies the verified installed image, either the original
    // version after a pre-install failure or the successfully updated version.
    ensure!(
        crate::plugin::version(&finish.version)?
            >= crate::plugin::version(env!("CARGO_PKG_VERSION"))?,
        "runtime rollback refused"
    );
    drop(locks);
    for (config, _) in peers {
        client::start_installed_daemon(&config, &finish.executable, &finish.version).await?;
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
