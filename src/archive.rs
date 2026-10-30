//! Compressed cold records remain in SQLite's transaction domain. Log files are
//! durably compressed before pruning; a crash can leave duplicates, never gaps.
use crate::{core::*, storage::Store};
use anyhow::{Context, Result, ensure};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use rusqlite::{Connection, OptionalExtension, params, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, Write},
    path::Path,
};

const SNAPSHOT_LIMIT: u64 = 32 * 1024 * 1024;
const TABLES: [&str; 11] = [
    "jobs",
    "runs",
    "events",
    "targets",
    "deliveries",
    "delivery_attempts",
    "target_recoveries",
    "monitor_rules",
    "monitor_changes",
    "runtime_observations",
    "stop_requests",
];
type Record = BTreeMap<String, Value>;
#[derive(Serialize, Deserialize)]
pub(crate) struct Snapshot {
    version: u32,
    pub job: Job,
    pub spec: String,
    tables: BTreeMap<String, Vec<Record>>,
}
impl Snapshot {
    /// Decode every detail section from this one validated cold snapshot.
    pub(crate) fn inspection(&self) -> Result<Value> {
        let raw = self.tables["runtime_observations"]
            .first()
            .and_then(|row| row.get("data"))
            .and_then(Value::as_str);
        let observation = raw.map(serde_json::from_str::<Value>).transpose()?;
        let rules = self.tables["monitor_rules"]
            .iter()
            .map(|row| crate::monitor::decode_rule(serde_json::to_value(row)?))
            .collect::<Result<Vec<_>>>()?;
        let events = self.tables["events"]
            .iter()
            .rev()
            .take(30)
            .map(event)
            .collect::<Result<Vec<_>>>()?;
        let deliveries = self.tables["deliveries"]
            .iter()
            .take(20)
            .map(delivery)
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({
            "job": self.job,
            "observation": observation,
            "rules": rules,
            "stop": self.tables["stop_requests"].first(),
            "events": events,
            "deliveries": {
                "next_cursor": deliveries.last().map_or(0, |d| d.sequence),
                "deliveries": deliveries,
            },
        }))
    }
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn pack(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() as u64 <= SNAPSHOT_LIMIT,
        "archive metadata exceeds 32 MiB"
    );
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?)
}
pub(crate) fn load(db: &Connection, job: &str) -> Result<Option<Snapshot>> {
    let data: Option<Vec<u8>> = db
        .query_row("SELECT snapshot FROM archive_jobs WHERE id=?", [job], |r| {
            r.get(0)
        })
        .optional()?;
    data.map(|data| {
        let mut bytes = Vec::new();
        GzDecoder::new(&data[..])
            .take(SNAPSHOT_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= SNAPSHOT_LIMIT,
            "archive metadata exceeds 32 MiB"
        );
        let mut snapshot: Snapshot = serde_json::from_slice(&bytes)?;
        for table in [
            "monitor_rules",
            "monitor_changes",
            "runtime_observations",
            "stop_requests",
        ] {
            snapshot.tables.entry(table.into()).or_default();
        }
        ensure!(
            snapshot.version == 1
                && snapshot.job.job_id == job
                && snapshot.job.status.is_terminal(),
            "invalid archive identity/version/state"
        );
        ensure!(
            snapshot.tables.len() == TABLES.len()
                && TABLES.iter().all(|t| snapshot.tables.contains_key(*t)),
            "archive tables are incomplete"
        );
        for table in TABLES {
            let mut info = db.prepare(&format!("PRAGMA table_info({table})"))?;
            let mut columns = vec!["_sequence".to_string()];
            columns.extend(
                info.query_map([], |r| r.get::<_, String>(1))?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            );
            ensure!(
                snapshot.tables[table]
                    .iter()
                    .all(|row| row.len() == columns.len()
                        && columns.iter().all(|c| row.contains_key(c))),
                "archive columns are incomplete"
            );
        }
        ensure!(
            snapshot.tables["jobs"].len() == 1 && snapshot.tables["jobs"][0]["id"] == job,
            "archive job row mismatch"
        );
        Ok(snapshot)
    })
    .transpose()
}
pub(crate) fn rows(db: &Connection, table: &str, filter: &str, job: &str) -> Result<Vec<Record>> {
    ensure!(TABLES.contains(&table), "unknown archive table");
    let mut q = db.prepare(&format!(
        "SELECT rowid AS _sequence,* FROM {table} WHERE {filter} ORDER BY rowid"
    ))?;
    let columns = q.column_names().map(|s| s.to_string()).collect::<Vec<_>>();
    let records = q
        .query_map([job], |r| {
            columns
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let value = match r.get_ref(i)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(n) => json!(n),
                        ValueRef::Real(n) => json!(n),
                        ValueRef::Text(s) => json!(std::str::from_utf8(s).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                i,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?),
                        ValueRef::Blob(_) => {
                            return Err(rusqlite::Error::InvalidColumnType(
                                i,
                                name.clone(),
                                rusqlite::types::Type::Blob,
                            ));
                        }
                    };
                    Ok((name.clone(), value))
                })
                .collect::<rusqlite::Result<Record>>()
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(records)
}
pub(crate) fn snapshot(db: &Connection, job: &str) -> Result<Snapshot> {
    let job = crate::storage::read_job(db, job)?;
    ensure!(
        job.status.is_terminal(),
        "only terminal jobs can be archived"
    );
    let busy: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM deliveries d JOIN targets t ON t.id=d.target_id WHERE t.job_id=? AND (d.status IN ('pending','retry_wait','leased') OR (d.status='dead_letter' AND d.expires>?)))", params![job.job_id, now()], |r| r.get(0))?;
    ensure!(!busy, "job still has recoverable or unresolved deliveries");
    let spec = db.query_row("SELECT spec FROM jobs WHERE id=?", [&job.job_id], |r| {
        r.get(0)
    })?;
    let mut tables = BTreeMap::new();
    for (table, filter) in [
        ("jobs", "id=?"),
        ("runs", "job_id=?"),
        ("monitor_rules", "job_id=?"),
        ("monitor_changes", "job_id=?"),
        ("runtime_observations", "job_id=?"),
        ("stop_requests", "job_id=?"),
        ("events", "job_id=?"),
        ("targets", "job_id=?"),
        (
            "deliveries",
            "target_id IN (SELECT id FROM targets WHERE job_id=?)",
        ),
        (
            "delivery_attempts",
            "delivery_id IN (SELECT d.id FROM deliveries d JOIN targets t ON t.id=d.target_id WHERE t.job_id=?)",
        ),
        (
            "target_recoveries",
            "from_target IN (SELECT id FROM targets WHERE job_id=?)",
        ),
    ] {
        tables.insert(table.to_string(), rows(db, table, filter, &job.job_id)?);
    }
    Ok(Snapshot {
        version: 1,
        job,
        spec,
        tables,
    })
}
pub(crate) fn delivery(record: &Record) -> Result<crate::delivery::Delivery> {
    Ok(serde_json::from_value(json!({
        "sequence":record["_sequence"], "id":record["id"], "event_id":record["event_id"],
        "target_id":record["target_id"], "status":record["status"], "attempts":record["attempts"],
        "expires_at":record["expires"], "last_error":record["last_error"], "failure_kind":record["failure_kind"], "next_attempt_at":record["next_at"]
    }))?)
}
pub(crate) fn archived_delivery(
    db: &Connection,
    id: &str,
) -> Result<Option<crate::delivery::Delivery>> {
    let job: Option<String> = db
        .query_row(
            "SELECT job_id FROM archive_deliveries WHERE id=?",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(job) = job {
        let snapshot = load(db, &job)?.context("archive catalog has no snapshot")?;
        return snapshot.tables["deliveries"]
            .iter()
            .find(|r| r["id"] == id)
            .map(delivery)
            .transpose();
    }
    Ok(None)
}
pub(crate) fn events(snapshot: &Snapshot, after: i64, limit: u32) -> Result<Vec<Event>> {
    snapshot.tables["events"]
        .iter()
        .filter(|r| r["seq"].as_i64().unwrap_or(0) > after)
        .take(limit as usize)
        .map(event)
        .collect()
}
fn event(row: &Record) -> Result<Event> {
    Ok(Event {
        seq: row["seq"].as_i64().context("event sequence")?,
        event_id: text(row, "id")?,
        job_id: text(row, "job_id")?,
        run_id: text(row, "run_id")?,
        kind: text(row, "kind")?,
        at: row["at"].as_i64().context("event time")?,
        payload: serde_json::from_str(&text(row, "payload")?)?,
    })
}
pub(crate) fn attempts(db: &Connection, id: &str, after: u32, limit: u32) -> Result<Option<Value>> {
    let job: Option<String> = db
        .query_row(
            "SELECT job_id FROM archive_deliveries WHERE id=?",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(job) = job {
        let snapshot = load(db, &job)?.context("archive catalog has no snapshot")?;
        let rows = snapshot.tables["delivery_attempts"].iter().filter(|r| r["delivery_id"] == id && r["attempt"].as_u64().unwrap_or(0) > after as u64).take(limit as usize).map(|r| json!({"attempt":r["attempt"],"started":r["started"],"ended":r["ended"],"outcome":r["outcome"],"error":r["error"]})).collect::<Vec<_>>();
        return Ok(Some(
            json!({"next_cursor":rows.last().map_or(after as u64, |r|r["attempt"].as_u64().unwrap_or(after as u64)),"attempts":rows}),
        ));
    }
    Ok(None)
}
fn text(r: &Record, key: &str) -> Result<String> {
    Ok(r.get(key)
        .and_then(Value::as_str)
        .context("invalid archive field")?
        .to_string())
}

/// Restore relational history only, never a runnable intent. Called inside the
/// same IMMEDIATE transaction as an explicit delivery retry/rebind.
pub(crate) fn restore(db: &Connection, job: &str) -> Result<()> {
    let Some(snapshot) = load(db, job)? else {
        return Ok(());
    };
    for table in TABLES {
        let mut info = db.prepare(&format!("PRAGMA table_info({table})"))?;
        let mut columns = vec!["_sequence".to_string()];
        columns.extend(
            info.query_map([], |r| r.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
        if !["jobs", "targets", "deliveries", "events"].contains(&table) {
            columns.remove(0);
        }
        for row in &snapshot.tables[table] {
            ensure!(
                row.len()
                    == columns.len() + usize::from(!columns.contains(&"_sequence".to_string()))
                    && columns.iter().all(|c| row.contains_key(c)),
                "archive columns do not match schema"
            );
            let names = columns
                .iter()
                .map(|c| {
                    if c == "_sequence" {
                        "rowid"
                    } else {
                        c.as_str()
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let values = columns
                .iter()
                .map(|c| match &row[c] {
                    Value::Null => Ok(rusqlite::types::Value::Null),
                    Value::Number(n) => n
                        .as_i64()
                        .map(rusqlite::types::Value::Integer)
                        .context("invalid archive integer"),
                    Value::String(s) => Ok(rusqlite::types::Value::Text(s.clone())),
                    _ => anyhow::bail!("invalid archive SQL value"),
                })
                .collect::<Result<Vec<_>>>()?;
            db.execute(
                &format!(
                    "INSERT INTO {table}({names}) VALUES({})",
                    vec!["?"; columns.len()].join(",")
                ),
                rusqlite::params_from_iter(values),
            )?;
        }
    }
    db.execute("DELETE FROM archive_deliveries WHERE job_id=?", [job])?;
    db.execute("DELETE FROM archive_targets WHERE job_id=?", [job])?;
    db.execute("DELETE FROM archive_jobs WHERE id=?", [job])?;
    Ok(())
}
pub(crate) fn restore_delivery(db: &Connection, id: &str) -> Result<()> {
    let job: Option<String> = db
        .query_row(
            "SELECT job_id FROM archive_deliveries WHERE id=?",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(job) = job {
        restore(db, &job)?;
    }
    Ok(())
}

fn commit(db: &mut Connection, snapshot: &Snapshot, bytes: &[u8], compressed: &[u8]) -> Result<()> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    ensure!(
        serde_json::to_vec(&self::snapshot(&tx, &snapshot.job.job_id)?)? == bytes,
        "job changed during archive preparation; retry later"
    );
    let spec: StartSpec = serde_json::from_str(&snapshot.spec)?;
    tx.execute("INSERT INTO archive_jobs(sequence,id,run_id,idem,spec_hash,snapshot,archived_at) VALUES(?,?,?,?,?,?,?)", params![snapshot.job.sequence,snapshot.job.job_id,snapshot.job.run_id,spec.idempotency_key,hash(snapshot.spec.as_bytes()),compressed,now()])?;
    tx.execute(
        "INSERT INTO archive_targets SELECT id,job_id FROM targets WHERE job_id=?",
        [&snapshot.job.job_id],
    )?;
    tx.execute("INSERT INTO archive_deliveries SELECT d.rowid,d.id,t.job_id,t.session_key,e.id,e.kind,d.status FROM deliveries d JOIN targets t ON t.id=d.target_id JOIN events e ON e.id=d.event_id WHERE t.job_id=?", [&snapshot.job.job_id])?;
    for (table, filter) in [
        (
            "target_recoveries",
            "from_target IN (SELECT id FROM targets WHERE job_id=?)",
        ),
        (
            "delivery_attempts",
            "delivery_id IN (SELECT d.id FROM deliveries d JOIN targets t ON t.id=d.target_id WHERE t.job_id=?)",
        ),
        (
            "deliveries",
            "target_id IN (SELECT id FROM targets WHERE job_id=?)",
        ),
        ("targets", "job_id=?"),
        ("monitor_rules", "job_id=?"),
        ("monitor_changes", "job_id=?"),
        ("runtime_observations", "job_id=?"),
        ("stop_requests", "job_id=?"),
        ("events", "job_id=?"),
        ("runs", "job_id=?"),
        ("jobs", "id=?"),
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE {filter}"),
            [&snapshot.job.job_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}
pub(crate) fn regular(path: &Path) -> Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => {
            ensure!(
                m.is_file() && !crate::platform::is_link(&m),
                "maintenance requires an owned regular file: {}",
                path.display()
            );
            Ok(Some(m))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn compress_log(path: &Path) -> Result<()> {
    let Some(meta) = regular(path)? else {
        return Ok(());
    };
    ensure!(
        meta.len() <= crate::runner::LOG_CAP,
        "log exceeds archive limit"
    );
    let gz = path.with_extension("log.gz");
    let temporary = path.with_extension(format!("{}.tmp", id()));
    // Always replace a previous prepared copy from a failed/interrupted attempt.
    regular(&gz)?;
    let result = (|| -> Result<()> {
        let mut encoder = GzEncoder::new(
            crate::platform::create_new_file(&temporary)?,
            Compression::default(),
        );
        std::io::copy(
            &mut crate::platform::read_file(path)?.take(crate::runner::LOG_CAP + 1),
            &mut encoder,
        )?;
        encoder.finish()?.sync_all()?;
        crate::platform::publish_file(&temporary, &gz)?;
        crate::platform::sync_directory(path.parent().context("log directory")?)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
pub(crate) fn remove_raw_logs(config: &crate::config::Config, run: &str) -> Result<()> {
    crate::platform::secure_dir(&config.run_dir(run))?;
    for stream in ["stdout", "stderr"] {
        let path = config.run_dir(run).join(format!("{stream}.log"));
        if regular(&path)?.is_some() {
            let gz = path.with_extension("log.gz");
            regular(&gz)?.context("raw log has no durable compressed copy")?;
            let original = crate::platform::read_file(&path)?;
            let mut decoded =
                GzDecoder::new(crate::platform::read_file(&gz)?).take(crate::runner::LOG_CAP + 1);
            let mut original = original.take(crate::runner::LOG_CAP + 1);
            let mut a = [0; 8192];
            let mut b = [0; 8192];
            loop {
                let n = original.read(&mut a)?;
                decoded.read_exact(&mut b[..n])?;
                ensure!(
                    a[..n] == b[..n],
                    "prepared archive log differs from original"
                );
                if n == 0 {
                    ensure!(
                        decoded.read(&mut b[..1])? == 0,
                        "archive log length differs"
                    );
                    break;
                }
            }
            std::fs::remove_file(&path)?;
            crate::platform::sync_directory(path.parent().context("log directory")?)?;
        }
    }
    Ok(())
}
impl Store {
    pub(crate) async fn archive_job(&self, job: String) -> Result<()> {
        let config = self.config.clone();
        let key = job.clone();
        let snapshot = self
            .read(move |db| {
                let tx = db.transaction()?;
                snapshot(&tx, &key)
            })
            .await?;
        let run = snapshot.job.run_id.clone();
        let lock = config.run_lock(&run)?;
        lock.try_lock()
            .context("runner still owns terminal run; defer archiving")?;
        let prepared = tokio::task::spawn_blocking(move || -> Result<_> {
            for stream in ["stdout", "stderr"] {
                compress_log(&config.run_dir(&run).join(format!("{stream}.log")))?;
            }
            let bytes = serde_json::to_vec(&snapshot)?;
            let compressed = pack(&bytes)?;
            Ok((snapshot, bytes, compressed, lock))
        })
        .await??;
        let run = prepared.0.job.run_id.clone();
        let lock = self
            .call(move |db| {
                commit(db, &prepared.0, &prepared.1, &prepared.2)?;
                Ok(prepared.3)
            })
            .await?;
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            let _lock = lock;
            remove_raw_logs(&config, &run)
        })
        .await??;
        self.call(move |db| {
            db.execute(
                "UPDATE archive_jobs SET cleanup_pending=0 WHERE id=?",
                [job],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }
}

pub async fn logs(
    config: crate::config::Config,
    run: String,
    stream: String,
    cursor: u64,
    limit: u32,
) -> Result<Value> {
    let data = log_bytes(config, run, stream, cursor, limit).await?;
    Ok(
        json!({"text":String::from_utf8_lossy(&data),"bytes":data.len(),"next_cursor":cursor+data.len() as u64,"untrusted":true,"encoding":"utf8_lossy","cap_bytes":crate::runner::LOG_CAP}),
    )
}

/// Preserve byte cursors across remote reconnects, including invalid UTF-8.
pub(crate) async fn log_bytes(
    config: crate::config::Config,
    run: String,
    stream: String,
    cursor: u64,
    limit: u32,
) -> Result<Vec<u8>> {
    validate_id(&run)?;
    ensure!(
        ["stdout", "stderr"].contains(&stream.as_str()) && (1..=65536).contains(&limit),
        "invalid log stream/limit"
    );
    ensure!(
        cursor <= crate::runner::LOG_CAP,
        "cursor is beyond log capacity"
    );
    tokio::task::spawn_blocking(move || {
        let dir = config.run_dir(&run);
        if dir.exists() {
            crate::platform::secure_dir(&dir)?;
        }
        let raw = dir.join(format!("{stream}.log"));
        let compressed = dir.join(format!("{stream}.log.gz"));
        let raw_file = if regular(&raw)?.is_some() {
            match crate::platform::read_file(&raw) {
                Ok(file) => Some(file),
                Err(e)
                    if e.downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
                {
                    None
                }
                Err(e) => return Err(e),
            }
        } else {
            None
        };
        let reader: Box<dyn Read> = if let Some(mut file) = raw_file {
            ensure!(
                cursor <= file.metadata()?.len(),
                "cursor is beyond end of log"
            );
            file.seek(std::io::SeekFrom::Start(cursor))?;
            Box::new(file)
        } else if regular(&compressed)?.is_some() {
            let mut decoder = GzDecoder::new(crate::platform::read_file(&compressed)?)
                .take(crate::runner::LOG_CAP + 1);
            let skipped = std::io::copy(&mut decoder.by_ref().take(cursor), &mut std::io::sink())?;
            ensure!(skipped == cursor, "cursor is beyond end of log");
            Box::new(decoder)
        } else {
            ensure!(cursor == 0, "log not created yet; cursor must be zero");
            Box::new(std::io::empty())
        };
        let mut data = Vec::new();
        reader.take(limit as u64).read_to_end(&mut data)?;
        ensure!(
            cursor + data.len() as u64 <= crate::runner::LOG_CAP,
            "archive log exceeds capacity"
        );
        Ok(data)
    })
    .await?
}

pub(crate) fn monitor_rows(db: &Connection, job: &str, table: &str) -> Result<Option<Vec<Value>>> {
    Ok(load(db, job)?.map(|s| {
        s.tables[table]
            .iter()
            .map(|r| serde_json::to_value(r).expect("record"))
            .collect()
    }))
}
