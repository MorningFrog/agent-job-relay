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
    let database = std::fs::read(r.state.join("relay.sqlite3")).unwrap();
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
    assert_eq!(
        std::fs::read(r.state.join("relay.sqlite3")).unwrap(),
        database
    );
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
async fn active_runtime_blocks_update_and_cache_refresh_leaves_the_daemon_running() {
    let r = Rig::new();
    r.pack();
    r.bootstrap(true);
    let cache = r.snapshot("cache", false);
    r.change_bundle_skill();
    let started = ok(r.installed(&["start", "--", "/bin/sleep", "0.4"]));
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
    r.wait(started["job"]["job_id"].as_str().unwrap());
    fails(r.update(&[], true), "active");
    r.stop();
    assert_eq!(ok(r.update(&[&cache], true))["install"]["changed"], true);
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
    let database = std::fs::read(r.state.join("relay.sqlite3")).unwrap();
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
    assert_eq!(
        std::fs::read(r.state.join("relay.sqlite3")).unwrap(),
        database
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
