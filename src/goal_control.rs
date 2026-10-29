//! Fixed, bounded public Codex goal RPCs. Never loads a thread or starts a turn.
use anyhow::{Context, Result, bail, ensure};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::PathBuf,
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub home: PathBuf,
    pub thread_id: String,
    /// Old controllers only changed storage, without notifying the owning host.
    #[serde(default)]
    pub host_pause: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub thread_id: String,
    pub objective: String,
    pub status: String,
    pub token_budget: Option<u64>,
    pub tokens_used: u64,
    pub time_used_seconds: u64,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, Value>,
}
impl Goal {
    pub fn same_identity(&self, other: &Self) -> bool {
        self.thread_id == other.thread_id
            && self.objective == other.objective
            && self.created_at == other.created_at
            && self.token_budget == other.token_budget
            && self.extra == other.extra
            && other.tokens_used >= self.tokens_used
            && other.time_used_seconds >= self.time_used_seconds
    }
    /// An outstanding submitting turn may still add its final usage after pause.
    pub fn unchanged_pause(&self, other: &Self) -> bool {
        self.same_identity(other)
            && other.status == "paused"
            && (self.updated_at == other.updated_at
                || other.tokens_used > self.tokens_used
                || other.time_used_seconds > self.time_used_seconds)
    }
}

pub fn executable_hash(path: &std::path::Path) -> Result<String> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, (String, String)>>> = OnceLock::new();
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 1024 * 1024 * 1024,
        "invalid Codex executable"
    );
    let stamp = crate::platform::file_identity(&file)?;
    let cache = CACHE.get_or_init(Default::default);
    if let Some((old, digest)) = cache
        .lock()
        .map_err(|_| anyhow::anyhow!("executable digest cache poisoned"))?
        .get(path)
        && old == &stamp
    {
        return Ok(digest.clone());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        ensure!(
            total <= 1024 * 1024 * 1024,
            "Codex executable grew beyond the size limit"
        );
        hash.update(&buffer[..n]);
    }
    ensure!(
        crate::platform::file_identity(&file)? == stamp,
        "Codex executable changed while hashing"
    );
    let digest = format!("{:x}", hash.finalize());
    let mut cache = cache
        .lock()
        .map_err(|_| anyhow::anyhow!("executable digest cache poisoned"))?;
    if cache.len() >= 16 {
        cache.pop_first();
    }
    cache.insert(path.into(), (stamp, digest.clone()));
    Ok(digest)
}
fn canonical(path: &std::path::Path) -> Result<PathBuf> {
    Ok(crate::platform::normalized_path(path.canonicalize()?))
}

impl Controller {
    pub async fn validate_async(&self) -> Result<()> {
        let controller = self.clone();
        tokio::task::spawn_blocking(move || controller.validate()).await?
    }

    pub fn validate(&self) -> Result<()> {
        crate::core::validate_id(&self.thread_id)?;
        ensure!(
            self.executable.is_absolute() && self.home.is_absolute(),
            "goal controller paths must be absolute"
        );
        ensure!(
            canonical(&self.executable)? == self.executable
                && canonical(&self.home)? == self.home
                && self.home.is_dir(),
            "goal controller paths changed"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && executable_hash(&self.executable)? == self.executable_sha256,
            "Codex executable changed; automatic goal control refused"
        );
        Ok(())
    }

    /// Capture only the real terminal's runtime selection, never the shared MCP's.
    pub async fn capture(thread_id: String) -> Result<(Self, Goal)> {
        tokio::time::timeout(Duration::from_secs(10), Self::capture_inner(thread_id))
            .await
            .context("current-terminal Codex goal capture timed out (10s)")?
    }
    async fn capture_inner(thread_id: String) -> Result<(Self, Goal)> {
        let executable = PathBuf::from(std::env::var_os("CODEX_APP_RUNTIME_CLI_PATH").context(
            "goal waiting requires the current Desktop runtime CLI in this chat's terminal",
        )?)
        .canonicalize()?;
        let executable = crate::platform::normalized_path(executable);
        let image = executable.clone();
        let hash = tokio::task::spawn_blocking(move || executable_hash(&image)).await??;
        let mut process = Process::open(&executable, None).await?;
        let initialization = process.initialize().await?;
        let home = PathBuf::from(
            initialization["codexHome"]
                .as_str()
                .context("Codex did not identify its profile")?,
        )
        .canonicalize()?;
        let home = crate::platform::normalized_path(home);
        let controller = Self {
            executable,
            executable_sha256: hash,
            home,
            thread_id,
            host_pause: true,
        };
        let goal = process
            .goal(&controller.thread_id)
            .await?
            .context("no goal exists in this chat; no job was submitted")?;
        ensure!(
            goal.status == "active",
            "goal waiting requires an active goal; a user-paused or terminal goal is not adopted"
        );
        process.close().await;
        controller.validate_async().await?;
        Ok((controller, goal))
    }

    pub async fn read(&self) -> Result<Option<Goal>> {
        self.exchange(None, None, std::future::ready(Ok(true)))
            .await
    }
    pub(crate) async fn set_status(
        &self,
        status: &str,
        expected: &Goal,
        permit: impl std::future::Future<Output = Result<bool>>,
    ) -> Result<Option<Goal>> {
        ensure!(
            status == "active" && self.host_pause,
            "Relay only restores goals after an originating-host pause acknowledgement"
        );
        self.exchange(Some(status), Some(expected), permit).await
    }
    async fn exchange(
        &self,
        status: Option<&str>,
        expected: Option<&Goal>,
        permit: impl std::future::Future<Output = Result<bool>>,
    ) -> Result<Option<Goal>> {
        self.validate_async().await?;
        tokio::time::timeout(Duration::from_secs(4), async {
            let mut process = Process::open(&self.executable, Some(&self.home)).await?;
            let initialization = process.initialize().await?;
            let actual = PathBuf::from(
                initialization["codexHome"]
                    .as_str()
                    .context("missing Codex profile")?,
            )
            .canonicalize()?;
            let actual = crate::platform::normalized_path(actual);
            ensure!(
                actual == self.home,
                "Codex selected a different profile; goal control refused"
            );
            // Never send objective or budget: an absent goal cannot be recreated.
            let result = if let Some(status) = status {
                let actual = process.goal(&self.thread_id).await?;
                if actual.is_none() {
                    process.close().await;
                    return Ok(None);
                }
                let actual = actual.context("goal missing")?;
                let expected = expected.context("goal mutation requires an expected snapshot")?;
                ensure!(
                    expected.same_identity(&actual)
                        && actual.status == expected.status
                        && (actual.status != "paused" || expected.unchanged_pause(&actual)),
                    "goal changed before status-only mutation"
                );
                ensure!(
                    permit.await?,
                    "goal waiting cancelled or binding changed before mutation"
                );
                process
                    .rpc(
                        "thread/goal/set",
                        json!({"threadId":self.thread_id,"status":status,"origin":"automatic"}),
                    )
                    .await
                    .and_then(|v| parse_goal(v, &self.thread_id))
            } else {
                process.goal(&self.thread_id).await
            };
            process.close().await;
            result
        })
        .await
        .context(
            "Codex goal control deadline exceeded (4s); an unacknowledged mutation is not replayed",
        )?
    }
}

struct Process {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: BufReader<tokio::process::ChildStdout>,
    next_id: u32,
}
impl Process {
    async fn open(executable: &std::path::Path, home: Option<&std::path::Path>) -> Result<Self> {
        let mut command = tokio::process::Command::new(executable);
        command
            .args(["-c", "features.plugins=false", "app-server", "--stdio"])
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_SESSION_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(home) = home {
            command.env("CODEX_HOME", home);
        }
        crate::platform::supervised_helper(command.as_std_mut());
        let mut child = command
            .spawn()
            .context("cannot start selected Codex goal controller")?;
        Ok(Self {
            input: child.stdin.take().context("Codex stdin missing")?,
            output: BufReader::new(child.stdout.take().context("Codex stdout missing")?),
            child,
            next_id: 0,
        })
    }
    async fn initialize(&mut self) -> Result<Value> {
        let result = self.rpc("initialize", json!({"clientInfo":{"name":"agent-job-relay-goal-control","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})).await?;
        self.input
            .write_all(b"{\"method\":\"initialized\"}\n")
            .await?;
        self.input.flush().await?;
        Ok(result)
    }
    async fn goal(&mut self, thread: &str) -> Result<Option<Goal>> {
        parse_goal(
            self.rpc("thread/goal/get", json!({"threadId":thread}))
                .await?,
            thread,
        )
    }
    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut bytes = serde_json::to_vec(&json!({"id":id,"method":method,"params":params}))?;
        bytes.push(b'\n');
        self.input.write_all(&bytes).await?;
        self.input.flush().await?;
        for _ in 0..64 {
            let mut line = Vec::new();
            loop {
                let buffer = self.output.fill_buf().await?;
                ensure!(!buffer.is_empty(), "Codex controller disconnected");
                let newline = buffer.iter().position(|c| *c == b'\n');
                let n = newline.map_or(buffer.len(), |n| n + 1);
                ensure!(
                    line.len() + n <= 1048576,
                    "Codex goal response exceeds 1 MiB"
                );
                line.extend_from_slice(&buffer[..n]);
                self.output.consume(n);
                if newline.is_some() {
                    break;
                }
            }
            let response: Value = serde_json::from_slice(&line)?;
            if response["id"] != id {
                continue;
            }
            if let Some(error) = response.get("error") {
                // Only expose a bounded diagnostic; never interpret it as instructions.
                bail!(
                    "Codex goal RPC rejected: {}",
                    error["message"]
                        .as_str()
                        .unwrap_or("unknown error")
                        .chars()
                        .take(256)
                        .collect::<String>()
                );
            }
            return response
                .get("result")
                .cloned()
                .context("Codex response has no result");
        }
        bail!("too many unrelated Codex controller frames")
    }
    async fn close(mut self) {
        let _ = self.input.shutdown().await;
        if tokio::time::timeout(Duration::from_millis(500), self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
            let _ = self.child.wait().await;
        }
    }
}
fn parse_goal(value: Value, thread: &str) -> Result<Option<Goal>> {
    ensure!(
        value.get("goal").is_some(),
        "Codex response has no goal field"
    );
    if value["goal"].is_null() {
        return Ok(None);
    }
    let goal: Goal = serde_json::from_value(value["goal"].clone())?;
    ensure!(
        goal.thread_id == thread
            && !goal.objective.is_empty()
            && goal.objective.chars().count() <= 4000,
        "Codex returned a different or invalid goal"
    );
    Ok(Some(goal))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cached_executable_digest_rejects_a_same_length_replacement() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::Builder::new()
            .prefix("goal-image-")
            .tempdir_in(root)
            .unwrap();
        let executable = temp.path().join("codex-image");
        std::fs::write(&executable, "old").unwrap();
        let controller = super::Controller {
            executable_sha256: super::executable_hash(&executable).unwrap(),
            executable: super::canonical(&executable).unwrap(),
            home: super::canonical(temp.path()).unwrap(),
            thread_id: crate::core::id(),
            host_pause: true,
        };
        controller.validate().unwrap();
        std::fs::write(&executable, "new").unwrap();
        assert!(
            controller.validate().is_err(),
            "an unchanged path/length must not reuse a stale digest"
        );
    }
}
