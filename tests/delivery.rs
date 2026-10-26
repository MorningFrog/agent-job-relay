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
        5
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
                "ALTER TABLE deliveries DROP COLUMN failure_kind; PRAGMA user_version=4;",
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
