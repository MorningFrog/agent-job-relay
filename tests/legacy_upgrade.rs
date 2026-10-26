#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    ipc::{self, Request},
    platform,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::Duration,
};

fn fixture() -> &'static Path {
    static DIRECTORY: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let dir = tempfile::Builder::new()
                .prefix("legacy-bin-")
                .tempdir_in(common::root())
                .unwrap();
            let result = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "legacy_fixture"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy-daemon.rs"))
                .arg("-o")
                .arg(dir.path().join("agent-job-relay"))
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            dir
        })
        .path()
}
struct Legacy {
    child: std::process::Child,
}
impl Drop for Legacy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn cli() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"));
    c.env_remove("AJR_STATE_DIR")
        .env_remove("CODEX_THREAD_ID")
        .env_remove("CODEX_APP_TOOLS_PIPE_PATH");
    c
}
fn success(out: std::process::Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
async fn launch(state: &Path, executable: &Path) -> Legacy {
    platform::secure_dir(state).unwrap();
    let child = Command::new(executable)
        .arg("daemon")
        .arg("--state-dir")
        .arg(state)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let legacy = Legacy { child };
    let config = Config::new(Some(state.into())).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while ipc::call(&config, Request::Doctor).await.is_err() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    legacy
}
#[tokio::test]
async fn installed_legacy_stop_keeps_profile_and_history_and_allows_immediate_upgrade() {
    let dir = tempfile::Builder::new()
        .prefix("upgrade-")
        .tempdir_in(common::root())
        .unwrap();
    let state = dir.path().join("original-state");
    let install = dir.path().join("local install's bundle");
    success(
        cli()
            .arg("--state-dir")
            .arg(&state)
            .args(["install", "--root"])
            .arg(&install)
            .arg("--apply")
            .output()
            .unwrap(),
    );
    // An owned older native image is replaced by the verified new installer.
    let executable = install.join("bin/agent-job-relay");
    std::fs::copy(fixture().join("agent-job-relay"), &executable).unwrap();
    let receipt_path = install.join(".agent-job-relay-install.json");
    let mut receipt: Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
    receipt["files"]["bin/agent-job-relay"] = json!(format!(
        "{:x}",
        Sha256::digest(std::fs::read(&executable).unwrap())
    ));
    // Keep both ownership records coherent when constructing the older-image fixture.
    let profile_path = install.join(".agent-job-relay-runtime.json");
    let mut profile: Value =
        serde_json::from_slice(&std::fs::read(&profile_path).unwrap()).unwrap();
    profile["files"]["bin/agent-job-relay"] = receipt["files"]["bin/agent-job-relay"].clone();
    std::fs::write(&profile_path, serde_json::to_vec(&profile).unwrap()).unwrap();
    receipt["files"][".agent-job-relay-runtime.json"] = json!(format!(
        "{:x}",
        Sha256::digest(std::fs::read(profile_path).unwrap())
    ));
    std::fs::write(receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let mut legacy = launch(&state, &executable).await;
    std::fs::write(state.join("history"), "keep all previous results").unwrap();
    let config = Config::new(Some(state.clone())).unwrap();
    assert!(
        platform::stop_legacy_daemon(&state, legacy.child.id(), "stale-process-identity")
            .await
            .is_err()
    );
    assert!(legacy.child.try_wait().unwrap().is_none());
    let blocked = cli()
        .args(["install", "--root"])
        .arg(&install)
        .arg("--apply")
        .output()
        .unwrap();
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("daemon stop --install-root"));
    let foreign_state = dir.path().join("wrong-exported-state");
    let stopped = success(
        cli()
            .env("AJR_STATE_DIR", &foreign_state)
            .args(["daemon", "stop", "--install-root"])
            .arg(&install)
            .output()
            .unwrap(),
    );
    assert_eq!(stopped["method"], "legacy_sigint");
    assert_eq!(stopped["state_dir"], state.to_str().unwrap());
    assert_eq!(stopped["daemon"], "stopped");
    assert!(legacy.child.wait().unwrap().success());
    let upgraded = success(
        cli()
            .args(["install", "--root"])
            .arg(&install)
            .arg("--apply")
            .output()
            .unwrap(),
    );
    assert_eq!(upgraded["changed"], true);
    assert_eq!(upgraded["state_dir"], state.to_str().unwrap());
    assert!(!foreign_state.exists());
    assert_eq!(
        std::fs::read_to_string(state.join("history")).unwrap(),
        "keep all previous results"
    );
    assert!(!config.socket().exists());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_shutdown_never_signals_an_unrelated_socket_peer() {
    let dir = tempfile::Builder::new()
        .prefix("peer-")
        .tempdir_in(common::root())
        .unwrap();
    let state = dir.path().join("state");
    platform::secure_dir(&state).unwrap();
    let listener = tokio::net::UnixListener::bind(state.join("relay.sock")).unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        ipc::read_frame(&mut stream, 131072).await.unwrap();
        ipc::write_frame(&mut stream, &json!({"protocol_version":1,"ok":false,"error":"unknown variant `shutdown`, expected one of `start`, `doctor`"})).await.unwrap();
        let (_stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    let result = cli()
        .arg("--state-dir")
        .arg(&state)
        .args(["daemon", "stop"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("not a recognized AgentJobRelay executable")
    );
    assert!(platform::identity(std::process::id()).is_some());
    task.await.unwrap();
}
#[tokio::test]
async fn other_shutdown_errors_do_not_trigger_legacy_signals() {
    let dir = tempfile::Builder::new()
        .prefix("deny-")
        .tempdir_in(common::root())
        .unwrap();
    let state: PathBuf = dir.path().join("state");
    let mut legacy = launch(&state, &fixture().join("agent-job-relay")).await;
    std::fs::write(state.join("reject"), "deny").unwrap();
    let result = cli()
        .arg("--state-dir")
        .arg(&state)
        .args(["daemon", "stop"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("shutdown permission denied"));
    assert!(legacy.child.try_wait().unwrap().is_none());
}
