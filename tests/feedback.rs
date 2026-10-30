mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    monitor::{Mutation, RuleSpec},
    session::{Adapter, Registration},
    storage::Store,
};
use serde_json::{Value, json};

async fn fixture() -> (tempfile::TempDir, Store, StartSpec) {
    agent_job_relay::platform::initialize();
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config);
    store.init().await.unwrap();
    let spec = StartSpec {
        command: CommandSpec::Exec {
            program: "must-never-execute".into(),
            args: vec![],
        },
        cwd: dir.path().into(),
        env: Default::default(),
        timeout_seconds: None,
        session: None,
        idempotency_key: Some("receipt".into()),
        monitor_rules: vec![],
    };
    (dir, store, spec)
}
fn registration(name: &str) -> Registration {
    Registration {
        agent_kind: "test".into(),
        session_id: name.into(),
        instance_id: name.into(),
        host_id: "fixture".into(),
        adapter: Adapter::Inbox,
        endpoint: None,
        caller_thread_id: None,
        lease_seconds: 3600,
        experimental: false,
        renew_on_delivery: false,
    }
}
async fn submit(store: &Store, spec: &StartSpec) -> Value {
    ipc::dispatch(store, Request::Start { spec: spec.clone() })
        .await
        .unwrap()
}
async fn inspect(store: &Store, id: &str) -> Value {
    store.inspect(Some(id.into()), None, 20).await.unwrap()
}

#[tokio::test]
async fn receipts_follow_persisted_rule_changes_and_do_not_replay_on_diagnostic_failure() {
    let (_dir, store, spec) = fixture().await;
    let initial = submit(&store, &spec).await;
    let id = initial["job"]["job_id"].as_str().unwrap();
    assert_eq!(initial["feedback"]["status"], "not_configured");
    let rule: RuleSpec = serde_json::from_value(json!({
        "condition":{"kind":"contains","text":"CHECKPOINT"},"action":"alert"
    }))
    .unwrap();
    store
        .monitor(id.into(), Some(Mutation::Add { rule }))
        .await
        .unwrap();
    let detail = inspect(&store, id).await;
    assert_eq!(detail["feedback"]["status"], "local_only");
    assert_eq!(detail["feedback"]["rules"][0]["application"], "pending");
    let rule_id = detail["feedback"]["rules"][0]["rule_id"].as_str().unwrap();
    // A lost-response retry must describe the current persisted rules, rather
    // than the original empty StartSpec, without creating another job/run.
    let retry = submit(&store, &spec).await;
    assert_eq!(initial["job"], retry["job"]);
    assert_eq!(retry["feedback"], detail["feedback"]);
    assert_eq!(
        agent_job_relay::output::compact("start", retry)["feedback"],
        detail["feedback"]
    );
    store
        .monitor(
            id.into(),
            Some(Mutation::Enable {
                rule_id: rule_id.into(),
                enabled: false,
                expected_revision: 1,
            }),
        )
        .await
        .unwrap();
    let disabled = inspect(&store, id).await;
    assert_eq!(disabled["feedback"]["status"], "not_configured");
    assert_eq!(disabled["feedback"]["rules"][0]["application"], "disabled");
    assert_eq!(disabled["feedback"]["rules"][0]["revision"], 2);
    let job_id = id.to_owned();
    store
        .call(move |db| {
            db.execute(
                "INSERT INTO runtime_observations(job_id,data) VALUES(?,'broken-json')",
                [&job_id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let degraded = submit(&store, &spec).await;
    assert_eq!(degraded["job"], initial["job"]);
    assert_eq!(degraded["feedback"]["status"], "unknown");
    assert_eq!(store.list(0, 20).await.unwrap().len(), 1);
    assert!(store.events(id.into(), 0, 20).await.unwrap().is_empty());
}

#[tokio::test]
async fn inbox_configuration_and_progress_samples_do_not_claim_agent_execution() {
    let (_dir, store, mut spec) = fixture().await;
    let session = store.register(registration("inbox")).await.unwrap();
    spec.session = Some(session.key);
    spec.monitor_rules = vec![serde_json::from_value(json!({
        "condition":{"kind":"progress","source":"train","trigger":{"kind":"stage"},"cooldown_seconds":60},
        "notify_agent":true
    })).unwrap()];
    let receipt = submit(&store, &spec).await;
    let id = receipt["job"]["job_id"].as_str().unwrap();
    assert_eq!(receipt["feedback"]["status"], "inbox_only");
    assert_eq!(receipt["feedback"]["rules"][0]["sample_observed"], false);
    assert_eq!(receipt["feedback"]["can_confirm_resume"], false);
    let job_id = id.to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE monitor_rules SET applied_revision=revision WHERE job_id=?",
                [&job_id],
            )?;
            db.execute(
                "INSERT INTO runtime_observations(job_id,data) VALUES(?,?)",
                rusqlite::params![
                    job_id,
                    json!({"progress":{"sources":{"train":{"seq":1,"stage":"checkpoint"}}}})
                        .to_string()
                ],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let sampled = inspect(&store, id).await;
    assert_eq!(sampled["feedback"]["rules"][0]["application"], "applied");
    assert_eq!(sampled["feedback"]["rules"][0]["sample_observed"], true);
    assert_eq!(sampled["feedback"]["status"], "inbox_only");
    assert!(
        sampled["deliveries"]["deliveries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let job_id = id.to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE runtime_observations SET data=? WHERE job_id=?",
                rusqlite::params![
                    json!({"progress":{"alerts":128,"sources":{"train":{"seq":128}}}}).to_string(),
                    job_id
                ],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(inspect(&store, id).await["feedback"]["status"], "exhausted");
    store
        .finish(id.into(), "completed", Some(0), None, "fixture".into())
        .await
        .unwrap();
    let terminal = submit(&store, &spec).await;
    assert_eq!(terminal["feedback"]["status"], "terminal");
    assert_eq!(terminal["feedback"]["rules"][0]["can_trigger_again"], false);
    assert!(
        terminal["feedback"]["targets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn host_feedback_follows_recorded_rebinds_lease_state_and_one_shot_exhaustion() {
    let (dir, store, mut spec) = fixture().await;
    let original = store.register(registration("original")).await.unwrap();
    spec.session = Some(original.key.clone());
    spec.monitor_rules = vec![
        serde_json::from_value(json!({
            "condition":{"kind":"contains","text":"CHECKPOINT"},"notify_agent":true
        }))
        .unwrap(),
    ];
    let initial = submit(&store, &spec).await;
    let id = initial["job"]["job_id"].as_str().unwrap();
    let old_target = initial["feedback"]["targets"][0]["target_id"]
        .as_str()
        .unwrap();
    let endpoint = dir.path().join("host.sock");
    let listener = std::os::unix::net::UnixListener::bind(&endpoint).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut host = registration("host");
    host.adapter = Adapter::DesktopExperimental;
    host.endpoint = Some(endpoint);
    host.caller_thread_id = Some("host".into());
    host.experimental = true;
    let bound = store.register(host).await.unwrap();
    store
        .rebind(old_target.into(), bound.key.clone())
        .await
        .unwrap();
    store.invalidate(original.key).await.unwrap();
    let rebound = submit(&store, &spec).await;
    assert_eq!(rebound["job"], initial["job"]);
    assert_eq!(rebound["feedback"]["status"], "configured");
    assert_eq!(rebound["feedback"]["targets"][0]["session_key"], bound.key);
    let compact = agent_job_relay::output::compact("start", rebound);
    assert_eq!(compact["feedback"]["can_confirm_resume"], false);
    assert!(compact["next_step"].as_str().unwrap().contains("mid-run"));
    // Inspection never probes or mutates the host as a side effect.
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let key = bound.key.clone();
    store
        .call(move |db| {
            db.execute(
                "UPDATE sessions SET expires=? WHERE key=?",
                rusqlite::params![now() - 1, key],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        inspect(&store, id).await["feedback"]["status"],
        "binding_unavailable"
    );
    store.invalidate(bound.key).await.unwrap();
    assert_eq!(
        inspect(&store, id).await["feedback"]["targets"][0]["status"],
        "invalidated"
    );
    let job_id = id.to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE monitor_rules SET triggered_revision=revision WHERE job_id=?",
                [job_id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let exhausted = inspect(&store, id).await;
    assert_eq!(exhausted["feedback"]["status"], "exhausted");
    let job_id = id.to_owned();
    store
        .call(move |db| {
            db.execute(
                "UPDATE monitor_rules SET triggered_revision=0 WHERE job_id=?",
                [&job_id],
            )?;
            db.execute("UPDATE targets SET active=0 WHERE job_id=?", [&job_id])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(inspect(&store, id).await["feedback"]["status"], "unbound");
    assert_eq!(
        exhausted["feedback"]["rules"][0]["can_trigger_again"],
        false
    );
}
