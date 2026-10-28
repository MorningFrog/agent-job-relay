#![cfg(any(target_os = "linux", windows))]
mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    remote::*,
    storage::Store,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    process::{Child, Command, Stdio},
    time::Duration,
};

const EXE: &str = env!("CARGO_BIN_EXE_agent-job-relay");
struct Rig {
    dir: tempfile::TempDir,
    config: Config,
    store: Store,
    daemon: Option<Child>,
}
impl Rig {
    async fn new() -> Self {
        let dir = tempfile::tempdir_in(common::root()).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config.clone());
        store.init().await.unwrap();
        #[cfg(not(windows))]
        let daemon = Some(
            Command::new(EXE)
                .args(["--state-dir"])
                .arg(&config.root)
                .arg("daemon")
                .env("AJR_WEB_UI", "off")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        #[cfg(windows)]
        let daemon = {
            // Exercise the native verified CIM bootstrap outside Cargo's
            // non-breakaway cleanup job; never weaken runner containment.
            let output = Command::new(EXE)
                .arg("--state-dir")
                .arg(&config.root)
                .args(["daemon", "start"])
                .env("AJR_WEB_UI", "off")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            None
        };
        let rig = Self {
            dir,
            config,
            store,
            daemon,
        };
        for _ in 0..200 {
            if ipc::call(&rig.config, Request::Doctor).await.is_ok() {
                return rig;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("test daemon unavailable");
    }
    fn spec(&self, command: CommandSpec) -> StartSpec {
        StartSpec {
            command,
            cwd: self.dir.path().to_path_buf(),
            env: BTreeMap::new(),
            timeout_seconds: Some(25),
            session: None,
            idempotency_key: Some(id()),
            monitor_rules: vec![],
        }
    }
    fn target(&self) -> (String, Transport) {
        let first = initialize(&self.config).unwrap();
        assert_eq!(first, initialize(&self.config).unwrap());
        let target = first["target_id"].as_str().unwrap().into();
        (
            target,
            Transport {
                program: EXE.into(),
                args: vec![
                    "--state-dir".into(),
                    self.config.root.to_string_lossy().into(),
                    "remote".into(),
                    "serve".into(),
                ],
            },
        )
    }
    async fn start(&self, spec: StartSpec) -> Job {
        let value = agent_job_relay::client::start(&self.config, spec, false)
            .await
            .unwrap();
        serde_json::from_value(value["job"].clone()).unwrap()
    }
    async fn wait(&self, job: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        let until = tokio::time::Instant::now() + Duration::from_secs(35);
        loop {
            let value = self.store.inspect(Some(job.into()), None, 1).await.unwrap();
            if predicate(&value) {
                return value;
            }
            assert!(tokio::time::Instant::now() < until, "wait expired: {value}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(child) = &mut self.daemon {
            let _ = child.kill();
            let _ = child.wait();
        } else {
            let _ = Command::new(EXE)
                .arg("--state-dir")
                .arg(&self.config.root)
                .args(["daemon", "stop"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}
fn command(script: &str) -> CommandSpec {
    #[cfg(windows)]
    return CommandSpec::Exec {
        program: "powershell.exe".into(),
        args: vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
    };
    #[cfg(not(windows))]
    CommandSpec::Shell {
        interpreter: "/bin/sh".into(),
        script: script.into(),
    }
}
fn long_command() -> CommandSpec {
    #[cfg(windows)]
    return command(
        "[IO.File]::AppendAllText('launches','x'); [Console]::OpenStandardOutput().Write([byte[]](65,255,0,66),0,4); Start-Sleep 15",
    );
    #[cfg(not(windows))]
    command("printf x >> launches; printf 'A\\377\\000B'; sleep 15")
}
fn req(target: &str, operation: Operation) -> BridgeRequest {
    BridgeRequest {
        protocol_version: 1,
        target_id: target.into(),
        request_id: id(),
        operation,
    }
}

#[tokio::test]
async fn remote_protocol_recovers_lost_launch_response_and_fences_target_run_attempt() {
    let r = Rig::new().await;
    let (target, transport) = r.target();
    let mut spec = r.spec(long_command());
    spec.idempotency_key = None;
    let origin_job = id();
    let origin_run = id();
    let launch = Operation::Launch {
        origin_job_id: origin_job.clone(),
        origin_run_id: origin_run.clone(),
        spec: Box::new(spec.clone()),
    };
    // Discard the first successful acknowledgement, then retry the identical intent.
    let discarded = exchange(&transport, &req(&target, launch.clone()))
        .await
        .unwrap();
    let again = exchange(&transport, &req(&target, launch)).await.unwrap();
    assert_eq!(discarded["run"], again["run"]);
    let run: RunRef = serde_json::from_value(again["run"].clone()).unwrap();
    r.wait(&run.job_id, |v| {
        v["observation"]["stdout"]["total_bytes"]
            .as_u64()
            .unwrap_or(0)
            >= 4
    })
    .await;
    let lookup = exchange(
        &transport,
        &req(
            &target,
            Operation::Lookup {
                origin_job_id: origin_job.clone(),
                origin_run_id: origin_run.clone(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(lookup["run"], again["run"]);
    spec.env.insert("CHANGED".into(), "true".into());
    assert!(
        exchange(
            &transport,
            &req(
                &target,
                Operation::Launch {
                    origin_job_id: origin_job,
                    origin_run_id: origin_run,
                    spec: Box::new(spec)
                }
            )
        )
        .await
        .is_err()
    );
    let mut wrong = run.clone();
    wrong.run_id = id();
    assert!(
        exchange(
            &transport,
            &req(
                &target,
                Operation::Stop {
                    run: wrong,
                    grace_seconds: 0
                }
            )
        )
        .await
        .is_err()
    );
    let mut wrong = run.clone();
    wrong.attempt = 2;
    assert!(
        exchange(
            &transport,
            &req(
                &target,
                Operation::Stop {
                    run: wrong,
                    grace_seconds: 0
                }
            )
        )
        .await
        .is_err()
    );
    assert!(
        exchange(
            &transport,
            &req(
                &id(),
                Operation::Stop {
                    run: run.clone(),
                    grace_seconds: 0
                }
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        r.store.status(run.job_id.clone()).await.unwrap().status,
        "running"
    );
    assert_eq!(std::fs::read(r.dir.path().join("launches")).unwrap(), b"x");
    let sample = exchange(
        &transport,
        &req(
            &target,
            Operation::Sample {
                run: run.clone(),
                cursors: [0, 0],
                after_event: 0,
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(sample["stdout"], json!([65, 255, 0, 66]));
    exchange(
        &transport,
        &req(
            &target,
            Operation::Stop {
                run: run.clone(),
                grace_seconds: 0,
            },
        ),
    )
    .await
    .unwrap();
    r.wait(&run.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(r.store.list(0, 100).await.unwrap().len(), 1);
}

#[tokio::test]
async fn remote_attach_observation_stop_and_control_have_separate_authority() {
    let remote = Rig::new().await;
    let local = Rig::new().await;
    let (target, transport) = remote.target();
    let remote_job = remote.start(remote.spec(long_command())).await;
    let reference = RunRef::from_job(&remote_job);
    let attach = |allow_stop| CommandSpec::Remote {
        remote: Box::new(RemoteSpec {
            target_id: target.clone(),
            transport: transport.clone(),
            intent: Intent::Attach {
                run: reference.clone(),
                allow_stop,
            },
        }),
    };
    let watcher = local.start(local.spec(attach(false))).await;
    local
        .wait(&watcher.job_id, |v| {
            v["observation"]["external"]["connection"] == "connected"
        })
        .await;
    local.store.stop(watcher.job_id.clone()).await.unwrap();
    local
        .wait(&watcher.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(
        remote
            .store
            .status(remote_job.job_id.clone())
            .await
            .unwrap()
            .status,
        "running"
    );
    let controller = local.start(local.spec(attach(true))).await;
    local
        .wait(&controller.job_id, |v| {
            v["observation"]["external"]["connection"] == "connected"
        })
        .await;
    local
        .store
        .stop_with(controller.job_id.clone(), 0, "test".into())
        .await
        .unwrap();
    let final_state = local
        .wait(&controller.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(
        final_state["observation"]["external"]["run"],
        json!(reference)
    );
    assert_eq!(
        final_state["observation"]["external"]["stop"]["terminal_observed"],
        true
    );
    assert_eq!(
        remote.store.status(remote_job.job_id).await.unwrap().status,
        "cancelled"
    );
}

#[cfg(target_os = "linux")]
fn script(path: &std::path::Path, text: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{text}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path.to_string_lossy().into()
}
#[cfg(target_os = "linux")]
#[tokio::test]
async fn remote_observer_recovers_process_loss_disconnect_and_pending_exact_run_stop() {
    let remote = Rig::new().await;
    let local = Rig::new().await;
    let (target, mut transport) = remote.target();
    let flag = local.dir.path().join("offline");
    let bridge = script(
        &local.dir.path().join("bridge"),
        "flag=$1; shift; [ ! -f \"$flag\" ] || exit 255; exec \"$@\"",
    );
    transport.args.insert(0, transport.program.clone());
    transport.args.insert(0, flag.to_string_lossy().into());
    transport.program = bridge;
    let mut remote_spec = remote.spec(long_command());
    remote_spec.idempotency_key = None;
    let watcher = local
        .start(local.spec(CommandSpec::Remote {
            remote: Box::new(RemoteSpec {
                target_id: target,
                transport,
                intent: Intent::Launch {
                    spec: Box::new(remote_spec),
                },
            }),
        }))
        .await;
    let first = local
        .wait(&watcher.job_id, |v| {
            v["observation"]["stdout"]["total_bytes"] == 4
                && v["observation"]["external"]["connection"] == "connected"
        })
        .await;
    let run: RunRef =
        serde_json::from_value(first["observation"]["external"]["run"].clone()).unwrap();
    let pid = first["job"]["runner_pid"].as_u64().unwrap() as u32;
    assert!(agent_job_relay::platform::identity(pid).is_some());
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
    let resumed = local
        .wait(&watcher.job_id, |v| {
            v["job"]["runner_pid"]
                .as_u64()
                .is_some_and(|p| p != pid as u64)
                && v["observation"]["external"]["connection"] == "connected"
        })
        .await;
    assert_eq!(resumed["job"]["run_id"], watcher.run_id);
    assert_eq!(resumed["observation"]["external"]["run"], json!(run));
    std::fs::write(&flag, "").unwrap();
    local
        .wait(&watcher.job_id, |v| {
            v["observation"]["external"]["connection"] == "disconnected"
        })
        .await;
    local
        .store
        .stop_with(watcher.job_id.clone(), 0, "test".into())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        local
            .store
            .status(watcher.job_id.clone())
            .await
            .unwrap()
            .status,
        "running"
    );
    assert_eq!(
        remote
            .store
            .status(run.job_id.clone())
            .await
            .unwrap()
            .status,
        "running"
    );
    std::fs::remove_file(flag).unwrap();
    let final_state = local
        .wait(&watcher.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(final_state["observation"]["external"]["run"], json!(run));
    assert_eq!(
        std::fs::read(remote.dir.path().join("launches")).unwrap(),
        b"x"
    );
    assert_eq!(
        std::fs::read(local.config.run_dir(&watcher.run_id).join("stdout.log")).unwrap(),
        b"A\xff\0B"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn workflow_attempt_observation_never_cancels_and_rejects_retargeting() {
    let r = Rig::new().await;
    let data = r.dir.path().join("api.json");
    let calls = r.dir.path().join("calls");
    let fail = r.dir.path().join("fail");
    let mock = script(
        &r.dir.path().join("gh"),
        &format!(
            "printf '%s\\n' \"$*\" >> '{}'; if [ -f '{}' ]; then echo 'HTTP 403 token=private-test-secret' >&2; exit 1; fi; cat '{}'",
            calls.display(),
            fail.display(),
            data.display()
        ),
    );
    let response = |status: &str, conclusion: Value, attempt: u32| json!({"id":123,"run_attempt":attempt,"repository":{"full_name":"owner/repo"},"status":status,"conclusion":conclusion,"html_url":"https://github.com/owner/repo/actions/runs/123"});
    std::fs::write(&data, response("in_progress", Value::Null, 1).to_string()).unwrap();
    let command = CommandSpec::GithubActions {
        workflow: ActionsSpec {
            repository: "owner/repo".into(),
            run_id: 123,
            attempt: 1,
            hostname: "github.com".into(),
            gh_program: mock,
        },
    };
    let watcher = r.start(r.spec(command.clone())).await;
    r.wait(&watcher.job_id, |v| {
        v["observation"]["external"]["workflow"]["status"] == "in_progress"
    })
    .await;
    r.store.stop(watcher.job_id.clone()).await.unwrap();
    r.wait(&watcher.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(
        serde_json::from_str::<Value>(&std::fs::read_to_string(&data).unwrap()).unwrap()["status"],
        "in_progress"
    );
    std::fs::write(
        &data,
        response("completed", json!("failure"), 1).to_string(),
    )
    .unwrap();
    let observer = r.start(r.spec(command.clone())).await;
    let final_state = r
        .wait(&observer.job_id, |v| v["job"]["status"] == "failed")
        .await;
    assert_eq!(
        final_state["observation"]["external"]["workflow"]["conclusion"],
        "failure"
    );
    std::fs::write(&fail, "").unwrap();
    let mut spec = r.spec(command.clone());
    spec.timeout_seconds = Some(3);
    let denied = r.start(spec).await;
    let denied = r
        .wait(&denied.job_id, |v| v["job"]["status"] == "orphaned")
        .await;
    assert!(denied.to_string().contains("authentication_or_permission"));
    assert!(!denied.to_string().contains("private-test-secret"));
    std::fs::remove_file(fail).unwrap();
    std::fs::write(
        &data,
        response("completed", json!("success"), 2).to_string(),
    )
    .unwrap();
    let mut spec = r.spec(command);
    spec.timeout_seconds = Some(3);
    let wrong = r.start(spec).await;
    let wrong = r
        .wait(&wrong.job_id, |v| v["job"]["status"] == "orphaned")
        .await;
    assert!(wrong.to_string().contains("attempt identity mismatch"));
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(calls.lines().all(|line| line.contains("--method GET")
        && line.ends_with("repos/owner/repo/actions/runs/123/attempts/1")));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn remote_mcp_cli_retry_and_milestone_keep_the_origin_session_after_mcp_exit() {
    use agent_job_relay::session::{Adapter, Registration};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let remote = Rig::new().await;
    let local = Rig::new().await;
    let register = |session_id: &str| Registration {
        agent_kind: "test".into(),
        session_id: session_id.into(),
        instance_id: id(),
        host_id: "remote-test".into(),
        adapter: Adapter::Inbox,
        endpoint: None,
        caller_thread_id: None,
        lease_seconds: 3600,
        experimental: false,
        renew_on_delivery: false,
    };
    let session = local.store.register(register("origin")).await.unwrap();
    let control = local.store.register(register("control")).await.unwrap();
    let (target, transport) = remote.target();
    let mut inner = remote.spec(command(
        "printf x >> launches; sleep 1; echo checkpoint; sleep 3; echo done",
    ));
    inner.idempotency_key = None;
    let mut spec = local.spec(CommandSpec::Remote {
        remote: Box::new(RemoteSpec {
            target_id: target,
            transport,
            intent: Intent::Launch {
                spec: Box::new(inner),
            },
        }),
    });
    spec.session = Some(session.key.clone());
    spec.monitor_rules = vec![
        serde_json::from_value(
            json!({"condition":{"kind":"contains","text":"checkpoint"},"notify_agent":true}),
        )
        .unwrap(),
    ];
    let mut child = tokio::process::Command::new(EXE)
        .arg("--state-dir")
        .arg(&local.config.root)
        .args(["mcp", "--shared-host"])
        .env("CODEX_THREAD_ID", "wrong-shared-caller")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    for (id, method, params) in [
        (
            1,
            "initialize",
            json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"remote-test","version":"1"}}),
        ),
        (
            2,
            "tools/call",
            json!({"name":"job_start","arguments":{"spec":spec}}),
        ),
    ] {
        if id == 2 {
            input
                .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
                .await
                .unwrap();
        }
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
        let value: Value = serde_json::from_str(&line).unwrap();
        assert!(value.get("error").is_none(), "{value}");
        if id == 2 {
            assert_ne!(value["result"]["isError"], true, "{value}");
        }
    }
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    // The CLI retries exactly the specification admitted through MCP.
    let file = local.dir.path().join("job.json");
    std::fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let output = tokio::process::Command::new(EXE)
        .arg("--state-dir")
        .arg(&local.config.root)
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
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let job = value["job"]["job_id"].as_str().unwrap();
    let final_state = local.wait(job, |v| v["job"]["status"] == "completed").await;
    assert_eq!(final_state["job"]["session"], session.key);
    assert_eq!(
        std::fs::read(remote.dir.path().join("launches")).unwrap(),
        b"x"
    );
    assert_eq!(local.store.list(0, 100).await.unwrap().len(), 1);
    for _ in 0..100 {
        let deliveries = local
            .store
            .deliveries(Some(job.into()), 0, 20)
            .await
            .unwrap();
        if deliveries["deliveries"].as_array().unwrap().len() == 2
            && deliveries["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|d| d["status"] == "notification_recorded")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let inbox = local.store.inbox(session.key, 0, 20).await.unwrap();
    assert_eq!(inbox["messages"].as_array().unwrap().len(), 2, "{inbox}");
    let inbox = local.store.inbox(control.key, 0, 20).await.unwrap();
    assert_eq!(inbox["messages"].as_array().unwrap().len(), 0, "{inbox}");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancelling_a_lost_launch_acknowledgement_uses_lookup_without_relaunch() {
    let remote = Rig::new().await;
    let local = Rig::new().await;
    let (target, mut transport) = remote.target();
    let bridge = script(
        &local.dir.path().join("lost-ack"),
        "root=$1; shift; body=$(cat); printf '%s\\n' \"$body\" >> \"$root/calls\"; printf '%s' \"$body\" | \"$@\" > \"$root/response\" || exit 1; if [ ! -f \"$root/lost\" ]; then touch \"$root/lost\"; exit 255; fi; cat \"$root/response\"",
    );
    transport.args.insert(0, transport.program.clone());
    transport
        .args
        .insert(0, local.dir.path().to_string_lossy().into());
    transport.program = bridge;
    let mut inner = remote.spec(long_command());
    inner.idempotency_key = None;
    let watcher = local
        .start(local.spec(CommandSpec::Remote {
            remote: Box::new(RemoteSpec {
                target_id: target,
                transport,
                intent: Intent::Launch {
                    spec: Box::new(inner),
                },
            }),
        }))
        .await;
    let uncertain = local
        .wait(&watcher.job_id, |v| {
            v["observation"]["external"]["connection"] == "disconnected"
        })
        .await;
    assert!(uncertain["observation"]["external"]["run"].is_null());
    local
        .store
        .stop_with(watcher.job_id.clone(), 0, "test".into())
        .await
        .unwrap();
    local
        .wait(&watcher.job_id, |v| v["job"]["status"] == "cancelled")
        .await;
    assert_eq!(
        std::fs::read(remote.dir.path().join("launches")).unwrap(),
        b"x"
    );
    let calls = std::fs::read_to_string(local.dir.path().join("calls")).unwrap();
    let calls: Vec<Value> = calls
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(
        calls
            .iter()
            .filter(|c| c["operation"]["op"] == "launch")
            .count(),
        1
    );
    assert!(calls.iter().any(|c| c["operation"]["op"] == "lookup"));
    let jobs = remote.store.list(0, 100).await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "cancelled");
}

#[tokio::test]
async fn remote_runner_loss_is_unknown_and_never_replayed_by_the_observer() {
    let remote = Rig::new().await;
    let local = Rig::new().await;
    let (target, transport) = remote.target();
    let running = remote.start(remote.spec(long_command())).await;
    let running = remote
        .wait(&running.job_id, |v| {
            v["observation"]["stdout"]["total_bytes"] == 4
        })
        .await;
    let running: Job = serde_json::from_value(running["job"].clone()).unwrap();
    let watcher = local
        .start(local.spec(CommandSpec::Remote {
            remote: Box::new(RemoteSpec {
                target_id: target,
                transport,
                intent: Intent::Attach {
                    run: RunRef::from_job(&running),
                    allow_stop: false,
                },
            }),
        }))
        .await;
    local
        .wait(&watcher.job_id, |v| {
            v["observation"]["external"]["connection"] == "connected"
        })
        .await;
    let pid = running.runner_pid.unwrap();
    let identity = running.runner_identity.unwrap();
    assert_eq!(
        agent_job_relay::platform::identity(pid).as_ref(),
        Some(&identity)
    );
    #[cfg(windows)]
    agent_job_relay::platform::terminate_instance(pid, &identity).unwrap();
    #[cfg(target_os = "linux")]
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
    let observed = local
        .wait(&watcher.job_id, |v| v["job"]["status"] == "orphaned")
        .await;
    assert_eq!(observed["observation"]["external"]["status"], "orphaned");
    assert!(observed["job"]["exit_code"].is_null());
    assert_eq!(
        std::fs::read(remote.dir.path().join("launches")).unwrap(),
        b"x"
    );
    // Clean only the test's identity-checked native workload group, whose effects
    // are deliberately unknown after its supervisor was killed.
    #[cfg(target_os = "linux")]
    if let Some(workload) = running.workload_pid
        && agent_job_relay::platform::identity(workload) == running.workload_identity
    {
        agent_job_relay::platform::signal_group(workload, 9).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn observer_storage_failure_does_not_become_an_external_execution_failure() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config.clone());
    store.init().await.unwrap();
    let spec = StartSpec {
        command: CommandSpec::Remote {
            remote: Box::new(RemoteSpec {
                target_id: id(),
                transport: Transport {
                    program: "/bin/false".into(),
                    args: vec![],
                },
                intent: Intent::Attach {
                    run: RunRef {
                        job_id: id(),
                        run_id: id(),
                        attempt: 1,
                    },
                    allow_stop: false,
                },
            }),
        },
        cwd: dir.path().into(),
        env: BTreeMap::new(),
        timeout_seconds: Some(3),
        session: None,
        idempotency_key: None,
        monitor_rules: vec![],
    };
    let job = store.start(spec).await.unwrap();
    agent_job_relay::platform::secure_dir(&config.run_dir(&job.run_id)).unwrap();
    std::fs::create_dir(config.run_dir(&job.run_id).join("stdout.log")).unwrap();
    let key = job.job_id.clone();
    let nonce = store
        .call(move |db| {
            let tx = db.transaction().unwrap();
            tx.execute("UPDATE jobs SET status='launching' WHERE id=?", [&key])?;
            tx.execute("UPDATE runs SET status='launching' WHERE job_id=?", [&key])?;
            let nonce: String =
                tx.query_row("SELECT nonce FROM runs WHERE job_id=?", [key], |r| r.get(0))?;
            tx.commit()?;
            Ok(nonce)
        })
        .await
        .unwrap();
    let output = tokio::process::Command::new(EXE)
        .arg("--state-dir")
        .arg(&config.root)
        .args(["runner", &job.job_id, &nonce])
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    let observed = store.status(job.job_id).await.unwrap();
    assert_eq!(observed.status, "orphaned");
    assert!(observed.exit_code.is_none());
    assert!(
        observed
            .detail
            .unwrap()
            .contains("external outcome unknown")
    );
}

#[tokio::test]
async fn observer_recovery_does_not_reopen_a_racing_terminal_run() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config.clone());
    store.init().await.unwrap();
    let spec = StartSpec {
        command: CommandSpec::Remote {
            remote: Box::new(RemoteSpec {
                target_id: id(),
                transport: Transport {
                    program: "unused-transport".into(),
                    args: vec![],
                },
                intent: Intent::Attach {
                    run: RunRef {
                        job_id: id(),
                        run_id: id(),
                        attempt: 1,
                    },
                    allow_stop: true,
                },
            }),
        },
        cwd: dir.path().into(),
        env: BTreeMap::new(),
        timeout_seconds: Some(3),
        session: None,
        idempotency_key: None,
        monitor_rules: vec![],
    };
    let job = store.start(spec).await.unwrap();
    // A queued control attachment must retain its stop intent until the observer
    // can contact the already-existing execution.
    let pending = store.stop(job.job_id.clone()).await.unwrap();
    assert_eq!(pending.status, "queued");
    assert!(pending.cancel_requested);
    let db = rusqlite::Connection::open(config.root.join("relay.sqlite3")).unwrap();
    db.execute("UPDATE jobs SET status='running' WHERE id=?", [&job.job_id])
        .unwrap();
    db.execute(
        "UPDATE runs SET status='running',started=? WHERE id=?",
        rusqlite::params![now(), job.run_id],
    )
    .unwrap();
    // Pin a real WAL writer while recovery reads the old active snapshot.
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    let observing = store.clone();
    let tick = tokio::spawn(async move { agent_job_relay::daemon::tick(&observing).await });
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let probe = config.run_lock(&job.run_id).unwrap();
        match probe.try_lock() {
            Err(std::fs::TryLockError::WouldBlock) => break,
            Ok(()) => {}
            Err(error) => panic!("{error}"),
        }
        drop(probe);
        assert!(
            tokio::time::Instant::now() < until,
            "recovery did not acquire its run lock"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    agent_job_relay::storage::finish_tx(
        &db,
        &job.job_id,
        "completed",
        Some(0),
        None,
        "racing terminal evidence",
    )
    .unwrap();
    db.execute_batch("COMMIT").unwrap();
    tick.await.unwrap().unwrap();
    let observed = store.status(job.job_id).await.unwrap();
    assert_eq!(observed.status, "completed");
    assert_eq!(observed.run.status, "completed");
    assert_eq!(observed.run.run_id, job.run_id);
}
