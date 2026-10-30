//! Bounded stdout/stderr progress frames. Data never supplies commands or targets.
//! State, milestones and outbox writes share the runner's SQLite transaction.
use crate::{
    core::*,
    monitor::{Action, Condition, Rule},
    storage::{Store, read_job},
};
use anyhow::{Result, ensure};
use rmcp::schemars;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const PREFIX: &[u8] = b"AJR_PROGRESS ";
const SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_FRAMES: usize = 64;
const MAX_LINE: usize = 4096;

pub(crate) fn validate_name(s: &str) -> Result<()> {
    ensure!(
        !s.is_empty()
            && s.len() <= 32
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)),
        "progress names must be 1..32 ASCII letters, digits, dot, underscore or hyphen"
    );
    Ok(())
}
fn validate_stage(s: &str) -> Result<()> {
    ensure!(
        !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control),
        "progress stage must be 1..64 bytes without control characters"
    );
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub v: u32,
    pub source: String,
    pub seq: u64,
    pub stage: Option<String>,
    pub step: Option<u64>,
    pub total: Option<u64>,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
    /// Explicit checker identity, never used to select execution or a session.
    pub observed_run: Option<String>,
}
impl Sample {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.v == 1, "unsupported progress version");
        validate_name(&self.source)?;
        ensure!(
            (1..=SAFE_INTEGER).contains(&self.seq),
            "invalid progress sequence"
        );
        if let Some(stage) = &self.stage {
            validate_stage(stage)?;
        }
        ensure!(
            self.step.is_none_or(|s| s <= SAFE_INTEGER)
                && self.total.is_none_or(|t| t > 0 && t <= SAFE_INTEGER)
                && self.total.zip(self.step).is_none_or(|(t, s)| s <= t),
            "invalid progress step/total"
        );
        ensure!(
            self.total.is_none() || self.step.is_some(),
            "total requires step"
        );
        ensure!(
            self.stage.is_some() || self.step.is_some() || !self.metrics.is_empty(),
            "empty progress sample"
        );
        ensure!(self.metrics.len() <= 8, "at most eight progress metrics");
        for (name, value) in &self.metrics {
            validate_name(name)?;
            ensure!(
                value.is_finite() && value.abs() <= 1e100,
                "invalid progress metric"
            );
        }
        if let Some(run) = &self.observed_run {
            ensure!(
                !run.is_empty() && run.len() <= 256 && !run.chars().any(char::is_control),
                "invalid observed run label"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    Stage {
        stage: Option<String>,
    },
    Step {
        every: u64,
    },
    Metric {
        name: String,
        comparison: Comparison,
        value: f64,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Lt,
    Le,
    Gt,
    Ge,
}
impl Trigger {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Stage { stage: Some(stage) } => validate_stage(stage)?,
            Self::Step { every } => ensure!(
                (1..=SAFE_INTEGER).contains(every),
                "invalid progress step interval"
            ),
            Self::Metric { name, value, .. } => {
                validate_name(name)?;
                ensure!(
                    value.is_finite() && value.abs() <= 1e100,
                    "invalid metric threshold"
                );
            }
            _ => {}
        }
        Ok(())
    }
    fn matches(&self, sample: &Sample, previous: Option<&Sample>) -> bool {
        match self {
            Self::Stage { stage } => {
                sample.stage.is_some()
                    && previous.and_then(|s| s.stage.as_ref()) != sample.stage.as_ref()
                    && stage
                        .as_ref()
                        .is_none_or(|s| Some(s) == sample.stage.as_ref())
            }
            Self::Step { every } => sample
                .step
                .is_some_and(|s| s / every > previous.and_then(|p| p.step).unwrap_or(0) / every),
            Self::Metric {
                name,
                comparison,
                value,
            } => sample.metrics.get(name).is_some_and(|n| match comparison {
                Comparison::Lt => n < value,
                Comparison::Le => n <= value,
                Comparison::Gt => n > value,
                Comparison::Ge => n >= value,
            }),
        }
    }
}

#[derive(Default)]
struct Line {
    bytes: Vec<u8>,
    discard: bool,
    applied: Vec<(String, i64)>,
}
#[derive(Default)]
pub(crate) struct Parser {
    lines: [Line; 2],
    pending: Batch,
}
#[derive(Default)]
pub(crate) struct Batch {
    frames: Vec<Frame>,
    rejected: u64,
    dropped: u64,
}
struct Frame {
    sample: Sample,
    applied: Vec<(String, i64)>,
}
impl Parser {
    pub fn output(&mut self, stream: usize, bytes: &[u8], applied: &[(String, i64)]) {
        let line = &mut self.lines[stream];
        for &byte in bytes {
            if byte == b'\n' {
                if !line.discard && line.bytes.starts_with(PREFIX) {
                    let parsed = serde_json::from_slice::<Sample>(&line.bytes[PREFIX.len()..])
                        .ok()
                        .filter(|s| s.validate().is_ok());
                    if let Some(sample) = parsed {
                        if self.pending.frames.len() < MAX_FRAMES {
                            self.pending.frames.push(Frame {
                                sample,
                                applied: line.applied.clone(),
                            });
                        } else {
                            self.pending.dropped = self.pending.dropped.saturating_add(1);
                        }
                    } else {
                        self.pending.rejected = self.pending.rejected.saturating_add(1);
                    }
                }
                line.bytes.clear();
                line.discard = false;
                line.applied.clear();
            } else if !line.discard {
                if line.bytes.is_empty() {
                    line.applied = applied.to_vec();
                }
                if line.bytes.len() == MAX_LINE {
                    if line.bytes.starts_with(PREFIX) {
                        self.pending.rejected = self.pending.rejected.saturating_add(1);
                    }
                    line.bytes.clear();
                    line.discard = true;
                } else {
                    line.bytes.push(byte);
                }
            }
        }
    }
    pub fn take(&mut self) -> Batch {
        std::mem::take(&mut self.pending)
    }
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct State {
    sources: BTreeMap<String, Sample>,
    accepted: u64,
    duplicates: u64,
    rejected: u64,
    dropped: u64,
    recorded_events: u32,
    alerts: u32,
    suppressed: u64,
    last_notification_at: i64,
    milestones: Vec<Value>,
    rules: BTreeMap<String, Counter>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct Counter {
    revision: i64,
    count: u32,
    last_at: i64,
}

pub(crate) fn read_observation(db: &Connection, job: &str) -> Result<Value> {
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM runtime_observations WHERE job_id=?",
            [job],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_else(|| json!({})))
}
pub(crate) fn read_state(db: &Connection, job: &str) -> Result<Option<State>> {
    read_observation(db, job)?
        .get("progress")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(Into::into)
}
/// Include bounded evidence even when terminal completion supersedes an alert.
pub(crate) fn final_summary(db: &Connection, job: &str) -> Result<Value> {
    // Legacy maintenance/recovery may finish an intent before the v9 migration.
    if db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))? < 9 {
        return Ok(Value::Null);
    }
    let observation = read_observation(db, job)?;
    // Pinned native remote observations carry their runner's progress snapshot.
    let state = observation
        .get("progress")
        .or_else(|| observation["external"]["observation"].get("progress"));
    let Some(state) = state else {
        return Ok(Value::Null);
    };
    let mut summary = json!({"sources":state["sources"], "milestones":state["milestones"], "accepted":state["accepted"], "suppressed":state["suppressed"], "dropped":state["dropped"], "rejected":state["rejected"], "truncated":false});
    // Leave room for exit detail and other lifecycle fields in the 8-KiB event.
    while serde_json::to_vec(&summary)?.len() > 3000 {
        summary["truncated"] = json!(true);
        if let Some(sources) = summary["sources"].as_object_mut()
            && let Some(key) = sources.keys().next().cloned()
        {
            sources.remove(&key);
            continue;
        }
        if let Some(items) = summary["milestones"].as_array_mut()
            && !items.is_empty()
        {
            items.remove(0);
            continue;
        }
        // Foreign adapters cannot turn an oversized summary into a terminal failure.
        return Ok(json!({"truncated":true,"details":"inspect the recorded progress snapshot"}));
    }
    Ok(summary)
}
impl Store {
    pub(crate) async fn record_progress(
        &self,
        job: String,
        batch: Batch,
        can_stop: bool,
    ) -> Result<()> {
        if batch.frames.is_empty() && batch.rejected == 0 && batch.dropped == 0 {
            return Ok(());
        }
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let j = read_job(&tx, &job)?;
            if j.status.is_terminal() { return Ok(()) }
            let mut observation = read_observation(&tx, &job)?;
            let mut state: State = observation.get("progress").cloned().map(serde_json::from_value).transpose()?.unwrap_or_default();
            state.rejected = state.rejected.saturating_add(batch.rejected);
            state.dropped = state.dropped.saturating_add(batch.dropped);
            let rules = crate::monitor::rules(&tx, &job)?;
            let at = now();
            for frame in batch.frames {
                let sample = frame.sample;
                let previous = state.sources.get(&sample.source).cloned();
                if previous.as_ref().is_some_and(|p| p.seq >= sample.seq) { state.duplicates = state.duplicates.saturating_add(1); continue; }
                if previous.is_none() && state.sources.len() >= 8 { state.dropped = state.dropped.saturating_add(1); continue; }
                state.accepted = state.accepted.saturating_add(1);
                if state.recorded_events < 256 {
                    crate::storage::insert_event(&tx, &job, &j.run_id, "progress", json!({"sample":sample}))?;
                    state.recorded_events += 1;
                }
                for rule in &rules {
                    let Condition::Progress { source, trigger, cooldown_seconds } = &rule.spec.condition else { continue };
                    if source != &sample.source || !rule.enabled || rule.deleted || !frame.applied.contains(&(rule.id.clone(),rule.revision)) || !trigger.matches(&sample, previous.as_ref()) { continue; }
                    let counter = state.rules.entry(rule.id.clone()).or_default();
                    if counter.revision != rule.revision { *counter = Counter { revision: rule.revision, ..Default::default() }; }
                    if state.alerts >= 128 || counter.count >= 128 || (counter.count > 0 && at.saturating_sub(counter.last_at) < (*cooldown_seconds as i64)*1000) {
                        state.suppressed = state.suppressed.saturating_add(1); continue;
                    }
                    counter.count += 1; counter.last_at = at; state.alerts += 1;
                    let milestone = json!({"rule_id":rule.id,"revision":rule.revision,"at":at,"sample":sample});
                    state.milestones.push(milestone.clone());
                    if state.milestones.len() > 8 { state.milestones.remove(0); }
                    // One progress notification/second per job, independent of rule count.
                    let notify = rule.spec.notify_agent && at.saturating_sub(state.last_notification_at) >= 1000;
                    if notify { state.last_notification_at = at; } else if rule.spec.notify_agent { state.suppressed = state.suppressed.saturating_add(1); }
                    emit(&tx, &j, rule, milestone, can_stop, notify)?;
                }
                state.sources.insert(sample.source.clone(), sample);
            }
            observation["progress"] = serde_json::to_value(state)?;
            tx.execute("INSERT INTO runtime_observations(job_id,data) VALUES(?,?) ON CONFLICT(job_id) DO UPDATE SET data=excluded.data",params![job,observation.to_string()])?;
            tx.commit()?;
            Ok(())
        }).await
    }
}
fn emit(
    db: &Connection,
    job: &Job,
    rule: &Rule,
    evidence: Value,
    can_stop: bool,
    notify: bool,
) -> Result<()> {
    db.execute(
        "UPDATE monitor_rules SET triggered_revision=?,triggered_at=? WHERE id=? AND revision=?",
        params![rule.revision, now(), rule.id, rule.revision],
    )?;
    crate::storage::insert_event(
        db,
        &job.job_id,
        &job.run_id,
        "monitor_alert",
        json!({"rule_id":rule.id,"revision":rule.revision,"rule":rule.spec,"evidence":evidence,"terminal_exit_observed":!can_stop,"notification_rate_limited":rule.spec.notify_agent && !notify}),
    )?;
    let event: String = db.query_row(
        "SELECT id FROM events WHERE seq=last_insert_rowid()",
        [],
        |r| r.get(0),
    )?;
    if rule.spec.action == Action::Stop && can_stop {
        crate::monitor::stop_tx(
            db,
            &job.job_id,
            rule.spec.grace_seconds,
            &format!("progress rule {} revision {}", rule.id, rule.revision),
        )?;
    }
    if notify {
        let mut q = db.prepare("SELECT id FROM targets WHERE job_id=? AND active=1")?;
        let targets = q
            .query_map([&job.job_id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for target in targets {
            crate::session::enqueue(db, &event, &target, now())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        monitor::{Mutation, RuleSpec},
        session::{Adapter, Registration},
    };

    pub(super) fn frame(seq: u64, stage: &str) -> Sample {
        Sample {
            v: 1,
            source: "train".into(),
            seq,
            stage: Some(stage.into()),
            step: Some(seq * 10),
            total: Some(10000),
            metrics: BTreeMap::from([("loss".into(), 0.1)]),
            observed_run: Some("pinned-run-attempt-1".into()),
        }
    }
    pub(super) fn batch(samples: Vec<Sample>, rules: &[Rule]) -> Batch {
        Batch {
            frames: samples
                .into_iter()
                .map(|sample| Frame {
                    sample,
                    applied: rules.iter().map(|r| (r.id.clone(), r.revision)).collect(),
                })
                .collect(),
            ..Default::default()
        }
    }
    pub(super) async fn fixture() -> (tempfile::TempDir, Store, Job, Vec<Rule>) {
        crate::platform::initialize();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp/progress");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config);
        store.init().await.unwrap();
        let session = store
            .register(Registration {
                agent_kind: "codex_cli".into(),
                session_id: "original-chat".into(),
                instance_id: "original-instance".into(),
                host_id: "test".into(),
                adapter: Adapter::Inbox,
                endpoint: None,
                caller_thread_id: None,
                lease_seconds: 3600,
                experimental: false,
                renew_on_delivery: false,
            })
            .await
            .unwrap();
        let job = store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "must-not-run".into(),
                    args: vec![],
                },
                cwd: dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                session: Some(session.key),
                idempotency_key: None,
                monitor_rules: vec![RuleSpec {
                    condition: Condition::Progress {
                        source: "train".into(),
                        trigger: Trigger::Stage { stage: None },
                        cooldown_seconds: 1,
                    },
                    action: Action::Alert,
                    notify_agent: true,
                    grace_seconds: 0,
                }],
            })
            .await
            .unwrap();
        // No daemon is running: exercise transactions without launching execution.
        let id = job.job_id.clone();
        let rules = store
            .read(move |db| crate::monitor::rules(db, &id))
            .await
            .unwrap();
        (dir, store, job, rules)
    }
    #[test]
    fn framing_is_bounded_validated_and_independent_between_streams() {
        let mut p = Parser::default();
        let line = format!(
            "AJR_PROGRESS {}\r\n",
            serde_json::to_string(&frame(1, "train")).unwrap()
        );
        p.output(0, &line.as_bytes()[..17], &[("rule".into(), 1)]);
        p.output(1, b"untrusted instructions\nAJR_PROGRESS {\"v\":99}\n", &[]);
        p.output(0, &line.as_bytes()[17..], &[("rule".into(), 2)]);
        let mut long = PREFIX.to_vec();
        long.extend(vec![b'x'; 10000]);
        long.push(b'\n');
        p.output(0, &long, &[]);
        p.output(0, b"AJR_PROGRESS \xff\n", &[]);
        p.output(0, b"AJR_PROGRESS {}", &[]); // incomplete frames never count
        let b = p.take();
        assert_eq!(b.frames.len(), 1);
        assert_eq!(b.frames[0].sample.seq, 1);
        assert_eq!(b.frames[0].applied, vec![("rule".into(), 1)]);
        assert_eq!(b.rejected, 3);
        let mut flood = Parser::default();
        for _ in 0..100 {
            flood.output(0, line.as_bytes(), &[]);
        }
        let b = flood.take();
        assert_eq!(b.frames.len(), 64);
        assert_eq!(b.dropped, 36);
        let mut invalid = frame(1, "train");
        invalid.metrics.insert("nan".into(), f64::NAN);
        assert!(invalid.validate().is_err());
        invalid.metrics.clear();
        invalid.step = Some(10001);
        assert!(invalid.validate().is_err());
        assert!(!Trigger::Step { every: 20 }.matches(&frame(1, "a"), None));
        assert!(Trigger::Step { every: 20 }.matches(&frame(2, "a"), Some(&frame(1, "a"))));
        assert!(
            Trigger::Metric {
                name: "loss".into(),
                comparison: Comparison::Le,
                value: 0.1
            }
            .matches(&frame(2, "a"), None)
        );
    }
    #[tokio::test]
    async fn sequences_cooldowns_revision_and_terminal_fences_survive_reopen() {
        let (_dir, store, job, rules) = fixture().await;
        let id = job.job_id.clone();
        store
            .record_progress(
                id.clone(),
                batch(
                    vec![
                        frame(2, "checkpoint"),
                        frame(2, "duplicate"),
                        frame(1, "old"),
                        frame(3, "validation"),
                    ],
                    &rules,
                ),
                true,
            )
            .await
            .unwrap();
        let state = store.inspect(Some(id.clone()), None, 20).await.unwrap();
        assert_eq!(state["observation"]["progress"]["accepted"], 2);
        assert_eq!(state["observation"]["progress"]["duplicates"], 2);
        assert_eq!(state["observation"]["progress"]["alerts"], 1);
        assert_eq!(state["observation"]["progress"]["suppressed"], 1);
        assert_eq!(
            state["deliveries"]["deliveries"].as_array().unwrap().len(),
            1
        );
        let claim = store.claim().await.unwrap().unwrap();
        assert_eq!(claim.session.registration.session_id, "original-chat");
        let delivery = claim.delivery.id.clone();
        store
            .complete_claim(claim, Err(anyhow::anyhow!("offline")))
            .await
            .unwrap();
        let key = id.clone();
        store
            .call(move |db| {
                let mut data = read_observation(db, &key)?;
                for counter in data["progress"]["rules"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                {
                    counter["last_at"] = json!(now() - 2000);
                }
                data["progress"]["last_notification_at"] = json!(now() - 2000);
                db.execute(
                    "UPDATE runtime_observations SET data=? WHERE job_id=?",
                    params![data.to_string(), key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let store = Store::new(store.config.clone()); // durable counters, not parser memory
        store
            .record_progress(id.clone(), batch(vec![frame(4, "saved")], &rules), true)
            .await
            .unwrap();
        let pending = batch(vec![frame(5, "stale-rule")], &rules);
        store
            .monitor(
                id.clone(),
                Some(Mutation::Enable {
                    rule_id: rules[0].id.clone(),
                    enabled: false,
                    expected_revision: 1,
                }),
            )
            .await
            .unwrap();
        store
            .record_progress(id.clone(), pending, true)
            .await
            .unwrap();
        let before = store.events(id.clone(), 0, 100).await.unwrap();
        assert_eq!(
            before.iter().filter(|e| e.kind == "monitor_alert").count(),
            2
        );
        store
            .finish(id.clone(), "completed", Some(0), None, "done".into())
            .await
            .unwrap();
        store
            .record_progress(id.clone(), batch(vec![frame(6, "too-late")], &rules), true)
            .await
            .unwrap();
        let events = store.events(id.clone(), 0, 100).await.unwrap();
        let terminal = events.iter().find(|e| e.kind == "completed").unwrap();
        assert_eq!(
            terminal.payload["progress"]["milestones"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(terminal.payload["progress"]["sources"]["train"]["seq"], 5);
        assert_eq!(events.len(), before.len() + 1);
        let deliveries = store.deliveries(Some(id.clone()), 0, 20).await.unwrap();
        assert!(
            deliveries["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["id"] == delivery && d["status"] == "cancelled" && d["attempts"] == 1)
        );
        // Existing archive/backup paths must carry the same evidence and IDs.
        store
            .data_policy(
                Some(crate::maintenance::Policy {
                    hot_jobs: 1,
                    archive_after_days: 0,
                    hot_log_mib: 1024,
                    backup_hours: 24,
                    backup_keep: 2,
                }),
                true,
            )
            .await
            .unwrap();
        // Resolve terminal inbox delivery before archival eligibility.
        if let Some(claim) = store.claim().await.unwrap() {
            store
                .complete_claim(claim, Ok("notification_recorded"))
                .await
                .unwrap();
        }
        let native_maintenance = cfg!(any(target_os = "linux", target_os = "windows"));
        let result = store.maintain(true).await;
        if native_maintenance {
            assert_eq!(result.unwrap()["archive"]["archived"], 1);
        } else {
            // Archival is portable, but publishing the automatic backup is not.
            assert!(result.unwrap_err().to_string().contains(
                "atomic new data-directory publication is supported only on Linux and Windows"
            ));
            assert_eq!(store.data_status().await.unwrap()["archived_jobs"], 1);
        }
        let archived = store.inspect(Some(id.clone()), None, 20).await.unwrap();
        assert_eq!(archived["observation"]["progress"]["alerts"], 2);
        assert_eq!(
            store
                .events(id.clone(), 0, 100)
                .await
                .unwrap()
                .last()
                .unwrap()
                .event_id,
            terminal.event_id
        );
        let backup = store.backup().await;
        if !native_maintenance {
            assert!(backup.unwrap_err().to_string().contains(
                "atomic new data-directory publication is supported only on Linux and Windows"
            ));
            assert!(
                store.data_status().await.unwrap()["backups"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            return;
        }
        let backup = backup.unwrap();
        let destination = store.config.root.parent().unwrap().join("restored");
        crate::maintenance::restore(backup["path"].as_str().unwrap().into(), destination.clone())
            .await
            .unwrap();
        let restored = Store::readonly(Config::new(Some(destination)).unwrap())
            .inspect(Some(id), None, 20)
            .await
            .unwrap();
        assert_eq!(
            restored["observation"]["progress"],
            archived["observation"]["progress"]
        );
    }
    #[tokio::test]
    async fn failed_outbox_transaction_rolls_back_samples_and_bounded_history() {
        let (_dir, store, job, rules) = fixture().await;
        store.call(|db|{db.execute_batch("CREATE TRIGGER fail_progress BEFORE INSERT ON deliveries BEGIN SELECT RAISE(ABORT,'injected outbox failure'); END;")?;Ok(())}).await.unwrap();
        assert!(
            store
                .record_progress(
                    job.job_id.clone(),
                    batch(vec![frame(1, "checkpoint")], &rules),
                    true
                )
                .await
                .is_err()
        );
        assert!(
            store
                .events(job.job_id.clone(), 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        store
            .call(|db| {
                db.execute_batch("DROP TRIGGER fail_progress")?;
                Ok(())
            })
            .await
            .unwrap();
        store
            .record_progress(
                job.job_id.clone(),
                batch(vec![frame(1, "checkpoint")], &rules),
                true,
            )
            .await
            .unwrap();
        for seq in 2..300 {
            store
                .record_progress(
                    job.job_id.clone(),
                    batch(vec![frame(seq, "checkpoint")], &rules),
                    true,
                )
                .await
                .unwrap();
        }
        let state = store
            .inspect(Some(job.job_id.clone()), None, 20)
            .await
            .unwrap();
        assert_eq!(state["observation"]["progress"]["recorded_events"], 256);
        assert_eq!(
            state["observation"]["progress"]["sources"]["train"]["seq"],
            299
        );
        assert_eq!(state["observation"]["progress"]["alerts"], 1);
        // Cross the durable job alert budget, then reopen and try again.
        let key = job.job_id.clone();
        store
            .call(move |db| {
                let mut observation = read_observation(db, &key)?;
                let mut state: State = serde_json::from_value(observation["progress"].clone())?;
                state.alerts = 127;
                state.rules.values_mut().for_each(|c| {
                    c.last_at = now() - 2000;
                    c.count = 127;
                });
                state.last_notification_at = now() - 2000;
                observation["progress"] = serde_json::to_value(state)?;
                db.execute(
                    "UPDATE runtime_observations SET data=? WHERE job_id=?",
                    params![observation.to_string(), key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        store
            .record_progress(
                job.job_id.clone(),
                batch(vec![frame(300, "budget-edge")], &rules),
                true,
            )
            .await
            .unwrap();
        let store = Store::new(store.config.clone());
        store
            .record_progress(
                job.job_id.clone(),
                batch(vec![frame(301, "over-budget")], &rules),
                true,
            )
            .await
            .unwrap();
        let bounded = store
            .inspect(Some(job.job_id.clone()), None, 20)
            .await
            .unwrap();
        assert_eq!(bounded["observation"]["progress"]["alerts"], 128);
        assert_eq!(
            bounded["observation"]["progress"]["sources"]["train"]["seq"],
            301
        );
        assert_eq!(
            bounded["observation"]["progress"]["milestones"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        // Terminal and ingest contend on the same transaction boundary.
        let (a, b) = tokio::join!(
            store.finish(
                job.job_id.clone(),
                "completed",
                Some(0),
                None,
                "done".into()
            ),
            store.record_progress(
                job.job_id.clone(),
                batch(vec![frame(302, "last")], &rules),
                true
            )
        );
        a.unwrap();
        b.unwrap();
        let final_state = store.inspect(Some(job.job_id), None, 20).await.unwrap();
        assert_eq!(final_state["events"][0]["kind"], "completed");
    }
}

#[cfg(test)]
mod limits_tests {
    use super::*;

    #[tokio::test]
    async fn notification_budget_and_terminal_summary_bound_preserve_milestones() {
        let (_dir, store, job, _rules) = super::tests::fixture().await;
        store
            .monitor(
                job.job_id.clone(),
                Some(crate::monitor::Mutation::Add {
                    rule: crate::monitor::RuleSpec {
                        condition: Condition::Progress {
                            source: "train".into(),
                            trigger: Trigger::Metric {
                                name: "loss".into(),
                                comparison: Comparison::Le,
                                value: 0.1,
                            },
                            cooldown_seconds: 1,
                        },
                        action: Action::Alert,
                        notify_agent: true,
                        grace_seconds: 0,
                    },
                }),
            )
            .await
            .unwrap();
        let key = job.job_id.clone();
        let rules = store
            .read(move |db| crate::monitor::rules(db, &key))
            .await
            .unwrap();
        let mut samples = vec![];
        for i in 0..9 {
            let mut sample = super::tests::frame(1, &"s".repeat(64));
            if i > 0 {
                sample.source = format!("source-{i}");
            }
            sample.observed_run = Some("r".repeat(256));
            sample
                .metrics
                .extend((0..7).map(|i| (format!("metric-{i}-{}", "m".repeat(23)), 1e100)));
            sample.validate().unwrap();
            samples.push(sample);
        }
        store
            .record_progress(
                job.job_id.clone(),
                super::tests::batch(samples, &rules),
                true,
            )
            .await
            .unwrap();
        store
            .observe(job.job_id.clone(), json!({"sampled_at":now()}), vec![])
            .await
            .unwrap();
        let inspected = store
            .inspect(Some(job.job_id.clone()), None, 20)
            .await
            .unwrap();
        assert_eq!(
            inspected["observation"]["progress"]["sources"]
                .as_object()
                .unwrap()
                .len(),
            8
        );
        assert_eq!(inspected["observation"]["progress"]["dropped"], 1);
        assert_eq!(inspected["observation"]["progress"]["alerts"], 2);
        assert_eq!(
            inspected["deliveries"]["deliveries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            inspected["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["payload"]["notification_rate_limited"] == true)
        );
        store
            .finish(
                job.job_id.clone(),
                "completed",
                Some(0),
                None,
                "x".repeat(4096),
            )
            .await
            .unwrap();
        let events = store.events(job.job_id, 0, 100).await.unwrap();
        let payload = &events.last().unwrap().payload;
        assert!(serde_json::to_vec(payload).unwrap().len() < 8192);
        assert!(serde_json::to_vec(&payload["progress"]).unwrap().len() <= 3000);
        assert_eq!(payload["progress"]["truncated"], true);
        assert_eq!(
            payload["progress"]["milestones"].as_array().unwrap().len(),
            2
        );
    }
}
