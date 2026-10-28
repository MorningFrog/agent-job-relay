use super::*;
use crate::{config::Config, storage::Store};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::io::{Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(crate) const LOG_PAGE: u32 = 32768;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunRef {
    pub job_id: String,
    pub run_id: String,
    pub attempt: u32,
}
impl RunRef {
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.job_id)?;
        validate_id(&self.run_id)?;
        ensure!(
            self.attempt == 1,
            "remote runtime currently supports attempt=1 only"
        );
        Ok(())
    }
    pub fn from_job(job: &Job) -> Self {
        Self {
            job_id: job.job_id.clone(),
            run_id: job.run_id.clone(),
            attempt: job.run.attempt,
        }
    }
    pub fn check(&self, job: &Job) -> Result<()> {
        ensure!(
            self == &Self::from_job(job),
            "remote execution identity/attempt mismatch"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeRequest {
    pub protocol_version: u32,
    pub target_id: String,
    pub request_id: String,
    pub operation: Operation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Launch {
        origin_job_id: String,
        origin_run_id: String,
        spec: Box<StartSpec>,
    },
    Lookup {
        origin_job_id: String,
        origin_run_id: String,
    },
    Sample {
        run: RunRef,
        cursors: [u64; 2],
        after_event: i64,
    },
    Stop {
        run: RunRef,
        grace_seconds: u32,
    },
}
fn identity(config: &Config) -> Result<String> {
    let file = crate::platform::read_file(&config.root.join("remote-target")).context(
        "remote profile is not initialized; explicitly run remote init on the remote host",
    )?;
    let mut value = String::new();
    file.take(128).read_to_string(&mut value)?;
    let value = value.trim().to_owned();
    validate_id(&value)?;
    Ok(value)
}
/// Explicit provisioning, never changes an existing target or starts a daemon.
pub fn initialize(config: &Config) -> Result<Value> {
    config.prepare()?;
    let lock = crate::platform::private_file(&config.root.join("remote-init.lock"))?;
    lock.lock()?;
    let path = config.root.join("remote-target");
    if !path.try_exists()? {
        let mut file = crate::platform::create_new_file(&path)?;
        writeln!(file, "{}", id())?;
        file.sync_all()?;
        crate::platform::sync_directory(&config.root)?;
    }
    Ok(
        json!({"protocol_version":1,"target_id":identity(config)?,"platform":std::env::consts::OS,"capabilities":{"launch":true,"attach":true,"exact_run_stop":true,"raw_byte_logs":true,"runner_rules":true},"credentials":"use existing authenticated transport; target_id is identity, not authentication"}),
    )
}

async fn handle(config: &Config, request: BridgeRequest) -> Result<Value> {
    ensure!(request.protocol_version == 1, "unsupported remote protocol");
    ensure!(
        request.target_id == identity(config)?,
        "remote target mismatch"
    );
    validate_id(&request.request_id)?;
    let store = Store::new(config.clone());
    match request.operation {
        Operation::Lookup {
            origin_job_id,
            origin_run_id,
        } => {
            use rusqlite::OptionalExtension;
            validate_id(&origin_job_id)?;
            validate_id(&origin_run_id)?;
            let key = format!("remote:{origin_job_id}:{origin_run_id}");
            let run = store.read(move |db| {
                let job: Option<String> = db.query_row("SELECT id FROM jobs WHERE idem=?1 UNION ALL SELECT id FROM archive_jobs WHERE idem=?1", [key], |r| r.get(0)).optional()?;
                job.map(|j| crate::storage::read_job(db, &j).map(|j| RunRef::from_job(&j))).transpose()
            }).await?;
            Ok(json!({"run":run,"origin_job_id":origin_job_id,"origin_run_id":origin_run_id}))
        }
        Operation::Launch {
            origin_job_id,
            origin_run_id,
            mut spec,
        } => {
            validate_id(&origin_job_id)?;
            validate_id(&origin_run_id)?;
            validate_launch(&spec)?;
            spec.idempotency_key = Some(format!("remote:{origin_job_id}:{origin_run_id}"));
            let result = crate::client::start(config, *spec, false).await?;
            let job: Job = serde_json::from_value(result["job"].clone())?;
            Ok(
                json!({"run":RunRef::from_job(&job),"origin_job_id":origin_job_id,"origin_run_id":origin_run_id}),
            )
        }
        Operation::Sample {
            run,
            cursors,
            after_event,
        } => {
            run.validate()?;
            ensure!(after_event >= 0, "invalid remote event cursor");
            let job = store.status(run.job_id.clone()).await?;
            run.check(&job)?;
            ensure!(
                !job.command.is_external(),
                "nested external execution is unsupported"
            );
            let stdout = crate::archive::log_bytes(
                config.clone(),
                run.run_id.clone(),
                "stdout".into(),
                cursors[0],
                LOG_PAGE,
            )
            .await?;
            let stderr = crate::archive::log_bytes(
                config.clone(),
                run.run_id.clone(),
                "stderr".into(),
                cursors[1],
                LOG_PAGE,
            )
            .await?;
            let mut inspected = store.inspect(Some(run.job_id.clone()), None, 1).await?;
            for stream in ["stdout", "stderr"] {
                if let Some(text) = inspected["observation"][stream]["text"].as_str() {
                    let tail: String = text
                        .chars()
                        .rev()
                        .take(4096)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    inspected["observation"][stream]["text"] = json!(tail);
                }
            }
            let events = store.events(run.job_id.clone(), after_event, 20).await?;
            // A fresh job read follows the log reads: terminal is only authoritative
            // once a subsequent empty page drains all persisted output.
            Ok(
                json!({"run":run,"status":job.status,"exit_code":job.exit_code,"signal":job.signal,"detail":job.detail,"stdout":stdout,"stderr":stderr,"events":events,"next_event":events.last().map_or(after_event, |e| e.seq),"observation":inspected["observation"],"sampled_at":now()}),
            )
        }
        Operation::Stop { run, grace_seconds } => {
            run.validate()?;
            // Job/Run is immutable; stop_with serializes terminal/cancel races.
            let job = store.status(run.job_id.clone()).await?;
            run.check(&job)?;
            ensure!(
                !job.command.is_external(),
                "nested external stopping is unsupported"
            );
            let job = store
                .stop_with(
                    run.job_id.clone(),
                    grace_seconds,
                    "remote exact-run request".into(),
                )
                .await?;
            Ok(
                json!({"run":run,"status":job.status,"stop_outcome":"requested; terminal evidence required"}),
            )
        }
    }
}
/// One bounded request per authenticated stdio connection; no TCP listener.
pub async fn serve(config: Config) -> Result<()> {
    let target = identity(&config)?;
    let mut input = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::io::stdin().take(131074).read_to_end(&mut input),
    )
    .await??;
    ensure!(input.len() <= 131073, "remote request exceeds limit");
    let request: BridgeRequest = serde_json::from_slice(&input)?;
    let request_id = request.request_id.clone();
    let response = match handle(&config, request).await {
        Ok(result) => {
            json!({"protocol_version":1,"target_id":target,"request_id":request_id,"ok":true,"result":result})
        }
        Err(error) => {
            // Do not serialize remote command arguments, paths or stderr into errors.
            tracing::debug!(error=%error,"remote request refused");
            json!({"protocol_version":1,"target_id":target,"request_id":request_id,"ok":false,"error_kind":"identity_or_operation_refused"})
        }
    };
    let mut output = serde_json::to_vec(&response)?;
    ensure!(
        output.len() <= super::transport::MAX_RESPONSE,
        "remote response exceeds limit"
    );
    output.push(b'\n');
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&output).await?;
    stdout.flush().await?;
    Ok(())
}
