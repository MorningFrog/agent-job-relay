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
        store.status(job.job_id.clone()).await.unwrap().status,
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
    store.call(|db| {db.execute_batch("DROP TABLE monitor_rules;DROP TABLE monitor_changes;DROP TABLE runtime_observations;DROP TABLE stop_requests;UPDATE sqlite_sequence SET seq=9000 WHERE name='events';PRAGMA user_version=8;")?;Ok(())}).await.unwrap();
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
        r.until(&id, |v| v["rules"][0]["applied_revision"] == 2)
            .await;
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
        assert_ne!(v["rules"][0]["triggered_revision"], 2);
        r.until(&id, |v| v["rules"][0]["triggered_revision"] == 2)
            .await;
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
                script: "while [ ! -f release ]; do sleep .02; done; printf READY; sleep 20".into(),
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
        assert_eq!(rows[0]["status"], "cancelled");
        assert_eq!(rows[0]["attempts"], 0);
        assert_eq!(rows[1]["status"], "pending");
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
