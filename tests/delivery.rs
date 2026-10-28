mod common;
use agent_job_relay::{
    adapters,
    config::Config,
    core::*,
    session::{Adapter, Registration},
    storage::Store,
};
use serde_json::Value;
struct Fixture {
    _dir: tempfile::TempDir,
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
        Self { _dir: dir, store }
    }
    async fn session(&self, logical: &str, instance: &str) -> String {
        self.store
            .register(Registration {
                agent_kind: "codex_cli".into(),
                session_id: logical.into(),
                instance_id: instance.into(),
                host_id: "host".into(),
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
    async fn job(&self, key: &str) -> Job {
        self.store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "true".into(),
                    args: vec![],
                },
                cwd: self._dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                session: Some(key.into()),
                idempotency_key: None,
                monitor_rules: vec![],
            })
            .await
            .unwrap()
    }
    async fn rows(&self, j: &str) -> Vec<Value> {
        self.store.deliveries(Some(j.into()), 0, 100).await.unwrap()["deliveries"]
            .as_array()
            .unwrap()
            .clone()
    }
}
#[tokio::test]
async fn idle_dispatch_reads_without_waiting_for_a_database_writer() {
    let f = Fixture::new().await;
    let db = rusqlite::Connection::open(f.store.config.root.join("relay.sqlite3")).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), f.store.claim())
            .await
            .expect("an idle dispatcher must not request the SQLite writer lock")
            .unwrap()
            .is_none()
    );
    db.execute_batch("ROLLBACK").unwrap();
}

#[tokio::test]
async fn pooled_operations_discard_unfinished_transactions_and_keep_profiles_separate() {
    let f = Fixture::new().await;
    for fail in [true, false] {
        assert!(
            f.store.call(move |db| {
                db.execute_batch("BEGIN IMMEDIATE; INSERT INTO sessions(key,agent,logical,instance,host,registration,expires,connected) VALUES('uncommitted','test','test','test','test','{}',0,0)")?;
                if fail { anyhow::bail!("operation failed before commit"); }
                Ok(())
            }).await.is_err()
        );
        assert_eq!(
            f.store
                .call(|db| Ok(
                    db.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))?
                ))
                .await
                .unwrap(),
            0
        );
    }
    let a = f.session("profile-a", "one").await;
    let other = Fixture::new().await;
    let mut switched = f.store.clone();
    switched.config = other.store.config.clone();
    assert!(switched.session(a.clone()).await.is_err());
    assert_eq!(f.store.session(a.clone()).await.unwrap().key, a);
}

#[tokio::test]
async fn concurrent_claims_serialize_the_logical_chat_and_skip_busy_destinations() {
    let f = Fixture::new().await;
    let a = f.session("a", "one").await;
    let same_chat = f.session("a", "two").await;
    let b = f.session("b", "one").await;
    let mut jobs = Vec::new();
    for key in [&a, &same_chat, &b] {
        let job = f.job(key).await;
        f.store
            .finish(
                job.job_id.clone(),
                "completed",
                Some(0),
                None,
                "done".into(),
            )
            .await
            .unwrap();
        jobs.push(job.job_id);
    }
    let (first, second) = tokio::join!(f.store.claim(), f.store.claim());
    let claims = [first.unwrap().unwrap(), second.unwrap().unwrap()];
    assert!(claims.iter().any(|c| c.event["job_id"] == jobs[0]));
    assert!(claims.iter().any(|c| c.event["job_id"] == jobs[2]));
    assert!(f.store.claim().await.unwrap().is_none());
    for claim in claims {
        f.store
            .complete_claim(claim, Ok("notification_recorded"))
            .await
            .unwrap();
    }
    let next = f.store.claim().await.unwrap().unwrap();
    assert_eq!(next.event["job_id"], jobs[1]);
}

#[tokio::test]
async fn completion_outbox_is_atomic_and_duplicate_finish_is_safe() {
    let f = Fixture::new().await;
    let s = f.session("a", "one").await;
    let j = f.job(&s).await;
    f.store.call(|db|{db.execute_batch("CREATE TRIGGER fail_outbox BEFORE INSERT ON deliveries BEGIN SELECT RAISE(ABORT,'fault injection'); END;")?;Ok(())}).await.unwrap();
    assert!(
        f.store
            .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
            .await
            .is_err()
    );
    assert_eq!(
        f.store.status(j.job_id.clone()).await.unwrap().status,
        "queued"
    );
    assert!(
        f.store
            .events(j.job_id.clone(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
    f.store
        .call(|db| {
            db.execute_batch("DROP TRIGGER fail_outbox;")?;
            Ok(())
        })
        .await
        .unwrap();
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    f.store
        .finish(j.job_id.clone(), "failed", Some(1), None, "late".into())
        .await
        .unwrap();
    assert_eq!(f.rows(&j.job_id).await.len(), 1);
    assert_eq!(f.store.status(j.job_id).await.unwrap().status, "completed");
}
#[tokio::test]
async fn inbox_routes_exact_session_and_duplicate_ack_is_fenced() {
    let f = Fixture::new().await;
    let a = f.session("a", "one").await;
    let b = f.session("b", "one").await;
    let j = f.job(&a).await;
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    let id = c.delivery.id.clone();
    assert_eq!(c.session.key, a);
    let duplicate = c.clone();
    let outcome = adapters::deliver(&f.store.config, &c).await;
    assert_eq!(outcome.as_ref().unwrap(), &"notification_recorded");
    assert!(f.store.complete_claim(c, outcome).await.unwrap());
    assert!(
        !f.store
            .complete_claim(duplicate, Ok("notification_recorded"))
            .await
            .unwrap()
    );
    assert!(f.store.claim().await.unwrap().is_none());
    assert!(
        f.store
            .config
            .root
            .join("inbox")
            .join(&a)
            .join(format!("{id}.json"))
            .exists()
    );
    assert!(!f.store.config.root.join("inbox").join(b).exists());
    assert_eq!(
        f.rows(&j.job_id).await[0]["status"],
        "notification_recorded"
    );
}
#[tokio::test]
async fn stale_lease_replay_retry_ttl_and_dead_letter() {
    let f = Fixture::new().await;
    let s = f.session("a", "one").await;
    let j = f.job(&s).await;
    f.store
        .finish(j.job_id.clone(), "failed", None, None, "done".into())
        .await
        .unwrap();
    let stale = f.store.claim().await.unwrap().unwrap();
    f.store
        .call(|db| {
            db.execute("UPDATE deliveries SET lease_until=0", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let fresh = f.store.claim().await.unwrap().unwrap();
    assert_eq!(fresh.delivery.attempts, 2);
    assert!(
        !f.store
            .complete_claim(stale, Ok("accepted_by_host"))
            .await
            .unwrap()
    );
    let id = fresh.delivery.id.clone();
    assert!(
        f.store
            .complete_claim(fresh, Err(anyhow::anyhow!("host unavailable")))
            .await
            .unwrap()
    );
    assert_eq!(f.rows(&j.job_id).await[0]["status"], "retry_wait");
    f.store
        .call(|db| {
            db.execute("UPDATE deliveries SET expires=0", [])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(f.store.claim().await.unwrap().is_none());
    assert_eq!(f.rows(&j.job_id).await[0]["status"], "dead_letter");
    f.store.retry_delivery(id.clone()).await.unwrap();
    assert!(f.store.claim().await.unwrap().is_some());
    assert_eq!(
        f.store.attempts(id, 0, 100).await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
async fn expired_host_instance_requires_explicit_rebind() {
    let f = Fixture::new().await;
    let a = f.session("a", "old").await;
    let j = f.job(&a).await;
    f.store.invalidate(a.clone()).await.unwrap();
    let b = f.session("a", "new").await;
    assert_ne!(a, b);
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    assert_eq!(c.session.key, a);
    assert!(adapters::deliver(&f.store.config, &c).await.is_err());
    let target = c.delivery.target_id.clone();
    f.store
        .complete_claim(c, Err(anyhow::anyhow!("old host gone")))
        .await
        .unwrap();
    f.store.rebind(target, b.clone()).await.unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    assert_eq!(c.session.key, b);
    f.store
        .complete_claim(c, Ok("notification_recorded"))
        .await
        .unwrap();
    assert_eq!(f.rows(&j.job_id).await.len(), 2);
}
#[tokio::test]
async fn temporary_failures_survive_five_attempts_and_store_reopen() {
    let f = Fixture::new().await;
    assert!(f.job_result_missing().await.is_err());
    let a = f.session("a", "one").await;
    let j = f.job(&a).await;
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    let mut delivery_id = String::new();
    for attempt in 1..=12 {
        f.store
            .call(|db| {
                db.execute("UPDATE deliveries SET next_at=0", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let c = f.store.claim().await.unwrap().unwrap();
        assert_eq!(c.delivery.attempts, attempt);
        if delivery_id.is_empty() {
            delivery_id = c.delivery.id.clone();
        }
        assert_eq!(c.delivery.id, delivery_id);
        f.store
            .complete_claim(c, Err(anyhow::anyhow!("offline")))
            .await
            .unwrap();
        let row = &f.rows(&j.job_id).await[0];
        assert_eq!(row["status"], "retry_wait");
        assert_eq!(row["failure_kind"], "temporary");
        assert!(row["next_attempt_at"].as_i64().unwrap() > now());
        assert!(row["next_attempt_at"].as_i64().unwrap() <= now() + 300_000);
    }
    let reopened = Store::new(f.store.config.clone());
    reopened.init().await.unwrap();
    reopened
        .call(|db| {
            db.execute("UPDATE deliveries SET next_at=0", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let c = reopened.claim().await.unwrap().unwrap();
    assert_eq!(c.delivery.id, delivery_id);
    let outcome = adapters::deliver(&reopened.config, &c).await;
    reopened.complete_claim(c, outcome).await.unwrap();
    assert_eq!(
        f.rows(&j.job_id).await[0]["status"],
        "notification_recorded"
    );
}

#[tokio::test]
async fn permanent_errors_stop_and_identity_errors_wait_for_explicit_recovery() {
    use agent_job_relay::delivery::{DeliveryError, FailureKind};
    let f = Fixture::new().await;
    let a = f.session("a", "one").await;
    let j = f.job(&a).await;
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    let id = c.delivery.id.clone();
    f.store
        .complete_claim(
            c,
            Err(DeliveryError(FailureKind::Permanent, "unsupported capability").into()),
        )
        .await
        .unwrap();
    let row = &f.rows(&j.job_id).await[0];
    assert_eq!(row["status"], "dead_letter");
    assert_eq!(row["attempts"], 1);
    assert_eq!(row["failure_kind"], "permanent");
    f.store.retry_delivery(id).await.unwrap();
    f.store.invalidate(a).await.unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    let outcome = adapters::deliver(&f.store.config, &c).await;
    f.store.complete_claim(c, outcome).await.unwrap();
    assert_eq!(
        f.rows(&j.job_id).await[0]["failure_kind"],
        "rebind_required"
    );
    assert!(f.store.claim().await.unwrap().is_none());
}
impl Fixture {
    async fn job_result_missing(&self) -> anyhow::Result<Job> {
        self.store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "true".into(),
                    args: vec![],
                },
                cwd: self._dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                session: Some(id()),
                idempotency_key: None,
                monitor_rules: vec![],
            })
            .await
    }
}

#[tokio::test]
async fn repeated_schema_init_and_unknown_future_version_are_safe() {
    let f = Fixture::new().await;
    f.store.init().await.unwrap();
    f.store
        .call(|db| {
            db.pragma_update(None, "user_version", 99)?;
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
            .contains("newer")
    );
}
#[cfg(unix)]
#[tokio::test]
async fn symlink_database_is_rejected() {
    let f = Fixture::new().await;
    let external = f._dir.path().join("unrelated");
    std::fs::write(&external, "untouched").unwrap();
    let db = f.store.config.root.join("relay.sqlite3");
    std::fs::remove_file(&db).unwrap();
    std::os::unix::fs::symlink(&external, &db).unwrap();
    assert!(f.store.init().await.is_err());
    assert_eq!(std::fs::read_to_string(external).unwrap(), "untouched");
}

#[tokio::test]
async fn version_one_database_upgrades_without_losing_job_or_run() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let j = id();
    let r = id();
    let spec = StartSpec {
        command: CommandSpec::Exec {
            program: "true".into(),
            args: vec![],
        },
        cwd: dir.path().into(),
        env: Default::default(),
        timeout_seconds: None,
        session: None,
        idempotency_key: None,
        monitor_rules: vec![],
    };
    {
        let db = rusqlite::Connection::open(config.root.join("relay.sqlite3")).unwrap();
        db.execute_batch(include_str!("../src/schema.sql")).unwrap();
        db.execute(
            "INSERT INTO jobs(id,run_id,spec,status,created,updated) VALUES(?,?,?,'completed',1,2)",
            rusqlite::params![j, r, serde_json::to_string(&spec).unwrap()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO runs(id,job_id,attempt,nonce,exit_code,ended) VALUES(?,?,1,?,0,2)",
            rusqlite::params![r, j, id()],
        )
        .unwrap();
    }
    let store = Store::new(config);
    store.init().await.unwrap();
    store.init().await.unwrap();
    let job = store.status(j).await.unwrap();
    assert_eq!(job.status, "completed");
    assert_eq!(job.run.status, "completed");
    assert_eq!(job.run.attempt, 1);
    assert_eq!(job.run.exit_code, Some(0));
    assert_eq!(job.run.ended_at, Some(2));
    assert_eq!(
        store
            .call(|db| Ok(db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))?))
            .await
            .unwrap(),
        10
    );
}

#[tokio::test]
async fn version_four_upgrade_preserves_pending_delivery_and_attempt_history() {
    let f = Fixture::new().await;
    let s = f.session("migration", "old").await;
    let j = f.job(&s).await;
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    let c = f.store.claim().await.unwrap().unwrap();
    let id = c.delivery.id.clone();
    f.store
        .complete_claim(c, Err(anyhow::anyhow!("offline")))
        .await
        .unwrap();
    f.store
        .call(|db| {
            db.execute_batch(
                "DROP TABLE goal_wait_audits; DROP TABLE goal_wait_jobs; DROP TABLE goal_waits; DROP TABLE monitor_rules; DROP TABLE monitor_changes; DROP TABLE runtime_observations; DROP TABLE stop_requests; DROP TABLE target_recoveries; ALTER TABLE deliveries DROP COLUMN failure_kind; PRAGMA user_version=4;",
            )?;
            Ok(())
        })
        .await
        .unwrap();
    f.store.init().await.unwrap();
    let row = &f.rows(&j.job_id).await[0];
    assert_eq!(row["id"], id);
    assert_eq!(row["attempts"], 1);
    assert_eq!(row["status"], "retry_wait");
    assert_eq!(
        f.store.attempts(id, 0, 100).await.unwrap()["attempts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn status_has_reserved_capacity_when_all_writer_workers_are_busy() {
    let f = Fixture::new().await;
    let job = f
        .store
        .start(StartSpec {
            command: CommandSpec::Exec {
                program: "true".into(),
                args: vec![],
            },
            cwd: f._dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
            monitor_rules: vec![],
        })
        .await
        .unwrap();
    let (started, mut entered) = tokio::sync::mpsc::channel(4);
    let mut release = Vec::new();
    let mut workers = Vec::new();
    for _ in 0..4 {
        let (send, recv) = std::sync::mpsc::channel();
        release.push(send);
        let started = started.clone();
        let store = f.store.clone();
        workers.push(tokio::spawn(async move {
            store
                .call(move |_| {
                    started.blocking_send(()).unwrap();
                    recv.recv().unwrap();
                    Ok(())
                })
                .await
                .unwrap();
        }));
    }
    for _ in 0..4 {
        entered.recv().await.unwrap();
    }
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        f.store.status(job.job_id),
    )
    .await;
    for send in release {
        send.send(()).unwrap();
    }
    for worker in workers {
        worker.await.unwrap();
    }
    assert_eq!(
        result
            .expect("reads must not queue behind occupied writer workers")
            .unwrap()
            .status,
        "queued"
    );
}
