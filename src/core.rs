use anyhow::{Result, bail, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn terminal(s: &str) -> bool {
    matches!(
        s,
        "completed" | "failed" | "cancelled" | "timeout" | "orphaned"
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandSpec {
    Exec {
        program: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Shell {
        interpreter: String,
        script: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartSpec {
    pub command: CommandSpec,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: Option<u64>,
    pub session: Option<String>,
    pub idempotency_key: Option<String>,
}
impl StartSpec {
    pub fn validate(&mut self) -> Result<()> {
        ensure!(
            self.cwd.is_absolute(),
            "cwd must be an explicit absolute directory"
        );
        self.cwd = self.cwd.canonicalize()?;
        ensure!(self.cwd.is_dir(), "cwd must be a directory");
        let strings = match &self.command {
            CommandSpec::Exec { program, args } => {
                ensure!(!program.is_empty(), "program is required");
                ensure!(args.len() <= 256, "at most 256 arguments");
                let mut s = vec![program.as_str()];
                s.extend(args.iter().map(String::as_str));
                s
            }
            CommandSpec::Shell {
                interpreter,
                script,
            } => {
                ensure!(
                    !interpreter.is_empty() && !script.is_empty(),
                    "interpreter and script are required"
                );
                vec![interpreter.as_str(), script.as_str()]
            }
        };
        ensure!(
            strings
                .iter()
                .all(|s| !s.contains('\0') && s.len() <= 32768),
            "invalid or oversized command argument"
        );
        ensure!(self.env.len() <= 64, "at most 64 environment overrides");
        for (k, v) in &self.env {
            ensure!(
                !k.is_empty() && !k.contains(['=', '\0']) && !v.contains('\0') && v.len() <= 8192,
                "invalid environment override"
            );
        }
        if let Some(t) = self.timeout_seconds {
            ensure!(
                (1..=604800).contains(&t),
                "timeout must be 1..604800 seconds"
            );
        }
        if let Some(k) = &self.idempotency_key {
            ensure!(
                !k.is_empty() && k.len() <= 128,
                "idempotency key must be 1..128 bytes"
            );
        }
        ensure!(
            serde_json::to_vec(self)?.len() <= 65536,
            "execution specification exceeds 64 KiB"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub sequence: i64,
    pub job_id: String,
    pub run_id: String,
    pub status: String,
    pub command: CommandSpec,
    pub cwd: PathBuf,
    pub session: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub cancel_requested: bool,
    pub runner_pid: Option<u32>,
    pub runner_identity: Option<String>,
    pub workload_pid: Option<u32>,
    pub workload_identity: Option<String>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub detail: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: i64,
    pub event_id: String,
    pub job_id: String,
    pub run_id: String,
    pub kind: String,
    pub at: i64,
    pub payload: serde_json::Value,
}
pub fn validate_id(value: &str) -> Result<()> {
    if uuid::Uuid::parse_str(value).is_err() {
        bail!("invalid ID: expected UUID");
    }
    Ok(())
}
