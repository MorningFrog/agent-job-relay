#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
    storage::Store,
};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn old_notification_runtime_is_refused_before_registration_or_workload() {
    let r = Rig::new();
    let host = Host::new(&r, true);
    r.config.prepare().unwrap();
    let listener = tokio::net::UnixListener::bind(r.config.socket()).unwrap();
    let old = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let v = ipc::read_frame(&mut stream, 65536).await.unwrap();
            assert_eq!(v["request"]["op"], "doctor");
            ipc::write_frame(&mut stream,&json!({"protocol_version":1,"ok":true,"result":{"runner_launch_from_loaded_image":true,"version":"0.1.2"}})).await.unwrap();
        }
    });
    let out = r
        .cli(
            &[
                "start",
                "--wake",
                "--cwd",
                r.dir.path().to_str().unwrap(),
                "--shell",
                "touch must-not-run",
            ],
            Some(("thread-a", &host.endpoint)),
        )
        .await;
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("predates notification recovery/renewal")
    );
    assert!(!r.config.root.join("relay.sqlite3").exists());
    assert!(!r.dir.path().join("must-not-run").exists());
    old.await.unwrap();
}

struct Rig {
    dir: tempfile::TempDir,
    config: Config,
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("ux-")
            .tempdir_in(common::root())
            .unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        Self { dir, config }
    }
    fn command(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"));
        c.arg("--state-dir")
            .arg(&self.config.root)
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .stdin(Stdio::null())
            .kill_on_drop(true);
        c
    }
    async fn cli(&self, args: &[&str], host: Option<(&str, &PathBuf)>) -> std::process::Output {
        let mut c = self.command();
        c.args(args);
        if let Some((thread, endpoint)) = host {
            c.env("CODEX_THREAD_ID", thread)
                .env("CODEX_APP_TOOLS_PIPE_PATH", endpoint);
        }
        tokio::time::timeout(Duration::from_secs(15), c.output())
            .await
            .unwrap()
            .unwrap()
    }
    async fn terminal(&self, j: &str) -> Job {
        let s = Store::new(self.config.clone());
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let job = s.status(j.into()).await.unwrap();
                if terminal(&job.status) {
                    break job;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap()
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        // Only this fixture's private socket; no PID-based termination or real host.
        use std::io::{Read, Write};
        if let Ok(mut s) = std::os::unix::net::UnixStream::connect(self.config.socket()) {
            let _ = s.set_read_timeout(Some(Duration::from_secs(1)));
            let _ = s.set_write_timeout(Some(Duration::from_secs(1)));
            let data =
                serde_json::to_vec(&json!({"protocol_version":1,"request":{"op":"shutdown"}}))
                    .unwrap();
            let _ = s.write_all(&(data.len() as u32).to_le_bytes());
            let _ = s.write_all(&data);
            let mut len = [0; 4];
            if s.read_exact(&mut len).is_ok() {
                let n = u32::from_le_bytes(len) as usize;
                if n <= 65536 {
                    let _ = s.read_exact(&mut vec![0; n]);
                }
            }
        }
    }
}
struct Host {
    endpoint: PathBuf,
    received: tokio::sync::mpsc::Receiver<Value>,
    task: tokio::task::JoinHandle<()>,
}
impl Host {
    fn new(r: &Rig, advertise: bool) -> Self {
        let endpoint = r.dir.path().join("desktop.sock");
        let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
        let (tx, received) = tokio::sync::mpsc::channel(16);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut s, _)) = listener.accept().await else {
                    break;
                };
                let Ok(v) = ipc::read_frame(&mut s, 65536).await else {
                    continue;
                };
                assert_eq!(v["method"], "tools/list");
                let tools = if advertise {
                    json!([{"namespace":"codex_app","name":"send_message_to_thread"}])
                } else {
                    json!([])
                };
                if ipc::write_frame(
                    &mut s,
                    &json!({"jsonrpc":"2.0","id":v["id"],"result":{"tools":tools}}),
                )
                .await
                .is_err()
                {
                    continue;
                }
                if let Ok(v) = ipc::read_frame(&mut s, 65536).await {
                    assert_eq!(v["method"], "tools/call");
                    assert_eq!(v["params"]["callerSource"], "codex");
                    assert_eq!(
                        v["params"]["threadId"],
                        v["params"]["arguments"]["threadId"]
                    );
                    ipc::write_frame(
                        &mut s,
                        &json!({"jsonrpc":"2.0","id":v["id"],"result":{"success":true}}),
                    )
                    .await
                    .unwrap();
                    if tx.send(v).await.is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            endpoint,
            received,
            task,
        }
    }
    async fn delivery(&mut self) -> Value {
        tokio::time::timeout(Duration::from_secs(8), self.received.recv())
            .await
            .unwrap()
            .unwrap()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn success(out: std::process::Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[tokio::test]
async fn daemon_launches_runner_after_its_executable_is_replaced_by_a_rebuild() {
    let r = Rig::new();
    let image = r.dir.path().join("native");
    std::fs::copy(env!("CARGO_BIN_EXE_agent-job-relay"), &image).unwrap();
    let first = success(
        tokio::process::Command::new(&image)
            .arg("--state-dir")
            .arg(&r.config.root)
            .args(["start", "--", "/bin/true"])
            .output()
            .await
            .unwrap(),
    );
    r.terminal(first["job"]["job_id"].as_str().unwrap()).await;
    let socket = tokio::net::UnixStream::connect(r.config.socket())
        .await
        .unwrap();
    let owner = socket.peer_cred().unwrap().pid().unwrap();
    drop(socket);
    let replacement = r.dir.path().join("rebuilt-native");
    std::fs::copy(env!("CARGO_BIN_EXE_agent-job-relay"), &replacement).unwrap();
    std::fs::rename(replacement, &image).unwrap();
    assert!(
        std::fs::read_link(format!("/proc/{owner}/exe"))
            .unwrap()
            .to_string_lossy()
            .ends_with(" (deleted)")
    );
    let started = success(
        r.cli(
            &[
                "start",
                "--cwd",
                r.dir.path().to_str().unwrap(),
                "--shell",
                "echo once >> execution-count",
            ],
            None,
        )
        .await,
    );
    let job = r.terminal(started["job"]["job_id"].as_str().unwrap()).await;
    assert_eq!(job.status, "completed", "{:?}", job.detail);
    assert_eq!(job.exit_code, Some(0));
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("execution-count")).unwrap(),
        "once\n"
    );
    let socket = tokio::net::UnixStream::connect(r.config.socket())
        .await
        .unwrap();
    assert_eq!(socket.peer_cred().unwrap().pid(), Some(owner));
    let health = ipc::call(&r.config, Request::Doctor).await.unwrap();
    assert_eq!(health["daemon_executable_path_available"], false);
    assert_eq!(health["runner_launch_from_loaded_image"], true);
}

#[tokio::test]
async fn installed_shared_mcp_uses_each_terminal_binding_and_never_its_inherited_thread() {
    let r = Rig::new();
    let mut host = Host::new(&r, true);
    let install = r.dir.path().join("plugin");
    success(
        r.cli(
            &["install", "--root", install.to_str().unwrap(), "--apply"],
            None,
        )
        .await,
    );
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(install.join(".mcp.json")).unwrap()).unwrap();
    let server = &manifest["mcpServers"]["agent-job-relay"];
    let program = server["command"]
        .as_str()
        .unwrap()
        .replace("${PLUGIN_ROOT}", install.to_str().unwrap());
    let mut child = tokio::process::Command::new(program)
        .args(
            server["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap()),
        )
        .env("AJR_STATE_DIR", r.dir.path().join("wrong-state"))
        .env("CODEX_THREAD_ID", "stale-server-thread")
        .env("CODEX_APP_TOOLS_PIPE_PATH", &host.endpoint)
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
        v: Value,
    ) -> Value {
        input.write_all(format!("{v}\n").as_bytes()).await.unwrap();
        let text = tokio::time::timeout(Duration::from_secs(8), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        serde_json::from_str(&text).unwrap()
    }
    let init = rpc(&mut input, &mut lines, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"installed-plugin-test","version":"1"}}})).await;
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("shared MCP process")
    );
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    let refused = rpc(&mut input, &mut lines, json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"job_start","arguments":{"wake_me":true,"spec":{"command":{"kind":"exec","program":"/bin/true","args":[]},"cwd":r.dir.path()}}}})).await;
    assert_eq!(refused["result"]["isError"], true);
    assert!(!r.config.root.exists());
    let doctor = rpc(&mut input, &mut lines, json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"job_doctor","arguments":{}}})).await;
    let doctor: Value =
        serde_json::from_str(doctor["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(doctor["daemon"], "stopped");
    assert!(
        doctor["mcp_host_connection"]
            .as_str()
            .unwrap()
            .starts_with("responding")
    );
    assert_eq!(doctor["desktop_context"]["available"], false);
    assert!(!r.config.root.exists());
    let mut jobs = Vec::new();
    for (i, thread) in ["thread-a", "thread-b"].iter().enumerate() {
        let bind = tokio::process::Command::new(install.join("relay"))
            .args(["session", "connect", "--brief"])
            .env("AJR_STATE_DIR", r.dir.path().join("wrong-state"))
            .env("CODEX_THREAD_ID", thread)
            .env("CODEX_APP_TOOLS_PIPE_PATH", &host.endpoint)
            .output()
            .await
            .unwrap();
        let binding = success(bind);
        assert!(binding.get("session").is_none()); // Brief responses never expose endpoints.
        assert_eq!(binding["probe"], "passed");
        let started = rpc(&mut input, &mut lines, json!({"jsonrpc":"2.0","id":4+i,"method":"tools/call","params":{"name":"job_start","arguments":{"spec":{"command":{"kind":"shell","interpreter":"/bin/sh","script":"sleep 0.2; echo installed MCP"},"cwd":r.dir.path(),"session":binding["session_key"]}}}})).await;
        assert_ne!(started["result"]["isError"], true, "{started}");
        let result: Value =
            serde_json::from_str(started["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        jobs.push(result["job"]["job_id"].as_str().unwrap().to_string());
        let recovery = rpc(&mut input, &mut lines, json!({"jsonrpc":"2.0","id":20+i,"method":"tools/call","params":{"name":"job_recover_session","arguments":{"session_key":binding["session_key"]}}})).await;
        assert_ne!(recovery["result"]["isError"], true, "{recovery}");
        let recovery: Value =
            serde_json::from_str(recovery["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(recovery["session_key"], binding["session_key"]);
        assert_eq!(recovery["apply"], false);
    }
    drop(input);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    for j in jobs {
        assert_eq!(r.terminal(&j).await.status, "completed");
    }
    let mut destinations = vec![
        host.delivery().await["params"]["arguments"]["threadId"]
            .as_str()
            .unwrap()
            .to_string(),
        host.delivery().await["params"]["arguments"]["threadId"]
            .as_str()
            .unwrap()
            .to_string(),
    ];
    destinations.sort();
    assert_eq!(destinations, vec!["thread-a", "thread-b"]);
    let sessions = ipc::call(
        &r.config,
        Request::Sessions {
            after: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 2);
    assert!(!r.dir.path().join("wrong-state").exists());
}

#[tokio::test]
async fn installed_recovery_previews_then_rebinds_only_current_chat_after_host_replacement() {
    let r = Rig::new();
    let host = Host::new(&r, true);
    let install = r.dir.path().join("plugin");
    success(
        r.cli(
            &["install", "--root", install.to_str().unwrap(), "--apply"],
            None,
        )
        .await,
    );
    let mut jobs = Vec::new();
    for thread in ["thread-a", "thread-b"] {
        let out = tokio::process::Command::new(install.join("relay"))
            .args([
                "start",
                "--wake",
                "--cwd",
                r.dir.path().to_str().unwrap(),
                "--shell",
                "sleep 0.4; echo recovery",
            ])
            .env("CODEX_THREAD_ID", thread)
            .env("CODEX_APP_TOOLS_PIPE_PATH", &host.endpoint)
            .env("AJR_STATE_DIR", r.dir.path().join("wrong-state"))
            .output()
            .await
            .unwrap();
        jobs.push(success(out)["job"]["job_id"].as_str().unwrap().to_owned());
    }
    let endpoint = host.endpoint.clone();
    drop(host);
    std::fs::remove_file(&endpoint).unwrap();
    let mut replacement = Host::new(&r, true);
    for job in &jobs {
        assert_eq!(r.terminal(job).await.status, "completed");
    }
    let store = Store::new(r.config.clone());
    let before = store
        .deliveries(Some(jobs[0].clone()), 0, 20)
        .await
        .unwrap()["deliveries"][0]
        .clone();
    let recover = |apply: bool| {
        let mut c = tokio::process::Command::new(install.join("relay"));
        c.args(["session", "recover"])
            .env("CODEX_THREAD_ID", "thread-a")
            .env("CODEX_APP_TOOLS_PIPE_PATH", &endpoint)
            .env("AJR_STATE_DIR", r.dir.path().join("wrong-state"));
        if apply {
            c.arg("--apply");
        }
        c
    };
    let preview = success(recover(false).output().await.unwrap());
    assert_eq!(preview["targets"].as_array().unwrap().len(), 1);
    assert_eq!(preview["targets"][0]["job_id"], jobs[0]);
    let result = success(recover(true).output().await.unwrap());
    assert_eq!(result["targets"][0]["applied"], true);
    let message = replacement.delivery().await;
    assert_eq!(message["params"]["arguments"]["threadId"], "thread-a");
    assert_eq!(message["params"]["callId"], before["id"]);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), replacement.received.recv())
            .await
            .is_err()
    );
    let history = success(recover(false).output().await.unwrap());
    assert_eq!(history["recent_recoveries"].as_array().unwrap().len(), 1);
    assert!(!r.dir.path().join("wrong-state").exists());
}

#[tokio::test]
async fn installed_long_job_renews_expired_binding_only_when_completion_is_due() {
    let r = Rig::new();
    let mut host = Host::new(&r, true);
    let install = r.dir.path().join("plugin");
    success(
        r.cli(
            &["install", "--root", install.to_str().unwrap(), "--apply"],
            None,
        )
        .await,
    );
    let out = tokio::process::Command::new(install.join("relay"))
        .args([
            "start",
            "--wake",
            "--timeout",
            "10",
            "--cwd",
            r.dir.path().to_str().unwrap(),
            "--shell",
            "while [ ! -f release ]; do sleep 0.02; done; echo long-job",
        ])
        .env("CODEX_THREAD_ID", "thread-a")
        .env("CODEX_APP_TOOLS_PIPE_PATH", &host.endpoint)
        .output()
        .await
        .unwrap();
    let started = success(out);
    let job = started["job"]["job_id"].as_str().unwrap();
    let store = Store::new(r.config.clone());
    store
        .call(|db| {
            db.execute("UPDATE sessions SET expires=?", [now() - 172_800_000])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), host.received.recv())
            .await
            .is_err()
    );
    std::fs::write(r.dir.path().join("release"), b"go").unwrap();
    assert_eq!(r.terminal(job).await.status, "completed");
    let message = host.delivery().await;
    assert_eq!(message["params"]["arguments"]["threadId"], "thread-a");
    let key = started["job"]["session"].as_str().unwrap().to_owned();
    assert!(store.session(key).await.unwrap().expires_at > now());
    let renewal_count = store
        .call(|db| {
            Ok(
                db.query_row("SELECT count(*) FROM target_recoveries", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(renewal_count, 1);
}

#[tokio::test]
async fn cli_wake_automatically_starts_registers_probes_and_deduplicates() {
    let r = Rig::new();
    let mut host = Host::new(&r, true);
    let args = [
        "start",
        "--wake",
        "--cwd",
        r.dir.path().to_str().unwrap(),
        "--idempotency-key",
        "automatic-once",
        "--shell",
        "echo executed >> counter",
    ];
    let first = success(r.cli(&args, Some(("thread-a", &host.endpoint))).await);
    assert_eq!(first["wake"]["automatic_binding"], true);
    assert_eq!(first["wake"]["probe"], "passed");
    let j = first["job"]["job_id"].as_str().unwrap();
    assert_eq!(r.terminal(j).await.status, "completed");
    let delivery = host.delivery().await;
    assert_eq!(delivery["params"]["arguments"]["threadId"], "thread-a");
    let again = success(r.cli(&args, Some(("thread-a", &host.endpoint))).await);
    assert_eq!(again["job"]["job_id"], j);
    assert_eq!(again["job"]["session"], first["job"]["session"]);
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("counter")).unwrap(),
        "executed\n"
    );
    let sessions = ipc::call(
        &r.config,
        Request::Sessions {
            after: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 1);
    let connected = success(
        r.cli(&["session", "connect"], Some(("thread-a", &host.endpoint)))
            .await,
    );
    assert_eq!(connected["session"]["key"], first["job"]["session"]);
}
#[tokio::test]
async fn missing_host_context_fails_before_creating_state_or_executing() {
    let r = Rig::new();
    let out = r.cli(&["start", "--wake", "--", "/bin/true"], None).await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("CODEX_THREAD_ID"));
    assert!(!r.config.root.exists());
    let out = r
        .cli(
            &[
                "start",
                "--wake",
                "--session",
                "explicit",
                "--",
                "/bin/true",
            ],
            None,
        )
        .await;
    assert!(!out.status.success());
    assert!(!r.config.root.exists());
    let v = success(r.cli(&["start", "--", "/bin/true"], None).await);
    assert_eq!(
        r.terminal(v["job"]["job_id"].as_str().unwrap())
            .await
            .status,
        "completed"
    );
}
#[tokio::test]
async fn unavailable_host_capability_does_not_execute_or_fall_back() {
    let r = Rig::new();
    let host = Host::new(&r, false);
    let out = r
        .cli(
            &["start", "--wake", "--", "/bin/true"],
            Some(("thread-a", &host.endpoint)),
        )
        .await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("does not advertise"));
    assert!(
        Store::new(r.config.clone())
            .list(0, 20)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!r.config.root.join("inbox").exists());
}
#[tokio::test]
async fn concurrent_callers_are_bound_to_their_own_threads() {
    let r = Rig::new();
    let mut host = Host::new(&r, true);
    let (a, b) = tokio::join!(
        r.cli(
            &["start", "--wake", "--", "/bin/true"],
            Some(("thread-a", &host.endpoint))
        ),
        r.cli(
            &["start", "--wake", "--", "/bin/true"],
            Some(("thread-b", &host.endpoint))
        )
    );
    let a = success(a);
    let b = success(b);
    assert_ne!(a["job"]["session"], b["job"]["session"]);
    r.terminal(a["job"]["job_id"].as_str().unwrap()).await;
    r.terminal(b["job"]["job_id"].as_str().unwrap()).await;
    let mut destinations = vec![];
    for _ in 0..2 {
        let v = host.delivery().await;
        destinations.push(
            v["params"]["arguments"]["threadId"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }
    destinations.sort();
    assert_eq!(destinations, vec!["thread-a", "thread-b"]);
    assert!(ipc::call(&r.config, Request::Doctor).await.is_ok());
}
#[tokio::test]
async fn stop_daemon_preserves_runner_and_next_submission_restarts_it() {
    let r = Rig::new();
    let v = success(
        r.cli(&["start", "--shell", "sleep 0.4; echo survived"], None)
            .await,
    );
    let j = v["job"]["job_id"].as_str().unwrap().to_string();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let job = Store::new(r.config.clone())
                .status(j.clone())
                .await
                .unwrap();
            if job.workload_pid.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let stop = success(r.cli(&["daemon", "stop"], None).await);
    assert_eq!(stop["runners"], "continue independently");
    tokio::time::timeout(Duration::from_secs(3), async {
        while ipc::call(&r.config, Request::Doctor).await.is_ok() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(r.terminal(&j).await.status, "completed");
    assert_eq!(
        success(r.cli(&["daemon", "stop"], None).await)["already_stopped"],
        true
    );
    let v = success(r.cli(&["start", "--", "/bin/true"], None).await);
    r.terminal(v["job"]["job_id"].as_str().unwrap()).await;
    assert_eq!(
        ipc::call(&r.config, Request::Status { job_id: j })
            .await
            .unwrap()["status"],
        "completed"
    );
}
#[tokio::test]
async fn mcp_wake_me_uses_current_context_and_survives_mcp_exit() {
    let r = Rig::new();
    let mut host = Host::new(&r, true);
    let mut command = r.command();
    command
        .arg("mcp")
        .env("CODEX_THREAD_ID", "mcp-thread-a")
        .env("CODEX_APP_TOOLS_PIPE_PATH", &host.endpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    async fn send(input: &mut tokio::process::ChildStdin, v: Value) {
        input.write_all(format!("{v}\n").as_bytes()).await.unwrap();
        input.flush().await.unwrap();
    }
    async fn receive(
        lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    ) -> Value {
        serde_json::from_str(
            &tokio::time::timeout(Duration::from_secs(12), lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }
    send(&mut input,json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"convenience-test","version":"1"}}})).await;
    assert!(receive(&mut lines).await.get("result").is_some());
    send(
        &mut input,
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    send(
        &mut input,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )
    .await;
    let tools = receive(&mut lines).await;
    let tools = tools["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "job_connect_session"));
    assert!(!tools.iter().any(|t| t["name"] == "job_shutdown"));
    assert!(tools.iter().any(|t| t["name"] == "job_start"
        && t["inputSchema"]["properties"]["wake_me"]["type"] == "boolean"));
    send(&mut input,json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"job_start","arguments":{"wake_me":true,"spec":{"command":{"kind":"shell","interpreter":"/bin/sh","script":"sleep 0.2; echo awake"},"cwd":r.dir.path()}}}})).await;
    let response = receive(&mut lines).await;
    assert_ne!(response["result"]["isError"], true, "{response}");
    let result: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result["wake"]["automatic_binding"], true);
    let j = result["job"]["job_id"].as_str().unwrap();
    drop(input);
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.terminal(j).await.status, "completed");
    let delivered = host.delivery().await;
    assert_eq!(delivered["params"]["arguments"]["threadId"], "mcp-thread-a");
}

#[tokio::test]
async fn host_replacement_gets_a_new_binding_without_moving_old_jobs() {
    let r = Rig::new();
    let host = Host::new(&r, true);
    let args = ["start", "--wake", "--shell", "sleep 0.3; echo old-host"];
    let old = success(r.cli(&args, Some(("thread-a", &host.endpoint))).await);
    let old_key = old["job"]["session"].as_str().unwrap().to_string();
    let old_job = old["job"]["job_id"].as_str().unwrap().to_string();
    let endpoint = host.endpoint.clone();
    drop(host);
    std::fs::remove_file(&endpoint).unwrap();
    let mut replacement = Host::new(&r, true);
    let connected = success(
        r.cli(
            &["session", "connect"],
            Some(("thread-a", &replacement.endpoint)),
        )
        .await,
    );
    assert_ne!(connected["session"]["key"], old_key);
    assert_eq!(
        Store::new(r.config.clone())
            .status(old_job.clone())
            .await
            .unwrap()
            .session,
        Some(old_key)
    );
    let next = success(
        r.cli(
            &["start", "--wake", "--", "/bin/true"],
            Some(("thread-a", &replacement.endpoint)),
        )
        .await,
    );
    assert_eq!(next["job"]["session"], connected["session"]["key"]);
    r.terminal(&old_job).await;
    r.terminal(next["job"]["job_id"].as_str().unwrap()).await;
    let delivered = replacement.delivery().await;
    let prompt = delivered["params"]["arguments"]["prompt"].as_str().unwrap();
    assert!(prompt.contains(next["job"]["job_id"].as_str().unwrap()));
    assert!(!prompt.contains(&old_job));
}

#[tokio::test]
async fn existing_incompatible_daemon_is_not_replaced_or_treated_as_absent() {
    let r = Rig::new();
    r.config.prepare().unwrap();
    let listener = tokio::net::UnixListener::bind(r.config.socket()).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = ipc::read_frame(&mut stream, 65536).await.unwrap();
        assert_eq!(request["request"]["op"], "doctor");
        ipc::write_frame(
            &mut stream,
            &json!({"protocol_version":99,"ok":true,"result":{}}),
        )
        .await
        .unwrap();
        listener
    });
    let out = r.cli(&["start", "--", "/bin/true"], None).await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unsupported daemon protocol"));
    assert!(!r.config.root.join("startup.lock").exists());
    assert!(!r.config.root.join("relay.sqlite3").exists());
    drop(server.await.unwrap());
}
