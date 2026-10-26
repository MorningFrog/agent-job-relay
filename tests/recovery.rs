#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    session::{Adapter, AgentSession, Registration},
    storage::Store,
};
use serde_json::Value;

struct Fixture {
    dir: tempfile::TempDir,
    store: Store,
    _listener: tokio::net::UnixListener,
}
impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir_in(common::root()).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config);
        store.init().await.unwrap();
        let listener = tokio::net::UnixListener::bind(dir.path().join("host.sock")).unwrap();
        Self {
            dir,
            store,
            _listener: listener,
        }
    }
    async fn session(&self, logical: &str, instance: &str) -> AgentSession {
        self.store
            .register(Registration {
                agent_kind: "codex_desktop".into(),
                session_id: logical.into(),
                instance_id: instance.into(),
                host_id: "local-desktop".into(),
                adapter: Adapter::DesktopExperimental,
                endpoint: Some(self.dir.path().join("host.sock")),
                caller_thread_id: Some(logical.into()),
                lease_seconds: 86400,
                experimental: true,
            })
            .await
            .unwrap()
    }
    async fn job(&self, session: &AgentSession, finish: bool) -> Job {
        let j = self
            .store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "true".into(),
                    args: vec![],
                },
                cwd: self.dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                session: Some(session.key.clone()),
                idempotency_key: None,
            })
            .await
            .unwrap();
        if finish {
            self.store
                .finish(j.job_id.clone(), "completed", Some(0), None, "done".into())
                .await
                .unwrap();
        }
        j
    }
    async fn recover(&self, session: &AgentSession, apply: bool) -> Value {
        self.store
            .recover_session(session.clone(), apply, 0, 100)
            .await
            .unwrap()
    }
    async fn delivery(&self, j: &Job) -> Value {
        self.store
            .deliveries(Some(j.job_id.clone()), 0, 100)
            .await
            .unwrap()["deliveries"][0]
            .clone()
    }
}

#[tokio::test]
async fn preview_apply_is_scoped_preserves_ids_ttl_and_never_replays_accepted_or_expired() {
    let f = Fixture::new().await;
    let old = f.session("a", "old").await;
    let other = f.session("b", "old").await;
    let active = f.job(&old, false).await;
    let pending = f.job(&old, true).await;
    let accepted = f.job(&old, true).await;
    let expired = f.job(&old, true).await;
    let foreign = f.job(&other, true).await;
    let before = f.delivery(&pending).await;
    let accepted_id = f.delivery(&accepted).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let expired_id = f.delivery(&expired).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    f.store
        .call(move |db| {
            db.execute(
                "UPDATE deliveries SET status='accepted_by_host' WHERE id=?",
                [accepted_id],
            )?;
            db.execute("UPDATE deliveries SET expires=0 WHERE id=?", [expired_id])?;
            Ok(())
        })
        .await
        .unwrap();
    let current = f.session("a", "new").await;
    let preview = f.recover(&current, false).await;
    assert_eq!(preview["targets"].as_array().unwrap().len(), 2);
    assert!(preview["recent_recoveries"].as_array().unwrap().is_empty());
    assert_eq!(before, f.delivery(&pending).await);
    let applied = f.recover(&current, true).await;
    assert_eq!(applied["targets"].as_array().unwrap().len(), 2);
    let after = f.delivery(&pending).await;
    assert_eq!(after["id"], before["id"]);
    assert_eq!(after["expires_at"], before["expires_at"]);
    assert_ne!(after["target_id"], before["target_id"]);
    assert_eq!(f.delivery(&accepted).await["status"], "accepted_by_host");
    assert_eq!(f.delivery(&expired).await["expires_at"], 0);
    assert_eq!(f.delivery(&foreign).await["status"], "pending");
    assert!(
        f.recover(&current, true).await["targets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.store.status(active.job_id.clone()).await.unwrap().status,
        "queued"
    );
    f.store
        .finish(
            active.job_id.clone(),
            "completed",
            Some(0),
            None,
            "done".into(),
        )
        .await
        .unwrap();
    let target = f.delivery(&active).await["target_id"]
        .as_str()
        .unwrap()
        .to_string();
    let bound = f
        .store
        .call(move |db| {
            Ok(db.query_row(
                "SELECT session_key FROM targets WHERE id=?",
                [target],
                |r| r.get::<_, String>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(bound, current.key);
}

#[tokio::test]
async fn recovery_skips_live_claims_fences_stale_ack_and_rolls_back_with_its_audit() {
    let f = Fixture::new().await;
    let old = f.session("a", "old").await;
    let job = f.job(&old, true).await;
    let stale = f.store.claim().await.unwrap().unwrap();
    let current = f.session("a", "new").await;
    let skipped = f.recover(&current, true).await;
    assert_eq!(skipped["targets"][0]["in_flight"], true);
    assert_eq!(skipped["targets"][0]["applied"], false);
    f.store.call(|db| {
        db.execute("UPDATE deliveries SET lease_until=0",[])?;
        db.execute_batch("CREATE TRIGGER fail_recovery BEFORE INSERT ON target_recoveries BEGIN SELECT RAISE(ABORT,'injected audit failure'); END;")?;
        Ok(())
    }).await.unwrap();
    let before = f.delivery(&job).await;
    assert!(
        f.store
            .recover_session(current.clone(), true, 0, 100)
            .await
            .is_err()
    );
    assert_eq!(f.delivery(&job).await, before);
    f.store
        .call(|db| {
            db.execute_batch("DROP TRIGGER fail_recovery;")?;
            Ok(())
        })
        .await
        .unwrap();
    let result = f.recover(&current, true).await;
    assert_eq!(result["targets"][0]["applied"], true);
    assert!(
        !f.store
            .complete_claim(stale, Ok("accepted_by_host"))
            .await
            .unwrap()
    );
    let c = f.store.claim().await.unwrap().unwrap();
    assert_eq!(c.session.key, current.key);
    assert_eq!(c.delivery.id, before["id"]);
    f.store
        .complete_claim(c, Ok("accepted_by_host"))
        .await
        .unwrap();
    assert_eq!(f.store.status(job.job_id).await.unwrap().run.attempt, 1);
}

#[tokio::test]
async fn existing_accepted_target_coalesces_without_duplicate_delivery_and_changed_binding_is_refused()
 {
    let f = Fixture::new().await;
    let old = f.session("a", "old").await;
    let job = f.job(&old, true).await;
    let current = f.session("a", "new").await;
    let new = f
        .store
        .target(job.job_id.clone(), current.key.clone(), true)
        .await
        .unwrap()["target_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.store
        .call(move |db| {
            db.execute(
                "UPDATE deliveries SET status='accepted_by_host' WHERE target_id=?",
                [new],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let result = f.recover(&current, true).await;
    assert!(result["targets"][0]["deliveries"][0]["superseded_by"].is_string());
    assert!(f.store.claim().await.unwrap().is_none());
    assert_eq!(f.delivery(&job).await["status"], "cancelled");
    f.store.invalidate(current.key.clone()).await.unwrap();
    assert!(
        f.store
            .recover_session(current, true, 0, 100)
            .await
            .is_err()
    );
}
