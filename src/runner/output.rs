use super::LOG_CAP;
use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
pub(super) struct Captures {
    stdout: tokio::task::JoinHandle<Result<bool>>,
    stderr: tokio::task::JoinHandle<Result<bool>>,
}
impl Captures {
    pub(super) fn start(
        stdout: impl AsyncRead + Unpin + Send + 'static,
        stderr: impl AsyncRead + Unpin + Send + 'static,
        dir: std::path::PathBuf,
        errors: tokio::sync::mpsc::Sender<String>,
        live: crate::monitor::SharedLive,
    ) -> Self {
        Self {
            stdout: tokio::spawn(capture_checked(
                stdout,
                dir.join("stdout.log"),
                errors.clone(),
                live.clone(),
                0,
            )),
            stderr: tokio::spawn(capture_checked(
                stderr,
                dir.join("stderr.log"),
                errors,
                live,
                1,
            )),
        }
    }
    pub(super) async fn drain(&mut self) -> Result<(bool, bool)> {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            Ok::<_, anyhow::Error>(((&mut self.stdout).await??, (&mut self.stderr).await??))
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!("output drain deadline exceeded; descendant cleanup unconfirmed")
        })?
    }
}
impl Drop for Captures {
    fn drop(&mut self) {
        self.stdout.abort();
        self.stderr.abort();
    }
}
async fn capture(
    mut source: impl AsyncRead + Unpin,
    path: std::path::PathBuf,
    live: crate::monitor::SharedLive,
    stream: usize,
) -> Result<bool> {
    let mut recent = crate::log_retention::Writer::new(path.with_extension("recent"));
    let file = crate::platform::private_file(&path)?;
    let mut file = tokio::fs::File::from_std(file);
    let mut buf = [0u8; 8192];
    let mut written = 0;
    let mut truncated = false;
    loop {
        let n = source.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        live.lock()
            .map_err(|_| anyhow::anyhow!("monitor state poisoned"))?
            .output(stream, &buf[..n]);
        let keep = (LOG_CAP - written).min(n as u64) as usize;
        if keep > 0 {
            file.write_all(&buf[..keep])
                .await
                .context("cannot write output log")?;
            written += keep as u64;
        }
        recent
            .output(&buf[keep..n])
            .await
            .context("cannot persist recent output")?;
        truncated |= keep < n;
    }
    recent
        .flush()
        .await
        .context("cannot synchronize recent output")?;
    // Tokio can acknowledge buffered writes before the blocking write finishes.
    // flush surfaces that operation's error; sync_all alone can miss it.
    file.flush().await.context("cannot flush output log")?;
    file.sync_all()
        .await
        .context("cannot synchronize output log")?;
    Ok(truncated)
}
pub(super) async fn capture_checked(
    source: impl AsyncRead + Unpin,
    path: std::path::PathBuf,
    errors: tokio::sync::mpsc::Sender<String>,
    live: crate::monitor::SharedLive,
    stream: usize,
) -> Result<bool> {
    let result = capture(source, path, live, stream).await;
    if let Err(e) = &result {
        let _ = errors
            .send(format!("output persistence failed: {e:#}"))
            .await;
    }
    result
}
