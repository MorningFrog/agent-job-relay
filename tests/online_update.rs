#![cfg(target_os = "linux")]
mod common;
use serde_json::{Value, json};
use std::{fs::File, path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Rig {
    dir: tempfile::TempDir,
    state: PathBuf,
    installation: PathBuf,
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("online-update-")
            .tempdir_in(common::root())
            .unwrap();
        let installation = dir.path().join("npm/agent-job-relay-runtime");
        std::fs::create_dir_all(&installation).unwrap();
        Self {
            state: dir.path().join("state"),
            dir,
            installation,
        }
    }
    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"));
        command
            .arg("--state-dir")
            .arg(&self.state)
            .env_remove("AJR_STATE_DIR")
            .env_remove("AJR_NPM_LAUNCHER")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::null())
            .kill_on_drop(true);
        command
    }
    async fn cli(&self, args: &[&str]) -> Value {
        let output = self.command().args(args).output().await.unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn guard(&self) -> tokio::process::Command {
        let mut command = self.command();
        command
            .args(["update-guard", "--installation"])
            .arg(&self.installation)
            .arg("--profile")
            .arg(&self.state)
            .args(["--target-version", env!("CARGO_PKG_VERSION")]);
        command
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::write(self.dir.path().join("finish"), b"");
        let _ = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&self.state)
            .args(["daemon", "stop"])
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .output();
    }
}

#[tokio::test]
async fn online_update_fences_replacement_restarts_only_online_profile_and_preserves_history() {
    let r = Rig::new();
    let job = r.cli(&["start", "--shell", "printf preserved"]).await;
    let id = job["job"]["job_id"].as_str().unwrap();
    r.cli(&["wait", id, "--seconds", "5"]).await;
    let mut child = r
        .guard()
        .arg("--apply")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let ready: Value = serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(5), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ready["online_profiles"], json!([r.state]));
    assert_eq!(r.cli(&["doctor"]).await["daemon"], "stopped");
    for path in [
        r.state.join("startup.lock"),
        r.state.join("daemon.lock"),
        r.installation
            .parent()
            .unwrap()
            .join(".agent-job-relay-npm-update.lock"),
    ] {
        assert!(File::open(path).unwrap().try_lock().is_err());
    }
    let competing = r.guard().arg("--apply").output().await.unwrap();
    assert!(!competing.status.success());
    child.stdin.take().unwrap().write_all(format!("{}\nrelease\n", json!({"executable":env!("CARGO_BIN_EXE_agent-job-relay"),"version":env!("CARGO_PKG_VERSION")})).as_bytes()).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(r.cli(&["status", id]).await["status"], "completed");
    assert_eq!(r.cli(&["logs", id]).await["text"], "preserved");
}

#[tokio::test]
async fn active_work_and_downgrades_are_rejected_before_daemon_shutdown() {
    let r = Rig::new();
    let job = r
        .cli(&[
            "start",
            "--cwd",
            r.dir.path().to_str().unwrap(),
            "--shell",
            "while [ ! -f finish ]; do sleep 0.05; done",
        ])
        .await;
    let id = job["job"]["job_id"].as_str().unwrap();
    for apply in [false, true] {
        let mut command = r.guard();
        if apply {
            command.arg("--apply");
        }
        let output = command.output().await.unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("active jobs"));
        assert_ne!(r.cli(&["doctor"]).await["daemon"], "stopped");
    }
    std::fs::write(r.dir.path().join("finish"), b"").unwrap();
    r.cli(&["wait", id, "--seconds", "5"]).await;
    let output = r
        .command()
        .args(["update-guard", "--installation"])
        .arg(&r.installation)
        .arg("--profile")
        .arg(&r.state)
        .args(["--target-version", "0.0.1", "--apply"])
        .output()
        .await
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("downgrade"));
    assert_ne!(r.cli(&["doctor"]).await["daemon"], "stopped");
}

#[tokio::test]
async fn controller_loss_leaves_profiles_stopped_and_preview_creates_no_profile() {
    let r = Rig::new();
    let preview = r.guard().output().await.unwrap();
    assert!(preview.status.success());
    assert!(!r.state.exists());
    let standalone = r.command().arg("update").output().await.unwrap();
    assert!(!standalone.status.success());
    assert!(!r.state.exists());
    let foreign = r.dir.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    let alias = r.dir.path().join("alias");
    std::os::unix::fs::symlink(&foreign, &alias).unwrap();
    let linked = r
        .guard()
        .arg("--profile")
        .arg(alias.join("new-profile"))
        .arg("--apply")
        .output()
        .await
        .unwrap();
    assert!(!linked.status.success());
    assert!(String::from_utf8_lossy(&linked.stderr).contains("symlink"));
    assert!(!foreign.join("new-profile").exists());
    r.cli(&["daemon", "start"]).await;
    let lost = r.guard().arg("--apply").output().await.unwrap();
    assert!(!lost.status.success());
    assert!(String::from_utf8_lossy(&lost.stderr).contains("disconnected"));
    assert_eq!(r.cli(&["doctor"]).await["daemon"], "stopped");
    // A retry starts from observed state and does not invent an online daemon.
    let preview: Value = serde_json::from_slice(&r.guard().output().await.unwrap().stdout).unwrap();
    assert_eq!(preview["online_profiles"], json!([]));
}
