#![cfg(target_os = "linux")]
mod common;
use serde_json::{Value, json};
use std::{fs::File, path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

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
    async fn state(&self, id: &str, expected: &str) -> Value {
        let mut last = Value::Null;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                last = self.cli(&["status", id]).await;
                if last["status"] == expected {
                    return last.clone();
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("job {id} did not enter {expected}; last state: {last}"))
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
    async fn peer(&self) -> (u32, String) {
        let config = agent_job_relay::config::Config {
            root: self.state.clone(),
        };
        let stream = agent_job_relay::platform::connect_host(&config.socket())
            .await
            .unwrap();
        let pid = agent_job_relay::platform::peer_pid(&stream)
            .unwrap()
            .unwrap();
        (pid, agent_job_relay::platform::identity(pid).unwrap())
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

async fn guard_ready(child: &mut tokio::process::Child) -> Value {
    let mut lines = BufReader::new(child.stdout.as_mut().unwrap()).lines();
    let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("maintenance guard readiness timed out")
        .expect("maintenance guard stdout failed");
    let Some(line) = line else {
        let status = child.wait().await.unwrap();
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            pipe.read_to_string(&mut stderr).await.unwrap();
        }
        panic!("maintenance guard exited before readiness ({status}): {stderr}");
    };
    serde_json::from_str(&line).unwrap()
}

#[tokio::test]
async fn online_update_fences_replacement_restarts_only_online_profile_and_preserves_history() {
    let r = Rig::new();
    let job = r
        .cli(&[
            "start",
            "--cwd",
            r.dir.path().to_str().unwrap(),
            "--shell",
            "while [ ! -f finish ]; do sleep 0.02; done; printf preserved",
        ])
        .await;
    let id = job["job"]["job_id"].as_str().unwrap();
    // Event waiting can return `started` while the original run is still active.
    let event = r.cli(&["wait", id, "--seconds", "5"]).await;
    assert_eq!(event["job"]["status"], "running");
    assert_eq!(event["job"]["run_id"], job["job"]["run_id"]);
    let refused = r.guard().arg("--apply").output().await.unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("active jobs"));
    std::fs::write(r.dir.path().join("finish"), b"").unwrap();
    r.state(id, "completed").await;
    let mut child = r
        .guard()
        .arg("--apply")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let ready = guard_ready(&mut child).await;
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
    r.state(id, "completed").await;
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

#[tokio::test]
async fn uninstall_defaults_to_idle_shutdown_and_force_cancels_exact_runs_without_purge() {
    let r = Rig::new();
    let job = r
        .cli(&[
            "start",
            "--cwd",
            r.dir.path().to_str().unwrap(),
            "--shell",
            "echo ready; while [ ! -f finish ]; do sleep 0.05; done",
        ])
        .await;
    let id = job["job"]["job_id"].as_str().unwrap();
    let run = job["job"]["run_id"].as_str().unwrap();
    let refused = r
        .guard()
        .args(["--remove", "--apply"])
        .output()
        .await
        .unwrap();
    assert!(!refused.status.success());
    assert_ne!(r.cli(&["doctor"]).await["daemon"], "stopped");
    let preview: Value = serde_json::from_slice(
        &r.guard()
            .args(["--remove", "--force"])
            .output()
            .await
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(preview["active_jobs"], 1);
    assert_eq!(r.cli(&["status", id]).await["cancel_requested"], false);
    let mut child = r
        .guard()
        .args(["--remove", "--force", "--apply"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let ready = guard_ready(&mut child).await;
    assert_eq!(ready["stopped_runs"], json!([run]));
    assert_eq!(r.cli(&["doctor"]).await["daemon"], "stopped");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"release_only\":true}\nrelease\n")
        .await
        .unwrap();
    assert!(child.wait().await.unwrap().success());
    let store = agent_job_relay::storage::Store::readonly(agent_job_relay::config::Config {
        root: r.state.clone(),
    });
    assert_eq!(
        store.status(id.into()).await.unwrap().status.as_str(),
        "cancelled"
    );
    assert!(r.state.join("relay.sqlite3").exists());
    r.cli(&["daemon", "start"]).await;
    let mut child = r
        .guard()
        .args(["--remove", "--apply"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert_eq!(guard_ready(&mut child).await["ready"], true);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"release_only\":true}\nrelease\n")
        .await
        .unwrap();
    assert!(child.wait().await.unwrap().success());
    assert_eq!(r.cli(&["doctor"]).await["daemon"], "stopped");
}

#[tokio::test]
async fn compatible_daemon_handover_preserves_active_run_and_refuses_schema_mismatch() {
    let r = Rig::new();
    let job = r
        .cli(&[
            "start",
            "--cwd",
            r.dir.path().to_str().unwrap(),
            "--shell",
            "echo once >> launches; echo ready; while [ ! -f finish ]; do sleep 0.05; done",
        ])
        .await;
    let id = job["job"]["job_id"].as_str().unwrap();
    r.state(id, "running").await;
    let original_peer = r.peer().await;
    let database = rusqlite::Connection::open(r.state.join("relay.sqlite3")).unwrap();
    database.busy_timeout(Duration::from_secs(5)).unwrap();
    for version in [11, 13] {
        database
            .pragma_update(None, "user_version", version)
            .unwrap();
        for apply in [false, true] {
            let mut command = r.guard();
            command.arg("--allow-active");
            if apply {
                command.arg("--apply");
            }
            let refused = command.output().await.unwrap();
            assert!(!refused.status.success());
            let error = String::from_utf8_lossy(&refused.stderr);
            assert!(
                error.contains(&format!("database schema {version}")),
                "{error}"
            );
            assert!(error.contains("requires schema 12"), "{error}");
            assert!(error.contains(r.state.to_str().unwrap()), "{error}");
            assert!(error.contains("let tasks finish"), "{error}");
            assert!(error.contains("retry ajr update"), "{error}");
            assert!(error.contains("Codex application"), "{error}");
            assert_eq!(
                database
                    .pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0))
                    .unwrap(),
                version
            );
            assert_eq!(r.peer().await, original_peer);
            let current = r.cli(&["status", id]).await;
            assert_eq!(current["run_id"], job["job"]["run_id"]);
            assert_eq!(current["status"], "running");
            assert_eq!(current["cancel_requested"], false);
        }
    }
    database.pragma_update(None, "user_version", 12).unwrap();
    let mut child = r
        .guard()
        .args(["--allow-active", "--apply"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let ready = guard_ready(&mut child).await;
    assert_eq!(ready["active_jobs"], 1);
    let store = agent_job_relay::storage::Store::readonly(agent_job_relay::config::Config {
        root: r.state.clone(),
    });
    assert_eq!(
        store.status(id.into()).await.unwrap().run_id,
        job["job"]["run_id"]
    );
    child.stdin.take().unwrap().write_all(format!("{}\nrelease\n", json!({"executable":env!("CARGO_BIN_EXE_agent-job-relay"),"version":env!("CARGO_PKG_VERSION")})).as_bytes()).await.unwrap();
    let output = child.wait_with_output().await.unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(r.dir.path().join("finish"), b"").unwrap();
    r.state(id, "completed").await;
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("launches")).unwrap(),
        "once\n"
    );
}
