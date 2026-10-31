//! A bounded, checksummed four-slot recent-log journal. Absolute offsets never
//! wrap. Invalidated slots are gaps after interrupted replacement, not old data.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
const MAGIC: &[u8; 8] = b"AJRLOG01";
pub(crate) const SEGMENT: usize = 1024 * 1024;
const SLOTS: usize = 4;
const HEADER: usize = 64;
pub(crate) const FILE_CAP: u64 = (SLOTS * (SEGMENT + HEADER)) as u64;
pub(crate) const NAMES: [&str; 8] = [
    "stdout.log",
    "stderr.log",
    "stdout.log.gz",
    "stderr.log.gz",
    "stdout.recent",
    "stderr.recent",
    "stdout.recent.gz",
    "stderr.recent.gz",
];

pub(crate) struct Writer {
    path: PathBuf,
    file: Option<File>,
    buffer: Vec<u8>,
    start: u64,
}
impl Writer {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            file: None,
            buffer: Vec::with_capacity(SEGMENT),
            start: crate::runner::LOG_CAP,
        }
    }
    pub(crate) async fn output(&mut self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let n = bytes.len().min(SEGMENT - self.buffer.len());
            self.buffer.extend_from_slice(&bytes[..n]);
            bytes = &bytes[n..];
            if self.buffer.len() == SEGMENT {
                self.flush().await?;
            }
        }
        Ok(())
    }
    pub(crate) async fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let bytes = std::mem::replace(&mut self.buffer, Vec::with_capacity(SEGMENT));
        let start = self.start;
        self.start = self
            .start
            .checked_add(bytes.len() as u64)
            .context("log offset overflow")?;
        let path = self.path.clone();
        let previous = self.file.take();
        self.file = Some(
            tokio::task::spawn_blocking(move || -> Result<File> {
                let mut file = match previous {
                    Some(file) => file,
                    None => crate::platform::create_new_file(&path)?,
                };
                file.lock()?;
                let result = (|| -> Result<()> {
                    let slot = ((start - crate::runner::LOG_CAP) / SEGMENT as u64) % SLOTS as u64;
                    let offset = slot * (SEGMENT + HEADER) as u64;
                    file.seek(SeekFrom::Start(offset))?;
                    file.write_all(&[0; HEADER])?;
                    file.sync_all()?; // Fence the old offset before overwriting its bytes.
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    let mut header = [0u8; HEADER];
                    header[..8].copy_from_slice(MAGIC);
                    header[8..16].copy_from_slice(&start.to_le_bytes());
                    header[16..24].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
                    header[24..56].copy_from_slice(&Sha256::digest(&bytes));
                    file.seek(SeekFrom::Start(offset))?;
                    file.write_all(&header)?;
                    file.sync_all()?;
                    crate::platform::sync_directory(path.parent().context("log directory")?)?;
                    Ok(())
                })();
                file.unlock()?;
                result?;
                Ok(file)
            })
            .await??,
        );
        Ok(())
    }
}
fn segments(dir: &Path, stream: &str) -> Result<Vec<(u64, Vec<u8>)>> {
    let raw = dir.join(format!("{stream}.recent"));
    let gz = dir.join(format!("{stream}.recent.gz"));
    let mut bytes = Vec::new();
    if crate::archive::regular(&raw)?.is_some() {
        // Archive publication can remove the raw file after our metadata read.
        match crate::platform::read_file(&raw) {
            Ok(mut file) => {
                file.lock_shared()?;
                ensure!(
                    file.metadata()?.len() <= FILE_CAP,
                    "recent log exceeds capacity"
                );
                file.read_to_end(&mut bytes)?;
            }
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
            Err(e) => return Err(e),
        }
    }
    if bytes.is_empty() && crate::archive::regular(&gz)?.is_some() {
        flate2::read::GzDecoder::new(crate::platform::read_file(&gz)?)
            .take(FILE_CAP + 1)
            .read_to_end(&mut bytes)?;
    }
    ensure!(
        bytes.len() as u64 <= FILE_CAP,
        "recent log exceeds capacity"
    );
    let mut result = Vec::new();
    for slot in 0..SLOTS {
        let offset = slot * (SEGMENT + HEADER);
        if offset >= bytes.len() {
            break;
        }
        ensure!(offset + HEADER <= bytes.len(), "partial recent-log header");
        let h = &bytes[offset..offset + HEADER];
        if h.iter().all(|b| *b == 0) {
            continue;
        }
        ensure!(
            &h[..8] == MAGIC && h[56..].iter().all(|b| *b == 0),
            "invalid recent-log header"
        );
        let start = u64::from_le_bytes(h[8..16].try_into()?);
        let len = u64::from_le_bytes(h[16..24].try_into()?);
        ensure!(
            start >= crate::runner::LOG_CAP
                && (start - crate::runner::LOG_CAP).is_multiple_of(SEGMENT as u64)
                && ((start - crate::runner::LOG_CAP) / SEGMENT as u64) % SLOTS as u64
                    == slot as u64
                && len > 0
                && len <= SEGMENT as u64
                && start.checked_add(len).is_some(),
            "invalid recent-log range"
        );
        let end = offset + HEADER + len as usize;
        ensure!(end <= bytes.len(), "partial recent-log segment");
        let data = &bytes[offset + HEADER..end];
        ensure!(
            Sha256::digest(data)[..] == h[24..56],
            "recent-log checksum mismatch"
        );
        result.push((start, data.to_vec()));
    }
    result.sort_by_key(|s| s.0);
    Ok(result)
}
fn prefix_len(dir: &Path, stream: &str) -> Result<u64> {
    let raw = dir.join(format!("{stream}.log"));
    if let Some(meta) = crate::archive::regular(&raw)? {
        return Ok(meta.len());
    }
    let gz = dir.join(format!("{stream}.log.gz"));
    if let Some(meta) = crate::archive::regular(&gz)? {
        ensure!(
            meta.len() >= 18 && meta.len() <= crate::runner::LOG_CAP + 65536,
            "invalid compressed log length"
        );
        let mut file = crate::platform::read_file(&gz)?;
        file.seek(SeekFrom::End(-4))?;
        let mut size = [0; 4];
        file.read_exact(&mut size)?;
        return Ok(u32::from_le_bytes(size) as u64);
    }
    Ok(0)
}
pub(crate) async fn read(
    config: crate::config::Config,
    run: String,
    stream: String,
    cursor: u64,
    limit: u32,
) -> Result<Value> {
    crate::core::validate_id(&run)?;
    ensure!(
        ["stdout", "stderr"].contains(&stream.as_str()) && (1..=65536).contains(&limit),
        "invalid log stream/limit"
    );
    let dir = config.run_dir(&run);
    let name = stream.clone();
    let (prefix, segments) = tokio::task::spawn_blocking(move || -> Result<_> {
        if dir.exists() {
            crate::platform::secure_dir(&dir)?;
        }
        let len = prefix_len(&dir, &name)?;
        ensure!(len <= crate::runner::LOG_CAP, "prefix log exceeds capacity");
        Ok((len, segments(&dir, &name)?))
    })
    .await??;
    let mut ranges = Vec::new();
    if prefix > 0 {
        ranges.push((0, prefix));
    }
    ranges.extend(segments.iter().map(|(s, b)| (*s, *s + b.len() as u64)));
    let end = ranges.last().map_or(0, |r| r.1);
    ensure!(cursor <= end, "cursor is beyond end of retained log");
    let actual = ranges
        .iter()
        .find(|r| r.1 > cursor)
        .map_or(end, |r| cursor.max(r.0));
    let data = if actual < prefix {
        crate::archive::log_bytes(
            config,
            run,
            stream,
            actual,
            limit.min((prefix - actual) as u32),
        )
        .await?
    } else if let Some((start, bytes)) = segments
        .iter()
        .find(|(s, b)| actual >= *s && actual < *s + b.len() as u64)
    {
        let offset = (actual - start) as usize;
        bytes[offset..(offset + limit as usize).min(bytes.len())].to_vec()
    } else {
        vec![]
    };
    let mut gaps = Vec::new();
    let mut previous = 0;
    for (start, finish) in &ranges {
        if *start > previous {
            gaps.push(json!({"from":previous,"to":start}));
        }
        previous = *finish;
    }
    Ok(
        json!({"text":String::from_utf8_lossy(&data),"bytes":data.len(),"cursor":actual,"next_cursor":actual+data.len() as u64,
        "retained_ranges":ranges.iter().map(|(s,e)|json!({"from":s,"to":e})).collect::<Vec<_>>(),"gaps":gaps,
        "skipped":if actual>cursor {json!({"from":cursor,"to":actual})}else{Value::Null},
        "retained_end":end,"untrusted":true,"encoding":"utf8_lossy","cap_bytes":crate::runner::LOG_CAP,"recent_cap_bytes":SLOTS*SEGMENT}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn interrupted_slot_replacement_is_a_gap_and_corruption_is_never_returned() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::tempdir_in(root).unwrap();
        let path = temp.path().join("stdout.recent");
        let mut writer = Writer::new(path.clone());
        writer.output(&vec![b'a'; SEGMENT]).await.unwrap();
        writer.output(&vec![b'b'; SEGMENT]).await.unwrap();
        let mut file = crate::platform::private_file(&path).unwrap();
        file.write_all(&[0; HEADER]).unwrap();
        file.sync_all().unwrap();
        let retained = segments(temp.path(), "stdout").unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].0, crate::runner::LOG_CAP + SEGMENT as u64);
        assert_eq!(retained[0].1[0], b'b');
        file.seek(SeekFrom::Start((SEGMENT + 2 * HEADER) as u64))
            .unwrap();
        file.write_all(b"X").unwrap();
        file.sync_all().unwrap();
        assert!(
            segments(temp.path(), "stdout")
                .unwrap_err()
                .to_string()
                .contains("checksum")
        );
    }
}
