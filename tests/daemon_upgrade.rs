#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    ipc::{self, Request},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

struct Rig {
    dir: tempfile::TempDir,
    config: Config,
    old: PathBuf,
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("daemon-upgrade-")
            .tempdir_in(common::root())
            .unwrap();
        // Exercise two real native processes with the same schema/IPC behavior.
        // Only the older fixture's fixed-width package version is changed.
        let current = env!("CARGO_PKG_VERSION").as_bytes();
        let older = format!("0.0.{}", "0".repeat(current.len() - 4));
        let mut bytes = std::fs::read(env!("CARGO_BIN_EXE_agent-job-relay")).unwrap();
        let mut replaced = 0;
        let mut offset = 0;
        while let Some(index) = bytes[offset..]
            .windows(current.len())
            .position(|w| w == current)
        {
            let start = offset + index;
            bytes[start..start + current.len()].copy_from_slice(older.as_bytes());
            offset = start + current.len();
            replaced += 1;
        }
        assert!(replaced > 0);
        let old = dir.path().join("old-runtime");
        std::fs::write(&old, bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        Self { dir, config, old }
    }
    fn command(&self, program: &Path) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(program);
        command
            .args(["--state-dir"])
            .arg(&self.config.root)
            .env_remove("AJR_STATE_DIR")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::null())
            .kill_on_drop(true);
        command
    }
    async fn cli(&self, program: &Path, args: &[&str]) -> Value {
        let output = self.command(program).args(args).output().await.unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    async fn current(&self, args: &[&str]) -> Value {
        self.cli(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")), args)
            .await
    }
    async fn old_daemon(&self) -> tokio::process::Child {
        let child = self
            .command(&self.old)
            .arg("daemon")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(info) = ipc::call(&self.config, Request::Doctor).await {
                    assert_ne!(info["version"], env!("CARGO_PKG_VERSION"));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        child
    }
    async fn peer(&self) -> i32 {
        tokio::net::UnixStream::connect(self.config.socket())
            .await
            .unwrap()
            .peer_cred()
            .unwrap()
            .pid()
            .unwrap()
    }
    async fn running_job(&self) -> Value {
        let job = self.cli(&self.old, &["start", "--cwd", self.dir.path().to_str().unwrap(), "--idempotency-key", "upgrade-once", "--shell",
            "echo launch >> launches; echo ready; while [ ! -f finish ]; do sleep 0.05; done; echo done"]).await;
        let id = job["job"]["job_id"].as_str().unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.current(&["logs", id]).await["text"]
                .as_str()
                .unwrap_or("")
                != "ready\n"
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        job
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::write(self.dir.path().join("finish"), b"");
        let _ = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&self.config.root)
            .args(["daemon", "stop"])
            .output();
    }
}

#[tokio::test]
async fn concurrent_new_entries_replace_old_daemon_once_and_preserve_running_job() {
    let r = Rig::new();
    let mut old = r.old_daemon().await;
    let old_pid = r.peer().await;
    let started = r.running_job().await;
    let id = started["job"]["job_id"].as_str().unwrap();
    // Read-only diagnosis still reports the actual old peer without restarting it.
    assert_ne!(
        r.current(&["doctor"]).await["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(r.peer().await, old_pid);
    let (a, b, ui) = tokio::join!(
        r.current(&["daemon", "start"]),
        r.current(&["daemon", "start"]),
        r.current(&["ui", "--no-open"])
    );
    assert_eq!(a["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(b["version"], env!("CARGO_PKG_VERSION"));
    assert!(ui["url"].as_str().unwrap().starts_with("http://127.0.0.1:"));
    assert!(
        tokio::time::timeout(Duration::from_secs(3), old.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let current_pid = r.peer().await;
    assert_ne!(current_pid, old_pid);
    assert_eq!(
        r.current(&["status", id]).await["run_id"],
        started["job"]["run_id"]
    );
    assert_eq!(r.current(&["status", id]).await["status"], "running");
    // A still-loaded older client cannot downgrade the new daemon.
    assert_eq!(
        r.cli(&r.old, &["daemon", "start"]).await["version"],
        env!("CARGO_PKG_VERSION")
    );
    r.current(&["daemon", "start"]).await;
    assert_eq!(r.peer().await, current_pid);
    std::fs::write(r.dir.path().join("finish"), b"").unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while r.current(&["status", id]).await["status"] != "completed" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("launches")).unwrap(),
        "launch\n"
    );
    assert_eq!(r.current(&["logs", id]).await["text"], "ready\ndone\n");
}

#[tokio::test]
async fn incompatible_schema_upgrade_leaves_old_daemon_and_active_runner_untouched() {
    let r = Rig::new();
    let _old = r.old_daemon().await;
    let job = r.running_job().await;
    let pid = r.peer().await;
    let db = rusqlite::Connection::open(r.config.root.join("relay.sqlite3")).unwrap();
    db.pragma_update(None, "user_version", 9).unwrap();
    let output = r
        .command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
        .args(["daemon", "start"])
        .output()
        .await
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("old daemon was left running"));
    assert_eq!(r.peer().await, pid);
    assert_eq!(
        r.current(&["status", job["job"]["job_id"].as_str().unwrap()])
            .await["status"],
        "running"
    );
    assert_eq!(
        db.pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        9
    );
    db.pragma_update(None, "user_version", 10).unwrap();
}
