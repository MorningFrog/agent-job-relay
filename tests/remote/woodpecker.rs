use super::*;
use axum::{
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use std::sync::{Arc, Mutex};

struct Api {
    data: Arc<Mutex<Data>>,
    server: tokio::task::JoinHandle<()>,
    origin: String,
}
struct Data {
    pipeline: Value,
    repo: Value,
    status: StatusCode,
    calls: Vec<String>,
}
impl Api {
    async fn new() -> Self {
        let data = Arc::new(Mutex::new(Data {
            pipeline: json!({"id":69,"number":42,"commit":"a".repeat(40),"status":"running",
                "workflows":[{"id":7,"name":"Linux","state":"running","children":[
                    {"id":8,"state":"success","name":"clone"},{"id":9,"state":"running","name":"compile"}]}]}),
            repo: json!({"id":1,"full_name":"owner/repo"}),
            status: StatusCode::OK,
            calls: vec![],
        }));
        async fn handle(
            State(state): State<Arc<Mutex<Data>>>,
            method: Method,
            uri: Uri,
            headers: HeaderMap,
        ) -> Response {
            let mut state = state.lock().unwrap();
            assert_eq!(method, Method::GET, "observation must never mutate CI");
            assert_eq!(
                headers.get("authorization").unwrap(),
                "Bearer private-test-secret"
            );
            state.calls.push(uri.path().into());
            if !state.status.is_success() {
                return (
                    state.status,
                    [("location", "/must-not-follow")],
                    "token=private-test-secret",
                )
                    .into_response();
            }
            let body = match uri.path() {
                "/api/repos/1" => state.repo.clone(),
                "/api/repos/1/pipelines/42" => state.pipeline.clone(),
                _ => panic!("unexpected read {uri}"),
            };
            axum::Json(body).into_response()
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let router = axum::Router::new()
            .fallback(handle)
            .with_state(data.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            data,
            server,
            origin,
        }
    }
    fn command(&self, r: &Rig) -> CommandSpec {
        let path = r.dir.path().join("token");
        std::fs::write(&path, "private-test-secret\n").unwrap();
        CommandSpec::Woodpecker {
            pipeline: WoodpeckerSpec {
                server_url: self.origin.clone(),
                repository: "owner/repo".into(),
                repo_id: 1,
                pipeline_number: 42,
                pipeline_id: 69,
                commit: "a".repeat(40),
                token_file: Some(path),
            },
        }
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.server.abort();
    }
}

#[tokio::test]
async fn woodpecker_progress_stop_timeout_and_terminal_notification_are_read_only() {
    use agent_job_relay::session::{Adapter, Registration};
    let r = Rig::new().await;
    let api = Api::new().await;
    let command = api.command(&r);
    let session = r
        .store
        .register(Registration {
            agent_kind: "test".into(),
            session_id: "woodpecker-origin".into(),
            instance_id: id(),
            host_id: "fixture".into(),
            adapter: Adapter::Inbox,
            endpoint: None,
            caller_thread_id: None,
            lease_seconds: 3600,
            experimental: false,
            renew_on_delivery: false,
        })
        .await
        .unwrap();
    let mut spec = r.spec(command.clone());
    spec.session = Some(session.key.clone());
    let job = r.start(spec.clone()).await;
    let observed = r
        .wait(&job.job_id, |v| {
            v["observation"]["external"]["pipeline"]["workflows"]["items"][0]["current_step"]
                == "compile"
        })
        .await;
    assert_eq!(
        observed["observation"]["external"]["pipeline"]["url"],
        format!("{}/repos/1/pipeline/42", api.origin)
    );
    let retry = r.start(spec).await;
    assert_eq!(
        (retry.job_id, retry.run_id),
        (job.job_id.clone(), job.run_id.clone())
    );
    r.store.stop(job.job_id.clone()).await.unwrap();
    r.wait(&job.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(api.data.lock().unwrap().pipeline["status"], "running");
    let mut spec = r.spec(command.clone());
    spec.timeout_seconds = Some(2);
    let timeout = r.start(spec).await;
    let unknown = r
        .wait(&timeout.job_id, |v| v["job"]["status"] == "orphaned")
        .await;
    assert_eq!(
        unknown["observation"]["external"]["connection"],
        "observer_timeout"
    );
    assert_eq!(api.data.lock().unwrap().pipeline["status"], "running");
    {
        let mut data = api.data.lock().unwrap();
        data.pipeline["status"] = json!("success");
        // Absent detail cannot hide a verified pipeline completion.
        data.pipeline.as_object_mut().unwrap().remove("workflows");
    }
    let mut spec = r.spec(command);
    spec.session = Some(session.key.clone());
    let file = r.dir.path().join("pipeline.json");
    std::fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let output = tokio::process::Command::new(EXE)
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["start-spec", "--file"])
        .arg(&file)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let submitted: Value = serde_json::from_slice(&output.stdout).unwrap();
    let done = r
        .wait(submitted["job"]["job_id"].as_str().unwrap(), |v| {
            v["job"]["status"] == "completed"
                && !v["deliveries"]["deliveries"].as_array().unwrap().is_empty()
        })
        .await;
    assert_eq!(
        done["observation"]["external"]["pipeline"]["workflows_error"],
        "missing Woodpecker workflows"
    );
    assert!(!done.to_string().contains("private-test-secret"));
    assert_eq!(done["job"]["session"], session.key);
    let inbox = ipc::call(
        &r.config,
        Request::Inbox {
            session_key: session.key,
            after: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert!(
        inbox
            .to_string()
            .contains(done["job"]["job_id"].as_str().unwrap())
    );
    assert!(
        api.data.lock().unwrap().calls.iter().all(|p| [
            "/api/repos/1",
            "/api/repos/1/pipelines/42"
        ]
        .contains(&p.as_str()))
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn woodpecker_disconnect_and_observer_recovery_keep_original_run_and_stale_evidence() {
    let r = Rig::new().await;
    let api = Api::new().await;
    let spec = r.spec(api.command(&r));
    let job = r.start(spec.clone()).await;
    r.wait(&job.job_id, |v| {
        v["observation"]["external"]["connection"] == "connected"
    })
    .await;
    api.data.lock().unwrap().status = StatusCode::FORBIDDEN;
    let stale = r
        .wait(&job.job_id, |v| {
            v["observation"]["external"]["evidence_stale"] == true
        })
        .await;
    assert_eq!(stale["job"]["status"], "running");
    assert_eq!(
        stale["observation"]["external"]["pipeline"]["status"],
        "running"
    );
    assert!(stale.to_string().contains("authentication_or_permission"));
    assert!(!stale.to_string().contains("private-test-secret"));
    let pid = stale["job"]["runner_pid"].as_u64().unwrap() as u32;
    assert_eq!(
        agent_job_relay::platform::identity(pid).unwrap(),
        stale["job"]["runner_identity"].as_str().unwrap()
    );
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
    r.wait(&job.job_id, |v| {
        v["job"]["runner_pid"]
            .as_u64()
            .is_some_and(|p| p != pid as u64)
    })
    .await;
    {
        let mut data = api.data.lock().unwrap();
        data.status = StatusCode::OK;
        // A replacement/rerun cannot supply completion for the pinned pipeline.
        data.pipeline["id"] = json!(70);
        data.pipeline["status"] = json!("success");
    }
    let wrong = r
        .wait(&job.job_id, |v| {
            v["observation"]["external"]["last_error"]
                .as_str()
                .is_some_and(|s| s.contains("pipeline identity mismatch"))
        })
        .await;
    assert_eq!(wrong["job"]["status"], "running");
    assert_eq!(
        wrong["observation"]["external"]["pipeline"]["pipeline_id"],
        69
    );
    {
        let mut data = api.data.lock().unwrap();
        data.pipeline["id"] = json!(69);
        data.pipeline["status"] = json!("failure");
    }
    let done = r
        .wait(&job.job_id, |v| v["job"]["status"] == "failed")
        .await;
    assert_eq!(done["job"]["run_id"], job.run_id);
    assert_eq!(
        done["observation"]["external"]["pipeline"]["pipeline_number"],
        42
    );
    let retry = r.start(spec).await;
    assert_eq!((retry.job_id, retry.run_id), (job.job_id, job.run_id));
}
