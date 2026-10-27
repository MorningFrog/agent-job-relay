use crate::{
    config::Config,
    ipc::{self, Request},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Read-only and useful before first use; never starts a daemon or probes a host.
pub async fn report(config: &Config, inspect_context: bool) -> Result<Value> {
    let mut report = match ipc::call(config, Request::Doctor).await {
        Ok(value) => value,
        Err(e) if crate::client::unavailable(&e) => {
            json!({"daemon":"stopped","store":"not checked while daemon is offline","next_step":"Task submission starts the daemon automatically."})
        }
        Err(e) => return Err(e),
    };
    #[cfg(windows)]
    {
        report["automatic_startup"] = json!({"mode":std::env::var("AJR_WINDOWS_STARTUP").unwrap_or_else(|_| "auto".into()),"broker":"verified same-user local CIM when a cleanup job blocks direct launch","persistent_registration":false,"outside_terminal_command":crate::platform::outside_start_command(config)?});
    }
    report["desktop_context"] = if inspect_context {
        crate::client::context_summary()
    } else {
        json!({"available":false,"reason":"shared server environment is not caller identity; use current-terminal binding"})
    };
    report["mcp_host_connection"] =
        json!("not observed by CLI; invoke job_doctor from the host to verify tool connectivity");
    report["wake_verification"] =
        json!("context availability only; no host probe or message was sent");
    report["state_dir"] = json!(config.root);
    if let Err(error) = crate::client::verify_runner_launch(&report) {
        report["runner_readiness"] =
            json!({"status":"restart_required","reason":error.to_string()});
    }
    if let Some(profile) = crate::deployment::runtime_state()? {
        report["installed_profile"] =
            json!({"state_dir":profile,"matches_active_state":profile==config.root});
    }
    Ok(report)
}

/// A stdio subprocess self-check is deliberately not a claim about host discovery.
pub async fn check_mcp(config: &Config) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut child = tokio::process::Command::new(std::env::current_exe()?)
            .args(["--state-dir"]).arg(&config.root).args(["mcp", "--shared-host"])
            .env_remove("CODEX_THREAD_ID").env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .kill_on_drop(true).spawn()?;
        let mut input = child.stdin.take().context("missing MCP stdin")?;
        let mut output = BufReader::new(child.stdout.take().context("missing MCP stdout")?);
        input.write_all(format!("{}\n",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"agent-job-relay-doctor","version":env!("CARGO_PKG_VERSION")}}})).as_bytes()).await?;
        let init = response(&mut output, 1).await?;
        ensure!(init["serverInfo"]["name"] == "agent-job-relay", "unexpected MCP server identity");
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n").await?;
        let list = response(&mut output, 2).await?;
        let tools = list["tools"].as_array().context("missing tool catalog")?;
        for required in ["job_start", "job_status", "job_logs", "job_stop", "job_doctor"] {
            ensure!(tools.iter().any(|t| t["name"] == required && t["inputSchema"]["type"] == "object"), "missing or malformed MCP tool {required}");
        }
        let count = tools.len();
        drop(input);
        ensure!(child.wait().await?.success(), "MCP process did not exit cleanly after EOF");
        Ok(json!({"status":"passed","tools":count,"initialize":"passed","discovery":"passed","host_connection":"not checked","host_message_sent":false}))
    }).await.context("MCP self-check timed out (5s)")?
}
async fn response<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R, id: u32) -> Result<Value> {
    for _ in 0..8 {
        let mut frame = Vec::new();
        use tokio::io::AsyncReadExt;
        let n = reader.take(131073).read_until(b'\n', &mut frame).await?;
        ensure!(
            n > 0 && n <= 131072 && frame.last() == Some(&b'\n'),
            "missing or oversized MCP response"
        );
        let value: Value = serde_json::from_slice(&frame)?;
        if value["id"] == id {
            ensure!(
                value.get("error").is_none(),
                "MCP self-check request rejected"
            );
            return value.get("result").cloned().context("MCP result missing");
        }
    }
    anyhow::bail!("MCP response correlation failed")
}
