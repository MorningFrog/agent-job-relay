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
        let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
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
    let outcome = adapters::deliver(&f.store.config, &c).await;
    assert_eq!(outcome.as_ref().unwrap(), &"notification_recorded");
    assert!(f.store.complete_claim(c, outcome).await.unwrap());
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
async fn fifth_failure_dead_letters_and_missing_session_never_launches() {
    let f = Fixture::new().await;
    assert!(f.job_result_missing().await.is_err());
    let a = f.session("a", "one").await;
    let j = f.job(&a).await;
    f.store
        .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    for _ in 0..5 {
        f.store
            .call(|db| {
                db.execute("UPDATE deliveries SET next_at=0", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let c = f.store.claim().await.unwrap().unwrap();
        f.store
            .complete_claim(c, Err(anyhow::anyhow!("offline")))
            .await
            .unwrap();
    }
    assert_eq!(f.rows(&j.job_id).await[0]["status"], "dead_letter");
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
