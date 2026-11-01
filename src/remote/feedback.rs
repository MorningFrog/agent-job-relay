//! Origin-owned subscriptions to the pinned remote runner's typed alerts.
//! Cursor, bounded evidence, local events and outbox commit together.
use super::*;
use crate::{
    monitor::{Condition, Rule, RuleSpec},
    storage::Store,
};
use anyhow::{Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const LIMIT: u32 = 128;

pub(super) struct Page {
    pub run: RunRef,
    pub after: i64,
    pub events: Vec<Event>,
    pub next: i64,
    pub terminal: bool,
    pub applied: Vec<(String, i64)>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    alerts: u32,
    suppressed: u64,
    coalesced: u64,
    last_notification_at: i64,
    rules: BTreeMap<String, Counter>,
    pending: BTreeMap<String, Candidate>,
    milestones: Vec<Value>,
}
#[derive(Default, Serialize, Deserialize)]
struct Counter {
    revision: i64,
    last_at: i64,
    count: u32,
}
#[derive(Serialize, Deserialize)]
struct Candidate {
    revision: i64,
    evidence: Value,
}

fn evidence(target: &str, run: &RunRef, event: &Event) -> Result<Option<Value>> {
    if event.kind != "monitor_alert" {
        return Ok(None);
    }
    validate_id(&event.event_id)?;
    let rule_id = event.payload["rule_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("remote alert has no rule identity"))?;
    validate_id(rule_id)?;
    let revision = event.payload["revision"].as_i64().unwrap_or(0);
    ensure!(
        (1..=128).contains(&revision),
        "invalid remote alert revision"
    );
    let rule: RuleSpec = serde_json::from_value(event.payload["rule"].clone())?;
    rule.validate()?;
    ensure!(
        !matches!(rule.condition, Condition::RemoteAlert { .. }),
        "nested remote alert forwarding is unsupported"
    );
    let mut source = event.payload["evidence"].clone();
    if serde_json::to_vec(&source)?.len() > 3000 {
        source = json!({"truncated":true,"reason":"inspect the original remote event for complete evidence"});
    }
    Ok(Some(
        json!({"remote":{"target_id":target,"run":run,"event_id":event.event_id,
        "event_sequence":event.seq,"event_at":event.at,"rule_id":rule_id,"revision":revision},
        "source_rule":rule,"source_evidence":source}),
    ))
}

fn matching(rule: &Rule, applied: &[(String, i64)]) -> bool {
    rule.enabled
        && !rule.deleted
        && matches!(rule.spec.condition, Condition::RemoteAlert { .. })
        && applied.contains(&(rule.id.clone(), rule.revision))
}

impl Store {
    pub(super) async fn observe_remote(
        &self,
        job: String,
        mut value: Value,
        page: Page,
    ) -> Result<()> {
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let j = crate::storage::read_job(&tx, &job)?;
            if j.status.is_terminal() { return Ok(()); }
            let CommandSpec::Remote { remote } = &j.command else { anyhow::bail!("remote feedback requires a native remote observation") };
            page.run.validate()?;
            if let Intent::Attach { run, .. } = &remote.intent { ensure!(*run == page.run, "remote feedback attachment changed"); }
            ensure!(value["external"]["target_id"] == remote.target_id && value["external"]["run"] == json!(page.run), "remote feedback mapping mismatch");
            let previous = crate::progress::read_observation(&tx, &job)?;
            ensure!(previous["external"]["next_event"].as_i64().unwrap_or(0) == page.after, "remote feedback cursor changed; inspect original observation");
            if let Some(run) = previous["external"].get("run").filter(|r| !r.is_null()) {
                ensure!(*run == json!(page.run), "remote feedback execution changed");
            }
            let mut cursor = page.after;
            let mut sources = Vec::new();
            let rules = crate::monitor::rules(&tx, &job)?;
            let subscribed = rules.iter().any(|r| matching(r, &page.applied));
            ensure!(page.after >= 0 && page.events.len() <= 20, "invalid remote feedback page");
            for event in &page.events {
                ensure!(event.job_id == page.run.job_id && event.run_id == page.run.run_id && event.seq > cursor, "remote feedback event identity/cursor mismatch");
                cursor = event.seq;
                if subscribed && let Some(source) = evidence(&remote.target_id, &page.run, event)? { sources.push(source); }
            }
            ensure!(cursor == page.next && value["external"]["next_event"] == page.next, "remote feedback cursor mismatch");
            let mut state: State = previous.get("remote_feedback").cloned().map(serde_json::from_value).transpose()?.unwrap_or_default();
            state.pending.retain(|id, candidate| rules.iter().any(|r| r.id == *id && r.revision == candidate.revision && matching(r, &page.applied)));
            for rule in rules.iter().filter(|r| matching(r, &page.applied)) {
                let Condition::RemoteAlert { rule_id, .. } = &rule.spec.condition else { unreachable!() };
                for source in &sources {
                    if rule_id.as_ref().is_some_and(|id| source["remote"]["rule_id"] != *id) { continue; }
                    if state.pending.insert(rule.id.clone(), Candidate { revision: rule.revision, evidence: source.clone() }).is_some() { state.coalesced = state.coalesced.saturating_add(1); }
                }
            }
            // Drain event backlog before selecting the latest alert per local
            // subscription. Pending candidates survive observer/daemon loss.
            if page.events.len() < 20 {
                let at = now();
                for (id, candidate) in std::mem::take(&mut state.pending) {
                    let rule = rules.iter().find(|r| r.id == id && r.revision == candidate.revision && matching(r, &page.applied)).expect("pending candidates were fenced");
                    let Condition::RemoteAlert { cooldown_seconds, .. } = rule.spec.condition else { unreachable!() };
                    let duplicate: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM events WHERE job_id=? AND kind='monitor_alert' AND json_extract(payload,'$.rule_id')=? AND json_extract(payload,'$.evidence.remote.event_id')=?)",params![job,id,candidate.evidence["remote"]["event_id"].as_str()],|r|r.get(0))?;
                    if duplicate { state.suppressed = state.suppressed.saturating_add(1); continue; }
                    let counter = state.rules.entry(id.clone()).or_default();
                    if counter.revision != rule.revision { *counter = Counter { revision: rule.revision, ..Default::default() }; }
                    if state.alerts >= LIMIT || (counter.count > 0 && at.saturating_sub(counter.last_at) < cooldown_seconds as i64 * 1000) {
                        state.suppressed = state.suppressed.saturating_add(1); continue;
                    }
                    counter.count += 1; counter.last_at = at; state.alerts += 1;
                    let notify = rule.spec.notify_agent && !page.terminal && !j.cancel_requested && at.saturating_sub(state.last_notification_at) >= 1000;
                    if notify { state.last_notification_at = at; }
                    let rate_limited = rule.spec.notify_agent && !page.terminal && !j.cancel_requested && !notify;
                    if rate_limited { state.suppressed = state.suppressed.saturating_add(1); }
                    let milestone = json!({"rule_id":id,"revision":rule.revision,"at":at,"evidence":candidate.evidence});
                    state.milestones.push(milestone);
                    if state.milestones.len() > 8 { state.milestones.remove(0); }
                    tx.execute("UPDATE monitor_rules SET triggered_revision=?,triggered_at=? WHERE id=? AND revision=?", params![rule.revision,at,id,rule.revision])?;
                    crate::storage::insert_event(&tx, &job, &j.run_id, "monitor_alert", json!({"rule_id":id,"revision":rule.revision,"rule":rule.spec,
                        "evidence":candidate.evidence,"remote_terminal_observed":page.terminal,"notification_rate_limited":rate_limited}))?;
                    let event: String = tx.query_row("SELECT id FROM events WHERE seq=last_insert_rowid()", [], |r| r.get(0))?;
                    if notify {
                        // Keep attempts/IDs and live or Goal-selected claims.
                        tx.execute("UPDATE deliveries SET status='cancelled',failure_kind='superseded',last_error='coalesced into newer remote feedback' WHERE status IN ('pending','retry_wait') AND event_id IN (SELECT id FROM events WHERE job_id=?1 AND kind='monitor_alert' AND json_extract(payload,'$.rule_id')=?2 AND json_type(payload,'$.evidence.remote')='object') AND NOT EXISTS(SELECT 1 FROM goal_waits g WHERE g.feedback_delivery=deliveries.id AND g.status IN ('resuming','resumed'))", params![job,id])?;
                        let mut q = tx.prepare("SELECT id FROM targets WHERE job_id=? AND active=1")?;
                        let targets = q.query_map([&job], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                        for target in targets { crate::session::enqueue(&tx, &event, &target, at)?; }
                    }
                }
            }
            if subscribed || previous.get("remote_feedback").is_some() { value["remote_feedback"] = serde_json::to_value(state)?; }
            if let Some(progress) = previous.get("progress") { value["progress"] = progress.clone(); }
            tx.execute("INSERT INTO runtime_observations(job_id,data) VALUES(?,?) ON CONFLICT(job_id) DO UPDATE SET data=excluded.data", params![job,value.to_string()])?;
            for (id, revision) in page.applied { tx.execute("UPDATE monitor_rules SET applied_revision=?,applied_at=? WHERE id=? AND job_id=? AND revision=? AND applied_revision<>?", params![revision,now(),id,job,revision,revision])?; }
            tx.commit()?;
            Ok(())
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        monitor::{Action, Mutation},
        session::{Adapter, Registration},
    };

    async fn fixture() -> (tempfile::TempDir, Store, Job, RunRef, String) {
        crate::platform::initialize();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config);
        store.init().await.unwrap();
        let session = store
            .register(Registration {
                agent_kind: "test".into(),
                session_id: "original".into(),
                instance_id: id(),
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
        let run = RunRef {
            job_id: id(),
            run_id: id(),
            attempt: 1,
        };
        let target = id();
        let job = store
            .start(StartSpec {
                title: None,
                command: CommandSpec::Remote {
                    remote: Box::new(RemoteSpec {
                        target_id: target.clone(),
                        transport: Transport {
                            program: "never-execute".into(),
                            args: vec![],
                        },
                        intent: Intent::Attach {
                            run: run.clone(),
                            allow_stop: false,
                        },
                    }),
                },
                cwd: dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                queue_timeout_seconds: None,
                session: Some(session.key),
                idempotency_key: None,
                monitor_rules: vec![RuleSpec {
                    condition: Condition::RemoteAlert {
                        rule_id: None,
                        cooldown_seconds: 1,
                    },
                    action: Action::Alert,
                    notify_agent: true,
                    grace_seconds: 0,
                }],
            })
            .await
            .unwrap();
        (dir, store, job, run, target)
    }
    fn alert(run: &RunRef, seq: i64, rule: &str) -> Event {
        Event {
            seq,
            event_id: id(),
            job_id: run.job_id.clone(),
            run_id: run.run_id.clone(),
            kind: "monitor_alert".into(),
            at: now(),
            payload: json!({"rule_id":rule,"revision":1,"rule":{"condition":{"kind":"elapsed","seconds":1},"action":"stop","notify_agent":false},"evidence":{"stage":"checkpoint"}}),
        }
    }
    async fn record(
        store: &Store,
        job: &Job,
        run: &RunRef,
        target: &str,
        after: i64,
        events: Vec<Event>,
        terminal: bool,
    ) -> Result<()> {
        let applied = store.monitor(job.job_id.clone(), None).await?["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["id"].as_str().unwrap().into(),
                    r["revision"].as_i64().unwrap(),
                )
            })
            .collect();
        record_applied(store, job, run, target, (after, events), terminal, applied).await
    }
    async fn record_applied(
        store: &Store,
        job: &Job,
        run: &RunRef,
        target: &str,
        page: (i64, Vec<Event>),
        terminal: bool,
        applied: Vec<(String, i64)>,
    ) -> Result<()> {
        let (after, events) = page;
        let next = events.last().map_or(after, |e| e.seq);
        store.observe_remote(job.job_id.clone(),json!({"external":{"target_id":target,"run":run,"next_event":next},"sampled_at":now()}),Page { run:run.clone(),after,events,next,terminal,applied }).await
    }
    async fn detail(store: &Store, job: &Job) -> Value {
        store
            .inspect(Some(job.job_id.clone()), None, 1)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn cursor_outbox_and_evidence_are_atomic_and_replay_and_identity_are_fenced() {
        let (_dir, store, job, run, target) = fixture().await;
        let source = id();
        let mut event = alert(&run, 1, &source);
        event.payload["evidence"] = json!({"oversized":"x".repeat(6000)});
        store.call(|db|{db.execute_batch("CREATE TRIGGER fail_remote_feedback BEFORE INSERT ON deliveries BEGIN SELECT RAISE(ABORT,'injected outbox failure'); END;")?;Ok(())}).await.unwrap();
        assert!(
            record(&store, &job, &run, &target, 0, vec![event.clone()], false)
                .await
                .is_err()
        );
        let rolled_back = detail(&store, &job).await;
        assert!(rolled_back["observation"].is_null());
        assert_eq!(rolled_back["events"], json!([]));
        store
            .call(|db| {
                db.execute_batch("DROP TRIGGER fail_remote_feedback")?;
                Ok(())
            })
            .await
            .unwrap();
        record(&store, &job, &run, &target, 0, vec![event.clone()], false)
            .await
            .unwrap();
        let value = detail(&store, &job).await;
        assert_eq!(value["observation"]["external"]["next_event"], 1);
        assert_eq!(value["observation"]["remote_feedback"]["alerts"], 1);
        let compact = crate::output::compact("inspect", value.clone());
        assert_eq!(
            compact["remote_feedback"]["milestones"][0]["evidence"]["remote"]["event_id"],
            event.event_id
        );
        assert_eq!(
            value["events"][0]["payload"]["evidence"]["remote"]["event_id"],
            event.event_id
        );
        assert_eq!(
            value["events"][0]["payload"]["evidence"]["source_evidence"]["truncated"],
            true
        );
        assert_eq!(value["stop"], Value::Null); // A remote stop action is evidence only.
        assert_eq!(
            value["deliveries"]["deliveries"].as_array().unwrap().len(),
            1
        );
        assert!(
            record(&store, &job, &run, &target, 0, vec![event], false)
                .await
                .is_err()
        );
        let mut wrong = run.clone();
        wrong.run_id = id();
        assert!(
            record(
                &store,
                &job,
                &wrong,
                &target,
                1,
                vec![alert(&wrong, 2, &source)],
                false
            )
            .await
            .is_err()
        );
        assert!(
            record(
                &store,
                &job,
                &run,
                &id(),
                1,
                vec![alert(&run, 2, &source)],
                false
            )
            .await
            .is_err()
        );
        assert_eq!(detail(&store, &job).await["events"], value["events"]);
        store
            .finish(
                job.job_id.clone(),
                "completed",
                Some(0),
                None,
                "remote terminal observed".into(),
            )
            .await
            .unwrap();
        let claim = store.claim().await.unwrap().unwrap();
        let result = crate::adapters::deliver(&store.config, &claim).await;
        store.complete_claim(claim, result).await.unwrap();
        let hot = detail(&store, &job).await;
        assert_eq!(
            hot["events"][0]["payload"]["progress"]["remote_feedback"]["alerts"],
            1
        );
        store.archive_job(job.job_id.clone()).await.unwrap();
        let cold = detail(&store, &job).await;
        assert_eq!(cold["observation"], hot["observation"]);
        assert_eq!(cold["notifications"], hot["notifications"]);
    }

    #[tokio::test]
    async fn backlog_is_durable_coalesced_and_fenced_by_current_rule_revision() {
        let (_dir, store, job, run, target) = fixture().await;
        let source = id();
        let events = (1..=20).map(|seq| alert(&run, seq, &source)).collect();
        record(&store, &job, &run, &target, 0, events, false)
            .await
            .unwrap();
        let pending = detail(&store, &job).await;
        assert_eq!(pending["observation"]["remote_feedback"]["alerts"], 0);
        assert_eq!(pending["observation"]["remote_feedback"]["coalesced"], 19);
        let store = Store::new(store.config.clone());
        store.init().await.unwrap();
        record(&store, &job, &run, &target, 20, vec![], false)
            .await
            .unwrap();
        let first = detail(&store, &job).await;
        assert_eq!(
            first["events"][0]["payload"]["evidence"]["remote"]["event_sequence"],
            20
        );
        assert_eq!(first["observation"]["remote_feedback"]["alerts"], 1);
        record(
            &store,
            &job,
            &run,
            &target,
            20,
            vec![alert(&run, 21, &source)],
            false,
        )
        .await
        .unwrap();
        assert_eq!(
            detail(&store, &job).await["observation"]["remote_feedback"]["suppressed"],
            1
        );
        let rule: Rule = serde_json::from_value(first["rules"][0].clone()).unwrap();
        store
            .monitor(
                job.job_id.clone(),
                Some(Mutation::Update {
                    rule_id: rule.id.clone(),
                    rule: rule.spec.clone(),
                    expected_revision: 1,
                }),
            )
            .await
            .unwrap();
        record_applied(
            &store,
            &job,
            &run,
            &target,
            (21, vec![alert(&run, 22, &source)]),
            false,
            vec![(rule.id, 1)],
        )
        .await
        .unwrap();
        assert_eq!(
            detail(&store, &job).await["observation"]["remote_feedback"]["alerts"],
            1
        );
        record(
            &store,
            &job,
            &run,
            &target,
            22,
            vec![alert(&run, 23, &source)],
            true,
        )
        .await
        .unwrap();
        let terminal = detail(&store, &job).await;
        assert_eq!(terminal["observation"]["remote_feedback"]["alerts"], 2);
        assert_eq!(
            terminal["events"][0]["payload"]["remote_terminal_observed"],
            true
        );
        assert_eq!(
            terminal["deliveries"]["deliveries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let key = job.job_id.clone();
        store
            .call(move |db| {
                let mut value = crate::progress::read_observation(db, &key)?;
                value["remote_feedback"]["alerts"] = json!(LIMIT);
                db.execute(
                    "UPDATE runtime_observations SET data=? WHERE job_id=?",
                    params![value.to_string(), key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        record(
            &store,
            &job,
            &run,
            &target,
            23,
            vec![alert(&run, 24, &source)],
            false,
        )
        .await
        .unwrap();
        let limited = detail(&store, &job).await;
        assert_eq!(limited["observation"]["remote_feedback"]["alerts"], LIMIT);
        assert_eq!(limited["feedback"]["status"], "exhausted");
    }

    #[tokio::test]
    async fn subscriptions_filter_original_ids_and_refuse_old_observers_and_stop_authority() {
        let (_dir, store, job, run, target) = fixture().await;
        let initial = detail(&store, &job).await;
        let rule: Rule = serde_json::from_value(initial["rules"][0].clone()).unwrap();
        let source = id();
        let mut spec = rule.spec.clone();
        spec.condition = Condition::RemoteAlert {
            rule_id: Some(source.clone()),
            cooldown_seconds: 1,
        };
        store
            .monitor(
                job.job_id.clone(),
                Some(Mutation::Update {
                    rule_id: rule.id.clone(),
                    rule: spec.clone(),
                    expected_revision: 1,
                }),
            )
            .await
            .unwrap();
        record(
            &store,
            &job,
            &run,
            &target,
            0,
            vec![alert(&run, 1, &id())],
            false,
        )
        .await
        .unwrap();
        assert_eq!(
            detail(&store, &job).await["observation"]["remote_feedback"]["alerts"],
            0
        );
        let mut event = alert(&run, 2, &source);
        record(&store, &job, &run, &target, 1, vec![event.clone()], false)
            .await
            .unwrap();
        let key = job.job_id.clone();
        store
            .call(move |db| {
                let mut value = crate::progress::read_observation(db, &key)?;
                value["remote_feedback"]["last_notification_at"] = json!(0);
                for counter in value["remote_feedback"]["rules"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                {
                    counter["last_at"] = json!(0);
                }
                db.execute(
                    "UPDATE runtime_observations SET data=? WHERE job_id=?",
                    params![value.to_string(), key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        // Even a malformed adapter reusing an event ID at a higher sequence
        // cannot create another delivery for this original subscription.
        event.seq = 3;
        record(&store, &job, &run, &target, 2, vec![event], false)
            .await
            .unwrap();
        assert_eq!(
            detail(&store, &job).await["observation"]["remote_feedback"]["alerts"],
            1
        );
        let key = job.job_id.clone();
        store
            .call(move |db| {
                db.execute("UPDATE jobs SET status='running' WHERE id=?", [key])?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(
            store
                .monitor(
                    job.job_id.clone(),
                    Some(Mutation::Add { rule: spec.clone() })
                )
                .await
                .unwrap_err()
                .to_string()
                .contains("observer has not advertised")
        );
        spec.action = Action::Stop;
        assert!(
            spec.validate()
                .unwrap_err()
                .to_string()
                .contains("only records alerts")
        );
        spec.action = Action::Alert;
        let native = CommandSpec::Exec {
            program: "never-execute".into(),
            args: vec![],
        };
        assert!(native.validate_rule(&spec).is_err());
    }
}
