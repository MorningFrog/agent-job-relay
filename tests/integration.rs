mod common;
use agent_job_relay::{
    config::Config,
    integration::{self, Client},
};
use std::path::Path;
fn fixture() -> (tempfile::TempDir, Config) {
    let d = tempfile::tempdir_in(common::root()).unwrap();
    let c = Config::new(Some(d.path().join("state"))).unwrap();
    (d, c)
}
fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}
#[test]
fn codex_preview_idempotence_backup_and_safe_remove() {
    let (d, c) = fixture();
    let p = d.path().join("config.toml");
    let original =
        "# preserve comment\nmodel = 'example'\n[mcp_servers.other]\ncommand = 'other'\n";
    std::fs::write(&p, original).unwrap();
    let preview = integration::configure(&c, Client::Codex, p.clone(), false, false).unwrap();
    assert_eq!(preview["mode"], "dry_run");
    assert_eq!(read(&p), original);
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    let applied = integration::configure(&c, Client::Codex, p.clone(), false, true).unwrap();
    assert_eq!(
        read(Path::new(applied["backup"].as_str().unwrap())),
        original
    );
    assert!(read(&p).contains("# preserve comment"));
    assert!(read(&p).contains("[mcp_servers.other]"));
    let a = read(&p);
    assert_eq!(
        integration::configure(&c, Client::Codex, p.clone(), false, true).unwrap()["changed"],
        false
    );
    assert_eq!(read(&p), a);
    integration::configure(&c, Client::Codex, p.clone(), true, false).unwrap();
    assert_eq!(read(&p), a);
    integration::configure(&c, Client::Codex, p.clone(), true, true).unwrap();
    assert!(!read(&p).contains("[mcp_servers.agent-job-relay]"));
    assert!(read(&p).contains("[mcp_servers.other]"));
    assert_eq!(
        integration::configure(&c, Client::Codex, p.clone(), true, true).unwrap()["changed"],
        false
    );
    assert!(!c.root.exists());
}
#[test]
fn json_preserves_foreign_entries_and_refuses_modified_owned_entry() {
    let (d, c) = fixture();
    let p = d.path().join("mcp.json");
    std::fs::write(
        &p,
        r#"{"theme":"dark","mcpServers":{"other":{"command":"keep"}}}"#,
    )
    .unwrap();
    integration::configure(&c, Client::Json, p.clone(), false, true).unwrap();
    let mut doc: serde_json::Value = serde_json::from_str(&read(&p)).unwrap();
    assert_eq!(doc["mcpServers"]["other"]["command"], "keep");
    doc["mcpServers"]["agent-job-relay"]["args"] = serde_json::json!(["custom"]);
    std::fs::write(&p, doc.to_string()).unwrap();
    assert!(
        integration::configure(&c, Client::Json, p.clone(), true, true)
            .unwrap_err()
            .to_string()
            .contains("user modifications")
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&read(&p)).unwrap(),
        doc
    );
}
#[test]
fn rejects_unowned_entry_malformed_config_and_symlink() {
    let (d, c) = fixture();
    let p = d.path().join("mcp.json");
    std::fs::write(
        &p,
        r#"{"mcpServers":{"agent-job-relay":{"command":"user-owned"}}}"#,
    )
    .unwrap();
    assert!(integration::configure(&c, Client::Json, p.clone(), false, true).is_err());
    assert!(integration::configure(&c, Client::Json, p.clone(), true, true).is_err());
    std::fs::write(&p, "{invalid").unwrap();
    assert!(integration::configure(&c, Client::Json, p.clone(), false, true).is_err());
    assert_eq!(read(&p), "{invalid");
    #[cfg(unix)]
    {
        let link = d.path().join("link.json");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(integration::configure(&c, Client::Json, link, false, true).is_err());
    }
}
#[test]
fn cli_preview_clean_home_and_doctor_are_actionable() {
    let (d, _) = fixture();
    let bin = env!("CARGO_BIN_EXE_agent-job-relay");
    let config = d.path().join("home/.codex/config.toml");
    let out = std::process::Command::new(bin)
        .env("HOME", d.path().join("home"))
        .env("CODEX_HOME", d.path().join("home/.codex"))
        .args(["setup", "codex", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!config.exists());
    let out = std::process::Command::new(bin)
        .arg("--state-dir")
        .arg(d.path().join("no-daemon"))
        .arg("doctor")
        .output()
        .unwrap();
    if cfg!(any(target_os = "linux", windows)) {
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["daemon"], "stopped");
        assert!(!d.path().join("no-daemon").exists());
    }
}
