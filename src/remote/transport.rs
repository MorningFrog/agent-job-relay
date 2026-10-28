use super::{BridgeRequest, Transport};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{process::Stdio, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) const MAX_RESPONSE: usize = 768 * 1024;
pub(crate) const DEADLINE: Duration = Duration::from_secs(15);

/// Bounded subprocess I/O. Only the disposable transport child is killed.
/// Raw stderr is never persisted (SSH/gh may print authentication material).
pub(crate) async fn process(
    program: &str,
    args: &[String],
    input: &[u8],
    cwd: Option<&std::path::Path>,
) -> Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>)> {
    let mut command = tokio::process::Command::new(program);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Never lend the local chat endpoint to a transport/plugin subprocess.
    command
        .env_remove("CODEX_THREAD_ID")
        .env_remove("CODEX_APP_TOOLS_PIPE_PATH");
    let mut child = command
        .spawn()
        .context("cannot start external transport; check the configured program")?;
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let operation = async {
        let send = async move {
            stdin.write_all(input).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, anyhow::Error>(())
        };
        let read = async {
            let mut output = Vec::new();
            stdout
                .take(MAX_RESPONSE as u64 + 1)
                .read_to_end(&mut output)
                .await?;
            ensure!(
                output.len() <= MAX_RESPONSE,
                "external response exceeds limit"
            );
            Ok::<_, anyhow::Error>(output)
        };
        let errors = async {
            let mut output = Vec::new();
            stderr.take(8193).read_to_end(&mut output).await?;
            ensure!(output.len() <= 8192, "external diagnostics exceed limit");
            Ok::<_, anyhow::Error>(output)
        };
        let (_, output, errors) = tokio::try_join!(send, read, errors)?;
        Ok::<_, anyhow::Error>((child.wait().await?, output, errors))
    };
    let result = tokio::time::timeout(DEADLINE, operation).await;
    match result {
        Ok(Ok(result)) => Ok(result),
        other => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            match other {
                Ok(Err(e)) => Err(e),
                _ => anyhow::bail!("external request timed out; execution outcome is unknown"),
            }
        }
    }
}
pub async fn exchange(transport: &Transport, request: &BridgeRequest) -> Result<Value> {
    exchange_at(transport, request, None).await
}
pub(crate) async fn exchange_at(
    transport: &Transport,
    request: &BridgeRequest,
    cwd: Option<&std::path::Path>,
) -> Result<Value> {
    transport.validate()?;
    let mut input = serde_json::to_vec(request)?;
    ensure!(input.len() <= 131072, "external request exceeds 128 KiB");
    input.push(b'\n');
    let (exit, output, _) = process(&transport.program, &transport.args, &input, cwd).await?;
    ensure!(
        exit.success(),
        "remote transport exited {}; check SSH/authentication/remote helper; no remote outcome inferred",
        exit.code()
            .map_or_else(|| "without status".into(), |v| v.to_string())
    );
    let response: Value =
        serde_json::from_slice(&output).context("invalid remote protocol response")?;
    ensure!(
        response["protocol_version"] == 1 && response["target_id"] == request.target_id,
        "remote protocol/target identity mismatch; refused"
    );
    ensure!(
        response["request_id"] == request.request_id,
        "remote response request identity mismatch"
    );
    // Helpers return fixed error categories, never workload output as instructions.
    ensure!(
        response["ok"] == true,
        "remote request refused ({})",
        response["error_kind"]
            .as_str()
            .unwrap_or("invalid_response")
    );
    Ok(response["result"].clone())
}
