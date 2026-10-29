//! Automatic, local data maintenance. Archives preserve history indefinitely;
//! only verified, owned rolling backups are removed automatically.
use crate::{archive, config::Config, core::*, storage::Store};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub hot_jobs: u32,
    pub archive_after_days: u32,
    pub hot_log_mib: u32,
    pub backup_hours: u32,
    pub backup_keep: u32,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            hot_jobs: 1000,
            archive_after_days: 30,
            hot_log_mib: 256,
            backup_hours: 24,
            backup_keep: 3,
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=9000).contains(&self.hot_jobs),
            "hot_jobs must be 1..9000"
        );
        ensure!(
            self.archive_after_days <= 36500,
            "archive_after_days must be 0..36500"
        );
        ensure!(
            (1..=1048576).contains(&self.hot_log_mib),
            "hot_log_mib must be 1..1048576"
        );
        ensure!(
            (1..=8760).contains(&self.backup_hours),
            "backup_hours must be 1..8760"
        );
        ensure!(
            (1..=30).contains(&self.backup_keep),
            "backup_keep must be 1..30"
        );
        Ok(())
    }
}
pub(crate) fn policy(db: &Connection) -> Result<Policy> {
    let raw: Option<String> = db
        .query_row(
            "SELECT value FROM maintenance_state WHERE key='policy'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let policy: Policy = raw
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    policy.validate()?;
    Ok(policy)
}
pub(crate) fn state(db: &Connection, key: &str, value: &str) -> Result<()> {
    db.execute("INSERT INTO maintenance_state(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value])?;
    Ok(())
}
pub(crate) fn capacity(db: &Connection) -> Result<Value> {
    let count = |sql: &str| -> Result<i64> { Ok(db.query_row(sql, [], |r| r.get(0))?) };
    let metrics: Option<String> = db
        .query_row(
            "SELECT value FROM maintenance_state WHERE key='metrics'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let error: Option<String> = db
        .query_row(
            "SELECT value FROM maintenance_state WHERE key='last_error'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let last_backup: Option<i64> =
        db.query_row("SELECT max(at) FROM maintenance_backups", [], |r| r.get(0))?;
    let backups = {
        let mut q = db.prepare(
            "SELECT id,at FROM maintenance_backups ORDER BY at DESC,rowid DESC LIMIT 30",
        )?;
        q.query_map([], |r| {
            Ok(json!({"id":r.get::<_,String>(0)?,"at":r.get::<_,i64>(1)?}))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let hot = count("SELECT count(*) FROM jobs")?;
    let sessions: i64 = db.query_row(
        "SELECT count(*) FROM sessions WHERE connected=1 AND expires>?",
        [now()],
        |r| r.get(0),
    )?;
    Ok(
        json!({"policy":policy(db)?,"hot_jobs":hot,"hot_job_limit":10000,"archived_jobs":count("SELECT count(*) FROM archive_jobs")?,
        "active_sessions":sessions,"active_session_limit":1000,"historical_sessions":count("SELECT count(*) FROM sessions")?,
        "database_bytes":count("PRAGMA page_count")?*count("PRAGMA page_size")?,"reusable_database_bytes":count("PRAGMA freelist_count")?*count("PRAGMA page_size")?,
        "last_backup_at":last_backup,"backups":backups,"last_error":error.filter(|s|!s.is_empty()),"last_scan":metrics.map(|s|serde_json::from_str::<Value>(&s)).transpose()?,
        "warning":if hot>=9000 {Some("hot history is near its hard limit; inspect maintenance errors and unresolved deliveries")} else if sessions>=900 {Some("active session registrations are near their limit")} else {None},
        "archive_retention":"indefinite; automatic archiving compresses data without deleting history"}),
    )
}
fn lock(config: &Config) -> Result<std::fs::File> {
    let lock = crate::platform::private_file(&config.root.join("maintenance.lock"))?;
    lock.try_lock()
        .context("data maintenance is already running for this profile")?;
    Ok(lock)
}
#[derive(Clone)]
struct Candidate {
    id: String,
    run: String,
    updated: i64,
    bytes: u64,
    terminal: bool,
    eligible: bool,
}
impl Store {
    pub async fn data_status(&self) -> Result<Value> {
        let mut value = self
            .read(|db| {
                let tx = db.transaction()?;
                capacity(&tx)
            })
            .await?;
        value["backup_directory"] = json!(self.config.root.join("backups"));
        Ok(value)
    }
    pub async fn data_policy(&self, update: Option<Policy>, apply: bool) -> Result<Value> {
        if let Some(p) = &update {
            p.validate()?;
        }
        self.call(move |db| {
            let p = update.unwrap_or(policy(db)?);
            if apply {
                state(db, "policy", &serde_json::to_string(&p)?)?;
            }
            Ok(json!({"policy":p,"applied":apply,"automatic":true,"reload_seconds":60}))
        })
        .await
    }
    pub async fn data_check(&self) -> Result<Value> {
        self.read(|db| {
            let integrity: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
            ensure!(
                integrity == "ok",
                "SQLite integrity check failed: {integrity}"
            );
            let foreign: bool = db.prepare("PRAGMA foreign_key_check")?.exists([])?;
            ensure!(!foreign, "SQLite foreign-key check failed");
            Ok(json!({"integrity":integrity,"foreign_keys":"ok"}))
        })
        .await
    }
    pub async fn maintain(&self, apply: bool) -> Result<Value> {
        let _lock = lock(&self.config)?;
        let p = self.read(|db| policy(db)).await?;
        let (mut candidates,mut hot)=self.read(|db|{
            let mut q=db.prepare("SELECT j.id,j.run_id,j.updated,j.status NOT IN ('queued','launching','running'),NOT EXISTS(SELECT 1 FROM deliveries d JOIN targets t ON t.id=d.target_id WHERE t.job_id=j.id AND (d.status IN ('pending','retry_wait','leased') OR (d.status='dead_letter' AND d.expires>?))) FROM jobs j ORDER BY j.updated,j.rowid")?;
            let rows=q.query_map([now()],|r|Ok(Candidate{id:r.get(0)?,run:r.get(1)?,updated:r.get(2)?,bytes:0,terminal:r.get(3)?,eligible:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((rows,db.query_row("SELECT count(*) FROM jobs",[],|r|r.get::<_,u32>(0))?))
        }).await?;
        let config = self.config.clone();
        candidates = tokio::task::spawn_blocking(move || -> Result<_> {
            for c in &mut candidates {
                validate_id(&c.run)?;
                let dir = config.run_dir(&c.run);
                if dir.exists() {
                    crate::platform::secure_dir(&dir)?;
                }
                for stream in ["stdout", "stderr"] {
                    c.bytes += archive::regular(&dir.join(format!("{stream}.log")))?
                        .map_or(0, |m| m.len());
                }
            }
            Ok(candidates)
        })
        .await??;
        let mut bytes = candidates.iter().map(|c| c.bytes).sum::<u64>();
        let before_bytes = bytes;
        let cutoff = now() - p.archive_after_days as i64 * 86400000;
        let mut archived = 0;
        let mut deferred = 0;
        for c in candidates {
            if !c.terminal {
                continue;
            }
            if self
                .maintenance_stop
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            if archived >= 32 {
                break;
            }
            if c.updated >= cutoff
                && hot <= p.hot_jobs
                && bytes <= p.hot_log_mib as u64 * 1024 * 1024
            {
                continue;
            }
            if !c.eligible {
                deferred += 1;
                continue;
            }
            if apply {
                match self.archive_job(c.id).await {
                    Ok(()) => {}
                    Err(e) => {
                        deferred += 1;
                        tracing::warn!(error=%e,"archive deferred");
                        continue;
                    }
                }
            }
            archived += 1;
            hot = hot.saturating_sub(1);
            bytes = bytes.saturating_sub(c.bytes);
        }
        if apply {
            // Finish raw-file cleanup after a crash between SQL commit and unlink.
            let config = self.config.clone();
            let runs = self
                .read(|db| {
                    let mut q = db.prepare(
                        "SELECT id,run_id FROM archive_jobs WHERE cleanup_pending=1 ORDER BY sequence LIMIT 32",
                    )?;
                    Ok(q.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?
                        .collect::<rusqlite::Result<Vec<_>>>()?)
                })
                .await?;
            let completed = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
                let mut completed = Vec::new();
                for (job, run) in runs {
                    validate_id(&run)?;
                    archive::remove_raw_logs(&config, &run)?;
                    completed.push(job);
                }
                Ok(completed)
            })
            .await??;
            self.call(move |db| {
                for job in completed {
                    db.execute(
                        "UPDATE archive_jobs SET cleanup_pending=0 WHERE id=?",
                        [job],
                    )?;
                }
                Ok(())
            })
            .await?;
        }
        let root = self.config.root.clone();
        let disk =
            tokio::task::spawn_blocking(move || match crate::platform::available_space(&root) {
                Ok(bytes) => json!({"available_bytes":bytes}),
                Err(e) => json!({"error":e.to_string()}),
            })
            .await?;
        let metrics = json!({"disk":disk,"at":now(),"archived":archived,"deferred":deferred,"hot_log_bytes_before":before_bytes,"hot_log_bytes_after_estimate":bytes});
        if apply {
            let raw = serde_json::to_string(&metrics)?;
            self.call(move|db|{state(db,"metrics",&raw)?;state(db,"last_error",if deferred>0 {"some jobs could not be archived; unresolved deliveries/runner ownership or storage errors may defer maintenance"}else{""})}).await?;
            let due = self
                .read(move |db| {
                    let last: Option<i64> =
                        db.query_row("SELECT max(at) FROM maintenance_backups", [], |r| r.get(0))?;
                    let jobs: bool = db.query_row(
                        "SELECT EXISTS(SELECT 1 FROM jobs) OR EXISTS(SELECT 1 FROM archive_jobs)",
                        [],
                        |r| r.get(0),
                    )?;
                    let pending: bool = db.query_row(
                        "SELECT EXISTS(SELECT 1 FROM maintenance_state WHERE key='pending_backup')",
                        [],
                        |r| r.get(0),
                    )?;
                    Ok(pending
                        || (jobs
                            && last.is_none_or(|t| now() - t >= p.backup_hours as i64 * 3600000)))
                })
                .await?;
            if due {
                self.backup_locked().await?;
            }
            self.rotate_backups(p.backup_keep).await?;
        }
        Ok(json!({"apply":apply,"archive":metrics,"capacity":self.data_status().await?}))
    }
    pub async fn backup(&self) -> Result<Value> {
        let _lock = lock(&self.config)?;
        self.backup_locked().await
    }
    async fn backup_locked(&self) -> Result<Value> {
        let pending = self
            .read(|db| {
                let saved: Option<String> = db
                    .query_row(
                        "SELECT value FROM maintenance_state WHERE key='pending_backup'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()?;
                Ok(saved
                    .map(|s| serde_json::from_str::<PendingBackup>(&s))
                    .transpose()?)
            })
            .await?;
        let config = self.config.clone();
        let recovered = if let Some(pending) = pending {
            let config = config.clone();
            tokio::task::spawn_blocking(move || recover_backup(&config, &pending)).await??
        } else {
            None
        };
        let result = if let Some(result) = recovered {
            result
        } else {
            let pending = self
                .call(|db| {
                    let mut q = db
                        .prepare("SELECT run_id FROM jobs UNION SELECT run_id FROM archive_jobs")?;
                    let runs = q
                        .query_map([], |r| r.get::<_, String>(0))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    drop(q);
                    let pending = PendingBackup { id: id(), runs };
                    state(db, "pending_backup", &serde_json::to_string(&pending)?)?;
                    Ok(pending)
                })
                .await?;
            let stopped = self.maintenance_stop.clone();
            self.read(move |db| create_backup(db, &config, &stopped, &pending.id))
                .await?
        };
        let id = result.0.clone();
        let digest = result.1.clone();
        let at = now();
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO maintenance_backups(id,at,manifest_hash) VALUES(?,?,?)",
                params![id, at, digest],
            )?;
            tx.execute(
                "DELETE FROM maintenance_state WHERE key='pending_backup'",
                [],
            )?;
            tx.commit()?;
            Ok(())
        })
        .await?;
        Ok(
            json!({"backup_id":result.0,"path":self.config.root.join("backups").join(&result.0),"active_logs":"bounded prefixes; restored active jobs never replay"}),
        )
    }
    async fn rotate_backups(&self, keep: u32) -> Result<()> {
        let old=self.read(move|db|{
            let mut q=db.prepare("SELECT id,manifest_hash FROM maintenance_backups ORDER BY at DESC,rowid DESC LIMIT -1 OFFSET ?")?;
            Ok(q.query_map([keep],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await?;
        for (id, digest) in old {
            validate_id(&id)?;
            let path = self.config.root.join("backups").join(&id);
            tokio::task::spawn_blocking(move || -> Result<()> {
                if path.exists() {
                    validate_backup(&path, Some(&digest))?;
                    std::fs::remove_dir_all(&path)?;
                    crate::platform::sync_directory(path.parent().context("backup parent")?)?;
                }
                Ok(())
            })
            .await??;
            self.call(move |db| {
                db.execute("DELETE FROM maintenance_backups WHERE id=?", [id])?;
                Ok(())
            })
            .await?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    bytes: u64,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    id: String,
    at: i64,
    files: BTreeMap<String, FileRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingBackup {
    id: String,
    runs: Vec<String>,
}
/// Only the explicitly recorded staging directory is disposable. Sealed bytes
/// finish forward unchanged; foreign additions fence automatic recovery.
fn recover_backup(config: &Config, pending: &PendingBackup) -> Result<Option<(String, String)>> {
    validate_id(&pending.id)?;
    for run in &pending.runs {
        validate_id(run)?;
    }
    let parent = config.root.join("backups");
    if !parent.exists() {
        return Ok(None);
    }
    crate::platform::secure_dir(&parent)?;
    let destination = parent.join(&pending.id);
    let stage = parent.join(format!(".{}.partial", pending.id));
    let sealed = if std::fs::symlink_metadata(&destination).is_ok() {
        Some(destination.clone())
    } else if std::fs::symlink_metadata(stage.join("manifest.json")).is_ok() {
        Some(stage.clone())
    } else {
        None
    };
    if let Some(path) = sealed {
        let manifest = validate_backup(&path, None)?;
        ensure!(manifest.id == pending.id, "pending backup identity changed");
        let digest = archive::hash(&std::fs::read(path.join("manifest.json"))?);
        if path == stage {
            crate::platform::publish_new_directory(&stage, &destination)?;
            crate::platform::sync_directory(&parent)?;
        }
        return Ok(Some((pending.id.clone(), digest)));
    }
    if std::fs::symlink_metadata(&stage).is_ok() {
        let mut actual = BTreeSet::new();
        inventory(&stage, &stage, &mut actual)?;
        let mut allowed = [
            "relay.sqlite3",
            "relay.sqlite3-journal",
            "relay.sqlite3-wal",
            "relay.sqlite3-shm",
            "runs/",
        ]
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
        for run in &pending.runs {
            allowed.insert(format!("runs/{run}/"));
            for name in ["stdout.log", "stderr.log", "stdout.log.gz", "stderr.log.gz"] {
                allowed.insert(format!("runs/{run}/{name}"));
            }
        }
        ensure!(
            actual.is_subset(&allowed),
            "incomplete backup contains foreign files; automatic restart refused"
        );
        std::fs::remove_dir_all(&stage)?;
        crate::platform::sync_directory(&parent)?;
    }
    Ok(None)
}
fn fingerprint(path: &Path) -> Result<FileRecord> {
    archive::regular(path)?.context("missing backup file")?;
    let mut file = crate::platform::read_file(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    let mut bytes = 0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        bytes += n as u64;
    }
    Ok(FileRecord {
        bytes,
        sha256: format!("{:x}", hash.finalize()),
    })
}
fn copy_file(source: &Path, destination: &Path, limit: u64) -> Result<()> {
    archive::regular(source)?.context("missing source file")?;
    let mut output = crate::platform::create_new_file(destination)?;
    std::io::copy(
        &mut crate::platform::read_file(source)?.take(limit),
        &mut output,
    )?;
    output.sync_all()?;
    Ok(())
}
fn create_backup(
    db: &mut Connection,
    config: &Config,
    stopped: &std::sync::atomic::AtomicBool,
    backup_id: &str,
) -> Result<(String, String)> {
    let parent = config.root.join("backups");
    crate::platform::secure_dir(&parent)?;
    let id = backup_id.to_string();
    let stage = parent.join(format!(".{id}.partial"));
    let destination = parent.join(&id);
    ensure!(
        !stage.exists() && !destination.exists(),
        "backup destination exists"
    );
    crate::platform::secure_dir(&stage)?;
    let tx = db.transaction()?;
    let mut q = tx.prepare("SELECT run_id FROM jobs UNION SELECT run_id FROM archive_jobs")?;
    let runs = q
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(q);
    // Record the exact snapshot's paths before creating any of them. A second
    // WAL connection can write this journal while the source read stays pinned.
    let mut journal = Connection::open(config.root.join("relay.sqlite3"))?;
    journal.busy_timeout(Duration::from_secs(5))?;
    journal.execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")?;
    let journal_tx = journal.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let saved: String = journal_tx.query_row(
        "SELECT value FROM maintenance_state WHERE key='pending_backup'",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        serde_json::from_str::<PendingBackup>(&saved)?.id == id,
        "backup claim changed"
    );
    state(
        &journal_tx,
        "pending_backup",
        &serde_json::to_string(&PendingBackup {
            id: id.clone(),
            runs: runs.clone(),
        })?,
    )?;
    journal_tx.commit()?;
    drop(journal);
    let database = stage.join("relay.sqlite3");
    drop(crate::platform::create_new_file(&database)?);
    let mut output = Connection::open(&database)?;
    {
        let backup = rusqlite::backup::Backup::new(&tx, &mut output)?;
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            ensure!(
                !stopped.load(std::sync::atomic::Ordering::Acquire),
                "backup interrupted by daemon shutdown; previous snapshots preserved"
            );
            match backup.step(128)? {
                rusqlite::backup::StepResult::Done => break,
                rusqlite::backup::StepResult::More => {}
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "SQLite backup exceeded 30 seconds"
            );
        }
    }
    output.execute_batch("PRAGMA journal_mode=DELETE;")?;
    drop(output);
    crate::platform::sync_file(&database)?;
    let mut files = BTreeMap::new();
    files.insert("relay.sqlite3".into(), fingerprint(&database)?);
    crate::platform::secure_dir(&stage.join("runs"))?;
    for run in runs {
        ensure!(
            !stopped.load(std::sync::atomic::Ordering::Acquire),
            "backup interrupted by daemon shutdown; previous snapshots preserved"
        );
        validate_id(&run)?;
        let source = config.run_dir(&run);
        if !source.exists() {
            continue;
        }
        crate::platform::secure_dir(&source)?;
        let mut has_logs = false;
        for stream in ["stdout", "stderr"] {
            // A readable raw prefix wins if compression was prepared but not committed.
            let name = if archive::regular(&source.join(format!("{stream}.log")))?.is_some() {
                format!("{stream}.log")
            } else {
                format!("{stream}.log.gz")
            };
            let path = source.join(&name);
            if let Some(meta) = archive::regular(&path)? {
                ensure!(
                    meta.len() <= crate::runner::LOG_CAP + 65536,
                    "backup log exceeds capacity"
                );
                crate::platform::secure_dir(&stage.join("runs").join(&run))?;
                has_logs = true;
                let relative = format!("runs/{run}/{name}");
                let target = stage.join(&relative);
                copy_file(&path, &target, meta.len())?;
                files.insert(relative, fingerprint(&target)?);
            }
        }
        if has_logs {
            crate::platform::sync_directory(&stage.join("runs").join(&run))?;
        }
    }
    tx.commit()?;
    let manifest = serde_json::to_vec(&Manifest {
        format: 1,
        id: id.clone(),
        at: now(),
        files,
    })?;
    let mut f = crate::platform::create_new_file(&stage.join("manifest.json"))?;
    f.write_all(&manifest)?;
    f.sync_all()?;
    drop(f);
    crate::platform::sync_directory(&stage.join("runs"))?;
    crate::platform::sync_directory(&stage)?;
    crate::platform::publish_new_directory(&stage, &destination)?;
    crate::platform::sync_directory(&parent)?;
    Ok((id, archive::hash(&manifest)))
}
fn valid_relative(name: &str) -> bool {
    if name == "relay.sqlite3" {
        return true;
    }
    let parts = name.split('/').collect::<Vec<_>>();
    parts.len() == 3
        && parts[0] == "runs"
        && validate_id(parts[1]).is_ok()
        && ["stdout.log", "stderr.log", "stdout.log.gz", "stderr.log.gz"].contains(&parts[2])
}
fn inventory(path: &Path, root: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    crate::platform::secure_dir(path)?;
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        let meta = std::fs::symlink_metadata(&path)?;
        ensure!(!crate::platform::is_link(&meta), "backup contains a link");
        if meta.is_dir() {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            let parts = relative.split('/').collect::<Vec<_>>();
            ensure!(
                relative == "runs"
                    || (parts.len() == 2 && parts[0] == "runs" && validate_id(parts[1]).is_ok()),
                "backup contains a foreign directory"
            );
            files.insert(format!("{relative}/"));
            inventory(&path, root, files)?;
        } else {
            ensure!(meta.is_file(), "backup contains a special file");
            files.insert(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
fn validate_backup(path: &Path, expected: Option<&str>) -> Result<Manifest> {
    crate::platform::secure_dir(path)?;
    let manifest_path = path.join("manifest.json");
    let meta = archive::regular(&manifest_path)?.context("backup manifest missing")?;
    ensure!(
        meta.len() <= 32 * 1024 * 1024,
        "backup manifest exceeds 32 MiB"
    );
    let bytes = std::fs::read(manifest_path)?;
    if let Some(expected) = expected {
        ensure!(
            archive::hash(&bytes) == expected,
            "backup manifest changed; automatic rotation refused"
        );
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    ensure!(
        manifest.format == 1
            && validate_id(&manifest.id).is_ok()
            && manifest.files.contains_key("relay.sqlite3"),
        "invalid backup manifest"
    );
    let mut actual = BTreeSet::new();
    inventory(path, path, &mut actual)?;
    let mut expected_files = manifest.files.keys().cloned().collect::<BTreeSet<_>>();
    expected_files.insert("manifest.json".into());
    expected_files.insert("runs/".into());
    for relative in manifest.files.keys() {
        ensure!(valid_relative(relative), "invalid backup path");
        if let Some((parent, _)) = relative.rsplit_once('/') {
            expected_files.insert(format!("{parent}/"));
        }
    }
    ensure!(
        actual == expected_files,
        "backup contains missing or foreign files"
    );
    for (relative, record) in &manifest.files {
        ensure!(valid_relative(relative), "invalid backup path");
        let actual = fingerprint(&path.join(relative))?;
        ensure!(
            actual.bytes == record.bytes && actual.sha256 == record.sha256,
            "backup checksum mismatch: {relative}"
        );
    }
    Ok(manifest)
}
/// Restore to a new profile only. No socket, lock or process identity is copied.
pub async fn restore(from: PathBuf, to: PathBuf) -> Result<Value> {
    tokio::task::spawn_blocking(move||->Result<Value>{
        ensure!(from.is_absolute()&&to.is_absolute(),"backup and destination paths must be absolute");
        ensure!(!to.exists()&&std::fs::symlink_metadata(&to).is_err(),"restore requires a new destination; existing state is never overwritten");
        let manifest=validate_backup(&from,None)?;
        let parent=to.parent().context("restore parent")?;ensure!(parent.is_dir(),"restore parent must exist");
        let stage=parent.join(format!(".ajr-restore-{}",id()));crate::platform::secure_dir(&stage)?;
        crate::platform::secure_dir(&stage.join("runs"))?;
        for (relative,record) in &manifest.files {
            let target=stage.join(relative);
            if relative!="relay.sqlite3" {crate::platform::secure_dir(target.parent().context("restore run directory")?)?;}
            copy_file(&from.join(relative),&target,record.bytes)?;
            if relative!="relay.sqlite3" {crate::platform::sync_directory(target.parent().context("restore run directory")?)?;}
            let actual=fingerprint(&target)?;
            ensure!(actual.sha256==record.sha256&&actual.bytes==record.bytes,"backup changed while restoring");
        }
        let mut db=Connection::open(stage.join("relay.sqlite3"))?;
        let schema:i64=db.pragma_query_value(None,"user_version",|r|r.get(0))?;ensure!(matches!(schema,8..=11),"backup schema is not supported by this executable");
        let integrity:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;ensure!(integrity=="ok","backup database integrity failed");
        if schema==8 {db.pragma_update(None,"foreign_keys",false)?;db.execute_batch(include_str!("monitor.sql"))?;}
        if schema<10 {db.execute_batch(include_str!("goal_wait.sql"))?;}
        if schema<11 {db.execute_batch(include_str!("goal_consent.sql"))?;}
        db.execute_batch("PRAGMA foreign_keys=ON;")?;
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut q=tx.prepare("SELECT id FROM jobs WHERE status IN ('queued','launching','running')")?;
        let active=q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;drop(q);
        for job in &active {crate::storage::finish_tx(&tx,job,"orphaned",None,None,"restored backup; execution was not replayed")?;}
        let t=now();
        tx.execute("UPDATE sessions SET connected=0",[])?;
        crate::goal_wait::restore_fence(&tx)?;
        crate::goal_consent::restore_fence(&tx)?;
        tx.execute("UPDATE delivery_attempts SET ended=?,outcome='restore_fenced',error='backup restored; explicit recovery required' WHERE ended IS NULL",[t])?;
        tx.execute("UPDATE deliveries SET status=CASE WHEN expires>? THEN 'retry_wait' ELSE 'dead_letter' END,failure_kind=CASE WHEN expires>? THEN 'rebind_required' ELSE 'expired' END,next_at=expires,token=NULL,lease_until=NULL,last_error='backup restored; explicit recovery required' WHERE status IN ('pending','leased','retry_wait')",params![t,t])?;
        tx.execute("UPDATE deliveries SET status='cancelled',failure_kind='goal_wait',last_error='backup restored; goal control and notification were not replayed' WHERE status IN ('pending','retry_wait','dead_letter') AND target_id IN (SELECT t.id FROM targets t JOIN goal_wait_jobs m ON m.job_id=t.job_id)",[])?;
        tx.execute("DELETE FROM maintenance_backups",[])?;tx.execute("DELETE FROM maintenance_state WHERE key='pending_backup'",[])?;state(&tx,"last_error","")?;
        ensure!(!tx.prepare("PRAGMA foreign_key_check")?.exists([])?,"backup foreign-key check failed");
        tx.commit()?;db.execute_batch("PRAGMA journal_mode=DELETE;")?;drop(db);
        crate::platform::sync_file(&stage.join("relay.sqlite3"))?;
        crate::platform::sync_directory(&stage.join("runs"))?;
        crate::platform::sync_directory(&stage)?;
        crate::platform::publish_new_directory(&stage,&to)?;crate::platform::sync_directory(parent)?;
        Ok(json!({"restored_to":to,"backup_id":manifest.id,"orphaned_jobs":active.len(),"execution_replayed":false,"sessions":"invalidated; explicit current-caller recovery required"}))
    }).await?
}
pub(crate) async fn worker(store: Store, mut stopped: tokio::sync::watch::Receiver<bool>) {
    loop {
        if *stopped.borrow() {
            break;
        }
        let mut delay = 60;
        if let Err(e) = store.maintain(true).await.map(|result| {
            if result["archive"]["archived"] == 32 {
                delay = 1;
            }
        }) {
            tracing::error!(error=%e,"automatic data maintenance failed");
            let message = format!("{e:#}");
            let _ = store
                .call(move |db| state(db, "last_error", &message))
                .await;
        }
        tokio::select! {biased;_ = stopped.changed()=>break,_ = tokio::time::sleep(Duration::from_secs(delay))=>{}}
    }
}
