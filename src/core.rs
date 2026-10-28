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
    /// A pinned remote runtime owns execution; this process owns observation.
    Remote {
        remote: Box<crate::remote::RemoteSpec>,
    },
    /// Observe one immutable workflow attempt without dispatch/cancel authority.
    GithubActions {
        workflow: crate::remote::ActionsSpec,
    },
}
impl CommandSpec {
    pub(crate) fn validate_rule(&self, rule: &crate::monitor::RuleSpec) -> Result<()> {
        use crate::monitor::Condition;
        match self {
            Self::Remote { .. } => ensure!(
                !matches!(rule.condition, Condition::NoOutput { .. }),
                "remote silence rules must run on the remote runner; put them in remote.intent.spec.monitor_rules"
            ),
            Self::GithubActions { .. } => ensure!(
                matches!(rule.condition, Condition::Elapsed { .. }),
                "workflow observation supports only elapsed rules; it has no workload output stream"
            ),
            _ => {}
        }
        Ok(())
    }
    pub fn is_external(&self) -> bool {
        matches!(self, Self::Remote { .. } | Self::GithubActions { .. })
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitor_rules: Vec<crate::monitor::RuleSpec>,
}
impl StartSpec {
    pub fn validate(&mut self) -> Result<()> {
        ensure!(
            self.cwd.is_absolute(),
            "cwd must be an explicit absolute directory"
        );
        self.cwd = crate::platform::normalized_path(self.cwd.canonicalize()?);
        ensure!(self.cwd.is_dir(), "cwd must be a directory");
        self.validate_options()
    }
    /// Validate a remote intent without resolving its cwd on the observing host.
    pub(crate) fn validate_options(&self) -> Result<()> {
        ensure!(self.monitor_rules.len() <= 16, "at most 16 monitor rules");
        for rule in &self.monitor_rules {
            rule.validate()?;
            self.command.validate_rule(rule)?;
        }
        let strings = match &self.command {
            CommandSpec::Exec { program, args } => {
                ensure!(!program.is_empty(), "program is required");
                #[cfg(windows)]
                ensure!(
                    !std::path::Path::new(program)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd")
                            || ext.eq_ignore_ascii_case("bat")),
                    "batch files require an explicit shell command/interpreter"
                );
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
            CommandSpec::Remote { remote } => {
                remote.validate()?;
                ensure!(
                    self.env.is_empty(),
                    "put remote env in remote.intent.spec.env"
                );
                vec![]
            }
            CommandSpec::GithubActions { workflow } => {
                workflow.validate()?;
                ensure!(
                    self.env.is_empty(),
                    "workflow observation does not accept environment overrides"
                );
                vec![]
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
pub struct Run {
    pub run_id: String,
    pub job_id: String,
    pub attempt: u32,
    pub status: String,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub runner_pid: Option<u32>,
    pub runner_identity: Option<String>,
    pub workload_pid: Option<u32>,
    pub workload_identity: Option<String>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub run: Run,
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
