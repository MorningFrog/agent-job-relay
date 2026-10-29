//! Durable, explicitly recoverable directory replacements. Recovery only finishes
//! the recorded replacement; it never rolls back a runtime or touches job data.
use crate::deployment::{self as deploy, Artifact, PROFILE, RECEIPT, RuntimeProfile};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

type Snapshot = BTreeMap<String, String>;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Install,
    Cache,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    format: u32,
    id: String,
    kind: Kind,
    root: PathBuf,
    install_root: PathBuf,
    state_dir: PathBuf,
    before_state_dir: Option<PathBuf>,
    before: Option<Snapshot>,
    after: Snapshot,
}
fn exists(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn journal(root: &Path) -> Result<PathBuf> {
    deploy::checked_path(root)?;
    Ok(root.parent().unwrap().join(format!(
        ".{}.ajr-operation.json",
        root.file_name()
            .context("directory name missing")?
            .to_str()
            .context("non-UTF-8 directory name")?
    )))
}
pub(crate) fn require_clear(root: &Path) -> Result<()> {
    ensure!(
        !exists(&journal(root)?)?,
        "interrupted plugin operation; run plugin repair --path '{}' --dry-run, then --apply with a new bundle/source executable",
        root.display()
    );
    Ok(())
}
pub(crate) fn lock(root: &Path, kind: Kind) -> Result<File> {
    deploy::checked_path(root)?;
    let name = if kind == Kind::Install {
        "install"
    } else {
        "cache"
    };
    let f = crate::platform::private_file(&root.parent().unwrap().join(format!(
        ".{}.{}.lock",
        root.file_name().unwrap().to_string_lossy(),
        name
    )))?;
    f.try_lock()
        .context("another plugin lifecycle operation is in progress")?;
    Ok(f)
}
fn sync_tree(root: &Path) -> Result<()> {
    for item in std::fs::read_dir(root)? {
        let path = item?.path();
        if std::fs::symlink_metadata(&path)?.is_dir() {
            sync_tree(&path)?;
        }
    }
    crate::platform::sync_directory(root)?;
    Ok(())
}
fn snapshot(root: &Path) -> Result<Snapshot> {
    let mut names = Default::default();
    deploy::actual_files(root, root, &mut names)?;
    names
        .into_iter()
        .filter(|n: &String| !n.ends_with('/'))
        .map(|n| {
            Ok((
                n.clone(),
                deploy::digest(&deploy::read_file(&root.join(n))?),
            ))
        })
        .collect()
}
impl Operation {
    fn stage(&self) -> PathBuf {
        self.root
            .parent()
            .unwrap()
            .join(format!(".ajr-stage-{}", self.id))
    }
    fn backup(&self) -> PathBuf {
        self.root
            .parent()
            .unwrap()
            .join(format!(".ajr-backup-{}", self.id))
    }
    fn validate(&self, root: &Path) -> Result<()> {
        ensure!(
            self.format == 1
                && self.root == root
                && uuid::Uuid::parse_str(&self.id)?.to_string() == self.id,
            "invalid plugin operation identity"
        );
        for path in [
            &self.root,
            &self.install_root,
            &self.state_dir,
            &self.stage(),
            &self.backup(),
        ] {
            deploy::checked_path(path)?;
        }
        ensure!(
            self.root != self.stage() && self.root != self.backup(),
            "operation path collision"
        );
        ensure!(
            self.before.is_some() == self.before_state_dir.is_some(),
            "operation original profile missing"
        );
        for state in self
            .before_state_dir
            .iter()
            .chain(std::iter::once(&self.state_dir))
        {
            deploy::checked_path(state)?;
            for destination in [&self.root, &self.stage(), &self.backup()] {
                ensure!(
                    !destination.starts_with(state) && !state.starts_with(destination),
                    "operation overlaps task data"
                );
            }
        }
        if self.kind == Kind::Cache {
            for path in [self.stage(), self.backup()] {
                ensure!(
                    !path.starts_with(&self.install_root) && !self.install_root.starts_with(&path),
                    "operation staging overlaps installation"
                );
            }
        }
        match self.kind {
            Kind::Install => ensure!(
                self.root == self.install_root,
                "operation install root mismatch"
            ),
            Kind::Cache => ensure!(
                !self.root.starts_with(&self.install_root)
                    && !self.install_root.starts_with(&self.root),
                "operation cache overlaps installation"
            ),
        }
        for files in self.before.iter().chain(std::iter::once(&self.after)) {
            for name in files.keys() {
                ensure!(
                    !name.is_empty()
                        && Path::new(name)
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_))),
                    "unsafe operation file path"
                );
            }
            ensure!(
                files.contains_key(crate::platform::NATIVE_FILE)
                    && (files.contains_key(PROFILE)
                        || (self.kind == Kind::Install && files.contains_key(RECEIPT))),
                "incomplete operation snapshot"
            );
        }
        Ok(())
    }
    fn verify(&self, path: &Path, files: &Snapshot) -> Result<()> {
        deploy::checked_path(path)?;
        crate::plugin::verify_tree(path, files)?;
        let state = if self.before.as_ref() == Some(files) {
            self.before_state_dir
                .as_ref()
                .context("original state missing")?
        } else {
            &self.state_dir
        };
        if !files.contains_key(PROFILE) {
            let r = deploy::read_receipt(path)?;
            let mut expected = r.files;
            expected.insert(RECEIPT.into(), files[RECEIPT].clone());
            ensure!(
                self.kind == Kind::Install
                    && r.root == self.install_root
                    && r.state_dir == *state
                    && expected == *files,
                "legacy operation receipt mismatch"
            );
            return Ok(());
        }
        let p: RuntimeProfile = serde_json::from_slice(&deploy::read_file(&path.join(PROFILE))?)?;
        ensure!(
            p.format == 1
                && p.install_root == self.install_root
                && p.state_dir == *state
                && p.platform == crate::platform::runtime_target(),
            "operation profile mismatch"
        );
        let mut expected = p.files;
        expected.insert(PROFILE.into(), files[PROFILE].clone());
        if files.contains_key(RECEIPT) {
            let r = deploy::read_receipt(path)?;
            ensure!(
                r.root == self.install_root && r.state_dir == *state && r.files == expected,
                "operation receipt mismatch"
            );
            expected.insert(RECEIPT.into(), files[RECEIPT].clone());
        } else {
            ensure!(self.kind == Kind::Cache, "installation receipt missing");
        }
        ensure!(
            expected == *files,
            "operation snapshot differs from profile"
        );
        Ok(())
    }
    fn phase(&self) -> Result<&'static str> {
        self.validate(&self.root)?;
        let stage = self.stage();
        let backup = self.backup();
        if backup.try_exists()? {
            self.verify(
                &backup,
                self.before
                    .as_ref()
                    .context("unexpected operation backup")?,
            )?;
        }
        if self.root.try_exists()? {
            if snapshot(&self.root)? == self.after {
                self.verify(&self.root, &self.after)?;
                ensure!(
                    !stage.try_exists()?,
                    "unexpected staging directory after replacement"
                );
                ensure!(
                    self.before.is_none() || backup.try_exists()?,
                    "recorded backup is missing"
                );
                return Ok("installed");
            }
            self.verify(
                &self.root,
                self.before
                    .as_ref()
                    .context("unexpected destination appeared")?,
            )?;
            ensure!(
                !backup.try_exists()?,
                "both old destination and backup exist"
            );
        } else if self.before.is_some() {
            ensure!(
                backup.try_exists()?,
                "recorded original and backup are missing"
            );
        }
        self.verify(&stage, &self.after)?;
        Ok(if self.root.try_exists()? {
            "prepared"
        } else {
            "destination_missing"
        })
    }
    fn finish(&self) -> Result<()> {
        if self.phase()? != "installed" {
            if self.root.try_exists()? {
                crate::platform::rename_directory(&self.root, &self.backup())?;
                crate::platform::sync_directory(self.root.parent().unwrap())?;
            }
            crate::platform::rename_directory(&self.stage(), &self.root)?;
            crate::platform::sync_directory(self.root.parent().unwrap())?;
        }
        self.verify(&self.root, &self.after)?;
        std::fs::remove_file(journal(&self.root)?)?;
        crate::platform::sync_directory(self.root.parent().unwrap())?;
        Ok(())
    }
}
/// Caller holds the appropriate destination lock and any idle runtime guards.
pub(crate) fn replace(
    root: &Path,
    install_root: &Path,
    state: &Path,
    kind: Kind,
    files: &BTreeMap<String, Artifact>,
) -> Result<Option<PathBuf>> {
    let op = prepare(root, install_root, state, kind, files)?;
    op.finish().context("plugin replacement interrupted; preserve the operation journal and use plugin repair --path with a new bundle/source executable")?;
    Ok(op.before.as_ref().map(|_| op.backup()))
}
fn prepare(
    root: &Path,
    install_root: &Path,
    state: &Path,
    kind: Kind,
    files: &BTreeMap<String, Artifact>,
) -> Result<Operation> {
    require_clear(root)?;
    let op = Operation {
        format: 1,
        id: crate::core::id(),
        kind,
        root: root.into(),
        install_root: install_root.into(),
        state_dir: state.into(),
        before_state_dir: if root.try_exists()? {
            Some(if kind == Kind::Install {
                deploy::read_receipt(root)?.state_dir
            } else {
                state.into()
            })
        } else {
            None
        },
        before: if root.try_exists()? {
            Some(snapshot(root)?)
        } else {
            None
        },
        after: files
            .iter()
            .map(|(n, f)| (n.clone(), deploy::digest(&f.bytes)))
            .collect(),
    };
    op.validate(root)?;
    if let Some(before) = &op.before {
        op.verify(root, before)?;
    }
    let stage = op.stage();
    crate::platform::secure_dir(&stage)?;
    for (name, file) in files {
        deploy::write_artifact(&stage.join(name), file)?;
    }
    op.verify(&stage, &op.after)?;
    sync_tree(&stage)?;
    publish(&op)?;
    Ok(op)
}

fn publish(op: &Operation) -> Result<()> {
    // A killed writer leaves either no journal or a complete immutable one.
    let path = journal(&op.root)?;
    let tmp = op
        .root
        .parent()
        .unwrap()
        .join(format!(".ajr-journal-{}", op.id));
    let mut out = crate::platform::create_new_file(&tmp)?;
    out.write_all(&serde_json::to_vec_pretty(op)?)?;
    out.sync_all()?;
    drop(out);
    ensure!(
        !exists(&path)?,
        "operation journal appeared; preserve and inspect it"
    );
    crate::platform::publish_file(&tmp, &path)?;
    crate::platform::sync_directory(op.root.parent().unwrap())?;
    Ok(())
}
/// Inspect only an explicitly selected operation; never discover caches/backups.
pub fn repair(root: PathBuf, apply: bool) -> Result<Value> {
    crate::platform::require_runtime()?;
    let path = journal(&root)?;
    if !exists(&path)? {
        return Ok(
            json!({"mode":if apply {"apply"} else {"dry_run"},"path":root,"changed":false,"phase":"no_pending_operation"}),
        );
    }
    let raw = deploy::read_file(&path)?;
    let op: Operation = serde_json::from_slice(&raw)
        .context("invalid operation journal; preserved for inspection")?;
    op.validate(&root)?;
    if op.kind == Kind::Cache {
        require_clear(&op.install_root)?;
    }
    let _install_lock = if apply && op.kind == Kind::Cache {
        Some(lock(&op.install_root, Kind::Install)?)
    } else {
        None
    };
    let _lock = if apply {
        Some(lock(&root, op.kind)?)
    } else {
        None
    };
    ensure!(
        deploy::read_file(&path)? == raw,
        "operation changed; retry preview"
    );
    let phase = op.phase()?;
    let mut guards = Vec::new();
    if apply && op.kind == Kind::Install {
        guards.extend(deploy::idle_guards(&op.state_dir, &op.install_root)?);
        if let Some(old) = &op.before_state_dir
            && old != &op.state_dir
        {
            guards.extend(deploy::idle_guards(old, &op.install_root)?);
        }
    }
    if apply {
        op.finish()?;
    }
    Ok(
        json!({"mode":if apply {"apply"} else {"dry_run"},"path":root,"changed":true,"phase":phase,"state_dir":op.state_dir,"backup":if op.before.is_some(){Some(op.backup())}else{None},"data":"preserved","next_step":"Retry the original bootstrap/update command to finish marketplace registration or other explicitly selected caches. Read current launcher instructions; refresh/reconnect only for unloaded tools or historical entries."}),
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{config::Config, deployment::Receipt};
    struct Rig {
        dir: tempfile::TempDir,
        root: PathBuf,
        install: PathBuf,
        state: PathBuf,
    }
    impl Rig {
        fn new(cache: bool) -> Self {
            let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
            std::fs::create_dir_all(&base).unwrap();
            let dir = tempfile::Builder::new()
                .prefix("repair-")
                .tempdir_in(base)
                .unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            let install = dir.path().join("installation");
            let root = if cache {
                dir.path().join("cache")
            } else {
                install.clone()
            };
            let state = dir.path().join("history");
            crate::platform::secure_dir(&state).unwrap();
            std::fs::write(state.join("keep"), b"job history").unwrap();
            Self {
                dir,
                root,
                install,
                state,
            }
        }
        fn files(&self, revision: &str, receipt: bool) -> BTreeMap<String, Artifact> {
            let entries: &[(&str, &[u8])] = &[
                (crate::platform::NATIVE_FILE, b"native fixture"),
                (
                    ".codex-plugin/plugin.json",
                    br#"{"name":"agent-job-relay","version":"0.1.4"}"#,
                ),
                (".mcp.json", br#"{"mcpServers":{"agent-job-relay":{}}}"#),
                (
                    crate::platform::LAUNCHER_FILE,
                    b"__AJR_INSTALLED_STATE__ __AJR_INSTALLED_EXECUTABLE__",
                ),
                ("LICENSE", revision.as_bytes()),
            ];
            let files = entries
                .iter()
                .map(|(n, b)| {
                    (
                        (*n).into(),
                        Artifact {
                            bytes: b.to_vec(),
                            executable: false,
                        },
                    )
                })
                .collect();
            let mut files = deploy::render(
                &Config {
                    root: self.state.clone(),
                },
                &self.install,
                files,
            )
            .unwrap();
            if receipt {
                let r = Receipt {
                    format: 1,
                    product: "agent-job-relay".into(),
                    root: self.install.clone(),
                    state_dir: self.state.clone(),
                    files: files
                        .iter()
                        .map(|(n, f)| (n.clone(), deploy::digest(&f.bytes)))
                        .collect(),
                };
                files.insert(
                    RECEIPT.into(),
                    Artifact {
                        bytes: serde_json::to_vec(&r).unwrap(),
                        executable: false,
                    },
                );
            }
            files
        }
        fn write(&self, files: &BTreeMap<String, Artifact>) {
            for (n, f) in files {
                deploy::write_artifact(&self.root.join(n), f).unwrap();
            }
        }
        fn prepare(&self, kind: Kind) -> Operation {
            prepare(
                &self.root,
                &self.install,
                &self.state,
                kind,
                &self.files("new", kind == Kind::Install),
            )
            .unwrap()
        }
    }
    #[test]
    fn interrupted_replacements_finish_forward_preserve_history_and_are_idempotent() {
        for (kind, old, receipt) in [
            (Kind::Install, false, true),
            (Kind::Install, true, true),
            (Kind::Cache, true, false),
            (Kind::Cache, true, true),
        ] {
            for phase in 0..3 {
                let r = Rig::new(kind == Kind::Cache);
                if old {
                    r.write(&r.files("old", receipt));
                }
                let op = r.prepare(kind);
                if phase > 0 && old {
                    std::fs::rename(&r.root, op.backup()).unwrap();
                }
                if phase == 2 {
                    std::fs::rename(op.stage(), &r.root).unwrap();
                }
                let preview = repair(r.root.clone(), false).unwrap();
                assert_eq!(preview["changed"], true);
                assert!(journal(&r.root).unwrap().exists());
                assert!(require_clear(&r.root).is_err());
                repair(r.root.clone(), true).unwrap();
                assert_eq!(std::fs::read(r.root.join("LICENSE")).unwrap(), b"new");
                assert_eq!(std::fs::read(r.state.join("keep")).unwrap(), b"job history");
                if old {
                    assert_eq!(std::fs::read(op.backup().join("LICENSE")).unwrap(), b"old");
                }
                assert_eq!(repair(r.root.clone(), true).unwrap()["changed"], false);
            }
        }
    }
    #[test]
    fn repair_refuses_modified_staged_original_backup_and_foreign_destination() {
        for scenario in 0..4 {
            let r = Rig::new(false);
            r.write(&r.files("old", true));
            let op = r.prepare(Kind::Install);
            let changed = match scenario {
                0 => op.stage().join("LICENSE"),
                1 => r.root.join("LICENSE"),
                2 => {
                    std::fs::rename(&r.root, op.backup()).unwrap();
                    op.backup().join("LICENSE")
                }
                _ => {
                    std::fs::rename(&r.root, op.backup()).unwrap();
                    std::fs::create_dir(&r.root).unwrap();
                    r.root.join("foreign")
                }
            };
            std::fs::write(&changed, b"my edits").unwrap();
            assert!(repair(r.root.clone(), false).is_err());
            assert!(repair(r.root.clone(), true).is_err());
            assert_eq!(std::fs::read(changed).unwrap(), b"my edits");
            assert!(journal(&r.root).unwrap().exists());
        }
    }
    #[test]
    fn repair_validates_journal_paths_symlinks_and_cooperating_locks() {
        let r = Rig::new(false);
        r.write(&r.files("old", true));
        let mut op = r.prepare(Kind::Install);
        let lock = lock(&r.root, Kind::Install).unwrap();
        assert!(repair(r.root.clone(), true).is_err());
        drop(lock);
        let file = journal(&r.root).unwrap();
        let original = std::fs::read(&file).unwrap();
        op.state_dir = r.root.clone();
        std::fs::write(&file, serde_json::to_vec(&op).unwrap()).unwrap();
        assert!(repair(r.root.clone(), true).is_err());
        std::fs::write(&file, &original).unwrap();
        std::fs::rename(&r.root, r.dir.path().join("saved")).unwrap();
        std::os::unix::fs::symlink(r.dir.path().join("saved"), &r.root).unwrap();
        assert!(repair(r.root.clone(), true).is_err());
        assert_eq!(
            std::fs::read(r.dir.path().join("saved/LICENSE")).unwrap(),
            b"old"
        );
    }
    #[test]
    fn installation_repair_refuses_active_daemon_but_cache_repair_is_independent() {
        for kind in [Kind::Install, Kind::Cache] {
            let r = Rig::new(kind == Kind::Cache);
            r.write(&r.files("old", true));
            r.prepare(kind);
            let daemon = crate::platform::private_file(&r.state.join("daemon.lock")).unwrap();
            daemon.try_lock().unwrap();
            let result = repair(r.root.clone(), true);
            assert_eq!(result.is_ok(), kind == Kind::Cache);
            drop(daemon);
            if kind == Kind::Install {
                repair(r.root.clone(), true).unwrap();
            }
        }
    }
    #[test]
    fn repair_preserves_pending_operation_when_staging_or_backup_is_missing() {
        for installed in [false, true] {
            let r = Rig::new(false);
            r.write(&r.files("old", true));
            let op = r.prepare(Kind::Install);
            if installed {
                std::fs::rename(&r.root, op.backup()).unwrap();
                std::fs::rename(op.stage(), &r.root).unwrap();
                std::fs::remove_dir_all(op.backup()).unwrap();
            } else {
                std::fs::remove_file(op.stage().join("LICENSE")).unwrap();
            }
            assert!(repair(r.root.clone(), true).is_err());
            assert!(journal(&r.root).unwrap().exists());
            assert_eq!(
                std::fs::read(r.root.join("LICENSE")).unwrap(),
                if installed { b"new" } else { b"old" }
            );
        }
    }
    #[test]
    fn replacement_accepts_legacy_owned_installation_without_runtime_profile() {
        let r = Rig::new(false);
        let mut files = r.files("old", true);
        files.remove(PROFILE);
        let mut receipt: Receipt = serde_json::from_slice(&files[RECEIPT].bytes).unwrap();
        receipt.files.remove(PROFILE);
        files.get_mut(RECEIPT).unwrap().bytes = serde_json::to_vec(&receipt).unwrap();
        r.write(&files);
        let op = r.prepare(Kind::Install);
        std::fs::rename(&r.root, op.backup()).unwrap();
        repair(r.root.clone(), true).unwrap();
        assert!(r.root.join(PROFILE).exists());
    }
}
