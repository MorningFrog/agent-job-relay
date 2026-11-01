mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    search::TaskQuery,
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
        title: Some("检索早期任务".into()),
        command: CommandSpec::Exec {
            program: "never-execute".into(),
            args: vec![format!("{}深处参数", "x".repeat(300))],
        },
        cwd: dir.path().into(),
        env: Default::default(),
        timeout_seconds: None,
        queue_timeout_seconds: None,
        session: None,
        idempotency_key: None,
        monitor_rules: vec![],
    };
    (dir, store, spec)
}
#[tokio::test]
async fn history_search_reaches_cold_records_after_empty_pages_and_preserves_state() {
    let (dir, store, mut spec) = fixture().await;
    let old = store.start(spec.clone()).await.unwrap();
    store
        .finish(
            old.job_id.clone(),
            "failed",
            Some(2),
            None,
            "fixture".into(),
        )
        .await
        .unwrap();
    store
        .data_policy(
            Some(agent_job_relay::maintenance::Policy {
                archive_after_days: 0,
                ..Default::default()
            }),
            true,
        )
        .await
        .unwrap();
    store.maintain(true).await.unwrap();
    assert_eq!(store.data_status().await.unwrap()["archived_jobs"], 1);
    spec.title = Some("unrelated".into());
    spec.command = CommandSpec::Exec {
        program: "unrelated".into(),
        args: vec![],
    };
    for _ in 0..130 {
        let job = store.start(spec.clone()).await.unwrap();
        store
            .finish(job.job_id, "completed", Some(0), None, "fixture".into())
            .await
            .unwrap();
    }
    let query = TaskQuery {
        text: "检索".into(),
        status: Some("failed".into()),
        cwd: Some(dir.path().to_string_lossy().into()),
        created_after: Some(old.created_at),
        created_before: Some(old.created_at),
    };
    let first = store.search_tasks(query.clone(), None, 20).await.unwrap();
    assert_eq!(first["jobs"], json!([]));
    assert_eq!(first["has_more"], true);
    let second = store
        .search_tasks(query, first["next_cursor"].as_i64(), 20)
        .await
        .unwrap();
    assert_eq!(second["jobs"][0]["job_id"], old.job_id);
    assert_eq!(second["has_more"], false);
    let deep = store
        .search_tasks(
            TaskQuery {
                text: "深处参数".into(),
                ..Default::default()
            },
            Some(2),
            20,
        )
        .await
        .unwrap();
    assert_eq!(deep["jobs"][0]["run_id"], old.run_id);
    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&store.config.root)
        .args(["search", "--text", "深处参数", "--before", "2"])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&cli.stdout).unwrap()["jobs"][0]["job_id"],
        old.job_id
    );
    assert_eq!(store.data_status().await.unwrap()["archived_jobs"], 1);
    assert_eq!(
        store.status(old.job_id).await.unwrap().status,
        JobState::Failed
    );
    assert!(ipc::call(&store.config, Request::Doctor).await.is_err());
    assert!(
        store
            .search_tasks(
                TaskQuery {
                    status: Some("invented".into()),
                    ..Default::default()
                },
                None,
                20
            )
            .await
            .is_err()
    );
    assert!(
        store
            .search_tasks(
                TaskQuery {
                    created_after: Some(2),
                    created_before: Some(1),
                    ..Default::default()
                },
                None,
                20
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn literal_log_search_uses_original_bytes_context_and_fixed_end() {
    let (_dir, store, spec) = fixture().await;
    let job = store.start(spec).await.unwrap();
    let path = store.config.run_dir(&job.run_id);
    agent_job_relay::platform::secure_dir(&path).unwrap();
    let mut bytes = vec![b'x'; 600000];
    bytes[524287..524293].copy_from_slice(b"needle");
    bytes[599990..599996].copy_from_slice(b"needle");
    bytes[0] = 255;
    // Use the runner's explicit owner/private permissions on Windows too.
    std::io::Write::write_all(
        &mut agent_job_relay::platform::create_new_file(&path.join("stderr.log")).unwrap(),
        &bytes,
    )
    .unwrap();
    let request = |cursor, end, limit| Request::SearchLogs {
        job_id: job.job_id.clone(),
        text: "needle".into(),
        stream: "stderr".into(),
        cursor,
        end,
        limit,
    };
    let first = ipc::dispatch(&store, request(0, None, 20)).await.unwrap();
    assert_eq!(first["matches"][0]["offset"], 524287);
    assert_eq!(first["next_cursor"], 524293);
    assert_eq!(first["end"], 600000);
    let second = ipc::dispatch(
        &store,
        request(
            first["next_cursor"].as_u64().unwrap(),
            first["end"].as_u64(),
            20,
        ),
    )
    .await
    .unwrap();
    assert_eq!(second["matches"].as_array().unwrap().len(), 1);
    assert_eq!(second["matches"][0]["offset"], 599990);
    assert_eq!(second["complete"], true);
    let limited = ipc::dispatch(&store, request(0, None, 1)).await.unwrap();
    assert_eq!(limited["next_cursor"], 524293);
    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&store.config.root)
        .args([
            "search-logs",
            &job.job_id,
            "--text",
            "needle",
            "--stream",
            "stderr",
            "--cursor",
            "524293",
            "--end",
            "600000",
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&cli.stdout).unwrap()["matches"],
        second["matches"]
    );
    assert!(
        ipc::dispatch(
            &store,
            Request::SearchLogs {
                job_id: job.job_id,
                text: "".into(),
                stream: "stderr".into(),
                cursor: 0,
                end: None,
                limit: 20
            }
        )
        .await
        .is_err()
    );
}
