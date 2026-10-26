//! Explicit recovery is scoped to a freshly probed, identical logical caller.
use crate::{
    core::*,
    session::{Adapter, AgentSession, add_target, read_session},
    storage::Store,
};
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

impl Store {
    /// The caller probes `expected` before entering this transaction. A CAS-style
    /// snapshot check fences invalidation/registration changes during that probe.
    pub async fn recover_session(
        &self,
        expected: AgentSession,
        apply: bool,
        after: i64,
        limit: u32,
    ) -> Result<Value> {
        ensure!(
            after >= 0 && (1..=100).contains(&limit),
            "invalid recovery pagination"
        );
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let s = read_session(&tx, &expected.key)?;
            ensure!(s.connected && s.expires_at > now()
                && s.expires_at == expected.expires_at
                && s.endpoint_identity == expected.endpoint_identity
                && serde_json::to_value(&s.registration)? == serde_json::to_value(&expected.registration)?,
                "session changed during recovery probe; obtain a fresh current-terminal binding");
            let r = &s.registration;
            ensure!(r.adapter == Adapter::DesktopExperimental
                && r.caller_thread_id.as_deref() == Some(r.session_id.as_str()),
                "recovery requires a current Desktop binding with identical caller and destination");
            let t = now();
            let mut q = tx.prepare("SELECT t.rowid,t.id,t.job_id FROM targets t
                JOIN sessions s ON s.key=t.session_key JOIN jobs j ON j.id=t.job_id
                WHERE t.active=1 AND t.rowid>? AND s.agent=? AND s.logical=? AND s.host=?
                AND json_extract(s.registration,'$.adapter')='desktop_experimental'
                AND json_extract(s.registration,'$.caller_thread_id')=?
                AND ((t.session_key<>? AND j.status IN ('queued','launching','running'))
                    OR EXISTS(SELECT 1 FROM deliveries d WHERE d.target_id=t.id AND d.expires>?
                        AND (d.status IN ('retry_wait','dead_letter','leased')
                             OR (t.session_key<>? AND d.status='pending'))))
                ORDER BY t.rowid LIMIT ?")?;
            let rows = q.query_map(params![after,r.agent_kind,r.session_id,r.host_id,r.session_id,s.key,t,s.key,limit],
                |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(q);
            let mut targets = Vec::new();
            for (_, old, job) in &rows {
                let existing: Option<String> = tx.query_row("SELECT id FROM targets WHERE job_id=? AND session_key=?",
                    params![job,s.key], |row| row.get(0)).optional()?;
                // In-flight external effects cannot be revoked. Leave both targets
                // untouched so their eventual acknowledgement can still commit.
                let busy: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM deliveries WHERE
                    (target_id=? OR target_id=?) AND status='leased' AND lease_until>?)",
                    params![old,existing,t], |row| row.get(0))?;
                let mut result = json!({"target_id":old,"job_id":job,"in_flight":busy,"applied":false});
                if apply && !busy {
                    let new = add_target(&tx, job, &s.key)?;
                    let mut q = tx.prepare("SELECT id,event_id FROM deliveries WHERE target_id=? AND expires>?
                        AND status IN ('pending','retry_wait','dead_letter','leased')")?;
                    let deliveries = q.query_map(params![old,t], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    drop(q);
                    let mut details = Vec::new();
                    for (delivery,event) in deliveries {
                        let duplicate: Option<String> = tx.query_row("SELECT id FROM deliveries WHERE event_id=? AND target_id=? AND id<>?",
                            params![event,new,delivery], |row| row.get(0)).optional()?;
                        tx.execute("UPDATE delivery_attempts SET ended=?,outcome='recovered',error='explicit same-chat recovery'
                            WHERE delivery_id=? AND ended IS NULL",params![t,delivery])?;
                        if let Some(canonical) = duplicate {
                            tx.execute("UPDATE deliveries SET status='cancelled',token=NULL,lease_until=NULL,last_error='superseded by existing target delivery'
                                WHERE id=?",[&delivery])?;
                            // An already accepted canonical delivery is never replayed.
                            tx.execute("UPDATE deliveries SET status='pending',next_at=?,failure_kind=NULL,last_error=NULL,token=NULL,lease_until=NULL
                                WHERE id=? AND expires>? AND status IN ('retry_wait','dead_letter','leased')",params![t,canonical,t])?;
                            details.push(json!({"delivery_id":delivery,"superseded_by":canonical}));
                        } else {
                            tx.execute("UPDATE deliveries SET target_id=?,status='pending',next_at=?,failure_kind=NULL,last_error=NULL,token=NULL,lease_until=NULL WHERE id=?",
                                params![new,t,delivery])?;
                            details.push(json!({"delivery_id":delivery,"action":"retry","ttl_renewed":false}));
                        }
                    }
                    if *old != new {
                        tx.execute("UPDATE targets SET active=0 WHERE id=?",[old])?;
                    }
                    let recovery = id();
                    tx.execute("INSERT INTO target_recoveries(id,at,from_target,to_target,session_key,details) VALUES(?,?,?,?,?,?)",
                        params![recovery,t,old,new,s.key,serde_json::to_string(&details)?])?;
                    result["applied"] = json!(true);
                    result["new_target_id"] = json!(new);
                    result["recovery_id"] = json!(recovery);
                    result["deliveries"] = json!(details);
                }
                targets.push(result);
            }
            let mut q=tx.prepare("SELECT id,at,from_target,to_target,details FROM target_recoveries WHERE session_key=? ORDER BY rowid DESC LIMIT ?")?;
            let history=q.query_map(params![s.key,limit],|row|Ok(json!({"recovery_id":row.get::<_,String>(0)?,"at":row.get::<_,i64>(1)?,"from_target":row.get::<_,String>(2)?,"to_target":row.get::<_,String>(3)?,"deliveries":serde_json::from_str::<Value>(&row.get::<_,String>(4)?).unwrap_or_default()})))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(q);
            tx.commit()?;
            Ok(json!({"session_key":s.key,"apply":apply,"targets":targets,"next_cursor":rows.last().map_or(after,|r|r.0),
                "recent_recoveries":history,"next_step":"Use --apply to recover this page; retry in-flight targets after their outcome is recorded. No command is rerun; expired or accepted events are not replayed."}))
        }).await
    }
}
