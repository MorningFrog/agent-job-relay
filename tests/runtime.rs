#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    storage::Store,
};
use serde_json::Value;
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};
struct Rig {
    dir: tempfile::TempDir,
    config: Config,
    daemon: Option<Child>,
}
impl Rig {
    async fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("rt-")
            .tempdir_in(common::root())
            .unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        let mut r = Self {
            dir,
            config,
            daemon: None,
        };
        r.boot().await;
        r
    }
    async fn boot(&mut self) {
        let c = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&self.config.root)
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.daemon = Some(c);
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if ipc::call(&self.config, Request::Doctor).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    fn crash(&mut self) {
        if let Some(mut c) = self.daemon.take() {
            c.kill().unwrap();
            c.wait().unwrap();
        }
    }
    async fn call(&self, r: Request) -> Value {
        ipc::call(&self.config, r).await.unwrap()
    }
    fn spec(&self, script: &str) -> StartSpec {
        StartSpec {
            command: CommandSpec::Shell {
                interpreter: "/bin/sh".into(),
                script: script.into(),
            },
            cwd: self.dir.path().to_path_buf(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
        }
    }
    async fn start(&self, script: &str) -> String {
        self.call(Request::Start {
            spec: self.spec(script),
        })
        .await["job"]["job_id"]
            .as_str()
            .unwrap()
            .into()
    }
    async fn state(&self, j: &str, states: &[&str]) -> Value {
        tokio::time::timeout(Duration::from_secs(12), async {
            loop {
                let v = self.call(Request::Status { job_id: j.into() }).await;
                if states.contains(&v["status"].as_str().unwrap()) {
                    break v;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap()
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(mut c) = self.daemon.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        // Cleanup only verified processes belonging to this isolated rig, including panic paths.
        if let Ok(db) = rusqlite::Connection::open(self.config.root.join("relay.sqlite3"))
            && let Ok(mut q) = db.prepare(
                "SELECT workload_pid,workload_identity,runner_pid,runner_identity FROM runs",
            )
            && let Ok(rows) = q.query_map([], |r| {
                Ok((
                    r.get::<_, Option<u32>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<u32>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
        {
            for (pid, identity, rpid, ridentity) in rows.flatten() {
                if let (Some(p), Some(i)) = (pid, identity)
                    && agent_job_relay::platform::identity(p).as_ref() == Some(&i)
                {
                    let _ = agent_job_relay::platform::signal_group(p, 9);
                }
                if let (Some(p), Some(i)) = (rpid, ridentity)
                    && agent_job_relay::platform::identity(p).as_ref() == Some(&i)
                {
                    unsafe {
                        libc::kill(p as i32, libc::SIGKILL);
                    }
                }
            }
        }
    }
}
#[tokio::test]
async fn output_exit_events_and_cursors() {
    let r = Rig::new().await;
    let j = r.start("printf 'hello'; printf 'error' >&2; exit 7").await;
    let v = r.state(&j, &["failed"]).await;
    assert_eq!(v["exit_code"], 7);
    let a = r
        .call(Request::Logs {
            job_id: j.clone(),
            stream: "stdout".into(),
            cursor: 0,
            limit: 3,
        })
        .await;
    assert_eq!(a["text"], "hel");
    assert_eq!(a["next_cursor"], 3);
    let b = r
        .call(Request::Logs {
            job_id: j.clone(),
            stream: "stdout".into(),
            cursor: 3,
            limit: 8,
        })
        .await;
    assert_eq!(b["text"], "lo");
    let e = r
        .call(Request::Logs {
            job_id: j.clone(),
            stream: "stderr".into(),
            cursor: 0,
            limit: 8,
        })
        .await;
    assert_eq!(e["text"], "error");
    let events = r
        .call(Request::Events {
            job_id: j.clone(),
            after: 0,
            limit: 100,
        })
        .await;
    assert_eq!(events["events"].as_array().unwrap().len(), 2);
    let again = r
        .call(Request::Events {
            job_id: j,
            after: events["next_cursor"].as_i64().unwrap(),
            limit: 100,
        })
        .await;
    assert_eq!(again["events"], serde_json::json!([]));
    let ok = r.start("printf '\\377\\000'; exit 0").await;
    r.state(&ok, &["completed"]).await;
    let v = r
        .call(Request::Logs {
            job_id: ok,
            stream: "stdout".into(),
            cursor: 0,
            limit: 10,
        })
        .await;
    assert_eq!(v["bytes"], 2);
}
#[tokio::test]
async fn duplicate_start_and_conflicting_key() {
    let r = Rig::new().await;
    let mut s = r.spec("echo executed >> count; sleep 0.2");
    s.idempotency_key = Some("same".into());
    let (a, b) = tokio::join!(
        ipc::call(&r.config, Request::Start { spec: s.clone() }),
        ipc::call(&r.config, Request::Start { spec: s.clone() })
    );
    assert_eq!(
        a.as_ref().unwrap()["job"]["job_id"],
        b.unwrap()["job"]["job_id"]
    );
    let j = a.unwrap()["job"]["job_id"].as_str().unwrap().to_string();
    r.state(&j, &["completed"]).await;
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("count")).unwrap(),
        "executed\n"
    );
    s.timeout_seconds = Some(5);
    assert!(
        ipc::call(&r.config, Request::Start { spec: s })
            .await
            .unwrap_err()
            .to_string()
            .contains("different specification")
    );
}
#[tokio::test]
async fn daemon_crash_does_not_own_runner_or_completion() {
    let mut r = Rig::new().await;
    let j = r.start("sleep 0.5; echo survived").await;
    r.state(&j, &["running"]).await;
    r.crash();
    let store = Store::new(r.config.clone());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if store.status(j.clone()).await.unwrap().status == "completed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    r.boot().await;
    assert_eq!(
        r.call(Request::Status { job_id: j.clone() }).await["status"],
        "completed"
    );
    let e = r
        .call(Request::Events {
            job_id: j,
            after: 0,
            limit: 100,
        })
        .await;
    assert_eq!(
        e["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "completed")
            .count(),
        1
    );
}
#[tokio::test]
async fn runner_crash_is_orphaned_and_never_success() {
    let r = Rig::new().await;
    let j = r.start("sleep 20").await;
    let v = r.state(&j, &["running"]).await;
    let pid = v["runner_pid"].as_u64().unwrap();
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    let v = r.state(&j, &["orphaned"]).await;
    assert!(v["exit_code"].is_null());
    assert!(v["detail"].as_str().unwrap().contains("not replayed"));
}
#[tokio::test]
async fn cancellation_and_timeout_kill_descendants_only() {
    let r = Rig::new().await;
    let j = r.start("sleep 20 & echo $! > grandchild; wait").await;
    r.state(&j, &["running"]).await;
    let pid: u32 = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(r.dir.path().join("grandchild"))
                && let Ok(pid) = text.trim().parse::<u32>()
            {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    r.call(Request::Stop { job_id: j.clone() }).await;
    r.state(&j, &["cancelled"]).await;
    assert!(agent_job_relay::platform::identity(pid).is_none());
    assert_eq!(
        r.call(Request::Stop { job_id: j }).await["status"],
        "cancelled"
    );
    let mut s = r.spec("sleep 20");
    s.timeout_seconds = Some(1);
    let v = r.call(Request::Start { spec: s }).await;
    let j = v["job"]["job_id"].as_str().unwrap();
    r.state(j, &["timeout"]).await;
    assert!(agent_job_relay::platform::identity(std::process::id()).is_some());
}
#[tokio::test]
async fn uncertain_launch_never_replays_and_delayed_runner_is_fenced() {
    let mut r = Rig::new().await;
    r.crash();
    let store = Store::new(r.config.clone());
    let job = store.start(r.spec("echo bad > unexpected")).await.unwrap();
    let j = job.job_id.clone();
    let nonce = store
        .call(move |db| {
            db.execute(
                "UPDATE jobs SET status='launching',updated=0 WHERE id=?",
                [&j],
            )?;
            Ok(
                db.query_row("SELECT nonce FROM runs WHERE job_id=?", [&j], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .await
        .unwrap();
    r.boot().await;
    r.state(&job.job_id, &["orphaned"]).await;
    assert!(
        agent_job_relay::runner::run(r.config.clone(), job.job_id, nonce)
            .await
            .is_err()
    );
    assert!(!r.dir.path().join("unexpected").exists());
}
#[tokio::test]
async fn log_cap_security_and_bad_frames() {
    use std::os::unix::fs::PermissionsExt;
    let r = Rig::new().await;
    let j = r.start("head -c 18000000 /dev/zero").await;
    let v = r.state(&j, &["completed"]).await;
    assert!(
        v["detail"]
            .as_str()
            .unwrap()
            .contains("stdout_truncated=true")
    );
    assert_eq!(
        std::fs::metadata(
            r.config
                .run_dir(v["run_id"].as_str().unwrap())
                .join("stdout.log")
        )
        .unwrap()
        .len(),
        agent_job_relay::runner::LOG_CAP
    );
    assert_eq!(
        std::fs::metadata(r.config.socket())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(&r.config.root)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    use tokio::io::AsyncWriteExt;
    let mut s = tokio::net::UnixStream::connect(r.config.socket())
        .await
        .unwrap();
    s.write_u32_le(200000).await.unwrap();
    assert!(ipc::read_frame(&mut s, 1024).await.is_err());
    assert!(
        ipc::call(
            &r.config,
            Request::Logs {
                job_id: j,
                stream: "../../bad".into(),
                cursor: 0,
                limit: 1
            }
        )
        .await
        .is_err()
    );
    assert_eq!(r.call(Request::Doctor).await["store"], "ok");
}

#[tokio::test]
async fn real_stdio_mcp_exit_leaves_background_job_running() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let r = Rig::new().await;
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    async fn send(input: &mut tokio::process::ChildStdin, v: Value) {
        input.write_all(format!("{v}\n").as_bytes()).await.unwrap();
        input.flush().await.unwrap();
    }
    async fn response(
        lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    ) -> Value {
        serde_json::from_str(
            &tokio::time::timeout(Duration::from_secs(5), lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }
    send(&mut input,serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"isolated-test","version":"1"}}})).await;
    let initialized = response(&mut lines).await;
    assert!(initialized.get("result").is_some(), "{initialized}");
    send(
        &mut input,
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    send(
        &mut input,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )
    .await;
    let tools = response(&mut lines).await;
    assert!(
        tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "job_start")
    );
    send(&mut input,serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"job_start","arguments":{"spec":r.spec("sleep 0.5; echo mcp-survived")}}})).await;
    let result = response(&mut lines).await;
    assert_ne!(result["result"]["isError"], true, "{result}");
    let job: Value =
        serde_json::from_str(result["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let j = job["job"]["job_id"].as_str().unwrap();
    drop(input);
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    r.state(j, &["completed"]).await;
}

#[tokio::test]
async fn output_write_failure_stops_workload_and_is_diagnostic() {
    let mut r = Rig::new().await;
    r.crash();
    let store = Store::new(r.config.clone());
    let job = store.start(r.spec("sleep 20")).await.unwrap();
    agent_job_relay::platform::secure_dir(&r.config.run_dir(&job.run_id)).unwrap();
    std::fs::create_dir(r.config.run_dir(&job.run_id).join("stdout.log")).unwrap();
    r.boot().await;
    let v = r.state(&job.job_id, &["failed"]).await;
    assert!(
        v["detail"]
            .as_str()
            .unwrap()
            .contains("output persistence failed")
    );
    if let Some(p) = v["workload_pid"].as_u64() {
        assert!(agent_job_relay::platform::identity(p as u32).is_none());
    }
}
#[tokio::test]
async fn concurrent_jobs_distinct_workdirs_quota_and_pagination() {
    let mut r = Rig::new().await;
    r.crash();
    let store = Store::new(r.config.clone());
    let mut jobs = vec![];
    for i in 0..8 {
        let dir = r.dir.path().join(format!("work-{i}"));
        std::fs::create_dir(&dir).unwrap();
        let mut s = r.spec("pwd; sleep 0.1");
        s.cwd = dir;
        jobs.push(store.start(s).await.unwrap());
    }
    assert!(
        store
            .start(r.spec("true"))
            .await
            .unwrap_err()
            .to_string()
            .contains("quota")
    );
    let first = store.list(0, 3).await.unwrap();
    let second = store.list(first.last().unwrap().sequence, 3).await.unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(second.len(), 3);
    assert_ne!(first[0].job_id, second[0].job_id);
    r.boot().await;
    for (i, j) in jobs.iter().enumerate() {
        r.state(&j.job_id, &["completed"]).await;
        let log = r
            .call(Request::Logs {
                job_id: j.job_id.clone(),
                stream: "stdout".into(),
                cursor: 0,
                limit: 8192,
            })
            .await;
        assert!(log["text"].as_str().unwrap().contains(&format!("work-{i}")));
    }
}
#[tokio::test]
async fn explicit_environment_is_not_returned_and_invalid_specs_are_rejected() {
    let r = Rig::new().await;
    let mut s = r.spec("true");
    s.env
        .insert("TEST_SECRET".into(), "sensitive-test-value".into());
    let v = r.call(Request::Start { spec: s }).await;
    assert!(!v.to_string().contains("sensitive-test-value"));
    let j = v["job"]["job_id"].as_str().unwrap();
    let v = r.state(j, &["completed"]).await;
    assert!(!v.to_string().contains("sensitive-test-value"));
    let mut s = r.spec("true");
    s.cwd = std::path::PathBuf::from("relative");
    assert!(
        ipc::call(&r.config, Request::Start { spec: s })
            .await
            .is_err()
    );
    assert!(
        ipc::call(
            &r.config,
            Request::Wait {
                job_id: j.into(),
                after: 0,
                seconds: 31
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn daemon_graceful_shutdown_keeps_runner_and_restart_dispatches_outbox() {
    use agent_job_relay::session::{Adapter, Registration};
    let mut r = Rig::new().await;
    let registered = r
        .call(Request::RegisterSession {
            registration: Registration {
                agent_kind: "codex_cli".into(),
                session_id: "same-thread".into(),
                instance_id: "same-instance".into(),
                host_id: "test".into(),
                adapter: Adapter::Inbox,
                endpoint: None,
                caller_thread_id: None,
                lease_seconds: 3600,
                experimental: false,
                renew_on_delivery: false,
            },
        })
        .await;
    let key = registered["session"]["key"].as_str().unwrap().to_string();
    let mut spec = r.spec("while [ ! -f release ]; do sleep 0.02; done; echo survived");
    spec.session = Some(key.clone());
    let v = r.call(Request::Start { spec }).await;
    let j = v["job"]["job_id"].as_str().unwrap().to_string();
    r.state(&j, &["running"]).await;
    // Queue reconciliation behind a real SQLite writer, then interrupt while
    // the daemon is awaiting that work rather than polling its select loop.
    let db = rusqlite::Connection::open(r.config.root.join("relay.sqlite3")).unwrap();
    db.busy_timeout(Duration::from_secs(5)).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    let daemon_pid = r.daemon.as_ref().unwrap().id();
    assert_eq!(unsafe { libc::kill(daemon_pid as i32, libc::SIGSTOP) }, 0);
    db.execute_batch("ROLLBACK").unwrap();
    Store::new(r.config.clone())
        .start(r.spec("true"))
        .await
        .unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert_eq!(unsafe { libc::kill(daemon_pid as i32, libc::SIGCONT) }, 0);
    // Reconciliation may wait for the writer; control reads remain available.
    tokio::time::timeout(
        Duration::from_millis(800),
        ipc::call(&r.config, Request::Doctor),
    )
    .await
    .unwrap()
    .unwrap();
    unsafe {
        assert_eq!(libc::kill(daemon_pid as i32, libc::SIGINT), 0);
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    db.execute_batch("ROLLBACK").unwrap();
    let daemon = r.daemon.as_mut().unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if daemon.try_wait().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    r.daemon.take();
    std::fs::write(r.dir.path().join("release"), b"ready").unwrap();
    let store = Store::new(r.config.clone());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if terminal(&store.status(j.clone()).await.unwrap().status) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    r.boot().await;
    let messages = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let v = r
                .call(Request::Inbox {
                    session_key: key.clone(),
                    after: 0,
                    limit: 20,
                })
                .await;
            if v["messages"][0]["status"] == "notification_recorded" {
                break v;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(messages["messages"][0]["job_id"], j);
}
#[tokio::test]
async fn stale_pid_identity_is_diagnostic_and_does_not_kill_unrelated_process() {
    let mut r = Rig::new().await;
    r.crash();
    let store = Store::new(r.config.clone());
    let job = store.start(r.spec("true")).await.unwrap();
    let j = job.job_id.clone();
    store.call(move|db|{db.execute("UPDATE jobs SET status='running' WHERE id=?",[&j])?;db.execute("UPDATE runs SET runner_pid=?,runner_identity='stale-boot:start-time',workload_pid=?,workload_identity='stale' WHERE job_id=?",rusqlite::params![std::process::id(),std::process::id(),j])?;Ok(())}).await.unwrap();
    r.boot().await;
    r.state(&job.job_id, &["orphaned"]).await;
    assert!(agent_job_relay::platform::identity(std::process::id()).is_some());
}

#[tokio::test]
async fn control_requests_remain_responsive_during_reconciliation_write_contention() {
    let mut r = Rig::new().await;
    let store = Store::new(r.config.clone());
    let job = store.start(r.spec("exit 0")).await.unwrap();
    let db = rusqlite::Connection::open(r.config.root.join("relay.sqlite3")).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut writers = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let config = r.config.clone();
        let spec = r.spec("exit 0");
        writers.spawn(async move { ipc::call(&config, Request::Start { spec }).await });
    }
    // Multiple requests span the reconciliation interval while its launch CAS
    // cannot acquire the writer. Neither read diagnostics nor shutdown need it.
    for _ in 0..4 {
        tokio::time::timeout(
            Duration::from_millis(800),
            ipc::call(&r.config, Request::Doctor),
        )
        .await
        .unwrap()
        .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let result = tokio::time::timeout(
        Duration::from_millis(800),
        ipc::call(&r.config, Request::Shutdown),
    )
    .await;
    assert_eq!(result.unwrap().unwrap()["daemon"], "stopping");
    let mut daemon = r.daemon.take().unwrap();
    let exit = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if daemon.try_wait().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    db.execute_batch("ROLLBACK").unwrap();
    writers.shutdown().await;
    if exit.is_err() {
        daemon.kill().unwrap();
        daemon.wait().unwrap();
    }
    exit.expect(
        "shutdown must discard queued writer requests while the external writer remains held",
    );
    // A launch already claimed during orderly shutdown still has its independent runner.
    r.boot().await;
    r.state(&job.job_id, &["completed", "orphaned"]).await;
}
