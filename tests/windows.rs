#![cfg(windows)]
mod common;
use agent_job_relay::{
    client,
    config::Config,
    core::*,
    ipc::{self, Request},
    platform,
    storage::Store,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
fn fixture() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = common::root().join(format!("windows-fixture-{}", id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("workload.exe");
        let result = Command::new("rustc")
            .arg("--edition=2024")
            .arg("--crate-name=windows_workload")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/windows-workload.rs"))
            .arg("-o")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        path
    })
}
// Cargo owns a non-breakaway Job Object on Windows. Launch fixture daemons
// through local CIM, under the same SID and outside Cargo's cleanup job, so the
// tests exercise real process independence rather than weakening the runtime.
struct DaemonProcess {
    pid: u32,
    identity: String,
}
impl DaemonProcess {
    fn kill(&self) -> anyhow::Result<()> {
        if platform::identity(self.pid).is_none() {
            return Ok(());
        }
        platform::terminate_instance(self.pid, &self.identity)
    }
    fn wait(&self) -> anyhow::Result<()> {
        for _ in 0..200 {
            if platform::identity(self.pid).as_ref() != Some(&self.identity) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        anyhow::bail!("fixture daemon did not exit")
    }
    fn try_wait(&self) -> anyhow::Result<Option<&'static str>> {
        Ok((platform::identity(self.pid).as_ref() != Some(&self.identity)).then_some("exited"))
    }
}
fn launch_daemon(dir: &Path, state: &Path) -> DaemonProcess {
    let literal = |path: &Path| path.to_str().unwrap().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue'
$pinfo=[Diagnostics.ProcessStartInfo]::new()
$pinfo.FileName='{}'
$pinfo.Arguments='--state-dir "{}" daemon'
$pinfo.UseShellExecute=$false;$pinfo.RedirectStandardError=$true;$pinfo.CreateNoWindow=$false
$p=[Diagnostics.Process]::Start($pinfo)
$errorText=$p.StandardError.ReadToEndAsync()
[IO.File]::WriteAllText('{}',$p.Id.ToString())
if(!$p.WaitForExit(180000)){{$p.Kill();$p.WaitForExit()}}
[IO.File]::WriteAllText('{}',$errorText.GetAwaiter().GetResult())
"#,
        literal(Path::new(env!("CARGO_BIN_EXE_agent-job-relay"))),
        literal(state),
        literal(&dir.join("daemon.pid")),
        literal(&dir.join("daemon.log"))
    );
    let script_path = dir.join("daemon-launch.ps1");
    std::fs::write(&script_path, script).unwrap();
    launch_fixture_script(&script_path);
    for _ in 0..200 {
        if let Ok(text) = std::fs::read_to_string(dir.join("daemon.pid"))
            && let Ok(pid) = text.parse()
            && let Some(identity) = platform::identity(pid)
        {
            return DaemonProcess { pid, identity };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!(
        "fixture launcher failed: {}",
        std::fs::read_to_string(dir.join("daemon.log")).unwrap_or_default()
    );
}
fn launch_fixture_script(script_path: &Path) {
    let ps = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let command_line = format!(
        "\"{}\" -NoLogo -NoProfile -NonInteractive -File \"{}\"",
        ps.display(),
        script_path.display()
    )
    .replace('\'', "''");
    let request = format!(
        "$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';$startup=New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly -Property @{{ShowWindow=[uint16]0;CreateFlags=[uint32]0x10}};$result=Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine='{command_line}';ProcessStartupInformation=$startup}};if($result.ReturnValue -ne 0){{throw ('fixture process creation failed: '+$result.ReturnValue)}}"
    );
    let out = Command::new(ps)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &request,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
struct Rig {
    dir: tempfile::TempDir,
    config: Config,
    daemon: Option<DaemonProcess>,
    jobs: Mutex<Vec<String>>,
}
impl Rig {
    async fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("windows-")
            .tempdir_in(common::root())
            .unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        let mut rig = Self {
            dir,
            config,
            daemon: None,
            jobs: Mutex::new(Vec::new()),
        };
        rig.boot().await;
        rig
    }
    async fn boot(&mut self) {
        let _ = std::fs::remove_file(self.dir.path().join("daemon.pid"));
        self.daemon = Some(launch_daemon(self.dir.path(), &self.config.root));
        for _ in 0..200 {
            if ipc::call(&self.config, Request::Doctor).await.is_ok() {
                return;
            }
            if let Some(status) = self.daemon.as_mut().unwrap().try_wait().unwrap() {
                panic!(
                    "daemon exited {status}: {}",
                    std::fs::read_to_string(self.dir.path().join("daemon.log")).unwrap()
                );
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("daemon readiness timeout");
    }
    fn spec(&self, args: &[&str]) -> StartSpec {
        StartSpec {
            command: CommandSpec::Exec {
                program: fixture().to_str().unwrap().into(),
                args: args.iter().map(|s| (*s).into()).collect(),
            },
            cwd: self.dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
        }
    }
    async fn start(&self, spec: StartSpec) -> Value {
        let value = client::start(&self.config, spec, false).await.unwrap();
        self.jobs
            .lock()
            .unwrap()
            .push(value["job"]["job_id"].as_str().unwrap().into());
        value["job"].clone()
    }
    async fn status(&self, job: &Value) -> Value {
        ipc::call(
            &self.config,
            Request::Status {
                job_id: job["job_id"].as_str().unwrap().into(),
            },
        )
        .await
        .unwrap()
    }
    async fn terminal(&self, job: &Value) -> Value {
        for _ in 0..400 {
            let state = self.status(job).await;
            if terminal(state["status"].as_str().unwrap()) {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("job did not finish: {}", self.status(job).await);
    }
    async fn ready(&self, name: &str) {
        for _ in 0..200 {
            if self.dir.path().join(name).is_file() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("workload did not publish {name}");
    }
    fn release(&self) {
        std::fs::write(self.dir.path().join("release"), b"release").unwrap();
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        for job in self.jobs.lock().unwrap().iter() {
            let _ = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
                .arg("--state-dir")
                .arg(&self.config.root)
                .args(["stop", job])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        std::thread::sleep(Duration::from_millis(700));
        let _ = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&self.config.root)
            .args(["daemon", "stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Some(child) = self.daemon.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[tokio::test]
async fn native_argv_exit_output_and_idempotency() {
    let r = Rig::new().await;
    let mut spec = r.spec(&["echo", "space argument", "中文", "a\"b&c%PATH%"]);
    spec.idempotency_key = Some("native-once".into());
    let job = r.start(spec.clone()).await;
    assert_eq!(r.start(spec.clone()).await["job_id"], job["job_id"]);
    if let CommandSpec::Exec { args, .. } = &mut spec.command {
        args.push("changed".into());
    }
    assert!(client::start(&r.config, spec, false).await.is_err());
    let state = r.terminal(&job).await;
    assert_eq!(state["status"], "completed", "{state}");
    assert_eq!(state["exit_code"], 0);
    let logs = ipc::call(
        &r.config,
        Request::Logs {
            job_id: job["job_id"].as_str().unwrap().into(),
            stream: "stdout".into(),
            cursor: 0,
            limit: 8192,
        },
    )
    .await
    .unwrap();
    assert_eq!(logs["text"], "space argument\n中文\na\"b&c%PATH%\n");
    let failed = r.start(r.spec(&["exit", "7"])).await;
    let failed = r.terminal(&failed).await;
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["exit_code"], 7);
    let events = ipc::call(
        &r.config,
        Request::Events {
            job_id: job["job_id"].as_str().unwrap().into(),
            after: 0,
            limit: 100,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "completed")
            .count(),
        1
    );
}
#[tokio::test]
async fn daemon_stop_and_crash_do_not_own_workload() {
    let mut r = Rig::new().await;
    let path = r.dir.path().to_str().unwrap();
    let job = r.start(r.spec(&["gate", path])).await;
    r.ready("ready").await;
    let pid: u32 = std::fs::read_to_string(r.dir.path().join("ready"))
        .unwrap()
        .parse()
        .unwrap();
    let identity = platform::identity(pid).unwrap();
    client::stop_daemon(&r.config).await.unwrap();
    r.daemon.take().unwrap().wait().unwrap();
    assert_eq!(platform::identity(pid).as_ref(), Some(&identity));
    r.boot().await;
    r.daemon.as_mut().unwrap().kill().unwrap();
    r.daemon.take().unwrap().wait().unwrap();
    assert_eq!(platform::identity(pid).as_ref(), Some(&identity));
    r.release();
    r.boot().await;
    let state = r.terminal(&job).await;
    assert_eq!(state["status"], "completed", "{state}");
    assert_eq!(state["run_id"], job["run_id"]);
}
#[tokio::test]
async fn cancellation_and_timeout_terminate_descendants_only() {
    let r = Rig::new().await;
    let path = r.dir.path().to_str().unwrap();
    let job = r.start(r.spec(&["tree", path])).await;
    r.ready("ready").await;
    r.ready("branch.pid").await;
    r.ready("leaf.pid").await;
    let pids: Vec<_> = ["ready", "branch.pid", "leaf.pid"]
        .iter()
        .map(|n| {
            std::fs::read_to_string(r.dir.path().join(n))
                .unwrap()
                .parse::<u32>()
                .unwrap()
        })
        .collect();
    let own = platform::identity(std::process::id()).unwrap();
    ipc::call(
        &r.config,
        Request::Stop {
            job_id: job["job_id"].as_str().unwrap().into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(r.terminal(&job).await["status"], "cancelled");
    assert!(pids.iter().all(|p| platform::identity(*p).is_none()));
    assert_eq!(platform::identity(std::process::id()).as_ref(), Some(&own));
    let mut spec = r.spec(&["gate", path]);
    spec.timeout_seconds = Some(1);
    let timed = r.start(spec).await;
    assert_eq!(r.terminal(&timed).await["status"], "timeout");
}
#[tokio::test]
async fn runner_death_kills_job_tree_and_recovery_never_replays() {
    let r = Rig::new().await;
    let path = r.dir.path().to_str().unwrap();
    let job = r.start(r.spec(&["tree", path])).await;
    r.ready("leaf.pid").await;
    let state = r.status(&job).await;
    let pid = state["runner_pid"].as_u64().unwrap() as u32;
    let identity = state["runner_identity"].as_str().unwrap();
    assert!(platform::terminate_instance(pid, "wrong-instance").is_err());
    assert_eq!(platform::identity(pid).as_deref(), Some(identity));
    platform::terminate_instance(pid, identity).unwrap();
    assert_eq!(r.terminal(&job).await["status"], "orphaned");
    for name in ["ready", "branch.pid", "leaf.pid"] {
        let pid = std::fs::read_to_string(r.dir.path().join(name))
            .unwrap()
            .parse::<u32>()
            .unwrap();
        for _ in 0..100 {
            if platform::identity(pid).is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            platform::identity(pid).is_none(),
            "descendant survived runner death"
        );
    }
    assert_eq!(r.status(&job).await["run_id"], job["run_id"]);
}
#[tokio::test]
async fn durable_inbox_is_bound_to_original_session() {
    let r = Rig::new().await;
    let registration = |session: &str| agent_job_relay::session::Registration {
        agent_kind: "codex_cli".into(),
        session_id: session.into(),
        instance_id: id(),
        host_id: "native-test".into(),
        adapter: agent_job_relay::session::Adapter::Inbox,
        endpoint: None,
        caller_thread_id: None,
        lease_seconds: 3600,
        experimental: false,
        renew_on_delivery: false,
    };
    let a = client::call(
        &r.config,
        Request::RegisterSession {
            registration: registration("A"),
        },
    )
    .await
    .unwrap();
    let b = client::call(
        &r.config,
        Request::RegisterSession {
            registration: registration("B"),
        },
    )
    .await
    .unwrap();
    let mut spec = r.spec(&["echo", "inbox"]);
    spec.session = Some(a["session"]["key"].as_str().unwrap().into());
    let job = r.start(spec).await;
    r.terminal(&job).await;
    let key = a["session"]["key"].as_str().unwrap();
    for _ in 0..200 {
        let inbox = ipc::call(
            &r.config,
            Request::Inbox {
                session_key: key.into(),
                after: 0,
                limit: 20,
            },
        )
        .await
        .unwrap();
        if inbox["messages"]
            .as_array()
            .is_some_and(|m| m.iter().any(|x| x["status"] == "notification_recorded"))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let records = std::fs::read_dir(r.config.root.join("inbox").join(key))
        .unwrap()
        .count();
    assert_eq!(records, 1);
    let other = ipc::call(
        &r.config,
        Request::Inbox {
            session_key: b["session"]["key"].as_str().unwrap().into(),
            after: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert!(other["messages"].as_array().unwrap().is_empty());
}
#[tokio::test]
async fn pipe_rejects_invalid_frames_and_duplicate_server_without_state_damage() {
    let r = Rig::new().await;
    assert!(platform::Listener::bind(&r.config.socket()).is_err());
    let mut pipe = platform::connect_host(&r.config.socket()).await.unwrap();
    pipe.write_u32_le(131073).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(6), ipc::read_frame(&mut pipe, 1048576))
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(
        ipc::call(&r.config, Request::Doctor).await.unwrap()["store"],
        "ok"
    );
    assert!(platform::validate_endpoint(Path::new(r"\\remote\pipe\bad")).is_err());
}
#[test]
fn private_state_refuses_broad_dacl_and_reparse_paths() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let state = dir.path().join("private");
    platform::secure_dir(&state).unwrap();
    let result = Command::new("icacls.exe")
        .arg(&state)
        .args(["/grant", "*S-1-1-0:(OI)(CI)F"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(platform::secure_dir(&state).is_err());
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("preserve"), b"original").unwrap();
    let junction = dir.path().join("junction");
    let result = Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(platform::secure_dir(&junction.join("state")).is_err());
    std::fs::remove_dir(junction).unwrap();
    assert_eq!(std::fs::read(target.join("preserve")).unwrap(), b"original");
}
#[tokio::test]
async fn logs_are_capped_and_cursors_preserve_raw_bytes() {
    let r = Rig::new().await;
    let job = r.start(r.spec(&["bytes"])).await;
    let state = r.terminal(&job).await;
    assert_eq!(state["status"], "completed", "{state}");
    assert!(
        state["detail"]
            .as_str()
            .unwrap()
            .contains("stdout_truncated=true")
    );
    let key = job["job_id"].as_str().unwrap();
    let out = ipc::call(
        &r.config,
        Request::Logs {
            job_id: key.into(),
            stream: "stdout".into(),
            cursor: 16 * 1024 * 1024 - 1,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(out["bytes"], 1);
    let err = ipc::call(
        &r.config,
        Request::Logs {
            job_id: key.into(),
            stream: "stderr".into(),
            cursor: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(err["text"], "�\n");
}
#[tokio::test]
async fn explicit_powershell_and_cmd_shells_keep_working_directory() {
    let r = Rig::new().await;
    for interpreter in ["powershell.exe", "cmd.exe"] {
        let mut spec = r.spec(&[]);
        spec.command = CommandSpec::Shell {
            interpreter: interpreter.into(),
            script: if interpreter == "cmd.exe" {
                "echo shell-marker".into()
            } else {
                "Write-Output shell-marker".into()
            },
        };
        let job = r.start(spec).await;
        assert_eq!(r.terminal(&job).await["status"], "completed");
        let log = ipc::call(
            &r.config,
            Request::Logs {
                job_id: job["job_id"].as_str().unwrap().into(),
                stream: "stdout".into(),
                cursor: 0,
                limit: 100,
            },
        )
        .await
        .unwrap();
        assert_eq!(log["text"].as_str().unwrap().trim(), "shell-marker");
    }
    let cwd = r.start(r.spec(&["cwd"])).await;
    r.terminal(&cwd).await;
    let log = ipc::call(
        &r.config,
        Request::Logs {
            job_id: cwd["job_id"].as_str().unwrap().into(),
            stream: "stdout".into(),
            cursor: 0,
            limit: 8192,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        Path::new(log["text"].as_str().unwrap().trim()),
        r.dir.path()
    );
}
#[tokio::test]
async fn mcp_exit_leaves_native_job_running() {
    let r = Rig::new().await;
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
    let mut output = BufReader::new(child.stdout.take().unwrap());
    async fn rpc(
        input: &mut tokio::process::ChildStdin,
        output: &mut BufReader<tokio::process::ChildStdout>,
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
        loop {
            let mut line = String::new();
            output.read_line(&mut line).await.unwrap();
            let value: Value = serde_json::from_str(&line).unwrap();
            if value["id"] == id {
                assert!(value.get("error").is_none(), "{value}");
                return value["result"].clone();
            }
        }
    }
    rpc(&mut input, &mut output, 1, "initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"native-mcp-check","version":"1"}})).await;
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    let result = rpc(&mut input, &mut output, 2, "tools/call", json!({"name":"job_start","arguments":{"spec":r.spec(&["gate", r.dir.path().to_str().unwrap()])}})).await;
    assert_ne!(result["isError"], true, "{result}");
    let value: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    let job = value["job"].clone();
    r.jobs
        .lock()
        .unwrap()
        .push(job["job_id"].as_str().unwrap().into());
    r.ready("ready").await;
    drop(input);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    assert_eq!(r.status(&job).await["status"], "running");
    r.release();
    assert_eq!(r.terminal(&job).await["status"], "completed");
}

#[tokio::test]
async fn named_pipe_desktop_routing_recovery_and_renewal_are_fenced() {
    use agent_job_relay::{
        adapters,
        session::{Adapter, Registration},
    };
    struct Host {
        endpoint: PathBuf,
        calls: std::sync::Arc<Mutex<Vec<Value>>>,
        task: tokio::task::JoinHandle<()>,
    }
    impl Host {
        fn new() -> Self {
            let endpoint = PathBuf::from(format!(r"\\.\pipe\AJR-fake-host-{}", id()));
            let mut listener = platform::Listener::bind(&endpoint).unwrap();
            let calls = std::sync::Arc::new(Mutex::new(Vec::new()));
            let seen = calls.clone();
            let task = tokio::spawn(async move {
                loop {
                    let Ok(mut pipe) = listener.accept().await else {
                        break;
                    };
                    let calls = seen.clone();
                    tokio::spawn(async move {
                        let result = tokio::time::timeout(Duration::from_secs(3), async {
                            let Ok(first) = ipc::read_frame(&mut pipe, 65536).await else { return; };
                            assert_eq!(first["method"], "tools/list");
                            ipc::write_frame(&mut pipe, &json!({"jsonrpc":"2.0","id":first["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).await.unwrap();
                            if let Ok(call) = ipc::read_frame(&mut pipe, 65536).await {
                                calls.lock().unwrap().push(call.clone());
                                ipc::write_frame(&mut pipe, &json!({"jsonrpc":"2.0","id":call["id"],"result":{"success":true}})).await.unwrap();
                            }
                        }).await;
                        let _ = result;
                    });
                }
            });
            Self {
                endpoint,
                calls,
                task,
            }
        }
    }
    impl Drop for Host {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config);
    store.init().await.unwrap();
    let old_host = Host::new();
    let registration = |host: &Host, instance: &str, chat: &str| Registration {
        agent_kind: "codex_desktop".into(),
        session_id: chat.into(),
        caller_thread_id: Some(chat.into()),
        instance_id: instance.into(),
        host_id: "fake-windows-host".into(),
        adapter: Adapter::DesktopExperimental,
        endpoint: Some(host.endpoint.clone()),
        lease_seconds: 86400,
        experimental: true,
        renew_on_delivery: true,
    };
    let old = store
        .register(registration(&old_host, "old", "chat-a"))
        .await
        .unwrap();
    let unrelated = store
        .register(registration(&old_host, "old", "chat-b"))
        .await
        .unwrap();
    let spec = StartSpec {
        command: CommandSpec::Exec {
            program: fixture().to_str().unwrap().into(),
            args: vec!["echo".into()],
        },
        cwd: dir.path().into(),
        env: Default::default(),
        timeout_seconds: None,
        session: Some(old.key.clone()),
        idempotency_key: None,
    };
    let job = store.start(spec).await.unwrap();
    store
        .finish(
            job.job_id.clone(),
            "completed",
            Some(0),
            None,
            "simulated completion".into(),
        )
        .await
        .unwrap();
    let stale = store.claim().await.unwrap().unwrap();
    let delivery = stale.delivery.id.clone();
    let expiry = stale.delivery.expires_at;
    drop(old_host);
    let host = Host::new();
    let current = store
        .register(registration(&host, "new", "chat-a"))
        .await
        .unwrap();
    adapters::probe(&current).await.unwrap();
    let d = delivery.clone();
    store
        .call(move |db| {
            db.execute("UPDATE deliveries SET lease_until=0 WHERE id=?", [d])?;
            Ok(())
        })
        .await
        .unwrap();
    let recovered = store
        .recover_session(current.clone(), true, 0, 100)
        .await
        .unwrap();
    assert_eq!(recovered["targets"].as_array().unwrap().len(), 1);
    store
        .complete_claim(stale, Ok("accepted_by_host"))
        .await
        .unwrap();
    let key = current.key.clone();
    store
        .call(move |db| {
            db.execute("UPDATE sessions SET expires=0 WHERE key=?", [key])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut claim = store.claim().await.unwrap().unwrap();
    assert_eq!(claim.delivery.id, delivery);
    assert_eq!(claim.delivery.expires_at, expiry);
    assert_eq!(claim.session.key, current.key);
    store.renew_delivery_session(&mut claim).await.unwrap();
    assert!(claim.session.expires_at > now());
    let result = adapters::deliver(&store.config, &claim).await.unwrap();
    assert_eq!(result, "accepted_by_host");
    store.complete_claim(claim, Ok(result)).await.unwrap();
    {
        let calls = host.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["params"]["arguments"]["threadId"], "chat-a");
        assert_eq!(calls[0]["params"]["callId"], delivery);
    }
    assert!(
        store.inbox(unrelated.key, 0, 20).await.unwrap()["messages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn offline_install_update_cache_and_removal_keep_state() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let install = dir.path().join("plugin");
    let state = dir.path().join("state");
    let bundle = dir.path().join("bundle");
    let exe = env!("CARGO_BIN_EXE_agent-job-relay");
    let run = |program: &Path, args: &[&std::ffi::OsStr]| -> Value {
        let out = Command::new(program).args(args).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let a = |s: &str| std::ffi::OsString::from(s);
    run(
        Path::new(exe),
        &[
            &a("--state-dir"),
            state.as_os_str(),
            &a("install"),
            &a("--root"),
            install.as_os_str(),
            &a("--apply"),
        ],
    );
    let installed = install.join("bin/agent-job-relay.exe");
    let config = Config::new(Some(state.clone())).unwrap();
    config.prepare().unwrap();
    let store = Store::new(config);
    store.init().await.unwrap();
    let saved = store
        .start(StartSpec {
            command: CommandSpec::Exec {
                program: fixture().to_str().unwrap().into(),
                args: vec![],
            },
            cwd: dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
        })
        .await
        .unwrap();
    store
        .finish(
            saved.job_id.clone(),
            "completed",
            Some(0),
            None,
            "retained fixture".into(),
        )
        .await
        .unwrap();
    let output = Command::new("cmd.exe")
        .args(["/D", "/C"])
        .arg(install.join("relay.cmd"))
        .arg("doctor")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(Path::new(doc["state_dir"].as_str().unwrap()), state);
    run(
        Path::new(exe),
        &[
            &a("plugin"),
            &a("pack"),
            &a("--output"),
            bundle.as_os_str(),
            &a("--apply"),
        ],
    );
    let fresh = dir.path().join("fresh");
    let market = dir.path().join(".agents/plugins/marketplace.json");
    run(
        &bundle.join("payload/bin/agent-job-relay.exe"),
        &[
            &a("--state-dir"),
            state.as_os_str(),
            &a("plugin"),
            &a("bootstrap"),
            &a("--root"),
            fresh.as_os_str(),
            &a("--config"),
            market.as_os_str(),
            &a("--apply"),
        ],
    );
    assert!(fresh.join("relay.cmd").is_file());
    run(
        Path::new(exe),
        &[
            &a("plugin"),
            &a("update"),
            &a("--root"),
            install.as_os_str(),
            &a("--from"),
            bundle.as_os_str(),
            &a("--apply"),
        ],
    );
    assert!(installed.is_file());
    assert_eq!(
        store.status(saved.job_id).await.unwrap().detail.as_deref(),
        Some("retained fixture")
    );
    run(
        Path::new(exe),
        &[
            &a("uninstall"),
            &a("--root"),
            install.as_os_str(),
            &a("--apply"),
        ],
    );
    assert!(!install.exists());
    assert!(state.join("relay.sqlite3").is_file());
}

#[tokio::test]
async fn cold_submission_does_not_inherit_callers_capture_pipes() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    let mut r = Rig {
        dir,
        config,
        daemon: None,
        jobs: Mutex::new(Vec::new()),
    };
    let quote = |p: &Path| p.to_str().unwrap().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue'
$info=[Diagnostics.ProcessStartInfo]::new()
$info.FileName='{}'
$info.Arguments='--state-dir "{}" start --cwd "{}" -- "{}" gate "{}"'
$info.UseShellExecute=$false;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
$p=[Diagnostics.Process]::Start($info)
$out=$p.StandardOutput.ReadToEndAsync();$err=$p.StandardError.ReadToEndAsync()
$ended=$p.WaitForExit(10000)
if(!$ended){{$p.Kill();$p.WaitForExit()}}
$captured=$out.Wait(1000) -and $err.Wait(1000)
[IO.File]::WriteAllText('{}',(@{{ended=$ended;captured=$captured;exit=$p.ExitCode;stdout=if($captured){{$out.Result}}else{{''}};stderr=if($captured){{$err.Result}}else{{''}}}} | ConvertTo-Json -Compress))
"#,
        quote(Path::new(env!("CARGO_BIN_EXE_agent-job-relay"))),
        quote(&r.config.root),
        quote(r.dir.path()),
        quote(fixture()),
        quote(r.dir.path()),
        quote(&r.dir.path().join("client-result.json"))
    );
    let script_path = r.dir.path().join("cold-submit.ps1");
    std::fs::write(&script_path, script).unwrap();
    launch_fixture_script(&script_path);
    r.ready("client-result.json").await;
    let report: Value =
        serde_json::from_slice(&std::fs::read(r.dir.path().join("client-result.json")).unwrap())
            .unwrap();
    let peer = platform::connect_host(&r.config.socket()).await.unwrap();
    let pid = platform::peer_pid(&peer).unwrap().unwrap();
    r.daemon = Some(DaemonProcess {
        pid,
        identity: platform::identity(pid).unwrap(),
    });
    assert_eq!(report["ended"], true, "{report}");
    assert_eq!(
        report["captured"], true,
        "background process retained a host capture handle: {report}"
    );
    assert_eq!(report["exit"], 0, "{report}");
    let value: Value = serde_json::from_str(report["stdout"].as_str().unwrap()).unwrap();
    let job = value["job"].clone();
    r.jobs
        .lock()
        .unwrap()
        .push(job["job_id"].as_str().unwrap().into());
    r.ready("ready").await;
    assert_eq!(r.status(&job).await["status"], "running");
    r.release();
    assert_eq!(r.terminal(&job).await["status"], "completed");
}

#[tokio::test]
async fn host_pipe_security_pins_server_and_applies_to_later_instances() {
    use tokio::net::windows::named_pipe::ServerOptions;
    let rejected = PathBuf::from(format!(r"\\.\pipe\AJR-node-refuse-{}", id()));
    let first = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&rejected)
        .unwrap();
    assert!(platform::secure_bridge_pipe(&rejected, std::process::id().wrapping_add(1)).is_err());
    drop(first);
    let endpoint = PathBuf::from(format!(r"\\.\pipe\AJR-node-private-{}", id()));
    let first = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&endpoint)
        .unwrap();
    let second = ServerOptions::new().create(&endpoint).unwrap();
    let result = platform::secure_bridge_pipe(&endpoint, std::process::id()).unwrap();
    assert_eq!(result["private_acl"], true);
    drop(first);
    let third = ServerOptions::new().create(&endpoint).unwrap();
    for _ in 0..2 {
        let client = platform::connect_host(&endpoint).await.unwrap();
        platform::verify_bridge_endpoint(&client).unwrap();
        drop(client);
    }
    drop((second, third));
}

#[tokio::test]
async fn outside_host_daemon_start_allows_restricted_clients_without_job_submission() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    let mut r = Rig {
        dir,
        config,
        daemon: None,
        jobs: Mutex::new(Vec::new()),
    };
    let quote = |p: &Path| p.to_str().unwrap().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue'
$result=& '{}' --state-dir '{}' daemon start
if($LASTEXITCODE -ne 0){{throw 'explicit daemon start failed'}}
[IO.File]::WriteAllText('{}',($result -join "`n"))
"#,
        quote(Path::new(env!("CARGO_BIN_EXE_agent-job-relay"))),
        quote(&r.config.root),
        quote(&r.dir.path().join("start.json"))
    );
    let script_path = r.dir.path().join("daemon-start.ps1");
    std::fs::write(&script_path, script).unwrap();
    launch_fixture_script(&script_path);
    r.ready("start.json").await;
    let peer = platform::connect_host(&r.config.socket()).await.unwrap();
    let pid = platform::peer_pid(&peer).unwrap().unwrap();
    r.daemon = Some(DaemonProcess {
        pid,
        identity: platform::identity(pid).unwrap(),
    });
    // These CLI clients inherit Cargo's cleanup job but reuse the outside daemon.
    for _ in 0..2 {
        let out = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&r.config.root)
            .args(["daemon", "start"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let store = Store::new(r.config.clone());
    assert!(store.list(0, 20).await.unwrap().is_empty());
    let job = r
        .start(r.spec(&["gate", r.dir.path().to_str().unwrap()]))
        .await;
    r.ready("ready").await;
    r.release();
    assert_eq!(r.terminal(&job).await["status"], "completed");
}

#[tokio::test]
async fn restricted_cold_start_preserves_environment_without_prestarting_daemon() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state with ' & ü"))).unwrap();
    let mut r = Rig {
        dir,
        config,
        daemon: None,
        jobs: Mutex::new(Vec::new()),
    };
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"));
    command
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["start", "--cwd"])
        .arg(r.dir.path())
        .arg("--")
        .arg(fixture())
        .args([
            "env",
            "AJR_AUTO_TEST_ENV",
            "CODEX_THREAD_ID",
            "CODEX_APP_TOOLS_PIPE_PATH",
        ])
        .env("AJR_AUTO_TEST_ENV", "继承 ü ' \" $ ;")
        .env("CODEX_THREAD_ID", "must-not-inherit")
        .env("CODEX_APP_TOOLS_PIPE_PATH", "must-not-inherit")
        .env("AJR_WINDOWS_STARTUP", "auto")
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let out = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .unwrap()
        .unwrap();
    let peer = platform::connect_host(&r.config.socket()).await;
    if let Ok(peer) = peer {
        let pid = platform::peer_pid(&peer).unwrap().unwrap();
        r.daemon = Some(DaemonProcess {
            pid,
            identity: platform::identity(pid).unwrap(),
        });
    }
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let job: Value = serde_json::from_slice(&out.stdout).unwrap();
    let job = job["job"].clone();
    r.jobs
        .lock()
        .unwrap()
        .push(job["job_id"].as_str().unwrap().into());
    assert_eq!(r.terminal(&job).await["status"], "completed");
    let doctor = ipc::call(&r.config, Request::Doctor).await.unwrap();
    assert_eq!(doctor["daemon_launch_method"], "windows_cim");
    let stored = Store::new(r.config.clone())
        .status(job["job_id"].as_str().unwrap().into())
        .await
        .unwrap();
    let logs =
        std::fs::read_to_string(r.config.run_dir(&stored.run_id).join("stdout.log")).unwrap();
    assert!(logs.contains("AJR_AUTO_TEST_ENV=继承 ü ' \" $ ;"));
    assert!(
        logs.contains("CODEX_THREAD_ID=\n") && logs.contains("CODEX_APP_TOOLS_PIPE_PATH=\n"),
        "{logs}"
    );
}

#[tokio::test]
async fn direct_only_policy_refuses_restricted_cold_launch_without_a_job() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let state = dir.path().join("state");
    let out = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&state)
        .args(["daemon", "start"])
        .env("AJR_WINDOWS_STARTUP", "direct")
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("mode=direct"));
    assert!(!state.join("relay.sqlite3").exists());
}

#[tokio::test]
async fn automatic_start_survives_host_job_destruction_and_recovers_after_daemon_crash() {
    let dir = tempfile::tempdir_in(common::root()).unwrap();
    let config = Config::new(Some(dir.path().join("state"))).unwrap();
    let mut r = Rig {
        dir,
        config,
        daemon: None,
        jobs: Mutex::new(Vec::new()),
    };
    let mut host = tokio::process::Command::new(fixture())
        .arg("host-start")
        .arg(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg(&r.config.root)
        .arg(r.dir.path())
        .env("AJR_WINDOWS_STARTUP", "auto")
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        while !r.dir.path().join("host.stdout.json").exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let peer = platform::connect_host(&r.config.socket()).await.unwrap();
    let pid = platform::peer_pid(&peer).unwrap().unwrap();
    r.daemon = Some(DaemonProcess {
        pid,
        identity: platform::identity(pid).unwrap(),
    });
    drop(peer);
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("host.exit")).unwrap(),
        "0",
        "{}",
        std::fs::read_to_string(r.dir.path().join("host.stderr")).unwrap()
    );
    let value: Value =
        serde_json::from_slice(&std::fs::read(r.dir.path().join("host.stdout.json")).unwrap())
            .unwrap();
    let job = value["job"].clone();
    r.jobs
        .lock()
        .unwrap()
        .push(job["job_id"].as_str().unwrap().into());
    r.ready("ready").await;
    std::fs::write(r.dir.path().join("close-host"), b"close").unwrap();
    tokio::time::timeout(Duration::from_secs(5), host.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.status(&job).await["status"], "running");
    let original = r.daemon.take().unwrap();
    original.kill().unwrap();
    original.wait().unwrap();
    let out = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
        .arg("--state-dir")
        .arg(&r.config.root)
        .args(["start", "--cwd"])
        .arg(r.dir.path())
        .arg("--")
        .arg(fixture())
        .args(["echo", "after-crash"])
        .env("AJR_WINDOWS_STARTUP", "auto")
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let second = value["job"].clone();
    r.jobs
        .lock()
        .unwrap()
        .push(second["job_id"].as_str().unwrap().into());
    let peer = platform::connect_host(&r.config.socket()).await.unwrap();
    let pid = platform::peer_pid(&peer).unwrap().unwrap();
    r.daemon = Some(DaemonProcess {
        pid,
        identity: platform::identity(pid).unwrap(),
    });
    drop(peer);
    assert_eq!(r.terminal(&second).await["status"], "completed");
    assert_eq!(r.status(&job).await["status"], "running");
    r.release();
    assert_eq!(r.terminal(&job).await["status"], "completed");
    let jobs = Store::new(r.config.clone()).list(0, 20).await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|j| j.run.attempt == 1));
}

#[tokio::test]
async fn bootstrap_refuses_changed_parent_or_another_executable_before_transfer() {
    for expected in [
        "windows:stale-instance".to_string(),
        platform::identity(std::process::id()).unwrap(),
    ] {
        let dir = tempfile::tempdir_in(common::root()).unwrap();
        let state = dir.path().join("state");
        let endpoint = PathBuf::from(format!(r"\\.\pipe\AgentJobRelay-startup-{}", id()));
        let mut listener = platform::Listener::bind(&endpoint).unwrap();
        let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&state)
            .arg("windows-bootstrap")
            .arg(&endpoint)
            .arg(expected)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let _socket = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let out = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        assert!(!out.status.success());
        let text = String::from_utf8_lossy(&out.stderr);
        assert!(
            text.contains("instance differs") || text.contains("executable differs"),
            "{text}"
        );
        assert!(!state.join("relay.sqlite3").exists());
    }
}
