mod common;
use agent_job_relay::{
    archive,
    config::Config,
    core::*,
    delivery::{DeliveryError, FailureKind},
    maintenance::{self, Policy},
    session::{Adapter, Registration},
    storage::Store,
};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf};

struct Fixture {
    dir: tempfile::TempDir,
    store: Store,
}
impl Fixture {
    async fn new() -> Self {
        agent_job_relay::platform::initialize();
        let dir = tempfile::tempdir_in(common::root()).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config);
        store.init().await.unwrap();
        Self { dir, store }
    }
    fn spec(&self, key: &str, session: Option<String>) -> StartSpec {
        StartSpec {
            command: CommandSpec::Exec {
                program: "must-never-execute".into(),
                args: vec![],
            },
            cwd: self.dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session,
            idempotency_key: Some(key.into()),
            monitor_rules: vec![],
        }
    }
    async fn job(&self, key: &str, session: Option<String>, finish: bool) -> Job {
        let job = self.store.start(self.spec(key, session)).await.unwrap();
        if finish {
            self.store
                .finish(
                    job.job_id.clone(),
                    "completed",
                    Some(0),
                    None,
                    "done".into(),
                )
                .await
                .unwrap();
        }
        job
    }
    async fn session(&self, logical: &str) -> String {
        self.store
            .register(Registration {
                agent_kind: "codex_cli".into(),
                session_id: logical.into(),
                instance_id: "one".into(),
                host_id: "test".into(),
                adapter: Adapter::Inbox,
                endpoint: None,
                caller_thread_id: None,
                lease_seconds: 3600,
                experimental: false,
                renew_on_delivery: false,
            })
            .await
            .unwrap()
            .key
    }
    async fn immediate(&self) {
        self.store
            .data_policy(
                Some(Policy {
                    archive_after_days: 0,
                    ..Default::default()
                }),
                true,
            )
            .await
            .unwrap();
    }
    fn log(&self, job: &Job, bytes: &[u8]) {
        let dir = self.store.config.run_dir(&job.run_id);
        agent_job_relay::platform::secure_dir(&dir).unwrap();
        let mut f = agent_job_relay::platform::private_file(&dir.join("stdout.log")).unwrap();
        f.write_all(bytes).unwrap();
        f.sync_all().unwrap();
    }
    async fn count(&self, table: &str) -> i64 {
        let table = table.to_string();
        self.store
            .read(move |db| {
                Ok(db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
            })
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn automatic_archive_preserves_history_logs_idempotency_and_monotonic_cursors() {
    let f = Fixture::new().await;
    let session = f.session("history").await;
    let a = f.job("a", Some(session.clone()), true).await;
    let claim = f.store.claim().await.unwrap().unwrap();
    let delivery = claim.delivery.clone();
    f.store
        .complete_claim(claim, Ok("notification_recorded"))
        .await
        .unwrap();
    let b = f.job("b", None, true).await;
    let bytes = (0..65536).map(|n| (n % 256) as u8).collect::<Vec<_>>();
    f.log(&a, &bytes);
    let events = f.store.events(a.job_id.clone(), 0, 100).await.unwrap();
    f.immediate().await;
    let result = f.store.maintain(true).await.unwrap();
    assert_eq!(result["archive"]["archived"], 2);
    assert_eq!(f.count("jobs").await, 0);
    assert_eq!(f.count("archive_jobs").await, 2);
    assert!(
        !f.store
            .config
            .run_dir(&a.run_id)
            .join("stdout.log")
            .exists()
    );
    assert!(
        f.store
            .config
            .run_dir(&a.run_id)
            .join("stdout.log.gz")
            .exists()
    );
    assert_eq!(
        f.store
            .status(a.job_id.clone())
            .await
            .unwrap()
            .run
            .exit_code,
        Some(0)
    );
    assert_eq!(
        serde_json::to_value(f.store.events(a.job_id.clone(), 0, 100).await.unwrap()).unwrap(),
        serde_json::to_value(events).unwrap()
    );
    let page = f.store.list(0, 1).await.unwrap();
    assert_eq!(page[0].job_id, a.job_id);
    let page = f.store.list(page[0].sequence, 1).await.unwrap();
    assert_eq!(page[0].job_id, b.job_id);
    assert_eq!(
        f.store
            .start(f.spec("a", Some(session.clone())))
            .await
            .unwrap()
            .job_id,
        a.job_id
    );
    let mut changed = f.spec("a", Some(session.clone()));
    changed.env.insert("CHANGED".into(), "yes".into());
    assert!(f.store.start(changed).await.is_err());
    let logs = archive::logs(
        f.store.config.clone(),
        a.run_id.clone(),
        "stdout".into(),
        255,
        8192,
    )
    .await
    .unwrap();
    assert_eq!(logs["bytes"], 8192);
    assert_eq!(logs["next_cursor"], 8447);
    assert_eq!(
        logs["text"],
        String::from_utf8_lossy(&bytes[255..8447]).as_ref()
    );
    assert!(
        archive::logs(
            f.store.config.clone(),
            a.run_id.clone(),
            "stdout".into(),
            65537,
            10
        )
        .await
        .is_err()
    );
    let inbox = f.store.inbox(session, 0, 100).await.unwrap();
    assert_eq!(inbox["messages"][0]["delivery_id"], delivery.id);
    assert_eq!(
        f.store.attempts(delivery.id.clone(), 0, 100).await.unwrap()["attempts"][0]["outcome"],
        "notification_recorded"
    );
    assert_eq!(
        f.store
            .deliveries(Some(a.job_id.clone()), 0, 100)
            .await
            .unwrap()["deliveries"][0]["sequence"],
        delivery.sequence
    );
    let next = f.job("next", None, false).await;
    assert!(next.sequence > b.sequence);
    assert_eq!(
        f.store.list(b.sequence, 10).await.unwrap()[0].job_id,
        next.job_id
    );
    assert!(f.store.data_check().await.is_ok());
}

#[tokio::test]
async fn archive_protects_pending_claims_and_recovery_then_restores_explicit_retry() {
    let f = Fixture::new().await;
    let key = f.session("pending").await;
    let a = f.job("leased", Some(key.clone()), true).await;
    let claim = f.store.claim().await.unwrap().unwrap();
    let b = f.job("pending", Some(key.clone()), true).await;
    let other = f.session("expired").await;
    let c = f.job("expired", Some(other), true).await;
    let old = f.store.claim().await.unwrap().unwrap();
    let delivery = old.delivery.clone();
    f.store
        .complete_claim(
            old.clone(),
            Err(DeliveryError(FailureKind::Permanent, "test refusal").into()),
        )
        .await
        .unwrap();
    f.immediate().await;
    f.store.maintain(true).await.unwrap(); // unexpired dead letters remain recoverable
    assert_eq!(f.count("archive_jobs").await, 0);
    let id = delivery.id.clone();
    f.store
        .call(move |db| {
            db.execute("UPDATE deliveries SET expires=0 WHERE id=?", [id])?;
            Ok(())
        })
        .await
        .unwrap();
    f.store.maintain(true).await.unwrap();
    assert_eq!(f.count("archive_jobs").await, 1);
    assert_eq!(
        f.store.status(c.job_id.clone()).await.unwrap().status,
        "completed"
    );
    let before = f.store.attempts(delivery.id.clone(), 0, 100).await.unwrap();
    assert!(
        !f.store
            .complete_claim(old, Ok("accepted_by_host"))
            .await
            .unwrap()
    );
    f.store.retry_delivery(delivery.id.clone()).await.unwrap();
    assert_eq!(f.count("archive_jobs").await, 0);
    assert_eq!(
        f.store.attempts(delivery.id.clone(), 0, 100).await.unwrap(),
        before
    );
    assert_eq!(
        f.store.deliveries(Some(c.job_id), 0, 100).await.unwrap()["deliveries"][0]["id"],
        delivery.id
    );
    assert!(
        f.store
            .complete_claim(claim, Ok("notification_recorded"))
            .await
            .unwrap()
    );
    assert_eq!(f.store.status(a.job_id).await.unwrap().status, "completed");
    assert_eq!(f.store.status(b.job_id).await.unwrap().status, "completed");
    assert!(f.store.data_check().await.is_ok());
}

#[tokio::test]
async fn archive_transaction_failure_and_post_commit_cleanup_are_recoverable() {
    let f = Fixture::new().await;
    let a = f.job("failure", None, true).await;
    f.log(&a, b"keep all output");
    f.immediate().await;
    f.store.call(|db|{db.execute_batch("CREATE TRIGGER fail_archive BEFORE DELETE ON jobs BEGIN SELECT RAISE(ABORT,'injected archive failure'); END;")?;Ok(())}).await.unwrap();
    let first = f.store.maintain(true).await.unwrap();
    assert_eq!(first["archive"]["deferred"], 1);
    assert_eq!(f.count("archive_jobs").await, 0);
    assert_eq!(f.count("jobs").await, 1);
    let dir = f.store.config.run_dir(&a.run_id);
    assert!(dir.join("stdout.log").exists());
    assert!(dir.join("stdout.log.gz").exists());
    f.store
        .call(|db| {
            db.execute_batch("DROP TRIGGER fail_archive")?;
            Ok(())
        })
        .await
        .unwrap();
    f.store.maintain(true).await.unwrap();
    // Reconstruct interruption after archive SQL commit, before unlinking raw logs.
    f.log(&a, b"keep all output");
    f.store
        .call(|db| {
            db.execute("UPDATE archive_jobs SET cleanup_pending=1", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let gz = std::fs::read(dir.join("stdout.log.gz")).unwrap();
    std::fs::write(dir.join("stdout.log.gz"), b"corrupt").unwrap();
    assert!(f.store.maintain(true).await.is_err());
    assert!(dir.join("stdout.log").exists());
    std::fs::write(dir.join("stdout.log.gz"), gz).unwrap();
    f.store.maintain(true).await.unwrap();
    assert!(!dir.join("stdout.log").exists());
    assert_eq!(
        archive::logs(f.store.config.clone(), a.run_id, "stdout".into(), 0, 100)
            .await
            .unwrap()["text"],
        "keep all output"
    );
}

#[tokio::test]
async fn count_and_log_budgets_archive_automatically_before_the_age_threshold() {
    for log_pressure in [false, true] {
        let f = Fixture::new().await;
        let a = f.job("older", None, true).await;
        let b = f.job("newer", None, true).await;
        if log_pressure {
            f.log(&a, &vec![b'a'; 768 * 1024]);
            f.log(&b, &vec![b'b'; 768 * 1024]);
        }
        f.store
            .data_policy(
                Some(Policy {
                    hot_jobs: if log_pressure { 1000 } else { 1 },
                    archive_after_days: 36500,
                    hot_log_mib: 1,
                    ..Default::default()
                }),
                true,
            )
            .await
            .unwrap();
        assert_eq!(
            f.store.maintain(true).await.unwrap()["archive"]["archived"],
            1
        );
        assert_eq!(f.count("jobs").await, 1);
        assert_eq!(f.count("archive_jobs").await, 1);
        assert_eq!(
            f.store.list(a.sequence, 10).await.unwrap()[0].job_id,
            b.job_id
        );
    }
}

#[tokio::test]
async fn schema_upgrade_refuses_an_old_daemon_or_in_flight_runner_before_changes() {
    let f = Fixture::new().await;
    let job = f.job("upgrade", None, false).await;
    f.store
        .call(|db| {
            db.execute_batch("DROP TABLE goal_wait_audits; DROP TABLE goal_wait_jobs; DROP TABLE goal_waits; DROP TABLE monitor_rules; DROP TABLE monitor_changes; DROP TABLE runtime_observations; DROP TABLE stop_requests; PRAGMA user_version=6;")?;
            Ok(())
        })
        .await
        .unwrap();
    let owner = f.store.config.daemon_lock().unwrap();
    assert!(
        f.store
            .init()
            .await
            .unwrap_err()
            .to_string()
            .contains("stop the selected daemon")
    );
    drop(owner);
    let id = job.job_id.clone();
    f.store
        .call(move |db| {
            db.execute("UPDATE jobs SET status='running' WHERE id=?", [id])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        f.store
            .init()
            .await
            .unwrap_err()
            .to_string()
            .contains("finish active runs")
    );
    assert_eq!(
        f.store
            .read(|db| Ok(db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))?))
            .await
            .unwrap(),
        6
    );
    f.store
        .finish(job.job_id, "orphaned", None, None, "old runner lost".into())
        .await
        .unwrap();
    f.store.init().await.unwrap();
    assert!(f.store.data_check().await.is_ok());
}

#[tokio::test]
async fn interrupted_backups_finish_sealed_bytes_or_rebuild_only_recorded_unsealed_paths() {
    for published in [false, true] {
        let f = Fixture::new().await;
        let job = f.job("backup", None, true).await;
        f.log(&job, b"durable snapshot");
        let result = f.store.backup().await.unwrap();
        let id = result["backup_id"].as_str().unwrap().to_string();
        let path = PathBuf::from(result["path"].as_str().unwrap());
        let original = std::fs::read(path.join("relay.sqlite3")).unwrap();
        let stage = path.parent().unwrap().join(format!(".{id}.partial"));
        if !published {
            std::fs::rename(&path, &stage).unwrap();
        }
        let saved = id.clone();
        let run = job.run_id.clone();
        f.store
            .call(move |db| {
                db.execute("DELETE FROM maintenance_backups", [])?;
                db.execute(
                    "INSERT INTO maintenance_state(key,value) VALUES('pending_backup',?)",
                    [serde_json::to_string(&json!({"id":saved,"runs":[run]}))?],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        f.store.maintain(true).await.unwrap();
        assert_eq!(std::fs::read(path.join("relay.sqlite3")).unwrap(), original);
        assert_eq!(f.count("maintenance_backups").await, 1);
        assert!(!stage.exists());
    }
    let f = Fixture::new().await;
    let job = f.job("partial", None, true).await;
    let pending = id();
    let parent = f.store.config.root.join("backups");
    agent_job_relay::platform::secure_dir(&parent).unwrap();
    let stage = parent.join(format!(".{pending}.partial"));
    agent_job_relay::platform::secure_dir(&stage).unwrap();
    std::fs::write(stage.join("relay.sqlite3"), b"incomplete").unwrap();
    std::fs::write(stage.join("foreign.txt"), b"keep").unwrap();
    let saved = pending.clone();
    f.store
        .call(move |db| {
            db.execute(
                "INSERT INTO maintenance_state(key,value) VALUES('pending_backup',?)",
                [serde_json::to_string(
                    &json!({"id":saved,"runs":[job.run_id]}),
                )?],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(f.store.maintain(true).await.is_err());
    assert_eq!(std::fs::read(stage.join("foreign.txt")).unwrap(), b"keep");
    std::fs::remove_file(stage.join("foreign.txt")).unwrap();
    f.store.maintain(true).await.unwrap();
    assert!(!stage.exists());
    assert_eq!(f.count("maintenance_backups").await, 1);
}

#[tokio::test]
async fn backup_restores_wal_history_and_logs_without_replaying_active_jobs_or_claims() {
    let f = Fixture::new().await;
    let key = f.session("restore").await;
    let done = f.job("done", None, true).await;
    f.log(&done, b"archived output");
    f.immediate().await;
    f.store.maintain(true).await.unwrap();
    let active = f.job("active", Some(key.clone()), false).await;
    f.log(&active, b"running prefix");
    let pending = f.job("pending", Some(key), true).await;
    let claim = f.store.claim().await.unwrap().unwrap();
    let backup = f.store.backup().await.unwrap();
    let path = PathBuf::from(backup["path"].as_str().unwrap());
    let destination = f.dir.path().join("restored");
    let result = maintenance::restore(path.clone(), destination.clone())
        .await
        .unwrap();
    assert_eq!(result["orphaned_jobs"], 1);
    let config = Config::new(Some(destination.clone())).unwrap();
    let restored = Store::new(config.clone());
    restored.init().await.unwrap();
    assert_eq!(
        restored.status(active.job_id.clone()).await.unwrap().status,
        "orphaned"
    );
    assert_eq!(
        f.store.status(active.job_id).await.unwrap().status,
        "queued"
    );
    assert_eq!(
        archive::logs(config.clone(), done.run_id, "stdout".into(), 0, 100)
            .await
            .unwrap()["text"],
        "archived output"
    );
    assert_eq!(
        archive::logs(config, active.run_id, "stdout".into(), 0, 100)
            .await
            .unwrap()["text"],
        "running prefix"
    );
    assert!(
        !restored
            .complete_claim(claim, Ok("accepted_by_host"))
            .await
            .unwrap()
    );
    let deliveries = restored
        .deliveries(Some(pending.job_id), 0, 100)
        .await
        .unwrap();
    assert_eq!(
        deliveries["deliveries"][0]["failure_kind"],
        "rebind_required"
    );
    assert!(restored.claim().await.unwrap().is_none());
    assert!(restored.data_check().await.is_ok());
    assert!(
        maintenance::restore(path.clone(), destination)
            .await
            .is_err()
    );
    std::fs::write(path.join("relay.sqlite3"), b"tampered").unwrap();
    let rejected = f.dir.path().join("rejected");
    assert!(maintenance::restore(path, rejected.clone()).await.is_err());
    assert!(!rejected.exists());
}

#[tokio::test]
async fn rolling_backups_preserve_foreign_files_and_expired_sessions_do_not_exhaust_quota() {
    let f = Fixture::new().await;
    f.job("history", None, true).await;
    let first = f.store.backup().await.unwrap();
    let first_path = PathBuf::from(first["path"].as_str().unwrap());
    f.store.backup().await.unwrap();
    f.store
        .data_policy(
            Some(Policy {
                backup_keep: 1,
                ..Default::default()
            }),
            true,
        )
        .await
        .unwrap();
    // The recorded oldest backup cannot be removed if unrelated content appears.
    std::fs::write(first_path.join("personal.txt"), b"keep").unwrap();
    assert!(f.store.maintain(true).await.is_err());
    assert_eq!(
        std::fs::read(first_path.join("personal.txt")).unwrap(),
        b"keep"
    );
    std::fs::remove_file(first_path.join("personal.txt")).unwrap();
    f.store.maintain(true).await.unwrap();
    assert!(!first_path.exists());
    assert_eq!(f.count("maintenance_backups").await, 1);
    for i in 0..1001 {
        let key = f.session(&format!("old-{i}")).await;
        f.store.invalidate(key).await.unwrap();
    }
    let key = f.session("new").await;
    assert!(f.store.session(key).await.unwrap().connected);
    assert_eq!(f.store.data_status().await.unwrap()["active_sessions"], 1);
}

#[cfg(unix)]
#[tokio::test]
async fn maintenance_refuses_linked_logs_and_backup_traversal() {
    let f = Fixture::new().await;
    let a = f.job("linked", None, true).await;
    let unrelated = f.dir.path().join("unrelated");
    std::fs::write(&unrelated, b"keep").unwrap();
    let dir = f.store.config.run_dir(&a.run_id);
    agent_job_relay::platform::secure_dir(&dir).unwrap();
    std::os::unix::fs::symlink(&unrelated, dir.join("stdout.log")).unwrap();
    f.immediate().await;
    assert!(f.store.maintain(true).await.is_err());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
    assert_eq!(f.count("jobs").await, 1);
    std::fs::remove_file(dir.join("stdout.log")).unwrap();
    let backup = f.store.backup().await.unwrap();
    let path = PathBuf::from(backup["path"].as_str().unwrap());
    let manifest = path.join("manifest.json");
    let mut value: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    value["files"]["../unrelated"] = json!({"bytes":4,"sha256":"fake"});
    std::fs::write(manifest, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        maintenance::restore(path, f.dir.path().join("rejected"))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(unrelated).unwrap(), b"keep");
}

#[test]
fn data_cli_previews_policy_without_starting_a_daemon() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let root = dir.path().join("state");
    let run = |args: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&root)
            .args(args)
            .env_remove("AJR_STATE_DIR")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    assert_eq!(
        run(&["data", "policy", "--hot-jobs", "7"])["applied"],
        false
    );
    assert_eq!(run(&["data", "status"])["policy"]["hot_jobs"], 1000);
    assert_eq!(
        run(&["data", "policy", "--hot-jobs", "7", "--apply"])["applied"],
        true
    );
    assert_eq!(run(&["data", "status"])["policy"]["hot_jobs"], 7);
    assert_eq!(run(&["data", "check"])["integrity"], "ok");
    assert!(!Config::new(Some(root)).unwrap().socket().exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn daemon_automatically_archives_and_backs_up_without_manual_maintenance() {
    let f = Fixture::new().await;
    let job = f.job("automatic", None, true).await;
    f.log(&job, b"automatic output");
    f.immediate().await;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&f.store.config.root)
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status = f.store.data_status().await.unwrap();
            if status["archived_jobs"] == 1 && status["last_backup_at"].is_number() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let exit = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(exit) = child.try_wait().unwrap() {
                break exit;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    if exit.is_err() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    result.unwrap();
    assert!(exit.unwrap().success());
    assert_eq!(f.count("jobs").await, 0);
    assert_eq!(
        f.store.status(job.job_id).await.unwrap().status,
        "completed"
    );
}

#[tokio::test]
async fn archives_and_backups_preserve_monitor_policy_audits_and_stop_evidence() {
    use agent_job_relay::monitor::{Action, Condition, Mutation, RuleSpec};
    let f = Fixture::new().await;
    let job = f.job("monitored", None, false).await;
    let rule = RuleSpec {
        condition: Condition::NoOutput { seconds: 30 },
        action: Action::Alert,
        notify_agent: false,
        grace_seconds: 5,
    };
    f.store
        .monitor(job.job_id.clone(), Some(Mutation::Add { rule }))
        .await
        .unwrap();
    f.store
        .stop_with(job.job_id.clone(), 30, "manual test".into())
        .await
        .unwrap();
    let before = f
        .store
        .inspect(Some(job.job_id.clone()), None, 20)
        .await
        .unwrap();
    f.log(&job, b"monitor history");
    f.immediate().await;
    f.store.maintain(true).await.unwrap();
    assert_eq!(f.count("monitor_rules").await, 0);
    let after = f
        .store
        .inspect(Some(job.job_id.clone()), None, 20)
        .await
        .unwrap();
    assert_eq!(before["rules"], after["rules"]);
    assert_eq!(after["stop"]["reason"], "manual test");
    let backup = f.store.backup().await.unwrap();
    let to = f.dir.path().join("restored-monitor");
    maintenance::restore(PathBuf::from(backup["path"].as_str().unwrap()), to.clone())
        .await
        .unwrap();
    let restored = Store::new(Config::new(Some(to)).unwrap());
    restored.init().await.unwrap();
    let history = restored.inspect(Some(job.job_id), None, 20).await.unwrap();
    assert_eq!(history["rules"], after["rules"]);
    assert_eq!(history["stop"]["reason"], "manual test");
}

#[tokio::test]
async fn schema_eight_backups_restore_forward_without_changing_the_original() {
    let f = Fixture::new().await;
    let job = f.job("old-backup", None, true).await;
    f.log(&job, b"old backup history");
    f.store.call(|db|{db.execute_batch("DROP TABLE goal_wait_audits; DROP TABLE goal_wait_jobs; DROP TABLE goal_waits; DROP TABLE monitor_rules;DROP TABLE monitor_changes;DROP TABLE runtime_observations;DROP TABLE stop_requests;PRAGMA user_version=8;")?;Ok(())}).await.unwrap();
    let backup = f.store.backup().await.unwrap();
    let from = PathBuf::from(backup["path"].as_str().unwrap());
    let to = f.dir.path().join("forward-restored");
    maintenance::restore(from.clone(), to.clone())
        .await
        .unwrap();
    assert_eq!(
        rusqlite::Connection::open(from.join("relay.sqlite3"))
            .unwrap()
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        8
    );
    let restored = Store::new(Config::new(Some(to)).unwrap());
    restored.init().await.unwrap();
    assert_eq!(
        restored.status(job.job_id).await.unwrap().status,
        "completed"
    );
}
