#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    goal_control::{Controller, Goal},
    goal_wait,
    ipc::{self, Request},
    session::{Adapter, Registration},
    storage::Store,
};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

struct Fixture {
    dir: tempfile::TempDir,
    store: Store,
    controller: Controller,
    goal: Goal,
    session: String,
    host: tokio::task::JoinHandle<()>,
    messages: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
}
struct OwnedDaemon(std::process::Child);
impl std::ops::Deref for OwnedDaemon {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for OwnedDaemon {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for OwnedDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Fixture {
    async fn daemon(&self) -> OwnedDaemon {
        let daemon = OwnedDaemon(
            Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
                .args([
                    "--state-dir",
                    self.store.config.root.to_str().unwrap(),
                    "daemon",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            while ipc::call(&self.store.config, Request::Doctor)
                .await
                .is_err()
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        daemon
    }
    fn cli(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"));
        command
            .args(["--state-dir", self.store.config.root.to_str().unwrap()])
            .env("CODEX_THREAD_ID", &self.controller.thread_id)
            .env(
                "CODEX_APP_TOOLS_PIPE_PATH",
                self.dir.path().join("desktop.sock"),
            )
            .env("CODEX_APP_RUNTIME_CLI_PATH", &self.controller.executable)
            .env("CODEX_HOME", &self.controller.home)
            .current_dir(self.dir.path());
        command
    }
    async fn new() -> Self {
        agent_job_relay::platform::initialize();
        let dir = tempfile::Builder::new()
            .prefix("goal-")
            .tempdir_in(common::root())
            .unwrap();
        let home = dir.path().join("codex");
        std::fs::create_dir(&home).unwrap();
        let executable = dir.path().join("fake-codex");
        std::fs::write(&executable, include_str!("fixtures/fake-codex-goal.py")).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let thread = id();
        let goal = Goal {
            thread_id: thread.clone(),
            objective: "Wait for the exact Relay members".into(),
            status: "active".into(),
            token_budget: Some(5000),
            tokens_used: 73,
            time_used_seconds: 9,
            created_at: 10,
            updated_at: 10,
            extra: Default::default(),
        };
        std::fs::write(home.join("goal.json"), serde_json::to_vec(&goal).unwrap()).unwrap();
        let controller = Controller {
            executable_sha256: agent_job_relay::goal_control::executable_hash(&executable).unwrap(),
            executable,
            home,
            thread_id: thread.clone(),
            host_pause: true,
        };
        let endpoint = dir.path().join("desktop.sock");
        let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
        let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let output = messages.clone();
        let host = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let output = output.clone();
                tokio::spawn(async move {
                    let Ok(request) = ipc::read_frame(&mut socket, 65536).await else {
                        return;
                    };
                    let response = json!({"jsonrpc":"2.0","id":request["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}});
                    if ipc::write_frame(&mut socket, &response).await.is_err() {
                        return;
                    }
                    if let Ok(request) = ipc::read_frame(&mut socket, 65536).await {
                        output.lock().unwrap().push(request.clone());
                        let _ = ipc::write_frame(
                            &mut socket,
                            &json!({"jsonrpc":"2.0","id":request["id"],"result":{"success":true}}),
                        )
                        .await;
                    }
                });
            }
        });
        let store = Store::new(Config::new(Some(dir.path().join("state"))).unwrap());
        store.config.prepare().unwrap();
        store.init().await.unwrap();
        let session = store
            .register(Registration {
                agent_kind: "codex_desktop".into(),
                session_id: thread.clone(),
                instance_id: "fixture".into(),
                host_id: "owned-fixture".into(),
                adapter: Adapter::DesktopExperimental,
                endpoint: Some(endpoint),
                caller_thread_id: Some(thread),
                lease_seconds: 86400,
                experimental: true,
                renew_on_delivery: true,
            })
            .await
            .unwrap()
            .key;
        Self {
            dir,
            store,
            controller,
            goal,
            session,
            host,
            messages,
        }
    }
    async fn prepare(&self) -> String {
        self.store
            .prepare_goal_wait(
                self.session.clone(),
                self.controller.clone(),
                self.goal.clone(),
            )
            .await
            .unwrap()["goal_wait_id"]
            .as_str()
            .unwrap()
            .into()
    }
    fn spec(&self, key: &str) -> StartSpec {
        StartSpec {
            command: CommandSpec::Exec {
                program: "true".into(),
                args: vec![],
            },
            cwd: self.dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session: Some(self.session.clone()),
            idempotency_key: Some(key.into()),
            monitor_rules: vec![],
        }
    }
    async fn submit(&self, wait: &str, key: &str) -> Value {
        let mut value = self.admit(wait, key).await;
        if value["goal_wait"]["status"] == "sealed" {
            value["goal_wait"] = self.pause_and_confirm(wait).await;
        }
        value
    }
    async fn admit(&self, wait: &str, key: &str) -> Value {
        ipc::dispatch(
            &self.store,
            Request::StartGoal {
                spec: self.spec(key),
                goal_wait_id: wait.into(),
            },
        )
        .await
        .unwrap()
    }
    fn native_pause(&self) {
        // Simulate the submitting Agent's native host tool, independently of
        // the out-of-process controller's mutation log.
        let mut goal: Goal =
            serde_json::from_slice(&std::fs::read(self.controller.home.join("goal.json")).unwrap())
                .unwrap();
        goal.status = "paused".into();
        goal.updated_at += 1;
        self.write_goal(&goal);
    }
    async fn pause_and_confirm(&self, wait: &str) -> Value {
        let revision = self.state(wait).await["revision"].as_i64();
        self.native_pause();
        self.store
            .confirm_goal_pause(wait.into(), revision)
            .await
            .unwrap()
    }
    async fn finish(&self, response: &Value) {
        self.store
            .finish(
                response["job"]["job_id"].as_str().unwrap().into(),
                "completed",
                Some(0),
                None,
                "fixture terminal evidence".into(),
            )
            .await
            .unwrap();
    }
    fn write_goal(&self, goal: &Goal) {
        std::fs::write(
            self.controller.home.join("goal.json"),
            serde_json::to_vec(goal).unwrap(),
        )
        .unwrap();
    }
    fn mutations(&self) -> Vec<Value> {
        std::fs::read_to_string(self.controller.home.join("mutations.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    async fn state(&self, wait: &str) -> Value {
        self.store
            .goal_wait(wait.into(), false, None)
            .await
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.host.abort();
    }
}

#[tokio::test]
async fn storage_only_pause_never_acknowledges_the_host_and_confirmation_is_fenced() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let result = f.admit(&wait, "needs-native-pause").await;
    let revision = result["goal_wait"]["revision"].as_i64();
    assert_eq!(result["goal_wait"]["status"], "sealed");
    assert_eq!(result["feedback"]["status"], "goal_completion_only");
    let detail = f
        .store
        .inspect(
            Some(result["job"]["job_id"].as_str().unwrap().into()),
            None,
            20,
        )
        .await
        .unwrap();
    assert_eq!(detail["feedback"], result["feedback"]);
    assert_eq!(result["goal_wait"]["host_pause_required"], true);
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "active"
    );
    assert!(
        f.store
            .confirm_goal_pause(wait.clone(), revision)
            .await
            .is_err()
    );
    f.native_pause();
    // Even if another process or a user changed storage, polling/restart cannot
    // manufacture the submitting Agent's native-host acknowledgement.
    goal_wait::progress(&f.store, &wait).await.unwrap();
    assert_eq!(f.state(&wait).await["status"], "sealed");
    assert!(f.store.claim().await.unwrap().is_none());
    assert!(
        f.store
            .confirm_goal_pause(wait.clone(), None)
            .await
            .is_err()
    );
    assert!(
        f.store
            .confirm_goal_pause(wait.clone(), Some(0))
            .await
            .is_err()
    );
    let confirmed = f
        .store
        .confirm_goal_pause(wait.clone(), revision)
        .await
        .unwrap();
    assert_eq!(confirmed["status"], "waiting");
    assert_eq!(confirmed["host_pause_required"], false);
    let retry = f
        .store
        .confirm_goal_pause(wait.clone(), revision)
        .await
        .unwrap();
    assert_eq!(retry["revision"], confirmed["revision"]);
    assert!(
        ipc::dispatch(
            &f.store,
            Request::GoalWait {
                goal_wait_id: wait.clone(),
                cancel: true,
                confirm_paused: true,
                expected_revision: confirmed["revision"].as_i64()
            }
        )
        .await
        .is_err()
    );
    f.store
        .goal_wait(wait.clone(), true, confirmed["revision"].as_i64())
        .await
        .unwrap();
    assert!(
        f.store
            .confirm_goal_pause(wait.clone(), revision)
            .await
            .is_err()
    );
    assert!(f.mutations().is_empty());
    assert_eq!(f.store.list(0, 100).await.unwrap().len(), 1);
}

#[tokio::test]
async fn missing_pause_confirmation_expires_and_legacy_waits_cannot_restore_goals() {
    for legacy in [false, true] {
        let f = Fixture::new().await;
        let wait = f.prepare().await;
        let result = if legacy {
            f.submit(&wait, "legacy").await
        } else {
            f.admit(&wait, "no-confirmation").await
        };
        f.finish(&result).await;
        let key = wait.clone();
        f.store
            .call(move |db| {
                if legacy {
                    let raw: String = db.query_row(
                        "SELECT controller FROM goal_waits WHERE id=?",
                        [&key],
                        |r| r.get(0),
                    )?;
                    let mut controller: Value = serde_json::from_str(&raw)?;
                    controller.as_object_mut().unwrap().remove("host_pause");
                    db.execute(
                        "UPDATE goal_waits SET controller=?,next_at=0 WHERE id=?",
                        rusqlite::params![controller.to_string(), key],
                    )?;
                } else {
                    db.execute(
                        "UPDATE goal_waits SET expires=0,next_at=0 WHERE id=?",
                        [key],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        goal_wait::tick(&f.store).await.unwrap();
        assert_eq!(f.state(&wait).await["status"], "attention");
        assert!(
            f.store
                .confirm_goal_pause(wait.clone(), f.state(&wait).await["revision"].as_i64())
                .await
                .is_err()
        );
        assert!(f.mutations().is_empty());
        assert!(f.store.claim().await.unwrap().is_none());
        assert_eq!(
            f.store
                .status(result["job"]["job_id"].as_str().unwrap().into())
                .await
                .unwrap()
                .status
                .as_str(),
            "completed"
        );
    }
}

#[tokio::test]
async fn daemon_start_fences_legacy_restored_wait_before_pending_notification() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let result = f.submit(&wait, "legacy-notification").await;
    f.finish(&result).await;
    goal_wait::progress(&f.store, &wait).await.unwrap();
    assert_eq!(f.state(&wait).await["status"], "resumed");
    let key = wait.clone();
    f.store
        .call(move |db| {
            let raw: String = db.query_row(
                "SELECT controller FROM goal_waits WHERE id=?",
                [&key],
                |r| r.get(0),
            )?;
            let mut controller: Value = serde_json::from_str(&raw)?;
            controller.as_object_mut().unwrap().remove("host_pause");
            db.execute(
                "UPDATE goal_waits SET controller=? WHERE id=?",
                rusqlite::params![controller.to_string(), key],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let before = f.mutations().len();
    let mut daemon = OwnedDaemon(
        Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "daemon",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while ipc::call(&f.store.config, Request::Doctor).await.is_err() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.state(&wait).await["status"], "attention");
    assert!(f.store.claim().await.unwrap().is_none());
    assert!(f.messages.lock().unwrap().is_empty());
    assert_eq!(f.mutations().len(), before);
    assert_eq!(f.store.list(0, 100).await.unwrap().len(), 1);
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}

#[tokio::test]
async fn batch_freezes_partial_admission_waits_for_every_run_and_merges_notification() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let a = f.spec("a");
    let b = f.spec("b");
    let mut bad = f.spec("bad");
    bad.session = Some(id());
    let request = || Request::StartGoalBatch {
        specs: vec![a.clone(), bad.clone(), b.clone()],
        goal_wait_id: wait.clone(),
    };
    let mut result = ipc::dispatch(&f.store, request()).await.unwrap();
    assert_eq!(result["accepted"], 2);
    assert_eq!(result["rejected"], 1);
    assert_eq!(result["goal_wait"]["status"], "sealed");
    result["goal_wait"] = f.pause_and_confirm(&wait).await;
    assert_eq!(result["goal_wait"]["status"], "waiting");
    let again = ipc::dispatch(&f.store, request()).await.unwrap();
    assert_eq!(
        again["results"][0]["result"]["job"]["job_id"],
        result["results"][0]["result"]["job"]["job_id"]
    );
    assert!(
        f.mutations().is_empty(),
        "Relay must never issue a storage-only pause"
    );
    let first = &result["results"][0]["result"];
    let second = &result["results"][2]["result"];
    f.finish(first).await;
    goal_wait::progress(&f.store, &wait).await.unwrap();
    assert!(f.store.claim().await.unwrap().is_none());
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "paused"
    );
    f.finish(second).await;
    goal_wait::progress(&f.store, &wait).await.unwrap();
    let active = f.controller.read().await.unwrap().unwrap();
    assert_eq!(active.status.as_str(), "active");
    assert_eq!(active.tokens_used, 73);
    assert_eq!(active.token_budget, Some(5000));
    let claim = f.store.claim().await.unwrap().unwrap();
    let same = claim.delivery.id.clone();
    let worker = tokio::spawn(agent_job_relay::delivery::worker(f.store.clone()));
    // Release this artificial claim so the real dispatcher can own the attempt.
    let mut expired = claim.clone();
    expired.token = "wrong".into();
    assert!(
        !f.store
            .complete_claim(expired, Ok("accepted_by_host"))
            .await
            .unwrap()
    );
    f.store
        .complete_claim(claim, Err(anyhow::anyhow!("fixture retry")))
        .await
        .unwrap();
    let delivery = same.clone();
    f.store
        .call(move |db| {
            db.execute("UPDATE deliveries SET next_at=0 WHERE id=?", [delivery])?;
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            if f.state(&wait).await["notification_accepted"] == true {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    worker.abort();
    let _ = worker.await;
    assert_eq!(f.messages.lock().unwrap().len(), 1);
    let prompt = f.messages.lock().unwrap()[0]["params"]["arguments"]["prompt"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        prompt.contains(first["job"]["job_id"].as_str().unwrap())
            && prompt.contains(second["job"]["job_id"].as_str().unwrap())
    );
    assert!(!prompt.contains(&f.goal.objective));
    assert!(f.store.claim().await.unwrap().is_none());
    assert_eq!(
        f.mutations()
            .iter()
            .filter(|v| v["status"] == "active")
            .count(),
        1
    );
    let job = first["job"]["job_id"].as_str().unwrap().to_string();
    assert!(
        ipc::dispatch(&f.store, Request::Status { job_id: job })
            .await
            .unwrap()
            .get("goal_wait")
            .is_some()
    );
}

#[tokio::test]
async fn deleted_replaced_manually_paused_and_late_deleted_goals_never_wake() {
    for mode in ["deleted", "replaced", "touched", "delete_on_resume"] {
        let f = Fixture::new().await;
        let wait = f.prepare().await;
        let result = f.submit(&wait, "one").await;
        f.finish(&result).await;
        match mode {
            "deleted" => std::fs::remove_file(f.controller.home.join("goal.json")).unwrap(),
            "replaced" => {
                let mut goal = f.goal.clone();
                goal.objective = "New user goal".into();
                goal.status = "paused".into();
                goal.created_at += 1;
                f.write_goal(&goal);
            }
            "touched" => {
                let mut goal = f.controller.read().await.unwrap().unwrap();
                goal.updated_at += 1;
                f.write_goal(&goal);
            }
            _ => std::fs::write(f.controller.home.join("mode"), mode).unwrap(),
        }
        goal_wait::progress(&f.store, &wait).await.unwrap();
        assert!(matches!(
            f.state(&wait).await["status"].as_str().unwrap(),
            "suppressed" | "attention"
        ));
        assert!(f.store.claim().await.unwrap().is_none());
        assert!(f.messages.lock().unwrap().is_empty());
        if mode.starts_with("delete") {
            assert!(f.controller.read().await.unwrap().is_none());
        }
        for mutation in f.mutations() {
            assert!(mutation.get("objective").is_none());
            assert!(mutation.get("tokenBudget").is_none());
        }
    }
}

#[tokio::test]
async fn lost_mutation_acknowledgements_are_durable_and_not_replayed() {
    {
        let f = Fixture::new().await;
        let wait = f.prepare().await;
        let result = f.submit(&wait, "one").await;
        std::fs::write(f.controller.home.join("mode"), "drop_resume").unwrap();
        f.finish(&result).await;
        goal_wait::progress(&f.store, &wait).await.unwrap();
        assert_eq!(f.state(&wait).await["status"], "attention");
        let before = f.mutations().len();
        let reopened = Store::new(f.store.config.clone());
        reopened.init().await.unwrap();
        goal_wait::tick(&reopened).await.unwrap();
        assert_eq!(f.mutations().len(), before);
        assert!(reopened.claim().await.unwrap().is_none());
    }
    for phase in ["pausing", "resuming"] {
        let f = Fixture::new().await;
        let wait = f.prepare().await;
        let _ = f.submit(&wait, "interrupted").await;
        let id = wait.clone();
        let state = phase.to_string();
        f.store
            .call(move |db| {
                db.execute(
                    "UPDATE goal_waits SET status=? WHERE id=?",
                    rusqlite::params![state, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let before = f.mutations().len();
        goal_wait::tick(&f.store).await.unwrap();
        assert_eq!(f.state(&wait).await["status"], "attention");
        assert_eq!(f.mutations().len(), before);
    }
}

#[tokio::test]
async fn explicit_cancel_manual_stop_and_stale_revisions_leave_host_goal_and_other_jobs_alone() {
    for manual_stop in [false, true] {
        let f = Fixture::new().await;
        let other = f.store.start(f.spec("unrelated")).await.unwrap();
        let wait = f.prepare().await;
        let result = f.submit(&wait, "one").await;
        let state = f.state(&wait).await;
        assert!(
            f.store
                .goal_wait(wait.clone(), true, Some(0))
                .await
                .is_err()
        );
        if manual_stop {
            f.store
                .stop(result["job"]["job_id"].as_str().unwrap().into())
                .await
                .unwrap();
        } else {
            f.store
                .goal_wait(wait.clone(), true, state["revision"].as_i64())
                .await
                .unwrap();
            f.finish(&result).await;
        }
        assert_eq!(f.state(&wait).await["status"], "cancelled");
        goal_wait::progress(&f.store, &wait).await.unwrap();
        assert_eq!(
            f.controller.read().await.unwrap().unwrap().status.as_str(),
            "paused"
        );
        assert_eq!(
            f.store.status(other.job_id).await.unwrap().status.as_str(),
            "queued"
        );
        assert!(f.store.claim().await.unwrap().is_none());
        assert!(f.mutations().is_empty());
    }
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let result = f.submit(&wait, "cancel-during-restore-read").await;
    f.finish(&result).await;
    std::fs::write(f.controller.home.join("mode"), "hold_second_read").unwrap();
    let (store, key) = (f.store.clone(), wait.clone());
    let resume = tokio::spawn(async move { goal_wait::progress(&store, &key).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !f.controller.home.join("blocked").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let state = f.state(&wait).await;
    assert_eq!(state["status"], "resuming");
    f.store
        .goal_wait(wait.clone(), true, state["revision"].as_i64())
        .await
        .unwrap();
    std::fs::write(f.controller.home.join("release"), "continue").unwrap();
    resume.await.unwrap().unwrap();
    assert_eq!(f.state(&wait).await["status"], "cancelled");
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "paused"
    );
    assert_eq!(
        f.mutations().len(),
        0,
        "cancelled control must be rechecked immediately before sending"
    );
}

#[tokio::test]
async fn expired_preparation_caller_conflicts_and_executable_replacement_refuse_changes() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let mut wrong = f.spec("wrong");
    wrong.session = Some(id());
    assert!(
        ipc::dispatch(
            &f.store,
            Request::StartGoal {
                spec: wrong,
                goal_wait_id: wait.clone()
            }
        )
        .await
        .is_err()
    );
    assert!(f.store.list(0, 100).await.unwrap().is_empty());
    assert_eq!(
        f.prepare().await,
        wait,
        "lost binding responses must retain the same unused wait ID and expiry"
    );
    let key = wait.clone();
    f.store
        .call(move |db| {
            db.execute("UPDATE goal_waits SET expires=0 WHERE id=?", [key])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        ipc::dispatch(
            &f.store,
            Request::StartGoal {
                spec: f.spec("expired"),
                goal_wait_id: wait
            }
        )
        .await
        .is_err()
    );
    assert!(f.store.list(0, 100).await.unwrap().is_empty());
    std::fs::write(&f.controller.executable, "changed").unwrap();
    assert!(f.controller.read().await.is_err());
    assert!(f.mutations().is_empty());
}

#[tokio::test]
async fn backup_restore_fences_waits_and_retains_member_audits_without_host_control() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let result = f.submit(&wait, "one").await;
    f.finish(&result).await;
    let backup = f.store.backup().await.unwrap();
    let to = f.dir.path().join("restored");
    agent_job_relay::maintenance::restore(
        PathBuf::from(backup["path"].as_str().unwrap()),
        to.clone(),
    )
    .await
    .unwrap();
    let restored = Store::new(Config::new(Some(to)).unwrap());
    restored.init().await.unwrap();
    assert_eq!(
        restored.goal_wait(wait.clone(), false, None).await.unwrap()["status"],
        "cancelled"
    );
    goal_wait::tick(&restored).await.unwrap();
    assert!(f.mutations().is_empty());
    assert!(restored.claim().await.unwrap().is_none());
    assert_eq!(
        restored
            .status(result["job"]["job_id"].as_str().unwrap().into())
            .await
            .unwrap()
            .status
            .as_str(),
        "completed"
    );
    assert_eq!(
        Store::readonly(restored.config.clone())
            .goal_wait(wait, false, None)
            .await
            .unwrap()["status"],
        "cancelled"
    );
}

#[tokio::test]
async fn missing_goal_pause_authority_stops_before_connect_single_or_batch_admission() {
    for route in ["connect", "single", "batch"] {
        let f = Fixture::new().await;
        let mut daemon = f.daemon().await;
        let mut command = f.cli();
        if route == "connect" {
            command.arg("connect");
        } else if route == "batch" {
            let file = f.dir.path().join("batch.json");
            let mut spec = f.spec("automatic-batch");
            spec.session = None;
            std::fs::write(&file, serde_json::to_vec(&vec![spec]).unwrap()).unwrap();
            command.args(["start-batch", "--wake", "--file"]).arg(file);
        } else {
            command.args(["start", "--wake", "--", "/bin/true"]);
        }
        let response = command.output().await.unwrap();
        assert!(
            response.status.success(),
            "{}",
            String::from_utf8_lossy(&response.stderr)
        );
        let response: Value = serde_json::from_slice(&response.stdout).unwrap();
        assert_eq!(response["status"], "needs_confirmation");
        assert_eq!(response["consent"]["status"], "pending");
        assert!(response.get("binding").is_none());
        assert!(f.mutations().is_empty());
        assert_eq!(f.controller.read().await.unwrap(), Some(f.goal.clone()));
        assert!(f.store.list(0, 100).await.unwrap().is_empty());
        assert_eq!(
            f.store
                .read(
                    |db| Ok(db.query_row("SELECT count(*) FROM goal_waits", [], |r| r
                        .get::<_, i64>(0))?)
                )
                .await
                .unwrap(),
            0
        );
        assert!(f.messages.lock().unwrap().is_empty());
        ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
        daemon.wait().unwrap();
    }
}

#[tokio::test]
async fn default_detection_preserves_inactive_goals_and_refuses_unverifiable_goal_state() {
    for mode in ["absent", "paused", "different-goal"] {
        let f = Fixture::new().await;
        let mut daemon = f.daemon().await;
        match mode {
            "absent" => std::fs::remove_file(f.controller.home.join("goal.json")).unwrap(),
            _ => {
                let mut goal = f.goal.clone();
                if mode == "paused" {
                    goal.status = "paused".into();
                } else {
                    goal.thread_id = id();
                }
                f.write_goal(&goal);
            }
        }
        let result = f.cli().arg("connect").output().await.unwrap();
        if mode == "different-goal" {
            assert!(!result.status.success());
        } else {
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let value: Value = serde_json::from_slice(&result.stdout).unwrap();
            assert!(value["binding"].get("goal_wait_id").is_none());
            assert_eq!(value["goal_state_verified"], true);
            assert_eq!(value["automatic_goal_pause_supported"], false);
            let started = f
                .cli()
                .args(["start", "--wake", "--", "/bin/true"])
                .output()
                .await
                .unwrap();
            assert!(
                started.status.success(),
                "{}",
                String::from_utf8_lossy(&started.stderr)
            );
            let started: Value = serde_json::from_slice(&started.stdout).unwrap();
            assert!(started.get("goal_wait").is_none());
            let job = started["job"]["job_id"].as_str().unwrap();
            tokio::time::timeout(Duration::from_secs(8), async {
                while !f
                    .store
                    .status(job.into())
                    .await
                    .unwrap()
                    .status
                    .is_terminal()
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();
            assert_eq!(f.store.status(job.into()).await.unwrap().exit_code, Some(0));
        }
        assert!(f.mutations().is_empty());
        assert_eq!(
            f.store.list(0, 100).await.unwrap().len(),
            usize::from(mode != "different-goal")
        );
        ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
        daemon.wait().unwrap();
    }
}

#[tokio::test]
async fn real_cli_binding_and_runner_survive_daemon_loss_and_finish_one_wait() {
    let f = Fixture::new().await;
    let mut daemon = OwnedDaemon(
        Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "daemon",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if ipc::call(&f.store.config, Request::Doctor).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let bind = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .args([
            "--state-dir",
            f.store.config.root.to_str().unwrap(),
            "connect",
            "--wait-goal",
        ])
        .env("CODEX_THREAD_ID", &f.controller.thread_id)
        .env(
            "CODEX_APP_TOOLS_PIPE_PATH",
            f.dir.path().join("desktop.sock"),
        )
        .env("CODEX_APP_RUNTIME_CLI_PATH", &f.controller.executable)
        .env("CODEX_HOME", &f.controller.home)
        .output()
        .await
        .unwrap();
    assert!(
        bind.status.success(),
        "{}",
        String::from_utf8_lossy(&bind.stderr)
    );
    let binding: Value = serde_json::from_slice(&bind.stdout).unwrap();
    let wait = binding["binding"]["goal_wait_id"]
        .as_str()
        .unwrap()
        .to_string();
    let session = binding["binding"]["session_key"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        binding["goal_wait_policy"],
        "required_for_active_desktop_goal"
    );
    let mut ordinary = f.spec("binding-must-not-be-stripped");
    ordinary.session = Some(session.clone());
    assert!(
        ipc::call(&f.store.config, Request::Start { spec: ordinary })
            .await
            .is_err()
    );
    assert!(f.store.list(0, 100).await.unwrap().is_empty());
    let gate = f.dir.path().join("go");
    let mut spec = f.spec("real");
    spec.session = Some(session);
    spec.timeout_seconds = Some(10);
    spec.command = CommandSpec::Shell {
        interpreter: "/bin/sh".into(),
        script: format!(
            "while [ ! -e '{}' ]; do sleep 0.05; done; echo survived",
            gate.display()
        ),
    };
    let response = ipc::call(
        &f.store.config,
        Request::StartGoal {
            spec: spec.clone(),
            goal_wait_id: wait.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(response["goal_wait"]["status"], "sealed");
    f.native_pause();
    let confirmed = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .args([
            "--state-dir",
            f.store.config.root.to_str().unwrap(),
            "goal-wait",
            &wait,
            "--confirm-paused",
            "--expected-revision",
            &response["goal_wait"]["revision"].to_string(),
        ])
        .output()
        .await
        .unwrap();
    assert!(
        confirmed.status.success(),
        "{}",
        String::from_utf8_lossy(&confirmed.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&confirmed.stdout).unwrap()["status"],
        "waiting"
    );
    let job = response["job"]["job_id"].as_str().unwrap().to_string();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if f.store.status(job.clone()).await.unwrap().status.as_str() == "running" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::write(gate, "release").unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if f.store.status(job.clone()).await.unwrap().status.as_str() == "completed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "paused"
    );
    daemon = OwnedDaemon(
        Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "daemon",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if f.state(&wait).await["notification_accepted"] == true {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    let again = ipc::call(
        &f.store.config,
        Request::StartGoal {
            spec,
            goal_wait_id: wait.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(again["job"]["job_id"], job);
    assert_eq!(f.messages.lock().unwrap().len(), 1);
    assert_eq!(f.mutations().len(), 1);
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}

#[tokio::test]
async fn interrupted_admission_recovers_committed_members_without_replaying_execution() {
    let f = Fixture::new().await;
    let spec = f.spec("interrupted-admission");
    let job = f.store.start(spec.clone()).await.unwrap();
    let wait = f.prepare().await;
    let (key, job_id, run_id) = (wait.clone(), job.job_id.clone(), job.run_id.clone());
    // Reconstruct a committed admission whose submitting process died before sealing.
    f.store
        .call(move |db| {
            db.execute(
                "INSERT INTO goal_wait_jobs(job_id,run_id,wait_id) VALUES(?,?,?)",
                rusqlite::params![job_id, run_id, key],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut daemon = OwnedDaemon(
        Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "daemon",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        while f.state(&wait).await["status"] == "prepared" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(f.state(&wait).await["status"], "sealed");
        assert!(
            f.mutations().is_empty(),
            "recovery cannot simulate the owning host's pause"
        );
        f.pause_and_confirm(&wait).await;
        while f.state(&wait).await["notification_accepted"] != true {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    let retry = ipc::call(
        &f.store.config,
        Request::StartGoal {
            spec,
            goal_wait_id: wait.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(retry["job"]["job_id"], job.job_id);
    assert_eq!(retry["job"]["run_id"], job.run_id);
    assert_eq!(f.store.list(0, 100).await.unwrap().len(), 1);
    assert_eq!(f.mutations().len(), 1);
    assert_eq!(f.messages.lock().unwrap().len(), 1);
    let state = f.state(&wait).await;
    assert!(state["audits"].as_array().unwrap().iter().any(|a| {
        a["detail"]
            .as_str()
            .unwrap()
            .contains("interrupted admission")
    }));
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}

#[tokio::test]
async fn recovery_skips_pinned_waits_and_still_recovers_unrelated_targets() {
    let f = Fixture::new().await;
    let other = f.store.start(f.spec("ordinary")).await.unwrap();
    let wait = f.prepare().await;
    let member = f.submit(&wait, "member").await;
    let mut registration = f
        .store
        .session(f.session.clone())
        .await
        .unwrap()
        .registration;
    registration.instance_id = "replacement-instance".into();
    let replacement = f.store.register(registration).await.unwrap();
    assert!(
        f.store
            .target(
                member["job"]["job_id"].as_str().unwrap().into(),
                replacement.key.clone(),
                false
            )
            .await
            .is_err()
    );
    let recovered = f
        .store
        .recover_session(replacement.clone(), true, 0, 100)
        .await
        .unwrap();
    let rows = recovered["targets"].as_array().unwrap();
    let pinned = rows
        .iter()
        .find(|r| r["job_id"] == member["job"]["job_id"])
        .unwrap();
    assert_eq!(pinned["applied"], false);
    assert!(
        pinned["skipped_reason"]
            .as_str()
            .unwrap()
            .contains("goal_wait")
    );
    let ordinary = rows.iter().find(|r| r["job_id"] == other.job_id).unwrap();
    assert_eq!(ordinary["applied"], true);
    assert_eq!(f.state(&wait).await["session_key"], f.session);
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "paused"
    );
}

#[tokio::test]
async fn cold_archival_preserves_wait_members_audits_and_idempotent_admission() {
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let result = f.submit(&wait, "archived").await;
    f.finish(&result).await;
    let state = f.state(&wait).await;
    f.store
        .goal_wait(wait.clone(), true, state["revision"].as_i64())
        .await
        .unwrap();
    f.store
        .data_policy(
            Some(agent_job_relay::maintenance::Policy {
                archive_after_days: 0,
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
    assert_eq!(
        f.store
            .read(|db| Ok(db.query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))?))
            .await
            .unwrap(),
        0
    );
    let archived = f.state(&wait).await;
    assert_eq!(archived["jobs"], state["jobs"]);
    assert_eq!(archived["status"], "cancelled");
    assert!(!archived["audits"].as_array().unwrap().is_empty());
    let retry = f.submit(&wait, "archived").await;
    assert_eq!(retry["job"]["job_id"], result["job"]["job_id"]);
    assert_eq!(retry["goal_wait"]["status"], "cancelled");
    assert!(f.mutations().is_empty());
}

#[tokio::test]
async fn shared_mcp_uses_only_the_bound_wait_and_exposes_revision_fenced_cancellation() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let f = Fixture::new().await;
    let wait = f.prepare().await;
    let mut daemon = OwnedDaemon(
        Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "daemon",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if ipc::call(&f.store.config, Request::Doctor).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .args([
            "--state-dir",
            f.store.config.root.to_str().unwrap(),
            "mcp",
            "--shared-host",
        ])
        .env("CODEX_THREAD_ID", id())
        .env(
            "CODEX_APP_TOOLS_PIPE_PATH",
            f.dir.path().join("not-the-caller.sock"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    async fn rpc(
        input: &mut tokio::process::ChildStdin,
        lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        id: u32,
        method: &str,
        params: Value,
    ) -> Value {
        input
            .write_all(
                format!(
                    "{}\n",
                    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        input.flush().await.unwrap();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let value: Value =
                    serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
                if value["id"] == id {
                    return value;
                }
            }
        })
        .await
        .unwrap()
    }
    rpc(&mut input,&mut lines,1,"initialize",json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"goal-binding-fixture","version":"1"}})).await;
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    input.flush().await.unwrap();
    let tools = rpc(&mut input, &mut lines, 2, "tools/list", json!({})).await;
    let tools = tools["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "job_goal_wait"));
    assert!(
        !tools
            .iter()
            .any(|t| t["name"] == "job_prepare_goal_wait" || t["name"] == "job_start_goal")
    );
    let report = rpc(
        &mut input,
        &mut lines,
        20,
        "tools/call",
        json!({"name":"job_doctor","arguments":{}}),
    )
    .await;
    let report: Value =
        serde_json::from_str(report["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["goal_wait_host_pause"], true);
    assert_eq!(report["automatic_goal_pause_supported"], false);
    assert_eq!(
        report["goal_pause_authorization_policy"],
        "ask_once_per_request"
    );
    let binding = json!({"session_key":f.session,"cwd":f.dir.path(),"goal_wait_id":wait});
    let bad=rpc(&mut input,&mut lines,3,"tools/call",json!({"name":"job_start","arguments":{"binding":binding,"spec":{"command":{"kind":"exec","program":"true","args":[]},"session":id()}}})).await;
    assert_eq!(bad["result"]["isError"], true);
    assert!(f.store.list(0, 100).await.unwrap().is_empty());
    let started=rpc(&mut input,&mut lines,4,"tools/call",json!({"name":"job_start","arguments":{"binding":binding,"spec":{"command":{"kind":"exec","program":"sleep","args":["10"]},"timeout_seconds":15}}})).await;
    assert_ne!(started["result"]["isError"], true, "{started}");
    let result: Value =
        serde_json::from_str(started["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result["goal_wait"]["status"], "sealed");
    f.native_pause();
    let confirmed=rpc(&mut input,&mut lines,5,"tools/call",json!({"name":"job_goal_wait","arguments":{"goal_wait_id":wait,"confirm_paused":true,"expected_revision":result["goal_wait"]["revision"]}})).await;
    assert_ne!(confirmed["result"]["isError"], true, "{confirmed}");
    let confirmed: Value =
        serde_json::from_str(confirmed["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(confirmed["status"], "waiting");
    let job = result["job"]["job_id"].as_str().unwrap();
    let cancelled=rpc(&mut input,&mut lines,6,"tools/call",json!({"name":"job_goal_wait","arguments":{"goal_wait_id":wait,"cancel":true,"expected_revision":confirmed["revision"]}})).await;
    assert_ne!(cancelled["result"]["isError"], true);
    assert_eq!(
        f.controller.read().await.unwrap().unwrap().status.as_str(),
        "paused"
    );
    assert!(matches!(
        f.store.status(job.into()).await.unwrap().status.as_str(),
        "queued" | "launching" | "running"
    ));
    ipc::call(
        &f.store.config,
        Request::Stop {
            job_id: job.into(),
            grace_seconds: 0,
            wait_seconds: 5,
        },
    )
    .await
    .unwrap();
    drop(input);
    let _ = child.wait().await;
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
    assert!(f.messages.lock().unwrap().is_empty());
}

impl Fixture {
    async fn consent(&self, jobs: u32) -> Value {
        self.store
            .prepare_goal_consent(
                self.session.clone(),
                self.controller.clone(),
                self.goal.clone(),
                self.dir.path().into(),
                agent_job_relay::goal_consent::Options {
                    label: Some("three separate scans".into()),
                    job_limit: Some(jobs),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
    }
    async fn approve_consent(&self, c: &Value) -> Value {
        self.store
            .decide_goal_consent(
                c["request_id"].as_str().unwrap().into(),
                self.session.clone(),
                c["revision"].as_i64().unwrap(),
                agent_job_relay::goal_consent::Decision::Allow,
                "simulated explicit user answer",
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn scoped_consent_survives_reconnect_and_admits_exactly_three_serial_runs() {
    let f = Fixture::new().await;
    let pending = f.consent(3).await;
    let request = pending["request_id"].as_str().unwrap().to_owned();
    let mut later = f.goal.clone();
    later.tokens_used += 5;
    later.updated_at += 1;
    f.write_goal(&later);
    let repeated = f.consent(3).await;
    assert_eq!(repeated["request_id"], pending["request_id"]);
    assert_eq!(repeated["expires_at"], pending["expires_at"]);
    assert!(f.store.start(f.spec("bypass-consent")).await.is_err());
    let approved = f.approve_consent(&pending).await;
    assert!(
        f.store
            .decide_goal_consent(
                request.clone(),
                f.session.clone(),
                i64::MAX,
                agent_job_relay::goal_consent::Decision::Allow,
                "invalid revision"
            )
            .await
            .is_err()
    );
    let repeated = f.approve_consent(&pending).await;
    assert_eq!(
        approved, repeated,
        "answer retry must not renew or expand consent"
    );
    let delivery = tokio::spawn(agent_job_relay::delivery::worker(f.store.clone()));
    for round in 0..3 {
        let wait = f
            .store
            .prepare_goal_wait_scoped(
                f.session.clone(),
                f.controller.clone(),
                f.goal.clone(),
                Some(request.clone()),
            )
            .await
            .unwrap();
        let wait = wait["goal_wait_id"].as_str().unwrap();
        let spec = f.spec(&format!("consent-round-{round}"));
        if round == 0 {
            let mut moved = spec.clone();
            moved.cwd = f.dir.path().parent().unwrap().into();
            assert!(
                ipc::dispatch(
                    &f.store,
                    Request::StartGoal {
                        spec: moved,
                        goal_wait_id: wait.into()
                    }
                )
                .await
                .is_err()
            );
            assert_eq!(
                f.store
                    .goal_consent_view(request.clone(), f.session.clone())
                    .await
                    .unwrap()["jobs_admitted"],
                0
            );
        }
        let start = ipc::dispatch(
            &f.store,
            Request::StartGoal {
                spec: spec.clone(),
                goal_wait_id: wait.into(),
            },
        )
        .await
        .unwrap();
        let duplicate = ipc::dispatch(
            &f.store,
            Request::StartGoal {
                spec,
                goal_wait_id: wait.into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(start["job"]["job_id"], duplicate["job"]["job_id"]);
        assert_eq!(f.pause_and_confirm(wait).await["status"], "waiting");
        f.finish(&start).await;
        goal_wait::progress(&f.store, wait).await.unwrap();
        tokio::time::timeout(Duration::from_secs(8), async {
            while f.state(wait).await["notification_accepted"] != true {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        let consent = f
            .store
            .goal_consent_view(request.clone(), f.session.clone())
            .await
            .unwrap();
        assert_eq!(consent["jobs_admitted"], round + 1);
        assert_eq!(
            consent["jobs"].as_array().unwrap().len(),
            round as usize + 1
        );
        assert_eq!(
            f.controller.read().await.unwrap().unwrap().status.as_str(),
            "active"
        );
    }
    assert_eq!(
        f.store
            .goal_consent_view(request.clone(), f.session.clone())
            .await
            .unwrap()["status"],
        "exhausted"
    );
    assert!(
        f.store
            .prepare_goal_wait_scoped(
                f.session.clone(),
                f.controller.clone(),
                f.goal.clone(),
                Some(request)
            )
            .await
            .is_err()
    );
    assert_eq!(f.store.list(0, 100).await.unwrap().len(), 3);
    assert_eq!(f.messages.lock().unwrap().len(), 3);
    delivery.abort();
    let _ = delivery.await;
}

#[tokio::test]
async fn scoped_consent_fences_question_claims_callers_changes_expiry_and_restore() {
    use agent_job_relay::goal_consent::{Decision, Options};
    let f = Fixture::new().await;
    let pending = f.consent(3).await;
    let id = pending["request_id"].as_str().unwrap().to_string();
    let question = f
        .store
        .claim_goal_question(id.clone(), f.session.clone(), 1, false)
        .await
        .unwrap();
    assert!(
        f.store
            .claim_goal_question(id.clone(), f.session.clone(), 1, false)
            .await
            .is_err()
    );
    assert_eq!(f.consent(3).await["status"], "asking");
    assert!(
        f.store
            .goal_consent_record(id.clone(), agent_job_relay::core::id())
            .await
            .is_err()
    );
    assert!(
        f.store
            .prepare_goal_consent(
                f.session.clone(),
                f.controller.clone(),
                f.goal.clone(),
                f.dir.path().into(),
                Options {
                    request_id: Some(id.clone()),
                    job_limit: Some(4),
                    ..Default::default()
                }
            )
            .await
            .is_err()
    );
    let approved = f
        .store
        .decide_goal_consent(
            id.clone(),
            f.session.clone(),
            question.revision,
            Decision::Allow,
            "simulated host question reply",
        )
        .await
        .unwrap();
    let mut moved_controller = f.controller.clone();
    moved_controller.home = f.dir.path().join("other-controller-profile");
    std::fs::create_dir(&moved_controller.home).unwrap();
    std::fs::write(
        moved_controller.home.join("goal.json"),
        serde_json::to_vec(&f.goal).unwrap(),
    )
    .unwrap();
    assert!(
        f.store
            .prepare_goal_wait_scoped(
                f.session.clone(),
                moved_controller,
                f.goal.clone(),
                Some(id.clone())
            )
            .await
            .is_err()
    );
    let backup = f.store.backup().await.unwrap();
    let restored_path = f.dir.path().join("consent-restore");
    agent_job_relay::maintenance::restore(
        backup["path"].as_str().unwrap().into(),
        restored_path.clone(),
    )
    .await
    .unwrap();
    let restored = Store::new(Config::new(Some(restored_path)).unwrap());
    restored.init().await.unwrap();
    assert_eq!(
        restored
            .read(move |db| Ok(db
                .query_row("SELECT status FROM goal_consents", [], |r| r
                    .get::<_, String>(0))?))
            .await
            .unwrap(),
        "invalidated"
    );
    let wait = f
        .store
        .prepare_goal_wait_scoped(
            f.session.clone(),
            f.controller.clone(),
            f.goal.clone(),
            Some(id.clone()),
        )
        .await
        .unwrap();
    let wait = wait["goal_wait_id"].as_str().unwrap();
    let started = f.admit(wait, "revoked-request").await;
    f.pause_and_confirm(wait).await;
    let revoked = f
        .store
        .decide_goal_consent(
            id.clone(),
            f.session.clone(),
            approved["revision"].as_i64().unwrap(),
            Decision::Revoke,
            "user revoked this request",
        )
        .await
        .unwrap();
    assert_eq!(revoked["status"], "revoked");
    assert_eq!(f.state(wait).await["status"], "cancelled");
    assert_eq!(
        f.store
            .status(started["job"]["job_id"].as_str().unwrap().into())
            .await
            .unwrap()
            .status
            .as_str(),
        "queued",
        "revocation must not stop or replay execution"
    );
    f.finish(&started).await;
    assert!(f.mutations().is_empty());
}

#[tokio::test]
async fn unavailable_controller_preserves_approved_consent_and_acknowledged_wait() {
    let f = Fixture::new().await;
    let pending = f.consent(2).await;
    let approved = f.approve_consent(&pending).await;
    let request = approved["request_id"].as_str().unwrap().to_owned();
    let wait = f
        .store
        .prepare_goal_wait_scoped(
            f.session.clone(),
            f.controller.clone(),
            f.goal.clone(),
            Some(request.clone()),
        )
        .await
        .unwrap();
    let wait = wait["goal_wait_id"].as_str().unwrap();
    let started = f.submit(wait, "read-failure-member").await;
    let before = f.state(wait).await;
    std::fs::write(f.controller.home.join("mode"), "disconnect_read").unwrap();
    assert!(
        f.store
            .goal_consent_record(request.clone(), f.session.clone())
            .await
            .is_err()
    );
    assert!(
        f.store
            .clear_obsolete_goal_consents(f.session.clone())
            .await
            .is_err()
    );
    assert_eq!(f.state(wait).await, before);
    let key = request.clone();
    assert_eq!(
        f.store
            .read(move |db| Ok(db.query_row(
                "SELECT status,revision,used FROM goal_consents WHERE id=?",
                [key],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?
                )),
            )?))
            .await
            .unwrap(),
        ("approved".into(), approved["revision"].as_i64().unwrap(), 1)
    );
    assert!(f.mutations().is_empty());
    std::fs::remove_file(f.controller.home.join("mode")).unwrap();
    let restored = f
        .store
        .goal_consent_view(request, f.session.clone())
        .await
        .unwrap();
    assert_eq!(restored["status"], "approved");
    assert_eq!(restored["jobs_admitted"], 1);
    f.finish(&started).await;
    goal_wait::progress(&f.store, wait).await.unwrap();
    assert_eq!(f.state(wait).await["status"], "resumed");
    assert_eq!(f.mutations().len(), 1);
    std::fs::write(f.controller.home.join("mode"), "disconnect_read").unwrap();
    let revoked = f
        .store
        .decide_goal_consent(
            approved["request_id"].as_str().unwrap().into(),
            f.session.clone(),
            approved["revision"].as_i64().unwrap(),
            agent_job_relay::goal_consent::Decision::Revoke,
            "user revocation while host unavailable",
        )
        .await
        .unwrap();
    assert_eq!(revoked["status"], "revoked");
    assert_eq!(f.state(wait).await["status"], "cancelled");
    assert_eq!(f.mutations().len(), 1);
}

#[tokio::test]
async fn stale_goal_observation_cannot_overwrite_a_concurrent_revocation() {
    let f = Fixture::new().await;
    let pending = f.consent(1).await;
    let approved = f.approve_consent(&pending).await;
    let request = approved["request_id"].as_str().unwrap().to_owned();
    let mut replacement = f.goal.clone();
    replacement.objective = "a different goal".into();
    f.write_goal(&replacement);
    std::fs::write(f.controller.home.join("mode"), "hold_first_read").unwrap();
    let store = f.store.clone();
    let (key, session) = (request.clone(), f.session.clone());
    let stale = tokio::spawn(async move { store.goal_consent_view(key, session).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !f.controller.home.join("blocked").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let revoked = f
        .store
        .decide_goal_consent(
            request.clone(),
            f.session.clone(),
            approved["revision"].as_i64().unwrap(),
            agent_job_relay::goal_consent::Decision::Revoke,
            "user revocation during observation",
        )
        .await
        .unwrap();
    std::fs::write(f.controller.home.join("release"), "").unwrap();
    assert_eq!(stale.await.unwrap().unwrap(), revoked);
    let statuses = f
        .store
        .read(move |db| {
            let mut q = db.prepare(
                "SELECT status FROM goal_consent_audits WHERE request_id=? ORDER BY sequence",
            )?;
            Ok(q.query_map([request], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
        .unwrap();
    assert_eq!(statuses, ["pending", "approved", "revoked"]);
    assert!(f.mutations().is_empty());
}

#[tokio::test]
async fn inactive_original_goal_retains_pending_consent_without_a_new_question() {
    for status in [
        "blocked",
        "paused",
        "budget_limited",
        "usage_limited",
        "unknown_status",
    ] {
        let f = Fixture::new().await;
        let pending = f.consent(1).await;
        let request = pending["request_id"].as_str().unwrap().to_owned();
        let mut goal = f.goal.clone();
        goal.status = status.into();
        goal.updated_at += 1;
        f.write_goal(&goal);
        let current = f
            .store
            .goal_consent_view(request.clone(), f.session.clone())
            .await
            .unwrap();
        assert_eq!(current["status"], "pending");
        assert_eq!(current["revision"], pending["revision"]);
        assert_eq!(current["goal_status"], status);
        assert!(
            f.store
                .claim_goal_question(request.clone(), f.session.clone(), 1, false)
                .await
                .is_err()
        );
        assert!(
            f.store
                .decide_goal_consent(
                    request.clone(),
                    f.session.clone(),
                    1,
                    agent_job_relay::goal_consent::Decision::Allow,
                    "no active goal"
                )
                .await
                .is_err()
        );
        assert!(f.store.list(0, 100).await.unwrap().is_empty());
        assert!(f.mutations().is_empty());
        goal.status = "active".into();
        goal.updated_at += 1;
        f.write_goal(&goal);
        assert_eq!(
            f.store
                .goal_consent_view(request, f.session.clone())
                .await
                .unwrap(),
            pending
        );
    }
}

#[tokio::test]
async fn native_answers_cannot_grant_after_expiry_or_goal_deletion_and_do_not_reprompt() {
    for removed in [false, true] {
        let f = Fixture::new().await;
        let pending = f.consent(1).await;
        let id = pending["request_id"].as_str().unwrap().to_string();
        let c = f
            .store
            .claim_goal_question(id.clone(), f.session.clone(), 1, true)
            .await
            .unwrap();
        let token = Store::question_token(&c).unwrap();
        if removed {
            std::fs::remove_file(f.controller.home.join("goal.json")).unwrap();
        } else {
            let key = id.clone();
            f.store
                .call(move |db| {
                    db.execute("UPDATE goal_consents SET expires=0 WHERE id=?", [key])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let answer = f
            .store
            .answer_goal_question(id.clone(), f.session.clone(), token, true)
            .await
            .unwrap();
        assert_eq!(
            answer["status"],
            if removed { "invalidated" } else { "expired" }
        );
        assert!(
            f.store
                .claim_goal_question(id, f.session.clone(), c.revision, true)
                .await
                .is_err()
        );
        assert!(f.store.list(0, 100).await.unwrap().is_empty());
        assert!(f.mutations().is_empty());
    }
}

struct ConsentMcp {
    meta: Option<Value>,
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    lines: tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
}
impl ConsentMcp {
    async fn start(f: &Fixture, version: &str, native: bool) -> Self {
        use tokio::io::AsyncBufReadExt;
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .args([
                "--state-dir",
                f.store.config.root.to_str().unwrap(),
                "mcp",
                "--shared-host",
            ])
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .env_remove("CODEX_APP_RUNTIME_CLI_PATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let lines = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
        let meta=(version=="2026-07-28").then(||json!({"io.modelcontextprotocol/protocolVersion":version,"io.modelcontextprotocol/clientCapabilities":if native{json!({"elicitation":{"form":{}}})}else{json!({})},"io.modelcontextprotocol/clientInfo":{"name":"scoped-consent-fixture","version":"1"}}));
        let mut this = Self {
            meta,
            child,
            input,
            lines,
        };
        if version == "2026-07-28" {
            this.send(json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}))
                .await;
            let discovery = this.receive().await;
            assert!(
                discovery["result"]["supportedVersions"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(version)),
                "{discovery}"
            );
            return this;
        }
        this.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":version,"capabilities":if native{json!({"elicitation":{"form":{}}})}else{json!({})},"clientInfo":{"name":"scoped-consent-fixture","version":"1"}}})).await;
        assert_eq!(
            this.receive().await["result"]["serverInfo"]["name"],
            "agent-job-relay"
        );
        this.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        this
    }
    async fn send(&mut self, mut value: Value) {
        use tokio::io::AsyncWriteExt;
        if value.get("method").is_some()
            && value.get("id").is_some()
            && let Some(meta) = &self.meta
        {
            value["params"]["_meta"] = meta.clone();
        }
        self.input
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
        self.input.flush().await.unwrap();
    }
    async fn receive(&mut self) -> Value {
        serde_json::from_str(
            &tokio::time::timeout(Duration::from_secs(12), self.lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }
    async fn call(&mut self, id: u32, args: Value) {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"job_goal_consent","arguments":args}})).await;
    }
    async fn close(mut self) {
        use tokio::io::AsyncWriteExt;
        self.input.shutdown().await.unwrap();
        drop(self.input);
        assert!(self.child.wait().await.unwrap().success());
    }
}
fn consent_result(v: &Value) -> Value {
    assert_ne!(v["result"]["isError"], true, "{v}");
    serde_json::from_str(v["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn native_mcp_questions_and_fallback_require_real_answers_and_preserve_one_prompt() {
    for version in ["2025-06-18", "2026-07-28"] {
        for allow in [true, false] {
            let f = Fixture::new().await;
            let mut daemon = f.daemon().await;
            let c = f.consent(1).await;
            let id = c["request_id"].as_str().unwrap();
            let mut mcp = ConsentMcp::start(&f, version, true).await;
            let args = json!({"request_id":id,"session_key":f.session});
            mcp.call(2, args.clone()).await;
            let question = mcp.receive().await;
            assert!(f.store.list(0, 100).await.unwrap().is_empty());
            assert_eq!(
                f.controller.read().await.unwrap().unwrap().status.as_str(),
                "active"
            );
            let answer = json!({"action":if allow{"accept"}else{"cancel"},"content":if allow{json!({"allow_goal_pause":true})}else{Value::Null}});
            if version == "2026-07-28" {
                assert_eq!(
                    question["result"]["resultType"], "input_required",
                    "{question}"
                );
                let schema = &question["result"]["inputRequests"][id]["params"]["requestedSchema"];
                assert!(
                    schema["properties"]["allow_goal_pause"]
                        .get("default")
                        .is_none()
                );
                mcp.call(3, args.clone()).await;
                assert_eq!(
                    consent_result(&mcp.receive().await)["question_pending"],
                    true
                );
                mcp.send(json!({"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"job_goal_consent","arguments":args,"requestState":"tampered-token","inputResponses":{id:answer}}})).await;
                assert_eq!(mcp.receive().await["result"]["isError"], true);
                assert_eq!(
                    f.store
                        .goal_consent_view(id.into(), f.session.clone())
                        .await
                        .unwrap()["status"],
                    "asking"
                );
                mcp.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"job_goal_consent","arguments":args,"requestState":question["result"]["requestState"],"inputResponses":{id:answer}}})).await;
            } else {
                assert_eq!(question["method"], "elicitation/create", "{question}");
                mcp.call(3, args.clone()).await;
                assert_eq!(
                    consent_result(&mcp.receive().await)["question_pending"],
                    true
                );
                mcp.send(json!({"jsonrpc":"2.0","id":question["id"],"result":answer}))
                    .await;
            }
            let result = consent_result(&mcp.receive().await);
            assert_eq!(
                result["status"],
                if allow { "approved" } else { "declined" }
            );
            mcp.call(5, args).await;
            assert_eq!(consent_result(&mcp.receive().await), result);
            assert!(f.store.list(0, 100).await.unwrap().is_empty());
            assert!(f.mutations().is_empty());
            mcp.close().await;
            ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
            daemon.wait().unwrap();
        }
    }
    let f = Fixture::new().await;
    let mut daemon = f.daemon().await;
    let c = f.consent(1).await;
    let mut mcp = ConsentMcp::start(&f, "2025-06-18", false).await;
    let args = json!({"request_id":c["request_id"],"session_key":f.session});
    mcp.call(2, args.clone()).await;
    let question = consent_result(&mcp.receive().await);
    assert_eq!(question["ask_user_now"], true);
    mcp.call(3, args.clone()).await;
    let repeated = consent_result(&mcp.receive().await);
    assert_eq!(repeated["question_pending"], true);
    assert!(repeated.get("ask_user_now").is_none());
    let mut answer = args;
    answer["decision"] = json!("allow");
    answer["expected_revision"] = question["revision"].clone();
    mcp.call(4, answer).await;
    assert_eq!(consent_result(&mcp.receive().await)["status"], "approved");
    mcp.close().await;
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}

#[tokio::test]
async fn scoped_terminal_request_reuses_one_answer_for_three_real_background_submissions() {
    use agent_job_relay::goal_consent::Decision;
    let f = Fixture::new().await;
    let mut daemon = f.daemon().await;
    let out = f
        .cli()
        .args([
            "connect",
            "--request-label",
            "three separate scans",
            "--job-count",
            "3",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pending: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(pending["status"], "needs_confirmation");
    assert!(pending.get("binding").is_none());
    let key = pending["session_key"].as_str().unwrap().to_string();
    let request = pending["consent"]["request_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        ipc::call(
            &f.store.config,
            Request::Start {
                spec: StartSpec {
                    session: Some(key.clone()),
                    ..f.spec("stripped-confirmation")
                }
            }
        )
        .await
        .is_err()
    );
    ipc::call(
        &f.store.config,
        Request::GoalConsent {
            request_id: request.clone(),
            session_key: key.clone(),
            decision: Decision::Allow,
            expected_revision: Some(1),
        },
    )
    .await
    .unwrap();
    for round in 0..3 {
        if round == 1 {
            // Reproduce a host blocker after the first result was delivered.
            // It gates the next round without revoking the original approval.
            let mut current = f.controller.read().await.unwrap().unwrap();
            current.status = "blocked".into();
            current.updated_at += 1;
            f.write_goal(&current);
            let batch = f.dir.path().join("blocked-batch.json");
            let mut spec = f.spec("blocked-batch");
            spec.session = None;
            std::fs::write(&batch, serde_json::to_vec(&vec![spec]).unwrap()).unwrap();
            for args in [
                vec!["connect", "--request-id", request.as_str()],
                vec!["connect"],
                vec!["start", "--wake", "--", "/bin/true"],
                vec!["start-batch", "--wake", "--file", batch.to_str().unwrap()],
            ] {
                let out = f.cli().args(args).output().await.unwrap();
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                let gated: Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(gated["status"], "goal_not_active");
                assert_eq!(gated["goal_status"], "blocked");
                assert!(gated.get("binding").is_none());
                if let Some(consent) = gated.get("consent") {
                    assert_eq!(consent["status"], "approved");
                    assert_eq!(consent["jobs_admitted"], 1);
                    assert_eq!(consent["jobs"].as_array().unwrap().len(), 1);
                }
            }
            assert_eq!(f.store.list(0, 100).await.unwrap().len(), 1);
            // Simulate the user's host Resume control, preserving accounting.
            current.status = "active".into();
            current.updated_at += 1;
            f.write_goal(&current);
        }
        let out = f
            .cli()
            .args(["connect", "--request-id", &request])
            .output()
            .await
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let binding: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(binding["status"], "ready");
        assert_eq!(binding["consent"]["jobs_admitted"], round);
        let wait = binding["binding"]["goal_wait_id"]
            .as_str()
            .unwrap()
            .to_string();
        let mut spec = f.spec(&format!("real-round-{round}"));
        spec.session = Some(key.clone());
        spec.command = CommandSpec::Shell {
            interpreter: "/bin/sh".into(),
            script: "printf 'once\\n' >> scoped-executions; sleep .05".into(),
        };
        let started = ipc::call(
            &f.store.config,
            Request::StartGoal {
                spec: spec.clone(),
                goal_wait_id: wait.clone(),
            },
        )
        .await
        .unwrap();
        let retried = ipc::call(
            &f.store.config,
            Request::StartGoal {
                spec,
                goal_wait_id: wait.clone(),
            },
        )
        .await
        .unwrap();
        assert_eq!(started["job"]["job_id"], retried["job"]["job_id"]);
        f.pause_and_confirm(&wait).await;
        tokio::time::timeout(Duration::from_secs(10), async {
            while f.state(&wait).await["notification_accepted"] != true {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    let out = f
        .cli()
        .args(["connect", "--request-id", &request])
        .output()
        .await
        .unwrap();
    assert!(out.status.success());
    let ended: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(ended["status"], "request_closed");
    assert_eq!(ended["consent"]["jobs"].as_array().unwrap().len(), 3);
    assert_eq!(
        std::fs::read_to_string(f.dir.path().join("scoped-executions")).unwrap(),
        "once\nonce\nonce\n"
    );
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}

#[tokio::test]
async fn native_mcp_does_not_prompt_for_an_inactive_original_goal() {
    let f = Fixture::new().await;
    let mut daemon = f.daemon().await;
    let c = f.consent(1).await;
    let mut current = f.goal.clone();
    current.status = "blocked".into();
    current.updated_at += 1;
    f.write_goal(&current);
    let mut mcp = ConsentMcp::start(&f, "2025-06-18", true).await;
    mcp.call(2, json!({"request_id":c["request_id"],"session_key":f.session,"expected_revision":c["revision"]})).await;
    // A tool result must arrive directly, without elicitation/create.
    let reply = mcp.receive().await;
    assert_eq!(reply["id"], 2);
    let retained = consent_result(&reply);
    assert_eq!(retained["status"], "pending");
    assert_eq!(retained["goal_status"], "blocked");
    assert_eq!(retained["revision"], c["revision"]);
    assert!(retained.get("ask_user_now").is_none());
    assert!(f.store.list(0, 100).await.unwrap().is_empty());
    assert!(f.mutations().is_empty());
    mcp.close().await;
    ipc::call(&f.store.config, Request::Shutdown).await.unwrap();
    daemon.wait().unwrap();
}
