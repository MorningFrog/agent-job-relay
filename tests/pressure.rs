mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    maintenance::Policy,
    storage::Store,
};
use serde_json::json;

#[tokio::test]
async fn disk_pressure_refuses_only_new_admissions_and_keeps_original_runs_and_controls() {
    agent_job_relay::platform::initialize();
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config);
    store.init().await.unwrap();
    let spec = StartSpec {
        title: None,
        command: CommandSpec::Exec {
            program: "never-execute".into(),
            args: vec![],
        },
        cwd: dir.path().into(),
        env: Default::default(),
        timeout_seconds: None,
        session: None,
        idempotency_key: Some("original".into()),
        monitor_rules: vec![],
    };
    let original = store.start(spec.clone()).await.unwrap();
    let available = agent_job_relay::platform::available_space(&store.config.root).unwrap();
    let threshold = u32::try_from(available / (1024 * 1024) + 1024).unwrap();
    let policy = Policy {
        disk_warn_mib: threshold,
        disk_stop_new_jobs_mib: threshold,
        ..Default::default()
    };
    let preview = store
        .data_policy(Some(policy.clone()), false)
        .await
        .unwrap();
    assert_eq!(preview["applied"], false);
    assert_eq!(
        store.data_status().await.unwrap()["policy"]["disk_stop_new_jobs_mib"],
        64
    );
    store.data_policy(Some(policy), true).await.unwrap();
    let health = ipc::dispatch(&store, Request::Doctor).await.unwrap();
    assert_eq!(health["capacity"]["disk_pressure"]["status"], "critical");
    assert_eq!(
        health["capacity"]["disk_pressure"]["new_jobs_allowed"],
        false
    );
    let retry = store.start(spec.clone()).await.unwrap();
    assert_eq!(retry.job_id, original.job_id);
    assert_eq!(retry.run_id, original.run_id);
    let mut next = spec.clone();
    next.idempotency_key = Some("new".into());
    let error = store.start(next.clone()).await.unwrap_err().to_string();
    assert!(error.contains("disk pressure"), "{error}");
    assert_eq!(store.list(0, 100).await.unwrap().len(), 1);
    assert!(
        store
            .events(original.job_id.clone(), 0, 20)
            .await
            .unwrap()
            .is_empty()
    );
    let batch = agent_job_relay::batch::submit(&store, vec![spec.clone(), next.clone()])
        .await
        .unwrap();
    assert_eq!(batch["accepted"], 1);
    assert_eq!(batch["rejected"], 1);
    let stop = store
        .stop_with(original.job_id.clone(), 0, "manual".into())
        .await
        .unwrap();
    assert!(stop.cancel_requested);
    store
        .finish(
            original.job_id.clone(),
            "cancelled",
            None,
            None,
            "fixture".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        store.status(original.job_id.clone()).await.unwrap().status,
        JobState::Cancelled
    );
    store
        .data_policy(
            Some(Policy {
                disk_warn_mib: threshold,
                disk_stop_new_jobs_mib: 0,
                ..Default::default()
            }),
            true,
        )
        .await
        .unwrap();
    let admitted = ipc::dispatch(&store, Request::Start { spec: next })
        .await
        .unwrap();
    assert_eq!(admitted["storage"]["status"], "warning");
    assert_eq!(admitted["storage"]["new_jobs_allowed"], true);
    let compact = agent_job_relay::output::compact("start", admitted.clone());
    assert_eq!(compact["storage"], admitted["storage"]);
    assert_eq!(
        store.data_status().await.unwrap()["archive_retention"],
        json!("indefinite; automatic archiving compresses data without deleting history")
    );
}
