//! A private local development install, also usable as a relocatable plugin.
use crate::{config::Config, core::id};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Write,
    path::{Component, Path, PathBuf},
};

const RECEIPT: &str = ".agent-job-relay-install.json";
const MAX_FILE: u64 = 128 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: u32,
    product: String,
    root: PathBuf,
    state_dir: PathBuf,
    files: BTreeMap<String, String>,
}
struct Artifact {
    bytes: Vec<u8>,
    executable: bool,
}
fn checked_path(path: &Path) -> Result<()> {
    ensure!(
        path.is_absolute() && path.parent().is_some(),
        "use an absolute, non-root install path"
    );
    let mut walked = PathBuf::new();
    for component in path.components() {
        ensure!(
            !matches!(component, Component::ParentDir | Component::CurDir),
            "path must not contain . or .."
        );
        walked.push(component.as_os_str());
        if let Ok(m) = std::fs::symlink_metadata(&walked) {
            ensure!(
                !m.file_type().is_symlink(),
                "refusing a symlink in install/state path: {}",
                walked.display()
            );
        }
    }
    Ok(())
}
fn read_file(path: &Path) -> Result<Vec<u8>> {
    let m = std::fs::symlink_metadata(path)?;
    ensure!(
        m.is_file() && !m.file_type().is_symlink() && m.len() <= MAX_FILE,
        "refusing non-regular or oversized install file: {}",
        path.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            m.uid() == unsafe { libc::geteuid() },
            "install file belongs to another user"
        );
    }
    Ok(std::fs::read(path)?)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_receipt(root: &Path) -> Result<Receipt> {
    let r: Receipt = serde_json::from_slice(&read_file(&root.join(RECEIPT))?)
        .context("invalid installation receipt; nothing was changed")?;
    ensure!(
        r.format == 1 && r.product == "agent-job-relay",
        "unknown installation receipt"
    );
    ensure!(
        r.root.is_absolute() && r.state_dir.is_absolute(),
        "invalid installation profile"
    );
    for name in r.files.keys() {
        ensure!(
            Path::new(name)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
                && name != RECEIPT,
            "unsafe path in installation receipt"
        );
    }
    ensure!(
        r.files.contains_key("bin/agent-job-relay") && r.files.contains_key("plugin.json"),
        "incomplete installation receipt"
    );
    Ok(r)
}
/// A cached plugin copy keeps its configured state, but never a host/thread ID.
pub fn runtime_state() -> Result<Option<PathBuf>> {
    let exe = std::env::current_exe()?;
    let Some(root) = exe.parent().and_then(Path::parent) else {
        return Ok(None);
    };
    if exe
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|p| p == "bin")
        && root.join(RECEIPT).exists()
    {
        return Ok(Some(read_receipt(root)?.state_dir));
    }
    Ok(None)
}
pub fn installed_root() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let root = exe
        .parent()
        .and_then(Path::parent)
        .context("cannot locate installed plugin")?;
    let r = read_receipt(root).context(
        "run install --apply first, then use the installed relay launcher for setup plugin",
    )?;
    ensure!(
        r.root.join("bin/agent-job-relay").is_file(),
        "original installation is unavailable; reinstall before configuring the marketplace"
    );
    Ok(r.root)
}
pub fn default_root() -> Result<PathBuf> {
    Ok(std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .context("HOME/XDG_DATA_HOME unavailable; pass --root")?
        .join("agent-job-relay"))
}
fn artifacts() -> Result<BTreeMap<String, Artifact>> {
    let mut files = BTreeMap::new();
    let entries: &[(&str, &[u8], bool)] = &[
        (
            "plugin.json",
            include_bytes!("../plugins/agent-job-relay/plugin.json"),
            false,
        ),
        (
            "mcp.json",
            include_bytes!("../plugins/agent-job-relay/mcp.json"),
            false,
        ),
        (
            "relay",
            include_bytes!("../plugins/agent-job-relay/relay"),
            true,
        ),
        (
            "skills/background-jobs/SKILL.md",
            include_bytes!("../plugins/agent-job-relay/skills/background-jobs/SKILL.md"),
            false,
        ),
        (
            "assets/icon.svg",
            include_bytes!("../assets/icons/agent-job-relay.svg"),
            false,
        ),
    ];
    for (name, bytes, executable) in entries {
        files.insert(
            (*name).into(),
            Artifact {
                bytes: bytes.to_vec(),
                executable: *executable,
            },
        );
    }
    files.insert(
        "bin/agent-job-relay".into(),
        Artifact {
            bytes: read_file(&std::env::current_exe()?)?,
            executable: true,
        },
    );
    Ok(files)
}
fn actual_files(root: &Path, dir: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    for item in std::fs::read_dir(dir)? {
        let path = item?.path();
        let m = std::fs::symlink_metadata(&path)?;
        ensure!(
            !m.file_type().is_symlink(),
            "unmanaged symlink in install; preserve and review it"
        );
        if m.is_dir() {
            files.insert(format!(
                "{}/",
                path.strip_prefix(root)?
                    .to_str()
                    .context("non-UTF-8 install directory")?
            ));
            actual_files(root, &path, files)?;
        } else {
            ensure!(m.is_file(), "unmanaged special file in install");
            files.insert(
                path.strip_prefix(root)?
                    .to_str()
                    .context("non-UTF-8 install filename")?
                    .into(),
            );
        }
    }
    Ok(())
}
fn verify_owned(root: &Path, receipt: &Receipt) -> Result<()> {
    ensure!(
        receipt.root == root,
        "installation was moved; use a fresh install root instead of replacing it"
    );
    let mut actual = BTreeSet::new();
    actual_files(root, root, &mut actual)?;
    let mut expected: BTreeSet<_> = receipt.files.keys().cloned().collect();
    expected.insert(RECEIPT.into());
    for name in receipt.files.keys() {
        let mut dir = Path::new(name).parent();
        while let Some(path) = dir.filter(|p| !p.as_os_str().is_empty()) {
            expected.insert(format!(
                "{}/",
                path.to_str().context("non-UTF-8 receipt directory")?
            ));
            dir = path.parent();
        }
    }
    ensure!(
        actual == expected,
        "install contains missing or unmanaged files; nothing was changed"
    );
    for (name, hash) in &receipt.files {
        ensure!(
            digest(&read_file(&root.join(name))?) == *hash,
            "installed file has user modifications: {name}; nothing was changed"
        );
    }
    Ok(())
}
/// Keep both locks until replacement/removal ends. No existing daemon is stopped.
fn idle_guards(state: &Path) -> Result<Vec<File>> {
    if !state.exists() {
        return Ok(Vec::new());
    }
    checked_path(state)?;
    let mut locks = Vec::new();
    for name in ["startup.lock", "daemon.lock"] {
        let path = state.join(name);
        if path.exists() {
            let f = File::open(&path)?;
            f.try_lock().context("daemon/startup is active; use the installed relay daemon stop before replacing/removing binaries")?;
            locks.push(f);
        }
    }
    let db = state.join("relay.sqlite3");
    if db.exists() {
        let conn =
            rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let active: i64 = conn.query_row(
            "SELECT count(*) FROM jobs WHERE status IN ('queued','launching','running')",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            active == 0,
            "active jobs exist; wait for completion or explicitly cancel them before replacing/removing binaries"
        );
    }
    Ok(locks)
}
fn write_artifact(path: &Path, file: &Artifact) -> Result<()> {
    crate::platform::secure_dir(path.parent().context("install parent missing")?)?;
    let mut out = crate::platform::private_file(path)?;
    out.write_all(&file.bytes)?;
    out.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(if file.executable { 0o700 } else { 0o600 }),
        )?;
    }
    Ok(())
}
pub fn change(config: &Config, root: PathBuf, remove: bool, apply: bool) -> Result<Value> {
    crate::platform::require_runtime()?;
    checked_path(&root)?;
    checked_path(&config.root)?;
    ensure!(
        !config.root.starts_with(&root) && !root.starts_with(&config.root),
        "install and state directories must be separate, not nested"
    );
    let parent = root.parent().context("install parent missing")?;
    let mode = if apply { "apply" } else { "dry_run" };
    let existing = if root.exists() {
        Some(
            read_receipt(&root)
                .context("destination is not an owned installation; choose an empty new path")?,
        )
    } else {
        None
    };
    if let Some(old) = &existing {
        verify_owned(&root, old)?;
    }
    let files = if remove {
        BTreeMap::new()
    } else {
        artifacts()?
    };
    let desired = Receipt {
        format: 1,
        product: "agent-job-relay".into(),
        root: root.clone(),
        state_dir: config.root.clone(),
        files: files
            .iter()
            .map(|(p, f)| (p.clone(), digest(&f.bytes)))
            .collect(),
    };
    let changed = if remove {
        existing.is_some()
    } else {
        existing
            .as_ref()
            .is_none_or(|old| old.files != desired.files || old.state_dir != desired.state_dir)
    };
    let mut backup = None;
    if apply && changed {
        std::fs::create_dir_all(parent)?;
        let lock = crate::platform::private_file(&parent.join(format!(
            ".{}.install.lock",
            root.file_name().unwrap().to_string_lossy()
        )))?;
        lock.try_lock()
            .context("another installation/removal is in progress")?;
        // A concurrent writer must not turn a preview into an unowned overwrite.
        ensure!(
            root.exists() == existing.is_some(),
            "installation changed while preparing; retry"
        );
        let mut guards = Vec::new();
        if let Some(old) = &existing {
            verify_owned(&root, old)?;
            guards.extend(idle_guards(&old.state_dir)?);
        }
        if remove {
            let old = existing.as_ref().unwrap();
            for name in old.files.keys() {
                std::fs::remove_file(root.join(name))?;
            }
            std::fs::remove_file(root.join(RECEIPT))?;
            remove_empty_dirs(&root)?;
        } else {
            let stage = parent.join(format!(".ajr-stage-{}", id()));
            let result = (|| -> Result<()> {
                crate::platform::secure_dir(&stage)?;
                for (name, artifact) in &files {
                    write_artifact(&stage.join(name), artifact)?;
                }
                write_artifact(
                    &stage.join(RECEIPT),
                    &Artifact {
                        bytes: serde_json::to_vec_pretty(&desired)?,
                        executable: false,
                    },
                )?;
                if root.exists() {
                    let old = parent.join(format!(".ajr-backup-{}", id()));
                    std::fs::rename(&root, &old)?;
                    backup = Some(old);
                }
                if let Err(e) = std::fs::rename(&stage, &root) {
                    if let Some(old) = &backup {
                        std::fs::rename(old, &root)
                            .context("could not restore installation backup")?;
                    }
                    return Err(e.into());
                }
                File::open(parent)?.sync_all()?;
                Ok(())
            })();
            if stage.exists() {
                let _ = std::fs::remove_dir_all(&stage);
            } // Only our newly created staging directory.
            result?;
        }
        drop(guards);
    }
    let state = if remove {
        existing
            .as_ref()
            .map(|r| &r.state_dir)
            .unwrap_or(&config.root)
    } else {
        &config.root
    };
    Ok(
        json!({"mode":mode,"changed":changed,"action":if remove{"remove owned local installation"}else{"install native executable and Desktop plugin"},"root":root,"executable":root.join("bin/agent-job-relay"),"launcher":root.join("relay"),"state_dir":state,"backup":backup,"data":"preserved","configuration":"not changed; setup codex/plugin is a separate preview/apply operation","next_step":if remove{"Remove any owned MCP/marketplace entry before uninstall; plugin cache copies are managed by the host. State and installation backups are retained."}else{"Run the installed relay launcher with setup plugin --dry-run or setup codex --dry-run. Install the plugin from its local marketplace, then start a new chat."}}),
    )
}
fn remove_empty_dirs(dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            remove_empty_dirs(&path)?;
        } else {
            bail!("unexpected file appeared during uninstall; preserved it");
        }
    }
    std::fs::remove_dir(dir)?;
    Ok(())
}
