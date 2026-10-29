#![cfg(target_os = "linux")]
mod common;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

struct Rig {
    dir: tempfile::TempDir,
    state: PathBuf,
    install: PathBuf,
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("pkg-")
            .tempdir_in(common::root())
            .unwrap();
        Self {
            state: dir.path().join("state"),
            install: dir.path().join("home/local relay's bundle"),
            dir,
        }
    }
    fn command(&self, program: &Path) -> Command {
        let mut c = Command::new(program);
        c.env_remove("AJR_STATE_DIR")
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
            .env_remove("CODEX_APP_RUNTIME_CLI_PATH")
            .env("HOME", self.dir.path().join("home"))
            .env("CODEX_HOME", self.dir.path().join("home/.codex"))
            .current_dir(self.dir.path());
        c
    }
    fn source(&self, args: &[&str]) -> Output {
        self.command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .output()
            .unwrap()
    }
    fn installed(&self, args: &[&str]) -> Output {
        self.command(&self.install.join("relay"))
            .args(args)
            .output()
            .unwrap()
    }
    fn install(&self) {
        ok(self.source(&[
            "install",
            "--root",
            self.install.to_str().unwrap(),
            "--apply",
        ]));
    }
    fn wait_done(&self, job: &str) {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if ok(self.installed(&["status", job]))["status"] == "completed" {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn stop(&self) {
        ok(self.source(&["daemon", "stop"]));
        let until = Instant::now() + Duration::from_secs(3);
        while self.state.join("relay.sock").exists() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        let _ = self.source(&["daemon", "stop"]);
    }
}
fn ok(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap() == ".agent-job-relay-install.json" {
            continue; // Runtime plugin resources must work without management receipts.
        }
        let target = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_tree(&p, &target);
        } else {
            std::fs::copy(p, target).unwrap();
        }
    }
}
#[test]
fn install_preview_stable_launcher_and_cached_plugin_keep_the_state_profile() {
    let r = Rig::new();
    let preview = ok(r.source(&["install", "--root", r.install.to_str().unwrap()]));
    assert_eq!(preview["mode"], "dry_run");
    assert!(!r.install.exists());
    assert!(!r.state.exists());
    r.install();
    let unchanged = ok(r
        .command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
        .args(["install", "--root", r.install.to_str().unwrap(), "--apply"])
        .output()
        .unwrap());
    assert_eq!(unchanged["changed"], false);
    assert_eq!(unchanged["state_dir"], r.state.to_str().unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(r.install.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["mcpServers"]["agent-job-relay"]["args"],
        json!(["--state-dir", r.state, "mcp", "--shared-host"])
    );
    assert_eq!(
        manifest["mcpServers"]["agent-job-relay"]["command"],
        r.install.join("bin/agent-job-relay").to_str().unwrap()
    );
    assert!(!r.install.join("plugin.json").exists()); // No portable-root precedence over the tested Codex manifest.
    let identity: Value = serde_json::from_slice(
        &std::fs::read(r.install.join(".codex-plugin/plugin.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(identity["mcpServers"], "./.mcp.json");
    let report = ok(r.installed(&["doctor", "--check-mcp"]));
    assert_eq!(report["daemon"], "stopped");
    assert_eq!(report["stdio_self_check"]["status"], "passed");
    assert_eq!(report["stdio_self_check"]["host_connection"], "not checked");
    assert!(!r.state.exists());
    let started = ok(r.installed(&["start", "--", "/bin/echo", "installed execution"]));
    let job = started["job"]["job_id"].as_str().unwrap();
    r.wait_done(job);
    let cache = r.dir.path().join("cached plugin");
    copy_tree(&r.install, &cache);
    let logs = ok(r
        .command(&cache.join("relay"))
        .args(["logs", job])
        .output()
        .unwrap());
    assert_eq!(logs["text"], "installed execution\n");
    assert!(!cache.join("state").exists());
    r.stop();
    let result = ok(r.source(&[
        "uninstall",
        "--root",
        r.install.to_str().unwrap(),
        "--apply",
    ]));
    assert_eq!(result["data"], "preserved");
    assert!(!r.install.exists());
    assert!(r.state.join("relay.sqlite3").exists());
    assert_eq!(
        ok(r.source(&[
            "uninstall",
            "--root",
            r.install.to_str().unwrap(),
            "--apply"
        ]))["changed"],
        false
    );
}
#[test]
fn marketplace_and_mcp_setup_are_once_only_preserve_foreign_entries_and_remove_safely() {
    let r = Rig::new();
    r.install();
    let marketplace = r.dir.path().join("home/.agents/plugins/marketplace.json");
    std::fs::create_dir_all(marketplace.parent().unwrap()).unwrap();
    let foreign = json!({"name":"personal","interface":{"displayName":"My tools"},"plugins":[{"name":"other","source":"./other","policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},"category":"Productivity"}]});
    std::fs::write(&marketplace, foreign.to_string()).unwrap();
    ok(r.installed(&["setup", "plugin", "--dry-run"]));
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&marketplace).unwrap()).unwrap(),
        foreign
    );
    let applied = ok(r.installed(&["setup", "plugin", "--apply"]));
    assert!(applied["backup"].is_string());
    let changed: Value = serde_json::from_slice(&std::fs::read(&marketplace).unwrap()).unwrap();
    assert_eq!(changed["name"], "personal");
    assert_eq!(changed["plugins"][0], foreign["plugins"][0]);
    assert_eq!(
        changed["plugins"][1]["source"]["path"],
        "./local relay's bundle"
    );
    assert_eq!(
        ok(r.installed(&["setup", "plugin", "--apply"]))["changed"],
        false
    );
    ok(r.installed(&["setup", "codex", "--apply"]));
    let toml = std::fs::read_to_string(r.dir.path().join("home/.codex/config.toml")).unwrap();
    assert!(toml.contains(r.install.join("bin/agent-job-relay").to_str().unwrap()));
    assert!(toml.contains("--shared-host"));
    let report = ok(r.installed(&["doctor", "--client", "plugin", "--check-mcp"]));
    assert_eq!(report["mcp_registration"]["matches_current_binary"], true);
    ok(r.installed(&["integration", "remove", "plugin", "--apply"]));
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&marketplace).unwrap()).unwrap(),
        foreign
    );
    assert_eq!(
        ok(r.installed(&["integration", "remove", "plugin", "--apply"]))["changed"],
        false
    );
    ok(r.installed(&["integration", "remove", "codex", "--apply"]));
    assert!(!r.state.exists());
}
#[test]
fn install_and_marketplace_refuse_user_changes_and_unmanaged_content() {
    let r = Rig::new();
    r.install();
    ok(r.installed(&["setup", "plugin", "--apply"]));
    let marketplace = r.dir.path().join("home/.agents/plugins/marketplace.json");
    let mut doc: Value = serde_json::from_slice(&std::fs::read(&marketplace).unwrap()).unwrap();
    doc["plugins"][0]["source"] = json!("./my-custom-plugin");
    std::fs::write(&marketplace, doc.to_string()).unwrap();
    assert!(
        !r.installed(&["integration", "remove", "plugin", "--apply"])
            .status
            .success()
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&marketplace).unwrap()).unwrap(),
        doc
    );
    let skill = r.install.join("skills/background-jobs/SKILL.md");
    let original = std::fs::read(&skill).unwrap();
    std::fs::write(&skill, "my customization").unwrap();
    for action in ["install", "uninstall"] {
        assert!(
            !r.source(&[action, "--root", r.install.to_str().unwrap(), "--apply"])
                .status
                .success()
        );
    }
    assert_eq!(std::fs::read_to_string(&skill).unwrap(), "my customization");
    std::fs::write(skill, original).unwrap();
    std::fs::write(r.install.join("keep.txt"), "keep").unwrap();
    assert!(
        !r.source(&[
            "uninstall",
            "--root",
            r.install.to_str().unwrap(),
            "--apply"
        ])
        .status
        .success()
    );
    assert_eq!(
        std::fs::read_to_string(r.install.join("keep.txt")).unwrap(),
        "keep"
    );
}
#[test]
fn replacement_and_removal_refuse_active_runtime_and_keep_data() {
    let r = Rig::new();
    r.install();
    let start = ok(r.installed(&["start", "--", "/bin/sleep", "0.5"]));
    assert!(
        !r.source(&[
            "uninstall",
            "--root",
            r.install.to_str().unwrap(),
            "--apply"
        ])
        .status
        .success()
    );
    r.wait_done(start["job"]["job_id"].as_str().unwrap());
    assert!(
        !r.source(&[
            "uninstall",
            "--root",
            r.install.to_str().unwrap(),
            "--apply"
        ])
        .status
        .success()
    );
    r.stop();
    let next = r.dir.path().join("next-state");
    let out = r
        .command(Path::new(env!("CARGO_BIN_EXE_agent-job-relay")))
        .arg("--state-dir")
        .arg(&next)
        .args(["install", "--root", r.install.to_str().unwrap(), "--apply"])
        .output()
        .unwrap();
    let result = ok(out);
    assert!(Path::new(result["backup"].as_str().unwrap()).is_dir());
    assert!(r.state.join("relay.sqlite3").exists());
    assert!(!next.exists());
    assert_eq!(ok(r.installed(&["doctor"]))["daemon"], "stopped");
}
#[test]
fn installation_rejects_symlink_ancestors_nested_state_and_foreign_destination() {
    let r = Rig::new();
    let foreign = r.dir.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("keep"), "keep").unwrap();
    assert!(
        !r.source(&["install", "--root", foreign.to_str().unwrap(), "--apply"])
            .status
            .success()
    );
    let link = r.dir.path().join("linked");
    std::os::unix::fs::symlink(&foreign, &link).unwrap();
    assert!(
        !r.source(&[
            "install",
            "--root",
            link.join("child").to_str().unwrap(),
            "--apply"
        ])
        .status
        .success()
    );
    let nested = r.state.join("bin");
    assert!(
        !r.source(&["install", "--root", nested.to_str().unwrap(), "--apply"])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read_to_string(foreign.join("keep")).unwrap(),
        "keep"
    );
    assert!(!r.state.exists());
}
