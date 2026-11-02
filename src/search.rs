//! Bounded read-only search across hot records and validated cold snapshots.
use crate::{core::*, storage::Store};
use anyhow::{Result, ensure};
use rmcp::schemars;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TaskQuery {
    pub text: String,
    pub status: Option<String>,
    pub cwd: Option<String>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
}
impl TaskQuery {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.text.len() <= 1024 && !self.text.contains('\0'),
            "search text must be at most 1024 bytes without NUL"
        );
        ensure!(
            self.cwd
                .as_ref()
                .is_none_or(|s| s.len() <= 4096 && !s.contains('\0')),
            "invalid search cwd"
        );
        if let Some(status) = &self.status {
            ensure!(
                status == "active" || status.parse::<JobState>().is_ok(),
                "invalid search status"
            );
        }
        ensure!(
            self.created_after.is_none_or(|t| t >= 0) && self.created_before.is_none_or(|t| t >= 0),
            "search timestamps must be nonnegative milliseconds"
        );
        ensure!(
            self.created_after
                .zip(self.created_before)
                .is_none_or(|(a, b)| a <= b),
            "search time range is reversed"
        );
        Ok(())
    }
    fn matches(&self, job: &Job) -> bool {
        let command = match &job.command {
            CommandSpec::Shell {
                interpreter,
                script,
            } => format!("{interpreter} {script}"),
            CommandSpec::Remote { remote } => match &remote.intent {
                crate::remote::Intent::Launch { spec } => {
                    let interpreter = match &spec.command {
                        CommandSpec::Shell { interpreter, .. } => interpreter.as_str(),
                        _ => "",
                    };
                    format!("{} {interpreter}", job.command.display_text())
                }
                _ => job.command.display_text(),
            },
            _ => job.command.display_text(),
        };
        self.status.as_ref().is_none_or(|s| {
            if s == "active" {
                !job.status.is_terminal()
            } else {
                s == job.status.as_str()
            }
        }) && self.created_after.is_none_or(|t| job.created_at >= t)
            && self.created_before.is_none_or(|t| job.created_at <= t)
            && self.cwd.as_ref().is_none_or(|s| {
                job.cwd
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&s.to_lowercase())
            })
            && format!(
                "{} {} {} {} {}",
                job.title.as_deref().unwrap_or(""),
                command,
                job.cwd.display(),
                job.job_id,
                job.run_id
            )
            .to_lowercase()
            .contains(&self.text.to_lowercase())
    }
}
impl Store {
    pub async fn search_tasks(
        &self,
        query: TaskQuery,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Value> {
        query.validate()?;
        ensure!(
            (1..=100).contains(&limit) && before.is_none_or(|s| s > 0),
            "invalid search page"
        );
        let mut value = self.read(move |db| {
            let tx = db.transaction()?;
            let mut q = tx.prepare("SELECT id,sequence FROM (SELECT id,rowid AS sequence FROM jobs UNION ALL SELECT id,sequence FROM archive_jobs) WHERE (?1 IS NULL OR sequence<?1) ORDER BY sequence DESC LIMIT 129")?;
            let rows = q.query_map(params![before], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let mut jobs = Vec::new();
            let mut scanned = 0;
            let mut cursor = before;
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
            for (id, sequence) in rows.iter().take(128) {
                let job = crate::storage::read_job(&tx, id)?;
                let summary = crate::monitor::summary(job.clone());
                scanned += 1; cursor = Some(*sequence);
                if query.matches(&job) { jobs.push(summary); }
                if jobs.len() >= limit as usize || std::time::Instant::now() >= deadline { break; }
            }
            let more = scanned < rows.len();
            Ok(json!({"jobs":jobs,"next_cursor":if more {cursor} else {None},"has_more":more,"scanned":scanned,"scope":"hot_and_archived","query":query,"sampled_at":now()}))
        }).await?;
        value["profile"] = json!(self.config.root);
        Ok(value)
    }
}

/// Search does not start a daemon. A stopped profile can be read offline.
pub async fn client(config: &crate::config::Config, request: crate::ipc::Request) -> Result<Value> {
    ensure!(
        matches!(
            &request,
            crate::ipc::Request::Search { .. } | crate::ipc::Request::SearchLogs { .. }
        ),
        "only search requests are supported"
    );
    // Request is JSON-roundtripped only to preserve the read-only request on an
    // unavailable connection; no mutating request is ever replayed here.
    let saved = serde_json::to_value(&request)?;
    match crate::ipc::call(config, request).await {
        Ok(value) => Ok(value),
        Err(error) if crate::client::unavailable(&error) => {
            ensure!(
                config.root.join("relay.sqlite3").exists(),
                "no recorded profile; search did not create state"
            );
            crate::ipc::dispatch(
                &Store::readonly(config.clone()),
                serde_json::from_value(saved)?,
            )
            .await
        }
        Err(error) => Err(error),
    }
}
