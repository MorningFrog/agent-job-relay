#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{ipc::Request, storage::Store};
use common::runtime::Rig;
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};

async fn occupied(r: &Rig) -> Vec<String> {
    let specs = (0..8)
        .map(|index| {
            let mut spec = r.spec("echo ready; while [ ! -f release ]; do sleep 0.02; done");
            spec.idempotency_key = Some(format!("occupy-{index}"));
            spec.timeout_seconds = Some(60);
            spec
        })
        .collect();
    let batch = r.call(Request::StartBatch { specs }).await;
    assert_eq!(batch["accepted"], 8);
    let ids: Vec<String> = batch["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["result"]["job"]["job_id"].as_str().unwrap().into())
        .collect();
    for id in &ids {
        r.state(id, &["running"]).await;
    }
    ids
}

#[tokio::test]
async fn queued_native_jobs_resume_fifo_after_restart_without_replay_or_early_timeouts() {
    let mut r = Rig::new().await;
    let blockers = occupied(&r).await;
    let specs: Vec<_> = (0..4).map(|index| {
        let mut spec = r.spec(&format!("echo {index} >> order; echo started; while [ ! -f release-q{index} ]; do sleep 0.02; done"));
        spec.idempotency_key = Some(format!("waiting-{index}"));
        spec.queue_timeout_seconds = Some(60);
        spec.timeout_seconds = Some(10);
        spec
    }).collect();
    let batch = r
        .call(Request::StartBatch {
            specs: specs.clone(),
        })
        .await;
    assert_eq!(batch["accepted"], 4);
    let jobs: Vec<_> = batch["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["result"]["job"].clone())
        .collect();
    for (index, job) in jobs.iter().enumerate() {
        assert_eq!(job["status"], "queued");
        assert_eq!(job["queue"]["position"], index + 1);
        assert!(job["run"]["started_at"].is_null());
    }
    assert!(!r.dir.path().join("order").exists());
    let health = r.call(Request::Doctor).await;
    assert_eq!(health["bounded_queue"]["waiting_limit"], 64);
    assert_eq!(health["queued_jobs"], 4);
    assert_eq!(health["active_jobs"], 12);
    r.crash();
    let store = Store::new(r.config.clone());
    // Simulate time spent in the durable queue. Execution's ten seconds have
    // not begun, and a fresh retry must retain the original queue deadline.
    let waiting = jobs[0]["job_id"].as_str().unwrap().to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE jobs SET created=created-20000 WHERE id=?",
                [waiting],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    r.boot().await;
    let retry = r.call(Request::StartBatch { specs }).await;
    for (index, job) in jobs.iter().enumerate() {
        assert_eq!(
            retry["results"][index]["result"]["job"]["job_id"],
            job["job_id"]
        );
        assert_eq!(
            retry["results"][index]["result"]["job"]["run_id"],
            job["run_id"]
        );
        if index != 0 {
            assert_eq!(
                retry["results"][index]["result"]["job"]["queue"]["expires_at"],
                job["queue"]["expires_at"]
            );
        }
    }
    r.call(Request::Stop {
        job_id: blockers[0].clone(),
        grace_seconds: 0,
        wait_seconds: 10,
    })
    .await;
    let first = jobs[0]["job_id"].as_str().unwrap();
    r.state(first, &["running"]).await;
    for job in &jobs[1..] {
        r.state(job["job_id"].as_str().unwrap(), &["queued"]).await;
    }
    let cancelled = jobs[1]["job_id"].as_str().unwrap();
    r.call(Request::Stop {
        job_id: cancelled.into(),
        grace_seconds: 0,
        wait_seconds: 10,
    })
    .await;
    assert!(r.state(cancelled, &["cancelled"]).await["run"]["started_at"].is_null());
    let expired = jobs[2]["job_id"].as_str().unwrap().to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE jobs SET created=created-61000 WHERE id=?",
                [expired],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let expired = r
        .state(jobs[2]["job_id"].as_str().unwrap(), &["timeout"])
        .await;
    assert!(expired["run"]["started_at"].is_null());
    assert!(
        expired["detail"]
            .as_str()
            .unwrap()
            .contains("queue deadline")
    );
    std::fs::write(r.dir.path().join("release-q0"), "").unwrap();
    r.state(first, &["completed"]).await;
    let last = jobs[3]["job_id"].as_str().unwrap();
    r.state(last, &["running"]).await;
    std::fs::write(r.dir.path().join("release-q3"), "").unwrap();
    r.state(last, &["completed"]).await;
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("order")).unwrap(),
        "0\n3\n"
    );
    for job in [first, last] {
        let events = store.events(job.into(), 0, 100).await.unwrap();
        assert_eq!(events.iter().filter(|e| e.kind == "started").count(), 1);
        assert_eq!(events.iter().filter(|e| e.kind == "completed").count(), 1);
    }
    std::fs::write(r.dir.path().join("release"), "").unwrap();
    for id in &blockers[1..] {
        r.state(id, &["completed"]).await;
    }
}

#[tokio::test]
async fn cli_and_shared_mcp_report_queued_admission_and_cancel_without_starting() {
    let r = Rig::new().await;
    occupied(&r).await;
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["start", "--timeout", "1", "--shell", "touch must-not-run"])
        .current_dir(r.dir.path())
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cli: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(cli["job"]["status"], "queued");
    assert_eq!(cli["job"]["queue"]["timeout_seconds"], 86400);
    let refused = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["start", "--no-queue", "--shell", "touch must-not-run"])
        .current_dir(r.dir.path())
        .output()
        .await
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("waiting was explicitly disabled"));
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["mcp", "--shared-host"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut result = Value::Null;
    for (id, method, params) in [
        (
            1,
            "initialize",
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"queue-test","version":"1"}}),
        ),
        (
            2,
            "tools/call",
            json!({"name":"job_start","arguments":{"spec":{"command":{"kind":"exec","program":"must-not-execute"},"cwd":r.dir.path(),"idempotency_key":"mcp-queue"}}}),
        ),
    ] {
        input
            .write_all(
                format!(
                    "{}\n",
                    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let line = tokio::time::timeout(Duration::from_secs(8), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert!(reply.get("error").is_none(), "{reply}");
        assert_ne!(reply["result"]["isError"], true, "{reply}");
        if id == 2 {
            result = serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        }
    }
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    assert_eq!(result["job"]["status"], "queued");
    assert_eq!(result["job"]["queue"]["position"], 2);
    assert_eq!(result["job"]["queue"]["timeout_seconds"], 86400);
    for value in [cli, result] {
        let id = value["job"]["job_id"].as_str().unwrap();
        let stopped = r
            .call(Request::Stop {
                job_id: id.into(),
                grace_seconds: 0,
                wait_seconds: 10,
            })
            .await;
        assert_eq!(stopped["status"], "cancelled");
        assert!(stopped["run"]["started_at"].is_null());
    }
    assert!(!r.dir.path().join("must-not-run").exists());
    std::fs::write(r.dir.path().join("release"), "").unwrap();
}
