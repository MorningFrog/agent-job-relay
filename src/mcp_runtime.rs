//! A retained MCP connection follows only its recorded, verified installation.
//! It never searches caches, changes profiles or replays an uncertain call.
use crate::{config::Config, deployment as install};
use anyhow::{Context, Result, ensure};
use rmcp::{RoleServer, model::*, service::RequestContext};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub(crate) struct Managed {
    root: PathBuf,
    state: PathBuf,
    version: String,
    digest: String,
    observed: std::sync::Arc<std::sync::Mutex<(String, String)>>,
}
pub(crate) struct Resolved {
    pub(crate) image: Option<PathBuf>,
    pub(crate) changed: bool,
    pub(crate) guard: std::fs::File,
}
impl Managed {
    pub(crate) fn entry(&self) -> PathBuf {
        self.root.join(crate::platform::NATIVE_FILE)
    }
    pub(crate) fn capture() -> Result<Option<Self>> {
        let image = std::env::current_exe()?;
        if image.parent().and_then(std::path::Path::file_name) != Some(std::ffi::OsStr::new("bin"))
        {
            return Ok(None);
        }
        let profile = image
            .parent()
            .and_then(std::path::Path::parent)
            .context("missing runtime directory")?
            .join(install::PROFILE);
        if !profile.try_exists()? {
            return Ok(None);
        }
        let profile: install::RuntimeProfile =
            serde_json::from_slice(&install::read_file(&profile)?)?;
        ensure!(
            profile.format == 1
                && profile.version == env!("CARGO_PKG_VERSION")
                && profile.platform == crate::platform::runtime_target(),
            "invalid loaded MCP runtime profile"
        );
        install::checked_path(&profile.install_root)?;
        let digest = crate::goal_control::executable_hash(&image)?;
        ensure!(
            profile.files.get(crate::platform::NATIVE_FILE) == Some(&digest),
            "loaded MCP image differs from its recorded snapshot"
        );
        Ok(Some(Self {
            observed: std::sync::Arc::new(std::sync::Mutex::new((
                profile.version.clone(),
                digest.clone(),
            ))),
            root: profile.install_root,
            state: profile.state_dir,
            version: profile.version,
            digest,
        }))
    }
    pub(crate) async fn resolve(&self) -> Result<Resolved> {
        let (root, state, version, digest) = (
            self.root.clone(),
            self.state.clone(),
            self.version.clone(),
            self.digest.clone(),
        );
        let observed = self.observed.clone();
        tokio::task::spawn_blocking(move || {
            install::checked_path(&root)?;
            let guard = crate::platform::private_file(
                &root
                    .parent()
                    .context("missing installation parent")?
                    .join(format!(
                        ".{}.install.lock",
                        root.file_name()
                            .context("missing installation name")?
                            .to_string_lossy()
                    )),
            )?;
            guard
                .try_lock_shared()
                .context("managed installation update is in progress; retry after it completes")?;
            crate::lifecycle::require_clear(&root)?;
            let receipt = install::read_receipt(&root)?;
            ensure!(
                receipt.root == root && receipt.state_dir == state,
                "managed MCP installation/profile changed; reconnect explicitly"
            );
            let raw = install::read_file(&root.join(install::PROFILE))?;
            ensure!(
                receipt.files.get(install::PROFILE) == Some(&install::digest(&raw)),
                "managed runtime profile was modified"
            );
            let profile: install::RuntimeProfile = serde_json::from_slice(&raw)?;
            ensure!(
                profile.format == 1
                    && profile.install_root == root
                    && profile.state_dir == state
                    && profile.platform == crate::platform::runtime_target(),
                "managed runtime selected another installation/profile/platform"
            );
            ensure!(
                crate::plugin::version(&profile.version)? >= crate::plugin::version(&version)?,
                "managed MCP refuses a runtime downgrade"
            );
            let image = root.join(crate::platform::NATIVE_FILE);
            install::checked_path(&image)?;
            let current = crate::goal_control::executable_hash(&image)?;
            ensure!(
                receipt.files.get(crate::platform::NATIVE_FILE) == Some(&current)
                    && profile.files.get(crate::platform::NATIVE_FILE) == Some(&current),
                "managed runtime image differs from its verified receipt"
            );
            let mut last = observed
                .lock()
                .map_err(|_| anyhow::anyhow!("managed MCP identity lock poisoned"))?;
            ensure!(
                crate::plugin::version(&profile.version)? >= crate::plugin::version(&last.0)?,
                "managed MCP refuses a runtime downgrade after handover"
            );
            let changed = last.1 != current;
            *last = (profile.version, current.clone());
            Ok(Resolved {
                image: if current == digest { None } else { Some(image) },
                changed,
                guard,
            })
        })
        .await?
    }
}

pub(crate) async fn exchange(
    image: &std::path::Path,
    config: &Config,
    shared: bool,
    method: &str,
    params: Value,
    context: &RequestContext<RoleServer>,
) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(90+crate::goal_consent::QUESTION_SECONDS), async {
        let mut command=tokio::process::Command::new(image);
        command.arg("--state-dir").arg(&config.root).args(["mcp","--worker"]);
        if shared {command.arg("--shared-host").env_remove("CODEX_THREAD_ID").env_remove("CODEX_SESSION_ID").env_remove("CODEX_APP_TOOLS_PIPE_PATH");}
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        crate::platform::supervised_helper(command.as_std_mut());
        let mut child=command.spawn().context("cannot start verified current MCP runtime")?;
        let mut input=child.stdin.take().context("missing current MCP stdin")?;
        let mut output=BufReader::new(child.stdout.take().context("missing current MCP stdout")?);
        let version=context.protocol_version().unwrap_or(ProtocolVersion::V_2025_06_18);
        let capabilities=context.client_capabilities().unwrap_or_default();
        let meta=RequestMetaObject::with_client_context(version.clone(),context.client_info().unwrap_or_else(||Implementation::new("agent-job-relay-managed-forwarder",env!("CARGO_PKG_VERSION"))),capabilities.clone());
        let modern=version>=ProtocolVersion::V_2026_07_28;
        let init=if modern {json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta}})}else{json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":version,"capabilities":capabilities,"clientInfo":{"name":"agent-job-relay-managed-forwarder","version":env!("CARGO_PKG_VERSION")}}})};
        input.write_all(format!("{init}\n").as_bytes()).await?;
        let initialized=tokio::time::timeout(Duration::from_secs(10),response(&mut output,1)).await??;
        if modern {
            ensure!(initialized["supportedVersions"].as_array().is_some_and(|v|v.contains(&json!(version))),"updated runtime lacks the caller's negotiated protocol");
        }else{
            ensure!(initialized["serverInfo"]["name"]=="agent-job-relay","unexpected managed MCP identity");
            input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").await?;
        }
        let mut params=params;
        if modern {params["_meta"]=serde_json::to_value(meta)?;}
        let mut request=serde_json::to_vec(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}))?;
        ensure!(request.len()<=131072,"managed MCP request exceeds 128 KiB");
        request.push(b'\n');input.write_all(&request).await?;input.flush().await?;
        let mut deadline=tokio::time::Instant::now()+Duration::from_secs(90);
        let mut asked=false;
        let result=loop {
            let raw=tokio::select! {
                _=context.ct.cancelled()=>anyhow::bail!("host cancelled current-runtime request; no replay was attempted"),
                raw=tokio::time::timeout_at(deadline,frame(&mut output))=>raw??,
            };
            if raw.get("method").is_some() {
                ensure!(!asked && method=="tools/call" && params["name"]=="job_goal_consent" && raw["method"]=="elicitation/create","unexpected worker request; no automatic replay");
                asked=true;
                let question=serde_json::from_value(raw["params"].clone())?;
                let started=tokio::time::Instant::now();
                let answer=tokio::select! {
                    _=context.ct.cancelled()=>anyhow::bail!("host cancelled native confirmation; no replay was attempted"),
                    answer=context.peer.create_elicitation_with_timeout(question,Some(Duration::from_secs(crate::goal_consent::QUESTION_SECONDS)))=>answer,
                };
                deadline+=started.elapsed();
                let answer=match answer {Ok(value)=>json!({"jsonrpc":"2.0","id":raw["id"],"result":value}),Err(_)=>json!({"jsonrpc":"2.0","id":raw["id"],"error":{"code":-32603,"message":"native question unavailable, cancelled or timed out"}})};
                input.write_all(format!("{answer}\n").as_bytes()).await?;input.flush().await?;
                continue;
            }
            ensure!(raw["id"]==2 && raw.get("error").is_none(),"current runtime response mismatch/rejection; no replay was attempted");
            break raw.get("result").cloned().context("current MCP result missing")?;
        };
        drop(input);
        ensure!(child.wait().await?.success(),"current MCP disconnected after its response; inspect original IDs before retrying");
        Ok(result)
    }).await.context("current MCP deadline exceeded; inspect original Job/Run IDs before retrying an uncertain submission")?
}
async fn frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<Value> {
    let mut raw = Vec::new();
    let n = reader.take(1048577).read_until(b'\n', &mut raw).await?;
    ensure!(
        n > 0 && n <= 1048576 && raw.last() == Some(&b'\n'),
        "missing or oversized current MCP response"
    );
    let value: Value = serde_json::from_slice(&raw)?;
    ensure!(
        value["jsonrpc"] == "2.0",
        "invalid current MCP response protocol"
    );
    Ok(value)
}
async fn response<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R, id: u32) -> Result<Value> {
    for _ in 0..16 {
        let mut raw = Vec::new();
        let n = reader.take(1048577).read_until(b'\n', &mut raw).await?;
        ensure!(
            n > 0 && n <= 1048576 && raw.last() == Some(&b'\n'),
            "missing or oversized current MCP response"
        );
        let value: Value = serde_json::from_slice(&raw)?;
        ensure!(
            value["jsonrpc"] == "2.0",
            "invalid current MCP response protocol"
        );
        if value["id"] == id {
            ensure!(
                value.get("error").is_none(),
                "current MCP rejected the request; no automatic replay was attempted"
            );
            return value
                .get("result")
                .cloned()
                .context("current MCP result missing");
        }
    }
    anyhow::bail!("current MCP response correlation limit exceeded")
}
