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

fn prefix_slice(dir: &Path, stream: &str, start: u64, len: usize) -> Result<Vec<u8>> {
    let raw = dir.join(format!("{stream}.log"));
    let file = match crate::archive::regular(&raw)? {
        Some(_) => match crate::platform::read_file(&raw) {
            Ok(file) => Some(file),
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                None
            }
            Err(e) => return Err(e),
        },
        None => None,
    };
    let mut reader: Box<dyn Read> = if let Some(mut file) = file {
        file.seek(SeekFrom::Start(start))?;
        Box::new(file)
    } else {
        let gz = dir.join(format!("{stream}.log.gz"));
        crate::archive::regular(&gz)?.context("retained prefix is unavailable")?;
        let mut decoder = flate2::read::GzDecoder::new(crate::platform::read_file(&gz)?)
            .take(crate::runner::LOG_CAP + 1);
        ensure!(
            std::io::copy(&mut decoder.by_ref().take(start), &mut std::io::sink())? == start,
            "retained prefix is truncated"
        );
        Box::new(decoder)
    };
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn retained_slice(
    dir: &Path,
    stream: &str,
    prefix: u64,
    segments: &[(u64, Vec<u8>)],
    from: u64,
    to: u64,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity((to - from) as usize);
    let mut cursor = from;
    if cursor < prefix {
        let end = to.min(prefix);
        bytes.extend(prefix_slice(dir, stream, cursor, (end - cursor) as usize)?);
        cursor = end;
    }
    for (start, data) in segments {
        let end = (*start + data.len() as u64).min(to);
        if cursor >= to {
            break;
        }
        if cursor >= *start && cursor < end {
            bytes.extend_from_slice(&data[(cursor - start) as usize..(end - start) as usize]);
            cursor = end;
        }
    }
    ensure!(cursor == to, "retained search range has a gap");
    Ok(bytes)
}

/// Literal byte matching on retained ranges; never match across a missing gap.
pub(crate) async fn search(
    config: crate::config::Config,
    run: String,
    stream: String,
    text: String,
    cursor: u64,
    end: Option<u64>,
    limit: u32,
) -> Result<Value> {
    crate::core::validate_id(&run)?;
    ensure!(
        ["stdout", "stderr"].contains(&stream.as_str()),
        "invalid log stream"
    );
    ensure!(
        !text.is_empty() && text.len() <= 1024 && (1..=100).contains(&limit),
        "log search needs 1..1024 text bytes and 1..100 matches"
    );
    tokio::task::spawn_blocking(move || {
        const BUDGET: u64 = 512 * 1024;
        const CONTEXT: u64 = 64;
        let dir = config.run_dir(&run);
        if dir.exists() { crate::platform::secure_dir(&dir)?; }
        let prefix = prefix_len(&dir, &stream)?;
        ensure!(prefix <= crate::runner::LOG_CAP, "prefix log exceeds capacity");
        let segments = segments(&dir, &stream)?;
        let mut ranges = Vec::new();
        if prefix > 0 { ranges.push((0, prefix)); }
        ranges.extend(segments.iter().map(|(s,b)| (*s, *s+b.len() as u64)));
        let mut contiguous: Vec<(u64,u64)> = Vec::new();
        for (start, end) in ranges {
            if let Some(last) = contiguous.last_mut() && last.1 == start { last.1 = end; }
            else { contiguous.push((start,end)); }
        }
        let ranges = contiguous;
        let retained_end = ranges.last().map_or(0, |r| r.1);
        let end = end.unwrap_or(retained_end);
        ensure!(cursor <= end && end <= retained_end, "search cursor/end is beyond retained output");
        let mut gaps = Vec::new();
        let mut previous = 0;
        for (start, finish) in &ranges {
            if *start >= end { break; }
            if *start > previous { gaps.push(json!({"from":previous,"to":(*start).min(end)})); }
            previous = (*finish).min(end);
        }
        if previous < end { gaps.push(json!({"from":previous,"to":end})); }
        let mut matches = Vec::new();
        let mut scanned = 0;
        let mut next = cursor;
        for (range_start, range_end) in &ranges {
            let start = cursor.max(*range_start);
            let finish = (*range_end).min(end);
            if start >= finish { continue; }
            if scanned >= BUDGET { break; }
            let scan_end = finish.min(start + BUDGET - scanned);
            let from = start.saturating_sub(CONTEXT).max(*range_start);
            let to = finish.min(scan_end + text.len() as u64 - 1 + CONTEXT);
            let data = retained_slice(&dir, &stream, prefix, &segments, from, to)?;
            let mut offset = (start-from) as usize;
            let mut last_end = start;
            while let Some(found) = memchr::memmem::find(&data[offset..], text.as_bytes()) {
                let at = offset + found;
                let absolute = from + at as u64;
                if absolute >= scan_end { break; }
                let context_start = at.saturating_sub(CONTEXT as usize);
                let context_end = (at + text.len() + CONTEXT as usize).min(data.len());
                matches.push(json!({"offset":absolute,"end":absolute+text.len() as u64,
                    "context_from":from+context_start as u64,"text":String::from_utf8_lossy(&data[context_start..context_end])}));
                offset = at + text.len();
                last_end = absolute + text.len() as u64;
                if matches.len() >= limit as usize { next = absolute + text.len() as u64; break; }
            }
            if matches.len() >= limit as usize { scanned += next.saturating_sub(start); break; }
            scanned += scan_end-start;
            next = scan_end.max(last_end);
        }
        // Exhausting retained ranges, including a final gap, completes this
        // fixed snapshot. The next request cannot wait for future output.
        if scanned < BUDGET && matches.len() < limit as usize { next = end; }
        Ok(json!({"matches":matches,"cursor":cursor,"next_cursor":next,"end":end,
            "complete":next>=end,"scanned_bytes":scanned,"scan_budget_bytes":BUDGET,
            "retained_ranges":ranges.iter().filter(|(s,_)|*s<end).map(|(s,e)|json!({"from":s,"to":(*e).min(end)})).collect::<Vec<_>>(),
            "gaps":gaps,"encoding":"utf8_lossy","untrusted":true,"match_policy":"case_sensitive_literal_nonoverlapping"}))
    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn search_spans_contiguous_segments_but_not_evicted_gaps_or_duplicate_pages() {
        crate::platform::initialize();
        let root =
            Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo test workspace"))
                .join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::tempdir_in(root).unwrap();
        let config = crate::config::Config::new(Some(temp.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let run = crate::core::id();
        let dir = config.run_dir(&run);
        crate::platform::secure_dir(&dir).unwrap();
        let mut prefix = vec![b'x'; crate::runner::LOG_CAP as usize];
        let len = prefix.len();
        prefix[len - 2..].copy_from_slice(b"NO");
        // Match runner creation: elevated Windows defaults can select a group owner.
        crate::platform::create_new_file(&dir.join("stdout.log"))
            .unwrap()
            .write_all(&prefix)
            .unwrap();
        let mut writer = Writer::new(dir.join("stdout.recent"));
        for index in 0..5 {
            let mut data = vec![b'x'; SEGMENT];
            if index == 1 {
                data[..2].copy_from_slice(b"PE");
            }
            if index == 3 {
                data[SEGMENT - 2..].copy_from_slice(b"ED");
            }
            if index == 4 {
                data[..2].copy_from_slice(b"GE");
            }
            writer.output(&data).await.unwrap();
        }
        let page = search(
            config.clone(),
            run.clone(),
            "stdout".into(),
            "EDGE".into(),
            crate::runner::LOG_CAP,
            None,
            20,
        )
        .await
        .unwrap();
        assert_eq!(
            page["gaps"],
            json!([{"from":crate::runner::LOG_CAP,"to":crate::runner::LOG_CAP+SEGMENT as u64}])
        );
        let mut cursor = page["next_cursor"].as_u64().unwrap();
        let end = page["end"].as_u64();
        let mut hits = Vec::new();
        while cursor < end.unwrap() {
            let page = search(
                config.clone(),
                run.clone(),
                "stdout".into(),
                "EDGE".into(),
                cursor,
                end,
                20,
            )
            .await
            .unwrap();
            cursor = page["next_cursor"].as_u64().unwrap();
            hits.extend(page["matches"].as_array().unwrap().clone());
        }
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0]["offset"],
            crate::runner::LOG_CAP + 4 * SEGMENT as u64 - 2
        );
        let absent = search(
            config,
            run,
            "stdout".into(),
            "NOPE".into(),
            crate::runner::LOG_CAP - 2,
            None,
            20,
        )
        .await
        .unwrap();
        assert!(absent["matches"].as_array().unwrap().is_empty());
    }
    #[tokio::test]
    async fn interrupted_slot_replacement_is_a_gap_and_corruption_is_never_returned() {
        let root =
            Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo test workspace"))
                .join(".tmp");
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
