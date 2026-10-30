mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    monitor::{Action, Condition, Mutation, RuleSpec, Stream},
    storage::Store,
};
use serde_json::json;
fn rule(condition: Condition, action: Action) -> RuleSpec {
    RuleSpec {
        condition,
        action,
        notify_agent: false,
        grace_seconds: 0,
    }
}
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
        idempotency_key: None,
        monitor_rules: vec![],
    };
    (dir, store, spec)
}
#[tokio::test]
async fn rules_validate_revisions_terminal_fences_and_offline_reads() {
    let (_dir, store, mut spec) = fixture().await;
    spec.monitor_rules = vec![rule(Condition::NoOutput { seconds: 1 }, Action::Alert)];
    let job = store.start(spec).await.unwrap();
    let rules = store.monitor(job.job_id.clone(), None).await.unwrap();
    let id = rules["rules"][0]["id"].as_str().unwrap().to_owned();
    assert_eq!(rules["rules"][0]["applied_revision"], 0);
    store
        .monitor(
            job.job_id.clone(),
            Some(Mutation::Enable {
                rule_id: id.clone(),
                expected_revision: 1,
                enabled: false,
            }),
        )
        .await
        .unwrap();
    assert!(
        store
            .monitor(
                job.job_id.clone(),
                Some(Mutation::Enable {
                    rule_id: id.clone(),
                    expected_revision: 1,
                    enabled: true
                })
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("revision changed")
    );
    assert!(
        store
            .monitor(
                job.job_id.clone(),
                Some(Mutation::Add {
                    rule: rule(
                        Condition::Contains {
                            text: "".into(),
                            stream: Stream::Both
                        },
                        Action::Stop
                    )
                })
            )
            .await
            .is_err()
    );
    let value =
        agent_job_relay::monitor::inspect_client(&store.config, Some(job.job_id.clone()), None, 20)
            .await
            .unwrap();
    assert_eq!(value["runtime"], "offline_snapshot");
    assert_eq!(value["rules"][0]["revision"], 2);
    store
        .finish(
            job.job_id.clone(),
            "completed",
            Some(0),
            None,
            "done".into(),
        )
        .await
        .unwrap();
    store
        .stop_with(job.job_id.clone(), 0, "manual".into())
        .await
        .unwrap();
    assert_eq!(
        store
            .status(job.job_id.clone())
            .await
            .unwrap()
            .status
            .as_str(),
        "completed"
    );
    assert!(
        store
            .monitor(
                job.job_id,
                Some(Mutation::Enable {
                    rule_id: id,
                    expected_revision: 2,
                    enabled: true
                })
            )
            .await
            .is_err()
    );
    let absent = Config::new(Some(store.config.root.join("absent"))).unwrap();
    assert_eq!(
        agent_job_relay::monitor::inspect_client(&absent, None, None, 20)
            .await
            .unwrap()["runtime"],
        "stopped"
    );
    assert!(!absent.root.exists());
}
#[tokio::test]
async fn migration_preserves_event_highwater_and_delivery_references() {
    let (_dir, store, spec) = fixture().await;
    let job = store.start(spec).await.unwrap();
    store
        .finish(
            job.job_id.clone(),
            "completed",
            Some(0),
            None,
            "done".into(),
        )
        .await
        .unwrap();
    store.call(|db| {db.execute_batch("DROP TABLE goal_wait_audits; DROP TABLE goal_wait_jobs; DROP TABLE goal_waits; DROP TABLE goal_consent_audits; DROP TABLE goal_consents; DROP TABLE monitor_rules;DROP TABLE monitor_changes;DROP TABLE runtime_observations;DROP TABLE stop_requests;UPDATE sqlite_sequence SET seq=9000 WHERE name='events';PRAGMA user_version=8;")?;Ok(())}).await.unwrap();
    store.init().await.unwrap();
    store
        .call(move |db| {
            agent_job_relay::storage::insert_event(
                db,
                &job.job_id,
                &job.run_id,
                "monitor_alert",
                json!({"historical":true}),
            )?;
            assert!(db.last_insert_rowid() > 9000);
            assert!(!db.prepare("PRAGMA foreign_key_check")?.exists([])?);
            Ok(())
        })
        .await
        .unwrap();
}

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use agent_job_relay::ipc::{self, Request};
    use serde_json::Value;
    use std::{
        process::{Child, Command, Stdio},
        time::Duration,
    };
    struct Runtime {
        dir: tempfile::TempDir,
        config: Config,
        daemon: Option<Child>,
    }
    impl Runtime {
        async fn new() -> Self {
            let dir = tempfile::tempdir_in(common::root()).unwrap();
            let config = Config::new(Some(dir.path().join("state"))).unwrap();
            let child = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
                .arg("--state-dir")
                .arg(&config.root)
                .arg("daemon")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let r = Self {
                dir,
                config,
                daemon: Some(child),
            };
            tokio::time::timeout(Duration::from_secs(8), async {
                loop {
                    if ipc::call(&r.config, Request::Doctor).await.is_ok() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();
            r
        }
        async fn start(&self, script: &str, rules: Vec<RuleSpec>) -> String {
            let spec = StartSpec {
                command: CommandSpec::Shell {
                    interpreter: "/bin/sh".into(),
                    script: script.into(),
                },
                cwd: self.dir.path().into(),
                env: Default::default(),
                timeout_seconds: Some(20),
                session: None,
                idempotency_key: None,
                monitor_rules: rules,
            };
            ipc::call(&self.config, Request::Start { spec })
                .await
                .unwrap()["job"]["job_id"]
                .as_str()
                .unwrap()
                .into()
        }
        async fn until(&self, job: &str, test: impl Fn(&Value) -> bool) -> Value {
            let store = Store::new(self.config.clone());
            tokio::time::timeout(Duration::from_secs(12), async {
                loop {
                    let v = store.inspect(Some(job.into()), None, 20).await.unwrap();
                    if test(&v) {
                        break v;
                    }
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
            })
            .await
            .unwrap()
        }
        fn crash(&mut self) {
            if let Some(mut child) = self.daemon.take() {
                child.kill().unwrap();
                child.wait().unwrap();
            }
        }
    }
    impl Drop for Runtime {
        fn drop(&mut self) {
            if let Ok(db) = rusqlite::Connection::open(self.config.root.join("relay.sqlite3")) {
                let _ = db.execute(
                    "UPDATE jobs SET cancel=1 WHERE status IN ('queued','launching','running')",
                    [],
                );
                let _ = db.execute("UPDATE stop_requests SET grace_seconds=0", []);
            }
            if let Some(mut child) = self.daemon.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    #[tokio::test]
    async fn runner_monitors_cross_chunk_output_and_stops_tree_without_daemon() {
        let mut r = Runtime::new().await;
        let id=r.start("while [ ! -f release ]; do sleep .02; done; sleep 20 & echo $! > descendant; printf 'FAT'; sleep .2; printf 'AL'; wait",vec![rule(Condition::Contains{text:"FATAL".into(),stream:Stream::Stdout},Action::Stop)]).await;
        r.until(&id, |v| v["rules"][0]["applied_revision"] == 1)
            .await;
        r.crash();
        std::fs::write(r.dir.path().join("release"), b"go").unwrap();
        let v = r.until(&id, |v| v["job"]["status"] == "cancelled").await;
        assert_eq!(
            v["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["kind"] == "monitor_alert")
                .count(),
            1
        );
        assert!(
            v["stop"]["reason"]
                .as_str()
                .unwrap()
                .contains("monitor rule")
        );
        let pid = std::fs::read_to_string(r.dir.path().join("descendant"))
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();
        assert!(agent_job_relay::platform::identity(pid).is_none());
        assert_eq!(v["observation"]["stdout"]["text"], "FATAL");
    }
    #[tokio::test]
    async fn output_cap_does_not_disable_detection_or_recent_tails() {
        let r = Runtime::new().await;
        let id = r
            .start(
                "head -c 17825792 /dev/zero; printf 'CAP_MARKER'; sleep 20",
                vec![rule(
                    Condition::Contains {
                        text: "CAP_MARKER".into(),
                        stream: Stream::Stdout,
                    },
                    Action::Stop,
                )],
            )
            .await;
        let v = r.until(&id, |v| v["job"]["status"] == "cancelled").await;
        assert!(
            v["observation"]["stdout"]["total_bytes"].as_u64().unwrap()
                > agent_job_relay::runner::LOG_CAP
        );
        assert!(
            v["observation"]["stdout"]["text"]
                .as_str()
                .unwrap()
                .ends_with("CAP_MARKER")
        );
        let run = v["job"]["run_id"].as_str().unwrap();
        assert_eq!(
            std::fs::metadata(r.config.run_dir(run).join("stdout.log"))
                .unwrap()
                .len(),
            agent_job_relay::runner::LOG_CAP
        );
    }
    #[tokio::test]
    async fn no_output_rules_do_not_repeat_and_rearming_has_a_fresh_window() {
        let r = Runtime::new().await;
        let id = r
            .start(
                "sleep 20",
                vec![rule(Condition::NoOutput { seconds: 1 }, Action::Alert)],
            )
            .await;
        let v = r
            .until(&id, |v| v["rules"][0]["triggered_revision"] == 1)
            .await;
        let rule_id = v["rules"][0]["id"].as_str().unwrap().to_owned();
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let v = ipc::call(
            &r.config,
            Request::Inspect {
                job_id: Some(id.clone()),
                before: None,
                limit: 20,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            v["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["kind"] == "monitor_alert")
                .count(),
            1
        );
        let rearmed_at = agent_job_relay::core::now();
        ipc::call(
            &r.config,
            Request::Monitor {
                job_id: id.clone(),
                mutation: Some(Mutation::Enable {
                    rule_id,
                    expected_revision: 1,
                    enabled: true,
                }),
            },
        )
        .await
        .unwrap();
        let v = r
            .until(&id, |v| {
                v["rules"][0]["triggered_revision"] == 2 && v["rules"][0]["applied_revision"] == 2
            })
            .await;
        // A delayed inspect can already include the trigger. Verify the fresh
        // silence window without requiring the reader to win that race.
        let alert = v["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["kind"] == "monitor_alert" && e["payload"]["revision"] == 2)
            .unwrap();
        assert!(
            alert["payload"]["evidence"]["observed_at"]
                .as_i64()
                .unwrap()
                >= rearmed_at + 1000
        );
        let v = ipc::call(
            &r.config,
            Request::Stop {
                job_id: id,
                grace_seconds: 0,
                wait_seconds: 3,
            },
        )
        .await
        .unwrap();
        assert_eq!(v["stop_outcome"], "terminal_observed");
    }
    #[tokio::test]
    async fn pending_agent_alerts_are_superseded_without_losing_history() {
        for mode in ["retry", "accepted", "expired"] {
            use agent_job_relay::session::{Adapter, Registration};
            let mut r = Runtime::new().await;
            let store = Store::new(r.config.clone());
            let session = store
                .register(Registration {
                    agent_kind: "codex_cli".into(),
                    session_id: "monitor-test".into(),
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
                .unwrap();
            let mut policy = rule(
                Condition::Contains {
                    text: "READY".into(),
                    stream: Stream::Stdout,
                },
                Action::Alert,
            );
            policy.notify_agent = true;
            let spec = StartSpec {
                command: CommandSpec::Shell {
                    interpreter: "/bin/sh".into(),
                    script: "while [ ! -f release ]; do sleep .02; done; printf READY; sleep 20"
                        .into(),
                },
                cwd: r.dir.path().into(),
                env: Default::default(),
                timeout_seconds: Some(20),
                session: Some(session.key),
                idempotency_key: None,
                monitor_rules: vec![policy],
            };
            let job = store.start(spec).await.unwrap();
            r.until(&job.job_id, |v| v["rules"][0]["applied_revision"] == 1)
                .await;
            r.crash();
            std::fs::write(r.dir.path().join("release"), b"go").unwrap();
            let triggered = r
                .until(&job.job_id, |v| v["rules"][0]["triggered_revision"] == 1)
                .await;
            let delivery = triggered["deliveries"]["deliveries"][0]["id"].clone();
            assert_eq!(
                triggered["deliveries"]["deliveries"][0]["status"],
                "pending"
            );
            let claim = store.claim().await.unwrap().unwrap();
            store
                .stop_with(job.job_id.clone(), 0, "manual".into())
                .await
                .unwrap();
            let finished = r
                .until(&job.job_id, |v| v["job"]["status"] == "cancelled")
                .await;
            let rows = finished["deliveries"]["deliveries"].as_array().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0]["id"], delivery);
            assert_eq!(rows[0]["status"], "leased");
            assert!(
                store.claim().await.unwrap().is_none(),
                "a live alert still excludes another attempt for this chat"
            );
            let expected = match mode {
                "accepted" => {
                    assert!(
                        store
                            .complete_claim(claim, Ok("notification_recorded"))
                            .await
                            .unwrap()
                    );
                    "notification_recorded"
                }
                "retry" => {
                    assert!(
                        store
                            .complete_claim(
                                claim,
                                Err(anyhow::anyhow!("temporary transport failure"))
                            )
                            .await
                            .unwrap()
                    );
                    "cancelled"
                }
                _ => {
                    let alert = delivery.as_str().unwrap().to_owned();
                    store
                        .call(move |db| {
                            db.execute(
                                "UPDATE deliveries SET lease_until=? WHERE id=?",
                                rusqlite::params![now() - 1, alert],
                            )?;
                            Ok(())
                        })
                        .await
                        .unwrap();
                    let terminal = store.claim().await.unwrap().unwrap();
                    assert_eq!(terminal.event["kind"], "cancelled");
                    assert!(
                        !store
                            .complete_claim(claim, Ok("notification_recorded"))
                            .await
                            .unwrap()
                    );
                    "cancelled"
                }
            };
            let updated = store
                .deliveries(Some(job.job_id.clone()), 0, 20)
                .await
                .unwrap();
            assert_eq!(updated["deliveries"][0]["status"], expected);
            assert_eq!(updated["deliveries"][0]["id"], delivery);
            assert_eq!(
                updated["deliveries"][0]["expires_at"],
                triggered["deliveries"]["deliveries"][0]["expires_at"]
            );
            assert_eq!(updated["deliveries"][0]["attempts"], 1);
            let attempts = store
                .attempts(delivery.as_str().unwrap().into(), 0, 20)
                .await
                .unwrap();
            assert_eq!(
                attempts["attempts"][0]["outcome"],
                if mode == "accepted" {
                    "notification_recorded"
                } else {
                    "superseded"
                }
            );
        }
    }

    #[tokio::test]
    async fn checker_sends_midrun_feedback_to_the_bound_fake_host_and_survives_daemon_loss() {
        use agent_job_relay::{
            progress::Trigger,
            session::{Adapter, Registration},
        };
        let mut r = Runtime::new().await;
        let store = Store::new(r.config.clone());
        let endpoint = r.dir.path().join("host.sock");
        let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
        let (sent, mut received) = tokio::sync::mpsc::channel(8);
        let host = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let sent = sent.clone();
                tokio::spawn(async move {
                    let catalog = ipc::read_frame(&mut stream, 1048576).await.unwrap();
                    ipc::write_frame(&mut stream,&json!({"jsonrpc":"2.0","id":catalog["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).await.unwrap();
                    let call = ipc::read_frame(&mut stream, 1048576).await.unwrap();
                    sent.send(call.clone()).await.unwrap();
                    ipc::write_frame(
                        &mut stream,
                        &json!({"jsonrpc":"2.0","id":call["id"],"result":{"success":true}}),
                    )
                    .await
                    .unwrap();
                });
            }
        });
        let registration = |chat: &str| Registration {
            agent_kind: "codex_desktop".into(),
            session_id: chat.into(),
            instance_id: "fake-instance".into(),
            host_id: "fake-host".into(),
            adapter: Adapter::DesktopExperimental,
            endpoint: Some(endpoint.clone()),
            caller_thread_id: Some("original-caller".into()),
            lease_seconds: 3600,
            experimental: true,
            renew_on_delivery: false,
        };
        let session = store.register(registration("original-chat")).await.unwrap();
        store
            .register(registration("unrelated-chat"))
            .await
            .unwrap();
        let checker =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/progress_checker.py");
        let job = store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "python3".into(),
                    args: vec![
                        checker.display().to_string(),
                        "--status-file".into(),
                        "status.json".into(),
                        "--observed-run".into(),
                        "target:job:run:attempt-1".into(),
                        "--interval-seconds".into(),
                        "0.05".into(),
                        "--timeout-seconds".into(),
                        "12".into(),
                        "--marker-stage".into(),
                        "checkpoint".into(),
                    ],
                },
                cwd: r.dir.path().into(),
                env: Default::default(),
                timeout_seconds: Some(15),
                session: Some(session.key),
                idempotency_key: None,
                monitor_rules: vec![
                    rule(
                        Condition::Contains {
                            text: "AJR_MILESTONE ".into(),
                            stream: Stream::Stdout,
                        },
                        Action::Alert,
                    ),
                    RuleSpec {
                        condition: Condition::Progress {
                            source: "checker".into(),
                            trigger: Trigger::Stage { stage: None },
                            cooldown_seconds: 1,
                        },
                        action: Action::Alert,
                        notify_agent: true,
                        grace_seconds: 0,
                    },
                ],
            })
            .await
            .unwrap();
        r.until(&job.job_id, |v| v["rules"][1]["applied_revision"] == 1)
            .await;
        let write = |seq: u64, stage: &str, observed: &str, terminal: bool| {
            let data = json!({"seq":seq,"stage":stage,"step":seq*10,"total":100,"observed_run":observed,"terminal":terminal});
            let temp = r.dir.path().join("status.tmp");
            std::fs::write(&temp, data.to_string()).unwrap();
            std::fs::rename(temp, r.dir.path().join("status.json")).unwrap();
        };
        // Identity mismatch must never emit progress or wake a different run/chat.
        write(1, "checkpoint", "foreign-run", false);
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(received.try_recv().is_err());
        write(1, "checkpoint", "target:job:run:attempt-1", false);
        let call = tokio::time::timeout(Duration::from_secs(5), received.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(call["params"]["arguments"]["threadId"], "original-chat");
        assert!(
            !call["params"]["arguments"]["prompt"]
                .as_str()
                .unwrap()
                .contains("checkpoint")
        );
        let first = r
            .until(&job.job_id, |v| {
                v["deliveries"]["deliveries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["status"] == "accepted_by_host")
            })
            .await;
        assert_eq!(first["job"]["status"], "running");
        assert_eq!(
            first["observation"]["progress"]["sources"]["checker"]["observed_run"],
            "target:job:run:attempt-1"
        );
        assert_eq!(first["observation"]["progress"]["alerts"], 1);
        r.crash();
        tokio::time::sleep(Duration::from_millis(1100)).await;
        // Continue with a new producer sequence. Runner owns parsing and outbox.
        let temp = r.dir.path().join("status.tmp");
        std::fs::write(&temp,json!({"seq":2,"stage":"validation","step":20,"total":100,"observed_run":"target:job:run:attempt-1"}).to_string()).unwrap();
        std::fs::rename(temp, r.dir.path().join("status.json")).unwrap();
        r.until(&job.job_id, |v| v["observation"]["progress"]["alerts"] == 2)
            .await;
        std::fs::write(r.dir.path().join("status.tmp"),json!({"seq":3,"stage":"validation","step":100,"total":100,"observed_run":"target:job:run:attempt-1","terminal":true}).to_string()).unwrap();
        std::fs::rename(
            r.dir.path().join("status.tmp"),
            r.dir.path().join("status.json"),
        )
        .unwrap();
        let done = r
            .until(&job.job_id, |v| v["job"]["status"] == "completed")
            .await;
        assert_eq!(
            done["events"][0]["payload"]["progress"]["milestones"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(
            done["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "monitor_alert"
                    && e["payload"]["rule"]["condition"]["kind"] == "contains")
        );
        assert!(
            done["deliveries"]["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["status"] == "cancelled")
        );
        host.abort();
    }

    #[tokio::test]
    async fn progress_cli_threshold_stops_and_exposes_shared_web_snapshot() {
        let r = Runtime::new().await;
        let script = "printf 'AJR_PROGRESS {\"v\":1,\"source\":\"train\",\"seq\":1,\"step\":10,\"total\":20,\"metrics\":{\"loss\":0.05}}\\n'; sleep 20";
        let output = Command::new(env!("CARGO_BIN_EXE_agent-job-relay")).args(["--state-dir"]).arg(&r.config.root).args(["start","--cwd"]).arg(r.dir.path()).args(["--progress",r#"{"source":"train","trigger":{"kind":"metric","name":"loss","comparison":"le","value":0.1},"cooldown_seconds":1}"#,"--action","stop","--grace","0","--shell",script]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        let id = value["job"]["job_id"].as_str().unwrap();
        let done = r.until(id, |v| v["job"]["status"] == "cancelled").await;
        assert_eq!(
            done["observation"]["progress"]["sources"]["train"]["step"],
            10
        );
        let info = ipc::call(&r.config, Request::UiInfo).await.unwrap();
        let token = info["access_url"]
            .as_str()
            .unwrap()
            .split("#token=")
            .nth(1)
            .unwrap();
        let response = http(
            info["url"].as_str().unwrap(),
            "/api/request",
            Some(token),
            None,
            Some(json!({"op":"inspect","job_id":id})),
        )
        .await;
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("milestones") && response.contains("loss"));
        // Exercise the actual shared stdio MCP path with the same condition.
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&r.config.root)
            .args(["mcp", "--shared-host"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut result = Value::Null;
        for (id, method, params) in [
            (
                1,
                "initialize",
                json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"progress-test","version":"1"}}),
            ),
            (
                2,
                "tools/call",
                json!({"name":"job_start","arguments":{"spec":{"command":{"kind":"shell","interpreter":"/bin/sh","script":script},"cwd":r.dir.path(),"monitor_rules":[{"condition":{"kind":"progress","source":"train","trigger":{"kind":"metric","name":"loss","comparison":"le","value":0.1},"cooldown_seconds":1},"action":"stop","grace_seconds":0}]}}}),
            ),
        ] {
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
            let line = tokio::time::timeout(Duration::from_secs(8), lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let reply: Value = serde_json::from_str(&line).unwrap();
            assert!(reply.get("error").is_none(), "{reply}");
            if id == 2 {
                assert_ne!(reply["result"]["isError"], true, "{reply}");
                result =
                    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
                        .unwrap();
            }
        }
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        let done = r
            .until(result["job"]["job_id"].as_str().unwrap(), |v| {
                v["job"]["status"] == "cancelled"
            })
            .await;
        assert_eq!(
            done["events"][0]["payload"]["progress"]["milestones"][0]["sample"]["metrics"]["loss"],
            0.05
        );
    }

    async fn http(
        url: &str,
        path: &str,
        token: Option<&str>,
        origin: Option<&str>,
        body: Option<Value>,
    ) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let host = url.strip_prefix("http://").unwrap().trim_end_matches('/');
        let mut socket = tokio::net::TcpStream::connect(host).await.unwrap();
        let body = body.map(|v| v.to_string());
        let mut text = format!(
            "{} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n",
            if body.is_some() { "POST" } else { "GET" }
        );
        if let Some(token) = token {
            text.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        if let Some(origin) = origin {
            text.push_str(&format!("Origin: {origin}\r\n"));
        }
        if let Some(body) = &body {
            text.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        text.push_str("\r\n");
        text.push_str(body.as_deref().unwrap_or(""));
        socket.write_all(text.as_bytes()).await.unwrap();
        let mut bytes = vec![];
        tokio::time::timeout(Duration::from_secs(8), socket.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        String::from_utf8_lossy(&bytes).into()
    }
    #[tokio::test]
    async fn shared_web_streams_keep_task_selection_and_observe_direct_stops() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
        type Feed = Lines<BufReader<tokio::net::TcpStream>>;
        async fn next(feed: &mut Feed) -> Value {
            tokio::time::timeout(Duration::from_secs(8), async {
                loop {
                    let line = feed.next_line().await.unwrap().expect("SSE ended");
                    if let Some(data) = line.strip_prefix("data: ") {
                        break serde_json::from_str(data).unwrap();
                    }
                }
            })
            .await
            .unwrap()
        }
        async fn open(url: &str, token: &str, job: Option<&str>) -> Feed {
            let host = url.strip_prefix("http://").unwrap().trim_end_matches('/');
            let mut socket = tokio::net::TcpStream::connect(host).await.unwrap();
            let query = job.map(|id| format!("?job_id={id}")).unwrap_or_default();
            socket.write_all(format!("GET /api/events{query} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\n\r\n").as_bytes()).await.unwrap();
            let mut feed = BufReader::new(socket).lines();
            assert!(
                feed.next_line()
                    .await
                    .unwrap()
                    .unwrap()
                    .starts_with("HTTP/1.1 200")
            );
            feed
        }
        let r = Runtime::new().await;
        let info = ipc::call(&r.config, Request::UiInfo).await.unwrap();
        let url = info["url"].as_str().unwrap();
        let token = info["access_url"]
            .as_str()
            .unwrap()
            .split("#token=")
            .nth(1)
            .unwrap();
        let id = r.start("sleep 20", vec![]).await;
        let (mut first, mut second, mut overview) = tokio::join!(
            open(url, token, Some(&id)),
            open(url, token, Some(&id)),
            open(url, token, None)
        );
        for feed in [&mut first, &mut second] {
            let value = next(feed).await;
            assert_eq!(value["detail"]["job"]["job_id"], id);
            assert!(value["overview"]["health"]["sampled_at"].is_i64());
            assert!(!value.to_string().contains(token));
        }
        assert_eq!(next(&mut overview).await["detail"], Value::Null);
        let response = http(
            url,
            "/api/request",
            Some(token),
            None,
            Some(json!({"op":"stop","job_id":id,"grace_seconds":0,"wait_seconds":0})),
        )
        .await;
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if next(&mut first).await["detail"]["job"]["status"] == "cancelled" {
                    break;
                }
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn default_web_ui_authenticates_requests_and_controls_the_same_job() {
        let r = Runtime::new().await;
        let info = ipc::call(&r.config, Request::UiInfo).await.unwrap();
        let url = info["url"].as_str().unwrap();
        let token = info["access_url"]
            .as_str()
            .unwrap()
            .split("#token=")
            .nth(1)
            .unwrap();
        let doctor = ipc::call(&r.config, Request::Doctor).await.unwrap();
        assert_eq!(doctor["web_ui"]["enabled"], true);
        assert!(doctor["web_ui"].get("access_url").is_none());
        let page = http(url, "/", None, None, None).await;
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("/assets/index-"));
        assert!(page.contains("frame-ancestors 'none'"));
        assert!(
            http(
                url,
                "/api/request",
                None,
                None,
                Some(json!({"op":"inspect"}))
            )
            .await
            .starts_with("HTTP/1.1 401")
        );
        assert!(
            http(
                url,
                "/api/request",
                Some(token),
                Some("https://unrelated.invalid"),
                Some(json!({"op":"inspect"}))
            )
            .await
            .starts_with("HTTP/1.1 403")
        );
        assert!(
            http(
                url,
                "/api/request",
                Some(token),
                None,
                Some(json!({"op":"shutdown"}))
            )
            .await
            .starts_with("HTTP/1.1 403")
        );
        assert!(
            http(url, "/relay.sqlite3", None, None, None)
                .await
                .starts_with("HTTP/1.1 404")
        );
        let id = r.start("sleep 20", vec![]).await;
        let response=http(url,"/api/request",Some(token),None,Some(json!({"op":"monitor","job_id":id,"mutation":{"kind":"add","rule":{"condition":{"kind":"elapsed","seconds":1},"action":"stop","grace_seconds":0}}}))).await;
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        r.until(&id, |v| v["job"]["status"] == "cancelled").await;
        let response = http(
            url,
            "/api/request",
            Some(token),
            None,
            Some(json!({"op":"inspect","job_id":id})),
        )
        .await;
        assert!(response.contains("monitor_alert"));
        assert!(response.contains("cancelled"));
    }
}
