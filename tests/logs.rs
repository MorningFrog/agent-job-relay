#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{archive, config::Config, ipc::Request, maintenance::Policy, storage::Store};
use common::runtime::Rig;
use serde_json::json;
const MIB: u64 = 1024 * 1024;

#[tokio::test]
async fn native_rotation_preserves_absolute_cursors_gaps_archives_and_restored_backups() {
    let r = Rig::new().await;
    let job=r.start("python3 -c 'import sys;sys.stdout.buffer.write(b\"P\"*(16*1024*1024));[sys.stdout.buffer.write(bytes([65+i])*1024*1024) for i in range(8)];sys.stdout.buffer.write(b\"FINAL\");sys.stdout.flush()'").await;
    let done = r.state(&job, &["completed", "failed"]).await;
    assert_eq!(done["status"], "completed", "{done}");
    let run = done["run_id"].as_str().unwrap().to_owned();
    let read =
        |config: Config, cursor| archive::logs(config, run.clone(), "stdout".into(), cursor, 32);
    let prefix = read(r.config.clone(), 0).await.unwrap();
    assert_eq!(prefix["text"], "P".repeat(32));
    let gap = read(r.config.clone(), 16 * MIB).await.unwrap();
    assert_eq!(gap["cursor"], 21 * MIB);
    assert_eq!(gap["skipped"], json!({"from":16*MIB,"to":21*MIB}));
    assert_eq!(gap["text"], "F".repeat(32));
    assert_eq!(
        read(r.config.clone(), 24 * MIB).await.unwrap()["text"],
        "FINAL"
    );
    assert!(
        r.config
            .run_dir(&run)
            .join("stdout.recent")
            .metadata()
            .unwrap()
            .len()
            <= 4 * MIB + 256
    );
    let store = Store::new(r.config.clone());
    let backup = store.backup().await.unwrap();
    let restored = r.dir.path().join("restored");
    agent_job_relay::maintenance::restore(
        backup["path"].as_str().unwrap().into(),
        restored.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        read(Config::new(Some(restored)).unwrap(), 16 * MIB)
            .await
            .unwrap(),
        gap
    );
    store
        .data_policy(
            Some(Policy {
                archive_after_days: 0,
                ..Default::default()
            }),
            true,
        )
        .await
        .unwrap();
    store.maintain(true).await.unwrap();
    assert!(!r.config.run_dir(&run).join("stdout.recent").exists());
    assert!(r.config.run_dir(&run).join("stdout.recent.gz").exists());
    assert_eq!(read(r.config.clone(), 16 * MIB).await.unwrap(), gap);
    assert_eq!(
        read(r.config.clone(), 24 * MIB).await.unwrap()["text"],
        "FINAL"
    );
    // Export uses exactly the retained text and marks the missing original range.
    let output = r.dir.path().join("export.txt");
    let exported = cli(&r, &["logs", &job, "--export", output.to_str().unwrap()]);
    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let text = std::fs::read_to_string(&output).unwrap();
    assert!(text.contains("AgentJobRelay missing bytes 16777216..22020096"));
    assert!(text.ends_with("FINAL"));
    assert!(
        !cli(&r, &["logs", &job, "--export", output.to_str().unwrap()])
            .status
            .success()
    );
    let before = r.call(Request::Status { job_id: job }).await;
    assert_eq!(before["run_id"], run);
}

fn cli(r: &Rig, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(args)
        .output()
        .unwrap()
}
