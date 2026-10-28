use crate::{core::*, storage::Store};
use anyhow::{Context, Result, ensure};
use rmcp::schemars;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Clone, Serialize, Deserialize, rmcp::schemars::JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    Inbox,
    DesktopExperimental,
    PluginBridge,
}
#[derive(Debug, Clone, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub agent_kind: String,
    pub session_id: String,
    pub instance_id: String,
    pub host_id: String,
    pub adapter: Adapter,
    pub endpoint: Option<std::path::PathBuf>,
    pub caller_thread_id: Option<String>,
    #[serde(default = "default_lease")]
    pub lease_seconds: u32,
    #[serde(default)]
    pub experimental: bool,
    /// Opt in to same-endpoint capability-verified renewal when delivering results.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub renew_on_delivery: bool,
}
fn default_lease() -> u32 {
    3600
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub key: String,
    pub endpoint_identity: Option<String>,
    pub registration: Registration,
    pub expires_at: i64,
    pub connected: bool,
}
#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub can_notify: bool,
    pub can_resume: bool,
    pub can_confirm_resume: bool,
    pub requires_live_host: bool,
    pub supports_reconnect: bool,
    pub verification: &'static str,
}
impl Adapter {
    pub fn owned_bridge(&self) -> bool {
        matches!(self, Self::DesktopExperimental | Self::PluginBridge)
    }
}
pub fn capabilities(adapter: &Adapter) -> Capabilities {
    match adapter {
        Adapter::Inbox => Capabilities {
            can_notify: true,
            can_resume: false,
            can_confirm_resume: false,
            requires_live_host: false,
            supports_reconnect: true,
            verification: "local inbox only; agent must inspect explicitly",
        },
        Adapter::PluginBridge => Capabilities {
            can_notify: true,
            can_resume: true,
            can_confirm_resume: false,
            requires_live_host: true,
            supports_reconnect: true,
            verification: "experimental plugin bridge; host acceptance is not model execution",
        },
        Adapter::DesktopExperimental => Capabilities {
            can_notify: true,
            can_resume: true,
            can_confirm_resume: false,
            requires_live_host: true,
            supports_reconnect: true,
            verification: "experimental; transport tests simulated; live resume not verified",
        },
    }
}
impl Store {
    pub async fn register(&self, r: Registration) -> Result<AgentSession> {
        for s in [&r.agent_kind, &r.session_id, &r.instance_id, &r.host_id] {
            ensure!(
                !s.is_empty() && s.len() <= 256 && !s.contains('\0'),
                "session identities must be 1..256 bytes"
            );
        }
        ensure!(
            (1..=86400).contains(&r.lease_seconds),
            "lease must be 1..86400 seconds"
        );
        ensure!(
            !r.renew_on_delivery
                || (r.adapter.owned_bridge()
                    && r.caller_thread_id.as_deref() == Some(r.session_id.as_str())),
            "delivery-time renewal requires an owned host caller/destination binding"
        );
        match r.adapter {
            Adapter::Inbox => ensure!(
                r.endpoint.is_none() && r.caller_thread_id.is_none(),
                "inbox registration does not take a host endpoint"
            ),
            Adapter::DesktopExperimental | Adapter::PluginBridge => {
                ensure!(
                    r.experimental,
                    "host adapters require explicit experimental=true opt-in"
                );
                if r.adapter == Adapter::PluginBridge {
                    ensure!(
                        r.caller_thread_id.as_deref() == Some(r.session_id.as_str()),
                        "plugin bridge requires identical caller and destination"
                    );
                }
                let ep = r
                    .endpoint
                    .as_ref()
                    .context("explicit host endpoint required; no session scanning is performed")?;
                crate::platform::validate_endpoint(ep)?;
                ensure!(
                    r.caller_thread_id
                        .as_ref()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 256),
                    "explicit caller_thread_id required"
                );
            }
        }
        let endpoint_identity = r
            .endpoint
            .as_ref()
            .map(|p| crate::platform::endpoint_identity(p))
            .transpose()?;
        self.call(move|db|{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let old:Option<String>=tx.query_row("SELECT key FROM sessions WHERE agent=? AND logical=? AND instance=? AND host=?",params![r.agent_kind,r.session_id,r.instance_id,r.host_id],|row|row.get(0)).optional()?;
            let already_active=if let Some(key)=&old {tx.query_row("SELECT connected=1 AND expires>? FROM sessions WHERE key=?",params![now(),key],|r|r.get::<_,bool>(0))?}else{false};
            if !already_active {let count:i64=tx.query_row("SELECT count(*) FROM sessions WHERE connected=1 AND expires>?",[now()],|r|r.get(0))?;ensure!(count<1000,"active session quota (1000) reached");}
            let key=old.unwrap_or_else(id);let expires=now()+r.lease_seconds as i64*1000;
            tx.execute("INSERT INTO sessions(key,agent,logical,instance,host,registration,expires,connected,endpoint_identity) VALUES(?,?,?,?,?,?,?,1,?) ON CONFLICT(key) DO UPDATE SET registration=excluded.registration,expires=excluded.expires,connected=1,endpoint_identity=excluded.endpoint_identity",params![key,r.agent_kind,r.session_id,r.instance_id,r.host_id,serde_json::to_string(&r)?,expires,endpoint_identity])?;
            tx.commit()?;Ok(AgentSession{key,registration:r,expires_at:expires,connected:true,endpoint_identity})
        }).await
    }
    pub async fn inbox(&self, key: String, after: i64, limit: u32) -> Result<Value> {
        validate_id(&key)?;
        ensure!(
            after >= 0 && (1..=100).contains(&limit),
            "invalid inbox pagination"
        );
        self.call(move|db|{
        read_session(db,&key)?;
        let mut q=db.prepare("SELECT sequence,id,event_id,job_id,kind,status FROM (SELECT d.rowid AS sequence,d.id,e.id AS event_id,e.job_id,e.kind,d.status FROM deliveries d JOIN targets t ON d.target_id=t.id JOIN events e ON e.id=d.event_id WHERE t.session_key=?1 UNION ALL SELECT sequence,id,event_id,job_id,kind,status FROM archive_deliveries WHERE session_key=?1) WHERE sequence>?2 ORDER BY sequence LIMIT ?3")?;
        let rows=q.query_map(params![key,after,limit],|r|Ok(json!({"sequence":r.get::<_,i64>(0)?,"delivery_id":r.get::<_,String>(1)?,"event_id":r.get::<_,String>(2)?,"job_id":r.get::<_,String>(3)?,"kind":r.get::<_,String>(4)?,"status":r.get::<_,String>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"messages":rows,"next_cursor":rows.last().and_then(|v|v["sequence"].as_i64()).unwrap_or(after),"auto_resume":false}))
    }).await
    }
    pub async fn session(&self, key: String) -> Result<AgentSession> {
        validate_id(&key)?;
        self.call(move |db| read_session(db, &key)).await
    }
    pub async fn sessions(&self, after: i64, limit: u32) -> Result<Value> {
        ensure!(
            after >= 0 && (1..=100).contains(&limit),
            "invalid session pagination"
        );
        self.call(move |db| {
            let mut q =
                db.prepare("SELECT rowid,key FROM sessions WHERE rowid>? ORDER BY rowid LIMIT ?")?;
            let keys = q
                .query_map(params![after, limit], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let sessions = keys
                .iter()
                .map(|(_, k)| read_session(db, k))
                .collect::<Result<Vec<_>>>()?;
            Ok(json!({"next_cursor":keys.last().map_or(after,|v|v.0),"sessions":sessions}))
        })
        .await
    }
    pub async fn invalidate(&self, key: String) -> Result<Value> {
        validate_id(&key)?;
        self.call(move |db| {
            read_session(db, &key)?;
            db.execute("UPDATE sessions SET connected=0 WHERE key=?", [key])?;
            Ok(json!({"invalidated":true,"deliveries":"retry until TTL or explicit rebind"}))
        })
        .await
    }
    pub async fn target(&self, j: String, session: String, replay: bool) -> Result<Value> {
        validate_id(&j)?;
        validate_id(&session)?;
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            crate::archive::restore(&tx, &j)?;
            crate::storage::read_job(&tx, &j)?;
            let target = add_target(&tx, &j, &session)?;
            if replay {
                let mut q =
                    tx.prepare("SELECT id,at FROM events WHERE job_id=? AND kind!='started'")?;
                let e = q
                    .query_map([&j], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                drop(q);
                for (event, at) in e {
                    enqueue(&tx, &event, &target, at)?;
                }
            }
            tx.commit()?;
            Ok(json!({"target_id":target,"session_key":session,"replayed":replay}))
        })
        .await
    }
    pub async fn rebind(&self, target: String, session: String) -> Result<Value> {
        validate_id(&target)?;
        validate_id(&session)?;
        self.call(move|db|{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let archived:Option<String>=tx.query_row("SELECT job_id FROM archive_targets WHERE id=?",[&target],|r|r.get(0)).optional()?;
        if let Some(job)=archived {crate::archive::restore(&tx,&job)?;}
        let j:String=tx.query_row("SELECT job_id FROM targets WHERE id=?",[&target],|r|r.get(0)).context("target not found")?;
        let new=add_target(&tx,&j,&session)?;ensure!(new!=target,"new session must differ from current target");
        tx.execute("UPDATE targets SET active=0 WHERE id=?",[&target])?;
        tx.execute("UPDATE deliveries SET status='cancelled',last_error='explicit rebind' WHERE target_id=? AND status IN ('pending','leased','retry_wait','dead_letter')",[target])?;
        let mut q=tx.prepare("SELECT id,at FROM events WHERE job_id=? AND kind!='started'")?;
        let e=q.query_map([j],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;drop(q);
        for (event,at) in e{enqueue(&tx,&event,&new,at)?;}tx.commit()?;Ok(json!({"target_id":new,"session_key":session}))
    }).await
    }
}
pub fn read_session(db: &Connection, key: &str) -> Result<AgentSession> {
    let (raw, expires_at, connected): (String, i64, bool) = db
        .query_row(
            "SELECT registration,expires,connected FROM sessions WHERE key=?",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .context("session not found; register an explicit host identity first")?;
    let endpoint_identity = db.query_row(
        "SELECT endpoint_identity FROM sessions WHERE key=?",
        [key],
        |r| r.get(0),
    )?;
    Ok(AgentSession {
        endpoint_identity,
        key: key.into(),
        registration: serde_json::from_str(&raw)?,
        expires_at,
        connected,
    })
}
pub fn add_target(db: &Connection, j: &str, key: &str) -> Result<String> {
    let session = read_session(db, key)?;
    ensure!(
        session.connected && session.expires_at > now(),
        "session expired or invalidated; refresh registration explicitly"
    );
    let existing: Option<String> = db
        .query_row(
            "SELECT id FROM targets WHERE job_id=? AND session_key=?",
            params![j, key],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        db.execute("UPDATE targets SET active=1 WHERE id=?", [&id])?;
        return Ok(id);
    }
    let count: i64 = db.query_row("SELECT count(*) FROM targets WHERE job_id=?", [j], |r| {
        r.get(0)
    })?;
    ensure!(count < 16, "wake target quota (16 per job) reached");
    let sequence = crate::storage::next_sequence(db, "targets")?;
    db.execute("INSERT INTO targets(rowid,id,job_id,session_key,active) VALUES(?,?,?,?,1) ON CONFLICT(job_id,session_key) DO UPDATE SET active=1",params![sequence,id(),j,key])?;
    Ok(db.query_row(
        "SELECT id FROM targets WHERE job_id=? AND session_key=?",
        params![j, key],
        |r| r.get(0),
    )?)
}
pub fn enqueue(db: &Connection, event: &str, target: &str, at: i64) -> Result<()> {
    let sequence = crate::storage::next_sequence(db, "deliveries")?;
    db.execute("INSERT OR IGNORE INTO deliveries(rowid,id,event_id,target_id,status,attempts,next_at,expires) VALUES(?,?,?,?,'pending',0,?,?)",params![sequence,id(),event,target,now(),at+86400000])?;
    Ok(())
}
