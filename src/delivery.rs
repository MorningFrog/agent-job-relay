use crate::{
    core::*,
    session::{AgentSession, read_session},
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
/// A failed identity check needs explicit recovery; it must never select a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Temporary,
    RebindRequired,
    Permanent,
}
impl FailureKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Temporary => "temporary",
            Self::RebindRequired => "rebind_required",
            Self::Permanent => "permanent",
        }
    }
}
#[derive(Debug)]
pub struct DeliveryError(pub FailureKind, pub &'static str);
impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}
impl std::error::Error for DeliveryError {}

fn retry_delay(attempt: u32) -> i64 {
    let base = (1000i64 * 2i64.pow(attempt.saturating_sub(1).min(9))).min(240_000);
    let jitter = uuid::Uuid::new_v4().as_bytes()[0] as i64 * base / 1024;
    base + jitter // At most five minutes, including jitter.
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delivery {
    pub sequence: i64,
    pub id: String,
    pub event_id: String,
    pub target_id: String,
    pub status: String,
    pub attempts: u32,
    pub expires_at: i64,
    pub last_error: Option<String>,
    pub failure_kind: Option<String>,
    pub next_attempt_at: i64,
}
#[derive(Debug, Clone)]
pub struct Claim {
    pub delivery: Delivery,
    pub token: String,
    pub session: AgentSession,
    pub event: Value,
}
impl Store {
    pub async fn deliveries(&self, job: Option<String>, after: i64, limit: u32) -> Result<Value> {
        ensure!(
            after >= 0 && (1..=100).contains(&limit),
            "invalid delivery pagination"
        );
        if let Some(j) = &job {
            validate_id(j)?;
        }
        self.call(move|db|{let mut q=db.prepare("SELECT d.rowid,d.id,d.event_id,d.target_id,d.status,d.attempts,d.expires,d.last_error,d.failure_kind,d.next_at FROM deliveries d JOIN targets t ON d.target_id=t.id WHERE (? IS NULL OR t.job_id=?) AND d.rowid>? ORDER BY d.rowid LIMIT ?")?;
            let rows=q.query_map(params![job,job,after,limit],read_delivery)?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(json!({"next_cursor":rows.last().map_or(after,|d|d.sequence),"deliveries":rows}))
        }).await
    }
    pub async fn retry_delivery(&self, d: String) -> Result<Value> {
        validate_id(&d)?;
        self.call(move|db|{
        let changed=db.execute("UPDATE deliveries SET status='pending',next_at=?,expires=?,last_error=NULL,failure_kind=NULL,token=NULL,lease_until=NULL WHERE id=? AND status IN ('retry_wait','dead_letter')",params![now(),now()+86400000,d])?;
        ensure!(changed==1,"only retry_wait/dead_letter deliveries can be explicitly retried");Ok(json!({"delivery_id":d,"status":"pending","ttl_renewed":true}))
    }).await
    }
    pub async fn attempts(&self, d: String, after: u32, limit: u32) -> Result<Value> {
        validate_id(&d)?;
        ensure!((1..=100).contains(&limit), "invalid attempt limit");
        self.call(move|db|{
        let mut q=db.prepare("SELECT attempt,started,ended,outcome,error FROM delivery_attempts WHERE delivery_id=? AND attempt>? ORDER BY attempt LIMIT ?")?;
        let rows=q.query_map(params![d,after,limit],|r|Ok(json!({"attempt":r.get::<_,u32>(0)?,"started":r.get::<_,i64>(1)?,"ended":r.get::<_,Option<i64>>(2)?,"outcome":r.get::<_,Option<String>>(3)?,"error":r.get::<_,Option<String>>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"attempts":rows,"next_cursor":rows.last().and_then(|v|v["attempt"].as_u64()).unwrap_or(after as u64)}))
    }).await
    }
    pub async fn claim(&self) -> Result<Option<Claim>> {
        self.call(|db|{
        // An empty queue needs only reads, not a write transaction every poll.
        let t=now();
        let expired:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM deliveries WHERE expires<=? AND status IN ('pending','retry_wait','leased'))",[t],|r|r.get(0))?;
        if !expired && candidate(db,t)?.is_none() {return Ok(None);}
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;let t=now();
        tx.execute("UPDATE deliveries SET status='dead_letter',last_error='delivery TTL expired',failure_kind='expired',token=NULL,lease_until=NULL WHERE expires<=? AND status IN ('pending','retry_wait','leased')",[t])?;
        tx.execute("UPDATE delivery_attempts SET ended=?,outcome='ttl_expired',error='delivery TTL expired' WHERE ended IS NULL AND delivery_id IN (SELECT id FROM deliveries WHERE status='dead_letter')",[t])?;
        let Some((d,key))=candidate(&tx,t)? else {tx.commit()?;return Ok(None);};
        tx.execute("UPDATE delivery_attempts SET ended=?,outcome='lease_expired',error='daemon disconnected before recording outcome' WHERE delivery_id=? AND ended IS NULL",params![t,d])?;
        let token=id();tx.execute("UPDATE deliveries SET status='leased',token=?,lease_until=?,attempts=attempts+1 WHERE id=?",params![token,t+10000,d])?;
        let delivery=tx.query_row("SELECT rowid,id,event_id,target_id,status,attempts,expires,last_error,failure_kind,next_at FROM deliveries WHERE id=?",[&d],read_delivery)?;
        tx.execute("INSERT INTO delivery_attempts(delivery_id,attempt,token,started) VALUES(?,?,?,?)",params![d,delivery.attempts,token,t])?;
        let session=read_session(&tx,&key)?;
        let event=tx.query_row("SELECT id,job_id,run_id,kind,at,seq FROM events WHERE id=?",[&delivery.event_id],|r|Ok(json!({"event_id":r.get::<_,String>(0)?,"job_id":r.get::<_,String>(1)?,"run_id":r.get::<_,String>(2)?,"kind":r.get::<_,String>(3)?,"at":r.get::<_,i64>(4)?,"sequence":r.get::<_,i64>(5)?,"untrusted_output":true})))?;
        tx.commit()?;Ok(Some(Claim{delivery,token,session,event}))
    }).await
    }
    pub async fn complete_claim(&self, c: Claim, outcome: Result<&'static str>) -> Result<bool> {
        self.call(move|db|{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;let t=now();
        let (state,error,kind,next_at)=match outcome {
            Ok(s)=>{ensure!(matches!(s,"accepted_by_host"|"notification_recorded"),"invalid adapter acknowledgement");(s,None,None,t)},
            Err(e)=>{
                let kind=e.downcast_ref::<DeliveryError>().map_or(FailureKind::Temporary,|e|e.0);
                let expired=t>=c.delivery.expires_at;
                let state=if expired||kind==FailureKind::Permanent {"dead_letter"}else{"retry_wait"};
                let next=if kind==FailureKind::RebindRequired {c.delivery.expires_at}else{(t+retry_delay(c.delivery.attempts)).min(c.delivery.expires_at)};
                (state,Some(format!("{e:#}").chars().take(1024).collect::<String>()),Some(if expired {"expired"}else{kind.as_str()}),next)
            }
        };
        let n=tx.execute("UPDATE deliveries SET status=?,last_error=?,failure_kind=?,next_at=?,token=NULL,lease_until=NULL WHERE id=? AND status='leased' AND token=? AND lease_until>?",params![state,error,kind,next_at,c.delivery.id,c.token,t])?;
        if n==1 {tx.execute("UPDATE delivery_attempts SET ended=?,outcome=?,error=? WHERE delivery_id=? AND token=?",params![t,state,error,c.delivery.id,c.token])?;}
        tx.commit()?;Ok(n==1)
    }).await
    }
}
fn candidate(db: &rusqlite::Connection, at: i64) -> Result<Option<(String, String)>> {
    // Serialize attempts for the same logical destination, even across host
    // registrations. Other chats can progress while this destination is slow.
    Ok(db.query_row(
        "SELECT d.id,t.session_key FROM deliveries d
         JOIN targets t ON t.id=d.target_id JOIN sessions s ON s.key=t.session_key
         WHERE t.active=1 AND d.expires>?1
         AND ((d.status IN ('pending','retry_wait') AND d.next_at<=?1)
              OR (d.status='leased' AND d.lease_until<=?1))
         AND NOT EXISTS(SELECT 1 FROM deliveries busy
             JOIN targets bt ON bt.id=busy.target_id JOIN sessions bs ON bs.key=bt.session_key
             WHERE busy.id<>d.id AND busy.status='leased' AND busy.lease_until>?1 AND busy.expires>?1
             AND bs.agent=s.agent AND bs.logical=s.logical AND bs.host=s.host
             AND json_extract(bs.registration,'$.adapter')=json_extract(s.registration,'$.adapter'))
         ORDER BY d.rowid LIMIT 1",
        [at],
        |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional()?)
}
fn read_delivery(r: &rusqlite::Row<'_>) -> rusqlite::Result<Delivery> {
    Ok(Delivery {
        sequence: r.get(0)?,
        id: r.get(1)?,
        event_id: r.get(2)?,
        target_id: r.get(3)?,
        status: r.get(4)?,
        attempts: r.get(5)?,
        expires_at: r.get(6)?,
        last_error: r.get(7)?,
        failure_kind: r.get(8)?,
        next_attempt_at: r.get(9)?,
    })
}
pub async fn worker(store: Store) {
    const CONCURRENCY: usize = 4;
    let mut tasks = tokio::task::JoinSet::new();
    let mut backoff = crate::scheduling::IdleBackoff::new();
    let mut next_scan = tokio::time::Instant::now();
    loop {
        tokio::select! {
            _=crate::scheduling::wait_for_work(&store.delivery_changes,next_scan), if tasks.len()<CONCURRENCY => {
                let mut claimed=false;
                while tasks.len()<CONCURRENCY {
                    match store.claim().await {
                        Ok(Some(claim))=>{
                            claimed=true;
                            let service=store.clone();
                            tasks.spawn(async move {deliver_claim(service,claim).await});
                        },
                        Ok(None)=>break,
                        Err(e)=>{tracing::error!(error=%e,"delivery dispatcher failed");break;}
                    }
                }
                next_scan=tokio::time::Instant::now()+backoff.next(claimed);
            },
            Some(result)=tasks.join_next()=>{
                match result {
                    Ok(Ok(()))=>{},
                    Ok(Err(e))=>tracing::error!(error=%e,"delivery attempt failed"),
                    Err(e)=>tracing::error!(error=%e,"delivery worker panicked"),
                }
                backoff.next(true);
                next_scan=tokio::time::Instant::now();
            }
        }
    }
}

async fn deliver_claim(store: Store, mut claim: Claim) -> Result<()> {
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        store.renew_delivery_session(&mut claim).await?;
        crate::adapters::deliver(&store.config, &claim).await
    })
    .await
    .context("host delivery deadline exceeded (3s)")
    .and_then(|r| r);
    store.complete_claim(claim, outcome).await?;
    Ok(())
}
