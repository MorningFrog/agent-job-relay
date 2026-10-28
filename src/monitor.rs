//! Durable, bounded rules. Detection belongs to the independent runner; viewers
//! and host notifications never own execution or monitoring lifetimes.
use crate::{
    core::*,
    storage::{Store, read_job},
};
use anyhow::{Result, ensure};
use rmcp::schemars;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const TAIL_BYTES: usize = 65536;
pub fn default_grace() -> u32 {
    5
}
#[derive(Clone, Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    NoOutput {
        seconds: u64,
    },
    Elapsed {
        seconds: u64,
    },
    Contains {
        text: String,
        #[serde(default)]
        stream: Stream,
    },
}
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq,
)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
    #[default]
    Both,
}
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq,
)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[default]
    Alert,
    Stop,
}
#[derive(Clone, Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuleSpec {
    pub condition: Condition,
    #[serde(default)]
    pub action: Action,
    #[serde(default)]
    pub notify_agent: bool,
    #[serde(default = "default_grace")]
    pub grace_seconds: u32,
}
impl RuleSpec {
    pub fn validate(&self) -> Result<()> {
        match &self.condition {
            Condition::NoOutput { seconds } | Condition::Elapsed { seconds } => ensure!(
                (1..=604800).contains(seconds),
                "monitor duration must be 1..604800 seconds"
            ),
            Condition::Contains { text, .. } => ensure!(
                !text.is_empty()
                    && text.len() <= 1024
                    && text
                        .chars()
                        .all(|c| !c.is_control() || c == '\n' || c == '\t'),
                "match text must be 1..1024 bytes without NUL"
            ),
        }
        ensure!(self.grace_seconds <= 300, "grace_seconds must be 0..300");
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub job_id: String,
    pub revision: i64,
    pub enabled: bool,
    pub deleted: bool,
    pub spec: RuleSpec,
    pub applied_revision: i64,
    pub applied_at: Option<i64>,
    pub triggered_revision: i64,
    pub triggered_at: Option<i64>,
}
#[derive(Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Mutation {
    Add {
        rule: RuleSpec,
    },
    Update {
        rule_id: String,
        rule: RuleSpec,
        expected_revision: i64,
    },
    Enable {
        rule_id: String,
        enabled: bool,
        expected_revision: i64,
    },
    Remove {
        rule_id: String,
        expected_revision: i64,
    },
}
fn decode_rule(v: Value) -> Result<Rule> {
    let mut v = v;
    v["spec"] = serde_json::from_str(v["spec"].as_str().unwrap_or(""))?;
    for field in ["enabled", "deleted"] {
        v[field] = json!(v[field].as_i64() == Some(1));
    }
    Ok(serde_json::from_value(v)?)
}
pub(crate) fn rules(db: &Connection, job: &str) -> Result<Vec<Rule>> {
    let records = if let Some(rows) = crate::archive::monitor_rows(db, job, "monitor_rules")? {
        rows
    } else {
        let mut q=db.prepare("SELECT id,job_id,revision,enabled,deleted,spec,applied_revision,applied_at,triggered_revision,triggered_at FROM monitor_rules WHERE job_id=? ORDER BY rowid")?;
        q.query_map([job],|r|Ok(json!({"id":r.get::<_,String>(0)?,"job_id":r.get::<_,String>(1)?,"revision":r.get::<_,i64>(2)?,"enabled":r.get::<_,i64>(3)?,"deleted":r.get::<_,i64>(4)?,"spec":r.get::<_,String>(5)?,"applied_revision":r.get::<_,i64>(6)?,"applied_at":r.get::<_,Option<i64>>(7)?,"triggered_revision":r.get::<_,i64>(8)?,"triggered_at":r.get::<_,Option<i64>>(9)?})))?.collect::<rusqlite::Result<Vec<_>>>()?
    };
    records.into_iter().map(decode_rule).collect()
}
pub(crate) fn add_rule(db: &Connection, job: &str, spec: &RuleSpec) -> Result<String> {
    spec.validate()?;
    ensure!(
        db.query_row(
            "SELECT count(*) FROM monitor_rules WHERE job_id=?",
            [job],
            |r| r.get::<_, u32>(0)
        )? < 16,
        "monitor rule quota (16, including removed rules) reached"
    );
    if spec.notify_agent {
        ensure!(
            read_job(db, job)?.session.is_some(),
            "notify_agent requires an explicitly bound job session"
        );
    }
    let id = id();
    let raw = serde_json::to_string(spec)?;
    db.execute(
        "INSERT INTO monitor_rules(id,job_id,revision,enabled,spec) VALUES(?,?,1,1,?)",
        params![id, job, raw],
    )?;
    audit(db, job, &id, 1, json!({"kind":"add","rule":spec}))?;
    Ok(id)
}
fn audit(db: &Connection, job: &str, rule: &str, revision: i64, details: Value) -> Result<()> {
    db.execute(
        "INSERT INTO monitor_changes(id,job_id,rule_id,revision,at,details) VALUES(?,?,?,?,?,?)",
        params![id(), job, rule, revision, now(), details.to_string()],
    )?;
    Ok(())
}
pub(crate) fn stop_tx(db: &Connection, job: &str, grace: u32, reason: &str) -> Result<()> {
    let j = read_job(db, job)?;
    if terminal(&j.status) {
        return Ok(());
    }
    db.execute("INSERT INTO stop_requests(job_id,grace_seconds,reason,at) VALUES(?,?,?,?) ON CONFLICT(job_id) DO UPDATE SET grace_seconds=min(grace_seconds,excluded.grace_seconds)",params![job,grace,reason,now()])?;
    db.execute("UPDATE jobs SET cancel=1 WHERE id=?", [job])?;
    if j.status == "queued" {
        crate::storage::finish_tx(db, job, "cancelled", None, None, reason)?;
    }
    Ok(())
}
impl Store {
    pub async fn monitor(&self, job: String, mutation: Option<Mutation>) -> Result<Value> {
        validate_id(&job)?;
        if let Some(mutation) = mutation {
            let key = job.clone();
            self.call(move|db| {
                let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                let j=read_job(&tx,&key)?;
                ensure!(!terminal(&j.status),"monitor changes require an active job");
                match mutation {
                    Mutation::Add {rule} => {add_rule(&tx,&key,&rule)?;},
                    mutation => {
                        let (rule_id, expected)=match &mutation {
                            Mutation::Update {rule_id,expected_revision,..}|Mutation::Enable {rule_id,expected_revision,..}|Mutation::Remove {rule_id,expected_revision} => (rule_id,*expected_revision),
                            _=>unreachable!(),
                        };
                        validate_id(rule_id)?;
                        let old=rules(&tx,&key)?.into_iter().find(|r|&r.id==rule_id && !r.deleted).ok_or_else(||anyhow::anyhow!("active rule not found for this job"))?;
                        ensure!(old.revision==expected,"rule revision changed; inspect before editing");
                        ensure!(expected<128,"rule revision quota (128) reached");
                        let (spec,enabled,deleted)=match &mutation {
                            Mutation::Update {rule,..}=>(rule.clone(),old.enabled,false),
                            Mutation::Enable {enabled,..}=>(old.spec.clone(),*enabled,false),
                            Mutation::Remove {..}=>(old.spec.clone(),false,true),
                            _=>unreachable!(),
                        };
                        spec.validate()?;
                        if spec.notify_agent {ensure!(j.session.is_some(),"notify_agent requires an explicitly bound job session");}
                        tx.execute("UPDATE monitor_rules SET revision=revision+1,spec=?,enabled=?,deleted=? WHERE id=?",params![serde_json::to_string(&spec)?,enabled,deleted,rule_id])?;
                        audit(&tx,&key,rule_id,expected+1,json!({"before":old,"after":{"spec":spec,"enabled":enabled,"deleted":deleted}}))?;
                    }
                }
                tx.commit()?;Ok(())
            }).await?;
        }
        self.read(move|db|{read_job(db,&job)?;Ok(json!({"job_id":job,"rules":rules(db,&job)?,"trigger_policy":"once per revision; enable again to rearm"}))}).await
    }
    pub(crate) async fn trigger_rule(
        &self,
        job: String,
        rule: Rule,
        evidence: Value,
        can_stop: bool,
    ) -> Result<bool> {
        self.call(move|db|{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let j=read_job(&tx,&job)?;
            if terminal(&j.status) {return Ok(false);}
            let changed=tx.execute("UPDATE monitor_rules SET triggered_revision=?,triggered_at=? WHERE id=? AND job_id=? AND revision=? AND enabled=1 AND deleted=0 AND triggered_revision<>revision",params![rule.revision,now(),rule.id,job,rule.revision])?;
            if changed==0 {return Ok(false);}
            crate::storage::insert_event(&tx,&job,&j.run_id,"monitor_alert",json!({"rule_id":rule.id,"revision":rule.revision,"rule":rule.spec,"evidence":evidence,"terminal_exit_observed":!can_stop}))?;
            let event:String=tx.query_row("SELECT id FROM events WHERE seq=last_insert_rowid()",[],|r|r.get(0))?;
            if rule.spec.action==Action::Stop && can_stop {stop_tx(&tx,&job,rule.spec.grace_seconds,&format!("monitor rule {} revision {}",rule.id,rule.revision))?;}
            if rule.spec.notify_agent {
                let mut q=tx.prepare("SELECT id FROM targets WHERE job_id=? AND active=1")?;
                let targets=q.query_map([&job],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                for target in targets {crate::session::enqueue(&tx,&event,&target,now())?;}
            }
            tx.commit()?;Ok(true)
        }).await
    }
    pub(crate) async fn control(&self, job: String) -> Result<(bool, u32, String, Vec<Rule>)> {
        self.read(move |db| {
            let tx = db.transaction()?;
            let cancel: bool =
                tx.query_row("SELECT cancel FROM jobs WHERE id=?", [&job], |r| r.get(0))?;
            let (grace, reason) = tx
                .query_row(
                    "SELECT grace_seconds,reason FROM stop_requests WHERE job_id=?",
                    [&job],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .unwrap_or((5, "manual".into()));
            Ok((cancel, grace, reason, rules(&tx, &job)?))
        })
        .await
    }
    pub(crate) async fn observe(
        &self,
        job: String,
        value: Value,
        applied: Vec<(String, i64)>,
    ) -> Result<()> {
        self.call(move|db| {
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute("INSERT INTO runtime_observations(job_id,data) VALUES(?,?) ON CONFLICT(job_id) DO UPDATE SET data=excluded.data",params![job,value.to_string()])?;
            for (id,revision) in applied {tx.execute("UPDATE monitor_rules SET applied_revision=?,applied_at=? WHERE id=? AND revision=? AND applied_revision<>?",params![revision,now(),id,revision,revision])?;}
            tx.commit()?;Ok(())
        }).await
    }
    pub async fn inspect(
        &self,
        job: Option<String>,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Value> {
        ensure!((1..=100).contains(&limit), "inspect limit must be 1..100");
        if let Some(key) = job {
            validate_id(&key)?;
            let id = key.clone();
            let mut value=self.read(move|db| {
                let tx=db.transaction()?;
                let j=read_job(&tx,&id)?;
                let records=crate::archive::monitor_rows(&tx,&id,"runtime_observations")?;
                let raw=if let Some(rows)=records {rows.first().and_then(|r|r["data"].as_str()).map(str::to_owned)} else {tx.query_row("SELECT data FROM runtime_observations WHERE job_id=?",[&id],|r|r.get::<_,String>(0)).optional()?};
                let observation=raw.map(|r|serde_json::from_str::<Value>(&r)).transpose()?.unwrap_or(Value::Null);
                let stop=if let Some(rows)=crate::archive::monitor_rows(&tx,&id,"stop_requests")? {rows.first().cloned().unwrap_or(Value::Null)}else{tx.query_row("SELECT grace_seconds,reason,at FROM stop_requests WHERE job_id=?",[&id],|r|Ok(json!({"grace_seconds":r.get::<_,u32>(0)?,"reason":r.get::<_,String>(1)?,"at":r.get::<_,i64>(2)?}))).optional()?.unwrap_or(Value::Null)};
                Ok(json!({"job":j,"observation":observation,"rules":rules(&tx,&id)?,"stop":stop,"sampled_at":now()}))
            }).await?;
            value["profile"] = json!(self.config.root);
            // Latest events are bounded independently from full event pagination.
            let event_job = key.clone();
            let events=self.read(move|db| {
                if let Some(mut events)=crate::archive::latest_events(db,&event_job)? {events.reverse();return Ok(events.into_iter().take(30).collect::<Vec<_>>());}
                let mut q=db.prepare("SELECT seq,id,job_id,run_id,kind,at,payload FROM events WHERE job_id=? ORDER BY seq DESC LIMIT 30")?;
                let rows=q.query_map([event_job],|r|Ok(Event{seq:r.get(0)?,event_id:r.get(1)?,job_id:r.get(2)?,run_id:r.get(3)?,kind:r.get(4)?,at:r.get(5)?,payload:serde_json::from_str(&r.get::<_,String>(6)?).map_err(|e|rusqlite::Error::FromSqlConversionFailure(6,rusqlite::types::Type::Text,Box::new(e)))?}))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
            }).await?;
            value["events"] = json!(events);
            value["deliveries"] = self.deliveries(Some(key), 0, 20).await?;
            return Ok(value);
        }
        let mut value=self.read(move|db| {
            let tx=db.transaction()?;
            let mut q=tx.prepare("SELECT id,sequence FROM (SELECT id,rowid AS sequence FROM jobs UNION ALL SELECT id,sequence FROM archive_jobs) WHERE (?1 IS NULL OR sequence<?1) ORDER BY sequence DESC LIMIT ?2")?;
            let ids=q.query_map(params![before,limit],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let jobs=ids.iter().map(|(id,_)|read_job(&tx,id).map(summary)).collect::<Result<Vec<_>>>()?;
            let mut q=tx.prepare("SELECT id FROM jobs WHERE status IN ('queued','launching','running') ORDER BY rowid DESC")?;
            let active=q.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.iter().map(|id|read_job(&tx,id).map(summary)).collect::<Result<Vec<_>>>()?;
            let mut q=tx.prepare("SELECT id,job_id,at,payload FROM events WHERE kind='monitor_alert' ORDER BY seq DESC LIMIT 20")?;
            let alerts=q.query_map([],|r|Ok(json!({"event_id":r.get::<_,String>(0)?,"job_id":r.get::<_,String>(1)?,"at":r.get::<_,i64>(2)?,"payload":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).unwrap_or(Value::Null)})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(json!({"recent_alerts":alerts,"jobs":jobs,"active_jobs":active,"next_cursor":ids.last().map(|(_,s)|s),"sampled_at":now()}))
        }).await?;
        value["profile"] = json!(self.config.root);
        Ok(value)
    }
}

struct ActiveRule {
    rule: Rule,
    since: Instant,
    buffers: [Vec<u8>; 2],
    pending: Option<Value>,
    fired: bool,
}
#[derive(Default)]
struct Tail {
    bytes: u64,
    data: VecDeque<u8>,
    last_output_at: Option<i64>,
}
pub(crate) struct Live {
    start: Instant,
    last_output: Instant,
    tails: [Tail; 2],
    rules: Vec<ActiveRule>,
}
pub(crate) type SharedLive = Arc<Mutex<Live>>;
impl Live {
    pub fn new() -> SharedLive {
        Arc::new(Mutex::new(Self {
            start: Instant::now(),
            last_output: Instant::now(),
            tails: Default::default(),
            rules: vec![],
        }))
    }
    pub fn apply(&mut self, rules: Vec<Rule>) {
        self.rules.retain(|a| {
            rules
                .iter()
                .any(|r| r.id == a.rule.id && r.revision == a.rule.revision)
        });
        for rule in rules {
            if !self
                .rules
                .iter()
                .any(|a| a.rule.id == rule.id && a.rule.revision == rule.revision)
            {
                let fired =
                    !rule.enabled || rule.deleted || rule.triggered_revision == rule.revision;
                self.rules.push(ActiveRule {
                    rule,
                    since: Instant::now(),
                    buffers: Default::default(),
                    pending: None,
                    fired,
                });
            }
        }
    }
    pub fn output(&mut self, stream: usize, bytes: &[u8]) {
        self.last_output = Instant::now();
        let tail = &mut self.tails[stream];
        tail.bytes += bytes.len() as u64;
        tail.last_output_at = Some(now());
        tail.data.extend(bytes);
        if tail.data.len() > TAIL_BYTES {
            tail.data.drain(..tail.data.len() - TAIL_BYTES);
        }
        for active in &mut self.rules {
            if active.fired || active.pending.is_some() {
                continue;
            }
            if let Condition::Contains {
                text,
                stream: which,
            } = &active.rule.spec.condition
            {
                if (*which == Stream::Stdout && stream != 0)
                    || (*which == Stream::Stderr && stream != 1)
                {
                    continue;
                }
                let buffer = &mut active.buffers[stream];
                buffer.extend_from_slice(bytes);
                if buffer
                    .windows(text.len())
                    .any(|part| part == text.as_bytes())
                {
                    active.pending = Some(
                        json!({"stream":if stream==0 {"stdout"}else{"stderr"},"matched":text,"byte_offset_end":tail.bytes,"observed_at":now()}),
                    );
                }
                let keep = text.len().saturating_sub(1);
                if buffer.len() > keep {
                    buffer.drain(..buffer.len() - keep);
                }
            }
        }
    }
    pub fn pending(&mut self) -> Vec<(Rule, Value)> {
        let mut out = vec![];
        for active in &mut self.rules {
            if active.fired {
                continue;
            }
            if active.pending.is_none() {
                let elapsed = match active.rule.spec.condition {
                    Condition::NoOutput { seconds } => {
                        Some((self.last_output.max(active.since).elapsed(), seconds))
                    }
                    Condition::Elapsed { seconds } => Some((self.start.elapsed(), seconds)),
                    _ => None,
                };
                if let Some((duration, seconds)) = elapsed
                    && duration >= Duration::from_secs(seconds)
                {
                    active.pending =
                        Some(json!({"seconds":duration.as_secs(),"observed_at":now()}));
                }
            }
            if let Some(evidence) = &active.pending {
                out.push((active.rule.clone(), evidence.clone()));
            }
        }
        out
    }
    pub fn fired(&mut self, id: &str, revision: i64) {
        for a in &mut self.rules {
            if a.rule.id == id && a.rule.revision == revision {
                a.fired = true;
                a.pending = None;
            }
        }
    }
    pub fn snapshot(&self, phase: &str) -> (Value, Vec<(String, i64)>) {
        let outputs=self.tails.iter().map(|t|json!({"total_bytes":t.bytes,"retained_from":t.bytes-t.data.len() as u64,"text":String::from_utf8_lossy(&t.data.iter().copied().collect::<Vec<_>>()).chars().map(|c|if c.is_control() && !matches!(c,'\n'|'\r'|'\t'){'\u{fffd}'}else{c}).collect::<String>(),"last_output_at":t.last_output_at,"truncated":t.bytes>t.data.len() as u64})).collect::<Vec<_>>();
        (
            json!({"sampled_at":now(),"elapsed_seconds":self.start.elapsed().as_secs(),"stdout":outputs[0],"stderr":outputs[1],"stop_phase":phase,"monitor_health":"running"}),
            self.rules
                .iter()
                .map(|a| (a.rule.id.clone(), a.rule.revision))
                .collect(),
        )
    }
}

pub async fn inspect_client(
    config: &crate::config::Config,
    job: Option<String>,
    before: Option<i64>,
    limit: u32,
) -> Result<Value> {
    let mut value = match crate::ipc::call(
        config,
        crate::ipc::Request::Inspect {
            job_id: job.clone(),
            before,
            limit,
        },
    )
    .await
    {
        Ok(mut v) => {
            v["runtime"] = json!("reachable");
            v
        }
        Err(e) if crate::client::unavailable(&e) => {
            if !config.root.join("relay.sqlite3").exists() {
                return Ok(
                    json!({"runtime":"stopped","profile":config.root,"jobs":[],"active_jobs":[],"sampled_at":now(),"warning":"No recorded state; inspection did not create a profile."}),
                );
            }
            let mut v = Store::readonly(config.clone())
                .inspect(job, before, limit)
                .await?;
            v["runtime"] = json!("offline_snapshot");
            v["warning"] = json!(
                "Persisted snapshot only; process liveness is not verified. No daemon was started."
            );
            v
        }
        Err(e) => return Err(e),
    };
    if let Some(at) = value["observation"]["sampled_at"].as_i64() {
        value["observation"]["stale"] = json!(value["runtime"] != "reachable" || now() - at > 5000);
    }
    Ok(value)
}
fn safe_text(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}
pub fn human(value: &Value) -> String {
    let mut lines = vec![format!(
        "AgentJobRelay · {} · {}",
        value["runtime"].as_str().unwrap_or("reachable"),
        value["profile"].as_str().unwrap_or("")
    )];
    if let Some(warning) = value["warning"].as_str() {
        lines.push(warning.into());
    }
    if let Some(job) = value.get("job") {
        lines.push(format!(
            "{}  {}  {}",
            job["job_id"].as_str().unwrap_or(""),
            job["status"].as_str().unwrap_or(""),
            job["cwd"].as_str().unwrap_or("")
        ));
        lines.push(format!("Stop: {}", value["stop"]));
        for stream in ["stdout", "stderr"] {
            lines.push(format!(
                "{stream} · {} bytes · last output {}\n{}",
                value["observation"][stream]["total_bytes"],
                value["observation"][stream]["last_output_at"],
                value["observation"][stream]["text"].as_str().unwrap_or("")
            ));
        }
        lines.push(format!("Rules: {}", value["rules"]));
        lines.push(format!("Recent events: {}", value["events"]));
    } else {
        let mut seen = std::collections::BTreeSet::new();
        for key in ["active_jobs", "jobs"] {
            if let Some(jobs) = value[key].as_array() {
                for job in jobs {
                    if !seen.insert(job["job_id"].to_string()) {
                        continue;
                    }
                    lines.push(format!(
                        "{}  {:12} {}",
                        job["job_id"].as_str().unwrap_or(""),
                        job["status"].as_str().unwrap_or(""),
                        job["cwd"].as_str().unwrap_or("")
                    ));
                }
            }
        }
    }
    safe_text(&lines.join("\n"))
}

fn summary(job: Job) -> Value {
    let preview = match &job.command {
        CommandSpec::Exec { program, args } => std::iter::once(program.as_str())
            .chain(args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        CommandSpec::Shell { script, .. } => script.clone(),
    };
    json!({"job_id":job.job_id,"run_id":job.run_id,"sequence":job.sequence,"status":job.status,"created_at":job.created_at,"updated_at":job.updated_at,"cancel_requested":job.cancel_requested,"exit_code":job.exit_code,"command_preview":preview.chars().take(256).collect::<String>(),"cwd":job.cwd.to_string_lossy().chars().take(512).collect::<String>(),"run":{"started_at":job.run.started_at,"ended_at":job.run.ended_at}})
}
