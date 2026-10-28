//! Pinned external executions. Transport is replaceable; execution identity is not.
mod actions;
mod observer;
mod protocol;
mod transport;

pub use observer::observe;
pub use protocol::{BridgeRequest, Operation, RunRef, initialize, serve};
pub use transport::exchange;

use crate::core::*;
use anyhow::{Result, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transport {
    /// Explicit argv, e.g. ssh with a preconfigured host alias and fixed helper command.
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}
impl Transport {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.program.is_empty() && !self.program.starts_with('-'),
            "transport program is required"
        );
        ensure!(self.args.len() <= 64, "at most 64 transport arguments");
        ensure!(
            std::iter::once(&self.program)
                .chain(&self.args)
                .all(|s| s.len() <= 4096 && !s.contains(['\0', '\n', '\r'])),
            "invalid transport argument"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RemoteSpec {
    /// Pin the value returned by `remote init` on the selected remote profile.
    pub target_id: String,
    pub transport: Transport,
    pub intent: Intent,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Launch {
        spec: Box<StartSpec>,
    },
    Attach {
        run: RunRef,
        /// Observation is the default; only an explicit opt-in grants stop authority.
        #[serde(default)]
        allow_stop: bool,
    },
}
impl RemoteSpec {
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.target_id)?;
        self.transport.validate()?;
        match &self.intent {
            Intent::Launch { spec } => validate_launch(spec)?,
            Intent::Attach { run, .. } => run.validate()?,
        }
        Ok(())
    }
    pub fn can_stop(&self) -> bool {
        matches!(
            self.intent,
            Intent::Launch { .. }
                | Intent::Attach {
                    allow_stop: true,
                    ..
                }
        )
    }
}
pub(crate) fn validate_launch(spec: &StartSpec) -> Result<()> {
    ensure!(
        !spec.command.is_external(),
        "nested external execution is unsupported"
    );
    ensure!(
        spec.session.is_none() && spec.idempotency_key.is_none(),
        "remote session/idempotency are managed by the origin; omit them"
    );
    let cwd = spec.cwd.to_str().unwrap_or("");
    // The remote OS resolves/canonicalizes this path before admission.
    ensure!(
        !cwd.is_empty() && cwd.len() <= 4096 && !cwd.contains(['\0', '\n', '\r']),
        "invalid remote cwd"
    );
    ensure!(
        cwd.starts_with('/')
            || (cwd.as_bytes().get(1) == Some(&b':')
                && matches!(cwd.as_bytes().get(2), Some(b'\\' | b'/'))),
        "remote cwd must be absolute on the remote OS"
    );
    ensure!(
        spec.monitor_rules.iter().all(|r| !r.notify_agent),
        "remote runner rules cannot bind a local chat; use origin rules for notifications"
    );
    spec.validate_options()
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActionsSpec {
    /// Exact owner/repository; no ambient repository selection.
    pub repository: String,
    pub run_id: u64,
    pub attempt: u32,
    #[serde(default = "github_host")]
    pub hostname: String,
    #[serde(default = "gh_program")]
    pub gh_program: String,
}
fn github_host() -> String {
    "github.com".into()
}
fn gh_program() -> String {
    "gh".into()
}
impl ActionsSpec {
    pub fn validate(&self) -> Result<()> {
        let parts: Vec<_> = self.repository.split('/').collect();
        ensure!(
            parts.len() == 2
                && parts.iter().all(|s| !s.is_empty()
                    && *s != "."
                    && *s != ".."
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))),
            "repository must be owner/repository"
        );
        ensure!(
            self.repository.len() <= 200 && self.run_id > 0 && self.attempt > 0,
            "invalid workflow run/attempt"
        );
        ensure!(
            !self.hostname.is_empty()
                && self.hostname.len() <= 253
                && !self.hostname.starts_with('-')
                && self
                    .hostname
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b)),
            "invalid GitHub hostname"
        );
        ensure!(
            !self.gh_program.is_empty()
                && self.gh_program.len() <= 4096
                && !self.gh_program.contains(['\0', '\n', '\r']),
            "invalid gh program"
        );
        Ok(())
    }
}
