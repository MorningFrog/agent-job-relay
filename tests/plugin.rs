#![cfg(target_os = "linux")]
mod common;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

const PROFILE: &str = ".agent-job-relay-runtime.json";
struct Rig {
    dir: tempfile::TempDir,
    state: PathBuf,
    install: PathBuf,
    bundle: PathBuf,
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("plugin-")
            .tempdir_in(common::root())
            .unwrap();
        Self {
            state: dir.path().join("custom history"),
            install: dir.path().join("home/local relay's bundle"),
            bundle: dir.path().join("offline bundle"),
            dir,
        }
    }
    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env_remove("AJR_STATE_DIR")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .env_remove("CODEX_APP_RUNTIME_CLI_PATH")
            .env("HOME", self.dir.path().join("home"))
            .env("XDG_DATA_HOME", self.dir.path().join("home/.local/share"))
            .env("XDG_STATE_HOME", self.dir.path().join("default-state"))
            .env("CODEX_HOME", self.dir.path().join("home/.codex"))
            .env("PATH", "/usr/bin:/bin") // Bootstrap must not need cargo, Python or Node.
            .current_dir(self.dir.path());
        command
    }
    fn source(&self, args: &[&str]) -> Output {
        self.command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
            .args(args)
            .output()
            .unwrap()
    }
    fn pack(&self) {
        ok(self.source(&["plugin", "pack", "--output", s(&self.bundle), "--apply"]));
    }
    fn bootstrap(&self, apply: bool) -> Value {
        ok(self
            .command(&self.bundle.join("relay"))
            .args([
                "--state-dir",
                s(&self.state),
                "plugin",
                "bootstrap",
                "--root",
                s(&self.install),
                if apply { "--apply" } else { "--dry-run" },
            ])
            .output()
            .unwrap())
    }
    fn installed(&self, args: &[&str]) -> Output {
        self.command(&self.install.join("relay"))
            .args(args)
            .output()
            .unwrap()
    }
    fn update(&self, caches: &[&Path], apply: bool) -> Output {
        let mut args = vec![
            "plugin",
            "update",
            "--root",
            s(&self.install),
            "--from",
            s(&self.bundle),
            if apply { "--apply" } else { "--dry-run" },
        ];
        for cache in caches {
            args.extend(["--cache", s(cache)]);
        }
        self.command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
            .env("AJR_STATE_DIR", self.dir.path().join("unrelated-state"))
            .args(args)
            .output()
            .unwrap()
    }
    fn refresh(&self, cache: &Path, apply: bool) -> Output {
        self.source(&[
            "plugin",
            "cache",
            "refresh",
            "--root",
            s(&self.install),
            "--path",
            s(cache),
            if apply { "--apply" } else { "--dry-run" },
        ])
    }
    fn snapshot(&self, name: &str, with_receipt: bool) -> PathBuf {
        let cache = self.dir.path().join(name);
        copy_tree(&self.install, &cache, with_receipt);
        cache
    }
    fn change_bundle_skill(&self) {
        let name = "payload/skills/background-jobs/SKILL.md";
        let mut bytes = std::fs::read(self.bundle.join(name)).unwrap();
        bytes.extend(b"\nTest fixture: revised workflow resource.\n");
        std::fs::write(self.bundle.join(name), &bytes).unwrap();
        let mut manifest = read_json(&self.bundle.join("bundle.json"));
        manifest["files"][name] = json!(hash(&bytes));
        write_json(&self.bundle.join("bundle.json"), &manifest);
    }
    fn stop(&self) {
        ok(self.installed(&["daemon", "stop"]));
    }
    fn database_hash(&self) -> String {
        // Terminal state and daemon-lock release can precede the last pooled
        // connection closing. Include committed WAL pages before hashing;
        // a later normal checkpoint must not look like lifecycle data damage.
        let path = self.state.join("relay.sqlite3");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.busy_timeout(Duration::from_secs(5)).unwrap();
        let busy: i64 = db
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
            .unwrap();
        assert_eq!(busy, 0, "test database checkpoint is blocked");
        drop(db);
        hash(&std::fs::read(path).unwrap())
    }
    fn wait(&self, job: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if ok(self.installed(&["status", job]))["status"] == "completed" {
                return;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        let _ = self
            .command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
            .args(["--state-dir", s(&self.state), "daemon", "stop"])
            .output();
    }
}
fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn ok(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn fails(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn copy_tree(from: &Path, to: &Path, with_receipt: bool) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        if !with_receipt && path.file_name().unwrap() == ".agent-job-relay-install.json" {
            continue;
        }
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target, with_receipt);
        } else {
            std::fs::copy(path, target).unwrap();
        }
    }
}

#[tokio::test]
async fn retained_mcp_and_cached_launcher_follow_verified_same_version_runtime_updates() {
    use std::{os::unix::fs::PermissionsExt, process::Stdio};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("unchanged host cache", true);
    let old_workflow = ok(r
        .command(&cache.join("relay"))
        .arg("instructions")
        .output()
        .unwrap());
    assert_eq!(
        old_workflow["goal_wait_policy"],
        "required_for_active_desktop_goal"
    );
    let cached_skill = std::fs::read(cache.join("skills/background-jobs/SKILL.md")).unwrap();
    let mut command =
        tokio::process::Command::from(r.command(&r.install.join("bin/agent-job-relay")));
    let mut child = command
        .args(["--state-dir", s(&r.state), "mcp", "--shared-host"])
        .env("CODEX_THREAD_ID", "stale-shared-caller")
        .env("CODEX_APP_TOOLS_PIPE_PATH", "must-not-be-used")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut notices = 0;
    async fn rpc(
        input: &mut tokio::process::ChildStdin,
        output: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        notices: &mut usize,
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
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let value: Value = serde_json::from_str(
                    &output.next_line().await.unwrap().expect("MCP disconnected"),
                )
                .unwrap();
                if value["method"] == "notifications/tools/list_changed" {
                    *notices += 1;
                }
                if value["id"] == id {
                    return value;
                }
            }
        })
        .await
        .unwrap()
    }
    rpc(&mut input,&mut output,&mut notices,1,"initialize",json!({"protocolVersion":"2025-06-18","capabilities":{"elicitation":{"form":{}}},"clientInfo":{"name":"retained-client","version":"1"}})).await;
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    let first = rpc(
        &mut input,
        &mut output,
        &mut notices,
        2,
        "tools/list",
        json!({}),
    )
    .await;
    assert_eq!(first["result"]["tools"].as_array().unwrap().len(), 25);
    let native = format!("payload/{}", agent_job_relay::platform::NATIVE_FILE);
    // ELF permits trailing bytes: this is a real native runtime with a distinct
    // image identity and the same version, exercising the actual MCP worker.
    let mut new_image = std::fs::read(r.bundle.join(&native)).unwrap();
    new_image.extend(b"\nAJR native same-version fixture\n");
    std::fs::write(r.bundle.join(&native), &new_image).unwrap();
    let mut manifest = read_json(&r.bundle.join("bundle.json"));
    manifest["files"][&native] = json!(hash(&new_image));
    write_json(&r.bundle.join("bundle.json"), &manifest);
    let first_update = ok(r.update(&[], true));
    let retained_image = PathBuf::from(first_update["install"]["backup"].as_str().unwrap())
        .join(agent_job_relay::platform::NATIVE_FILE);
    let current = rpc(
        &mut input,
        &mut output,
        &mut notices,
        10,
        "tools/call",
        json!({"name":"job_doctor","arguments":{}}),
    )
    .await;
    let current: Value =
        serde_json::from_str(current["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(current["managed_mcp_updates_supported"], true);
    assert_eq!(
        current["runtime_workflow_sha256"],
        old_workflow["workflow_sha256"]
    );
    assert!(
        !r.state.exists(),
        "read-only forwarding must not create state"
    );
    let started=rpc(&mut input,&mut output,&mut notices,11,"tools/call",json!({"name":"job_start","arguments":{"spec":{"command":{"kind":"shell","interpreter":"/bin/sh","script":"echo once >> execution-count"},"cwd":r.dir.path(),"idempotency_key":"retained-native-call"}}})).await;
    let started: Value =
        serde_json::from_str(started["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    r.wait(started["job"]["job_id"].as_str().unwrap());
    r.stop();
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("execution-count")).unwrap(),
        "once\n"
    );
    let history = r.database_hash();
    let replacement = include_bytes!("fixtures/current-mcp.py");
    std::fs::write(r.bundle.join(&native), replacement).unwrap();
    std::fs::set_permissions(
        r.bundle.join(&native),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let mut manifest = read_json(&r.bundle.join("bundle.json"));
    manifest["files"][&native] = json!(hash(replacement));
    write_json(&r.bundle.join("bundle.json"), &manifest);
    assert_eq!(ok(r.update(&[], true))["install"]["changed"], true);
    assert_eq!(
        std::fs::read(cache.join("skills/background-jobs/SKILL.md")).unwrap(),
        cached_skill
    );
    assert_eq!(
        ok(r.command(&cache.join("relay"))
            .arg("instructions")
            .output()
            .unwrap())["workflow"],
        "current-runtime-workflow"
    );
    let updated = rpc(
        &mut input,
        &mut output,
        &mut notices,
        3,
        "tools/list",
        json!({}),
    )
    .await;
    assert_eq!(
        updated["result"]["tools"][0]["name"], "job_current_fixture",
        "{updated}"
    );
    let arguments = json!({"binding":{"session_key":"actual-caller-key","cwd":r.dir.path()},"idempotency_key":"unchanged"});
    let called = rpc(
        &mut input,
        &mut output,
        &mut notices,
        4,
        "tools/call",
        json!({"name":"job_current_fixture","arguments":arguments}),
    )
    .await;
    let value: Value =
        serde_json::from_str(called["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(value["runtime"], "current");
    assert_eq!(value["state"], s(&r.state));
    assert_eq!(value["arguments"], arguments);
    assert_eq!(
        notices, 2,
        "one catalog notification per runtime revision avoids reload loops"
    );
    // Human input may take longer than the active 90-second RPC budget.
    input.write_all(format!("{}\n",json!({"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"job_goal_consent","arguments":{"question":true}}})).as_bytes()).await.unwrap();
    input.flush().await.unwrap();
    let question: Value = serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(8), output.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(question["method"], "elicitation/create", "{question}");
    tokio::time::sleep(Duration::from_secs(91)).await;
    input.write_all(format!("{}\n",json!({"jsonrpc":"2.0","id":question["id"],"result":{"action":"accept","content":{"allow":true}}})).as_bytes()).await.unwrap();
    input.flush().await.unwrap();
    let answered: Value = serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(8), output.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(answered["id"], 40, "{answered}");
    let answered: Value =
        serde_json::from_str(answered["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(answered["answer"]["content"]["allow"], true);
    assert_eq!(answered["client_protocol"], "2025-06-18");
    assert_eq!(answered["state"], s(&r.state));
    assert_eq!(
        r.database_hash(),
        history,
        "confirmation transport must not mutate job history"
    );
    // New-protocol input requests must survive stateless worker replacement.
    let mut modern = tokio::process::Command::from(r.command(&retained_image))
        .args(["--state-dir", s(&r.state), "mcp", "--shared-host"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut modern_in = modern.stdin.take().unwrap();
    let mut modern_out = BufReader::new(modern.stdout.take().unwrap()).lines();
    let mut modern_notices = 0;
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"retained-modern-fixture","version":"1"},"io.modelcontextprotocol/clientCapabilities":{"elicitation":{"form":{}}}});
    let scope_args = json!({"mrtr":true,"binding":{"session_key":"original-caller","cwd":r.dir.path()},"idempotency_key":"same-scope"});
    let first = rpc(
        &mut modern_in,
        &mut modern_out,
        &mut modern_notices,
        50,
        "tools/call",
        json!({"_meta":meta,"name":"job_goal_consent","arguments":scope_args}),
    )
    .await;
    assert_eq!(first["result"]["resultType"], "input_required", "{first}");
    let accepted=rpc(&mut modern_in,&mut modern_out,&mut modern_notices,51,"tools/call",json!({"_meta":meta,"name":"job_goal_consent","arguments":scope_args,"requestState":first["result"]["requestState"],"inputResponses":{"scope":{"action":"accept","content":{"allow":true}}}})).await;
    let accepted: Value =
        serde_json::from_str(accepted["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(accepted["arguments"], scope_args);
    assert_eq!(
        accepted["inputResponses"]["scope"]["content"]["allow"],
        true
    );
    modern_in.shutdown().await.unwrap();
    drop(modern_in);
    assert!(modern.wait().await.unwrap().success());
    assert_eq!(r.database_hash(), history);
    let entered = r.dir.path().join("entered-forwarded-call");
    let release = r.dir.path().join("release-forwarded-call");
    r.change_bundle_skill();
    let held = rpc(
        &mut input,
        &mut output,
        &mut notices,
        5,
        "tools/call",
        json!({"name":"job_current_fixture","arguments":{"hold":true,"entered":entered,"release":release}}),
    );
    let update = async {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !entered.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        fails(
            r.update(&[], true),
            "another installation/removal is in progress",
        );
        std::fs::write(release, "release").unwrap();
    };
    let (held, ()) = tokio::join!(held, update);
    assert_ne!(held["result"]["isError"], true, "{held}");
    let record = r.dir.path().join("uncertain-calls");
    let lost=rpc(&mut input,&mut output,&mut notices,6,"tools/call",json!({"name":"job_current_fixture","arguments":{"drop_ack":true,"record":record,"idempotency_key":"one-attempt"}})).await;
    assert_eq!(lost["result"]["isError"], true);
    assert_eq!(
        std::fs::read_to_string(&record).unwrap(),
        "one-attempt\n",
        "uncertain calls must never be replayed by the bridge"
    );
    let saved_profile = read_json(&r.install.join(PROFILE));
    let saved_receipt = read_json(&r.install.join(".agent-job-relay-install.json"));
    for downgrade in [false, true] {
        let mut profile = saved_profile.clone();
        if downgrade {
            profile["version"] = json!("0.1.10");
        } else {
            profile["state_dir"] = json!(r.dir.path().join("must-not-select-another-profile"));
        }
        write_json(&r.install.join(PROFILE), &profile);
        let mut receipt = saved_receipt.clone();
        receipt["files"][PROFILE] = json!(hash(&std::fs::read(r.install.join(PROFILE)).unwrap()));
        write_json(&r.install.join(".agent-job-relay-install.json"), &receipt);
        let denied = rpc(
            &mut input,
            &mut output,
            &mut notices,
            if downgrade { 13 } else { 12 },
            "tools/call",
            json!({"name":"job_doctor","arguments":{}}),
        )
        .await;
        assert_eq!(denied["result"]["isError"], true, "{denied}");
    }
    write_json(&r.install.join(PROFILE), &saved_profile);
    write_json(
        &r.install.join(".agent-job-relay-install.json"),
        &saved_receipt,
    );
    std::fs::write(r.install.join("bin/agent-job-relay"), b"modified").unwrap();
    let refused = rpc(
        &mut input,
        &mut output,
        &mut notices,
        7,
        "tools/call",
        json!({"name":"job_doctor","arguments":{}}),
    )
    .await;
    assert_eq!(refused["result"]["isError"], true);
    assert_eq!(
        notices, 2,
        "rejected changes must not start a catalog reload loop"
    );
    assert_eq!(
        r.database_hash(),
        history,
        "workflow/caller refresh and uncertain fixture calls must preserve native history"
    );
    drop(input);
    child.wait().await.unwrap();
}

#[test]
fn unified_install_handles_first_install_and_updates_the_recorded_profile() {
    let r = Rig::new();
    r.pack();
    let first = ok(r
        .command(&r.bundle.join("relay"))
        .args([
            "--state-dir",
            s(&r.state),
            "plugin",
            "install",
            "--root",
            s(&r.install),
        ])
        .output()
        .unwrap());
    assert_eq!(first["mode"], "dry_run");
    assert!(!r.install.exists());
    let applied = ok(r
        .command(&r.bundle.join("relay"))
        .args([
            "--state-dir",
            s(&r.state),
            "plugin",
            "install",
            "--root",
            s(&r.install),
            "--apply",
        ])
        .output()
        .unwrap());
    assert_eq!(applied["install"]["state_dir"], s(&r.state));
    assert_eq!(applied["marketplace"]["changed"], true);
    let job = ok(r.installed(&["start", "--", "/bin/echo", "kept"]));
    let job = job["job"]["job_id"].as_str().unwrap();
    r.wait(job);
    r.stop();
    r.change_bundle_skill();
    let update = ok(r
        .command(&r.install.join("relay"))
        .env("AJR_STATE_DIR", r.dir.path().join("unrelated"))
        .args(["plugin", "install", "--from", s(&r.bundle), "--apply"])
        .output()
        .unwrap());
    assert_eq!(update["install"]["changed"], true);
    assert_eq!(update["install"]["state_dir"], s(&r.state));
    assert!(Path::new(update["install"]["backup"].as_str().unwrap()).exists());
    assert!(!r.dir.path().join("unrelated").exists());
    ok(r.installed(&["daemon", "start"]));
    assert_eq!(ok(r.installed(&["status", job]))["status"], "completed");
    r.stop();
}

#[test]
fn relocated_offline_bundle_bootstraps_without_toolchain_or_author_profile() {
    let r = Rig::new();
    let output = r.dir.path().join("not-created/bundle");
    let preview = ok(r.source(&["plugin", "pack", "--output", s(&output)]));
    assert_eq!(preview["mode"], "dry_run");
    assert!(!output.parent().unwrap().exists());
    r.pack();
    for name in ["bundle.json", "relay", "payload/.mcp.json", "payload/relay"] {
        let text = std::fs::read_to_string(r.bundle.join(name)).unwrap();
        assert!(!text.contains(s(r.dir.path())));
        assert!(!text.contains(s(&r.state)));
    }
    let moved = r.dir.path().join("moved 'offline bundle");
    std::fs::rename(&r.bundle, &moved).unwrap();
    let bootstrap = |apply| {
        ok(r.command(&moved.join("relay"))
            .args([
                "--state-dir",
                s(&r.state),
                "plugin",
                "bootstrap",
                "--root",
                s(&r.install),
                if apply { "--apply" } else { "--dry-run" },
            ])
            .output()
            .unwrap())
    };
    assert_eq!(bootstrap(false)["install"]["changed"], true);
    assert!(!r.install.exists());
    assert!(!r.state.exists());
    let result = bootstrap(true);
    assert_eq!(result["marketplace"]["changed"], true);
    assert_eq!(
        read_json(&r.install.join(".codex-plugin/plugin.json"))["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(!r.state.exists());
    assert_eq!(bootstrap(true)["install"]["changed"], false);
    std::fs::remove_dir_all(&moved).unwrap();
    assert_eq!(
        ok(r.installed(&["doctor", "--check-mcp"]))["stdio_self_check"]["status"],
        "passed"
    );
}

#[test]
fn update_keeps_history_and_marketplace_and_refreshes_checked_cache_snapshots() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let job = ok(r.installed(&["start", "--", "/bin/echo", "preserved history"]));
    let job = job["job"]["job_id"].as_str().unwrap();
    r.wait(job);
    r.stop();
    let database = r.database_hash();
    let marketplace = r.dir.path().join("home/.agents/plugins/marketplace.json");
    let registration = std::fs::read(&marketplace).unwrap();
    let first = r.snapshot("cache with receipt", true);
    let second = r.snapshot("cache without receipt", false);
    r.change_bundle_skill();
    let preview = ok(r.update(&[&first, &second], false));
    assert_eq!(preview["install"]["changed"], true);
    assert_eq!(preview["caches"][0]["changed"], true);
    assert_eq!(preview["caches"][1]["changed"], true);
    let updated = ok(r.update(&[&first, &second], true));
    assert_eq!(updated["install"]["state_dir"], s(&r.state));
    assert!(Path::new(updated["install"]["backup"].as_str().unwrap()).exists());
    assert_eq!(r.database_hash(), database);
    assert_eq!(std::fs::read(marketplace).unwrap(), registration);
    assert!(!r.dir.path().join("unrelated-state").exists());
    let status = ok(r.source(&[
        "plugin",
        "status",
        "--root",
        s(&r.install),
        "--cache",
        s(&first),
        "--cache",
        s(&second),
    ]));
    assert_eq!(status["caches"][0]["matches_installed"], true);
    assert_eq!(status["caches"][1]["matches_installed"], true);
    assert_eq!(ok(r.refresh(&first, true))["changed"], false);
    assert_eq!(ok(r.update(&[], true))["install"]["changed"], false);
    assert_eq!(
        ok(r.command(&second.join("relay"))
            .args(["doctor", "--check-mcp"])
            .output()
            .unwrap())["stdio_self_check"]["status"],
        "passed"
    );
}

#[test]
fn stale_cached_native_is_not_used_by_the_cached_launcher() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("stale cache", false);
    // Simulate a valid historical cache carrying a different native image.
    let trap = b"#!/bin/sh\nexit 66\n";
    std::fs::write(cache.join("bin/agent-job-relay"), trap).unwrap();
    let mut profile = read_json(&cache.join(PROFILE));
    profile["files"]["bin/agent-job-relay"] = json!(hash(trap));
    write_json(&cache.join(PROFILE), &profile);
    let version = r
        .command(&cache.join("relay"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));
    let status = ok(r.source(&[
        "plugin",
        "status",
        "--root",
        s(&r.install),
        "--cache",
        s(&cache),
    ]));
    assert_eq!(status["caches"][0]["matches_installed"], false);
    assert_eq!(ok(r.refresh(&cache, true))["changed"], true);
    assert_eq!(ok(r.refresh(&cache, true))["changed"], false);
}

#[test]
fn corrupt_or_incompatible_bundles_do_not_change_the_installation() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let before = std::fs::read(r.install.join(PROFILE)).unwrap();
    let manifest_path = r.bundle.join("bundle.json");
    let original = read_json(&manifest_path);
    let mut incompatible = original.clone();
    incompatible["platform"] = json!("unverified-native-platform");
    write_json(&manifest_path, &incompatible);
    fails(r.update(&[], true), "does not match");
    let mut traversal = original.clone();
    traversal["files"]["../outside"] = json!("bad");
    write_json(&manifest_path, &traversal);
    fails(r.update(&[], true), "unsafe bundle file layout");
    write_json(&manifest_path, &original);
    std::fs::write(r.bundle.join("payload/relay"), "tampered").unwrap();
    fails(r.update(&[], true), "was modified");
    assert_eq!(std::fs::read(r.install.join(PROFILE)).unwrap(), before);
}

#[test]
fn cache_preflight_preserves_modified_unknown_legacy_and_foreign_content() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("changed cache", false);
    let skill = cache.join("skills/background-jobs/SKILL.md");
    let original = std::fs::read(&skill).unwrap();
    let before = std::fs::read(r.install.join(PROFILE)).unwrap();
    r.change_bundle_skill();
    std::fs::write(&skill, "my edits").unwrap();
    fails(r.update(&[&cache], true), "was modified");
    assert_eq!(std::fs::read(r.install.join(PROFILE)).unwrap(), before);
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), "my edits");
    std::fs::write(&skill, original).unwrap();
    std::fs::write(cache.join("foreign.txt"), "keep me").unwrap();
    fails(r.refresh(&cache, true), "unmanaged content");
    assert_eq!(
        std::fs::read_to_string(cache.join("foreign.txt")).unwrap(),
        "keep me"
    );
    std::fs::remove_file(cache.join("foreign.txt")).unwrap();
    let mut profile = read_json(&cache.join(PROFILE));
    profile["state_dir"] = json!(r.dir.path().join("foreign-state"));
    write_json(&cache.join(PROFILE), &profile);
    fails(r.refresh(&cache, true), "different installation");
    std::fs::remove_file(cache.join(PROFILE)).unwrap();
    fails(r.refresh(&cache, true), "legacy cache");
}

#[test]
fn unsafe_cache_paths_and_symlinks_do_not_create_lock_files_in_the_installation() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("cache", false);
    let link = r.dir.path().join("cache link");
    std::os::unix::fs::symlink(&cache, &link).unwrap();
    fails(r.refresh(&link, true), "symlink");
    fails(r.refresh(&r.install.join("skills"), true), "separate");
    assert!(!r.install.join(".skills.cache.lock").exists());
    assert_eq!(
        ok(r.installed(&["plugin", "status", "--root", s(&r.install)]))["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        ok(r.installed(&["plugin", "status"]))["root"],
        s(&r.install)
    );
    fails(r.update(&[&cache, &cache], true), "overlapping");
}

#[tokio::test]
async fn update_restarts_idle_daemon_but_active_jobs_and_cache_refresh_preserve_it() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("cache", false);
    r.change_bundle_skill();
    let started = ok(r.installed(&[
        "start",
        "--shell",
        "while [ ! -f finish-work ]; do sleep 0.05; done",
    ]));
    let before = std::fs::read(r.install.join(PROFILE)).unwrap();
    fails(r.update(&[&cache], true), "active");
    assert_eq!(std::fs::read(r.install.join(PROFILE)).unwrap(), before);
    let socket = tokio::net::UnixStream::connect(r.state.join("relay.sock"))
        .await
        .unwrap();
    let owner = socket.peer_cred().unwrap().pid().unwrap();
    drop(socket);
    ok(r.refresh(&cache, true));
    let socket = tokio::net::UnixStream::connect(r.state.join("relay.sock"))
        .await
        .unwrap();
    assert_eq!(socket.peer_cred().unwrap().pid(), Some(owner));
    drop(socket);
    std::fs::write(r.dir.path().join("finish-work"), b"").unwrap();
    r.wait(started["job"]["job_id"].as_str().unwrap());
    let preview = ok(r.update(&[&cache], false));
    assert_eq!(preview["install"]["changed"], true);
    let socket = tokio::net::UnixStream::connect(r.state.join("relay.sock"))
        .await
        .unwrap();
    assert_eq!(socket.peer_cred().unwrap().pid(), Some(owner));
    drop(socket);
    let updated = ok(r.update(&[&cache], true));
    assert_eq!(updated["install"]["changed"], true);
    assert_eq!(updated["install"]["daemon_restarted"], true);
    let socket = tokio::net::UnixStream::connect(r.state.join("relay.sock"))
        .await
        .unwrap();
    let new_owner = socket.peer_cred().unwrap().pid();
    assert_ne!(new_owner, Some(owner));
    assert_eq!(
        std::fs::read_link(format!("/proc/{}/exe", new_owner.unwrap())).unwrap(),
        r.install.join("bin/agent-job-relay")
    );
    drop(socket);
    assert_eq!(
        ok(r.installed(&["doctor"]))["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        ok(r.installed(&["status", started["job"]["job_id"].as_str().unwrap()]))["status"],
        "completed"
    );
    let unchanged = ok(r.update(&[&cache], true));
    assert_eq!(unchanged["install"]["changed"], false);
    let socket = tokio::net::UnixStream::connect(r.state.join("relay.sock"))
        .await
        .unwrap();
    assert_eq!(socket.peer_cred().unwrap().pid(), new_owner);
}

#[test]
fn bootstrap_preflights_foreign_marketplace_before_installing() {
    let r = Rig::new();
    r.pack();
    let nested_config = r.install.join(".agents/plugins/marketplace.json");
    fails(
        r.command(&r.bundle.join("relay"))
            .args([
                "plugin",
                "bootstrap",
                "--root",
                s(&r.install),
                "--config",
                s(&nested_config),
                "--apply",
            ])
            .output()
            .unwrap(),
        "outside the installation",
    );
    assert!(!r.install.exists());
    let nested_root = r.bundle.join("installed-here");
    fails(
        r.command(&r.bundle.join("relay"))
            .args(["plugin", "bootstrap", "--root", s(&nested_root), "--apply"])
            .output()
            .unwrap(),
        "separate, not nested",
    );
    assert!(!nested_root.exists());
    let marketplace = r.dir.path().join("home/.agents/plugins/marketplace.json");
    std::fs::create_dir_all(marketplace.parent().unwrap()).unwrap();
    let foreign =
        json!({"name":"other","plugins":[{"name":"agent-job-relay","source":"./foreign"}]});
    write_json(&marketplace, &foreign);
    let output = r
        .command(&r.bundle.join("relay"))
        .args(["plugin", "bootstrap", "--root", s(&r.install), "--apply"])
        .output()
        .unwrap();
    fails(output, "without our ownership receipt");
    assert!(!r.install.exists());
    assert_eq!(read_json(&marketplace), foreign);
}

#[test]
fn update_refuses_downgrade_and_bootstrap_ignores_unrelated_exported_state() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let manifest = r.bundle.join("bundle.json");
    let mut old = read_json(&manifest);
    let identity = r.bundle.join("payload/.codex-plugin/plugin.json");
    let mut plugin = read_json(&identity);
    plugin["version"] = json!("0.0.1");
    write_json(&identity, &plugin);
    old["version"] = json!("0.0.1");
    old["files"]["payload/.codex-plugin/plugin.json"] =
        json!(hash(&std::fs::read(&identity).unwrap()));
    write_json(&manifest, &old);
    fails(r.update(&[], true), "downgrade");
    plugin["version"] = json!(env!("CARGO_PKG_VERSION"));
    write_json(&identity, &plugin);
    old["version"] = json!(env!("CARGO_PKG_VERSION"));
    old["files"]["payload/.codex-plugin/plugin.json"] =
        json!(hash(&std::fs::read(&identity).unwrap()));
    write_json(&manifest, &old);
    let result = ok(r
        .command(&r.bundle.join("relay"))
        .env("AJR_STATE_DIR", r.dir.path().join("unrelated-state"))
        .args(["plugin", "bootstrap", "--root", s(&r.install), "--apply"])
        .output()
        .unwrap());
    assert_eq!(result["install"]["state_dir"], s(&r.state));
    assert_eq!(result["install"]["changed"], false);
    assert!(!r.dir.path().join("unrelated-state").exists());
}

#[test]
fn cli_repairs_a_missing_installation_after_interrupted_update_without_losing_history() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let job = ok(r.installed(&["start", "--", "/bin/echo", "history before repair"]));
    r.wait(job["job"]["job_id"].as_str().unwrap());
    r.stop();
    // Seed committed WAL history before taking the preservation snapshot,
    // without depending on when the runner/daemon closes its pool.
    let wal = rusqlite::Connection::open(r.state.join("relay.sqlite3")).unwrap();
    wal.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    wal.execute(
        "UPDATE runs SET detail='history retained in WAL' WHERE job_id=?",
        [job["job"]["job_id"].as_str().unwrap()],
    )
    .unwrap();
    assert!(
        std::fs::metadata(r.state.join("relay.sqlite3-wal"))
            .unwrap()
            .len()
            > 32
    );
    let database = r.database_hash();
    let operation = uuid::Uuid::new_v4().to_string();
    let stage = r
        .install
        .parent()
        .unwrap()
        .join(format!(".ajr-stage-{operation}"));
    let backup = r
        .install
        .parent()
        .unwrap()
        .join(format!(".ajr-backup-{operation}"));
    let receipt_name = ".agent-job-relay-install.json";
    let snapshot = |path: &Path| {
        let receipt = read_json(&path.join(receipt_name));
        let mut files = receipt["files"].as_object().unwrap().clone();
        files.insert(
            receipt_name.into(),
            json!(hash(&std::fs::read(path.join(receipt_name)).unwrap())),
        );
        files
    };
    let before = snapshot(&r.install);
    copy_tree(&r.install, &stage, true);
    let skill_name = "skills/background-jobs/SKILL.md";
    let skill = stage.join(skill_name);
    let revised = [
        std::fs::read(&skill).unwrap(),
        b"\nRevised fixture\n".to_vec(),
    ]
    .concat();
    std::fs::write(&skill, &revised).unwrap();
    let mut profile = read_json(&stage.join(PROFILE));
    profile["files"][skill_name] = json!(hash(&revised));
    write_json(&stage.join(PROFILE), &profile);
    let mut receipt = read_json(&stage.join(receipt_name));
    receipt["files"][skill_name] = json!(hash(&revised));
    receipt["files"][PROFILE] = json!(hash(&std::fs::read(stage.join(PROFILE)).unwrap()));
    write_json(&stage.join(receipt_name), &receipt);
    let journal = r.install.parent().unwrap().join(format!(
        ".{}.ajr-operation.json",
        r.install.file_name().unwrap().to_str().unwrap()
    ));
    write_json(
        &journal,
        &json!({"format":1,"id":operation,"kind":"install","root":r.install,"install_root":r.install,"state_dir":r.state,"before_state_dir":r.state,"before":before,"after":snapshot(&stage)}),
    );
    // Reproduce the persistent filesystem state at the interrupted rename gap.
    std::fs::rename(&r.install, &backup).unwrap();
    fails(r.update(&[], true), "plugin repair");
    let repair = |apply| {
        r.source(&[
            "plugin",
            "repair",
            "--path",
            s(&r.install),
            if apply { "--apply" } else { "--dry-run" },
        ])
    };
    assert_eq!(ok(repair(false))["phase"], "destination_missing");
    assert!(!r.install.exists());
    let result = ok(repair(true));
    assert_eq!(result["state_dir"], s(&r.state));
    assert_eq!(result["backup"], s(&backup));
    assert_eq!(std::fs::read(r.install.join(skill_name)).unwrap(), revised);
    assert_eq!(r.database_hash(), database);
    assert_eq!(
        wal.query_row(
            "SELECT detail FROM runs WHERE job_id=?",
            [job["job"]["job_id"].as_str().unwrap()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "history retained in WAL"
    );
    assert!(backup.join(receipt_name).exists());
    assert!(!journal.exists());
    assert_eq!(ok(repair(true))["changed"], false);
    assert_eq!(
        ok(r.installed(&["doctor", "--check-mcp"]))["stdio_self_check"]["status"],
        "passed"
    );
}

#[test]
fn installed_launcher_exposes_bundled_ui_and_agent_free_monitoring() {
    use std::io::{Read, Write};
    let r = Rig::new();
    r.pack();
    let installed = r.bootstrap(true);
    assert_eq!(
        installed["management"]["program"],
        r.install.join("relay").to_string_lossy().as_ref()
    );
    let info = ok(r.installed(&["ui", "--no-open"]));
    assert_eq!(info["profile"], r.state.to_string_lossy().as_ref());
    let host = info["url"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let mut socket = std::net::TcpStream::connect(host).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        socket,
        "GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut page = String::new();
    socket.read_to_string(&mut page).unwrap();
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains("/assets/index-"));
    let job = ok(r.installed(&[
        "start",
        "--elapsed",
        "1s",
        "--action",
        "stop",
        "--grace",
        "0s",
        "--shell",
        "sleep 10",
    ]));
    let id = job["job"]["job_id"].as_str().unwrap();
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        let state = ok(r.installed(&["inspect", id, "--json"]));
        if state["job"]["status"] == "cancelled" {
            assert!(
                state["stop"]["reason"]
                    .as_str()
                    .unwrap()
                    .contains("monitor rule")
            );
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(40));
    }
    r.stop();
    let snapshot = ok(r.installed(&["inspect", id, "--json"]));
    assert_eq!(snapshot["runtime"], "offline_snapshot");
}
