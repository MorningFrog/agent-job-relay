use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// The persisted/wire vocabulary stays stable; unknown execution states fail
/// at decoding instead of silently behaving like an active job.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Launching,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
    Orphaned,
}
impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Launching => "launching",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::Orphaned => "orphaned",
        }
    }
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Timeout | Self::Orphaned
        )
    }
}
impl std::str::FromStr for JobState {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        Ok(match value {
            "queued" => Self::Queued,
            "launching" => Self::Launching,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "timeout" => Self::Timeout,
            "orphaned" => Self::Orphaned,
            _ => bail!("unknown execution state: {value}"),
        })
    }
}
impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl rusqlite::ToSql for JobState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}
