//! No periodic host polling: long-job bindings renew only when a result is due.
use crate::{
    core::*,
    delivery::{Claim, DeliveryError, FailureKind},
    session::read_session,
    storage::Store,
};
use anyhow::{Result, ensure};
use rusqlite::params;
use serde_json::json;

impl Store {
    pub async fn renew_delivery_session(&self, claim: &mut Claim) -> Result<()> {
        let expected = claim.session.clone();
        if !expected.registration.renew_on_delivery || expected.expires_at > now() + 3_600_000 {
            return Ok(());
        }
        ensure!(
            expected.connected,
            DeliveryError(
                FailureKind::RebindRequired,
                "session invalidated; explicit recovery required"
            )
        );
        let mut probe = expected.clone();
        probe.expires_at = now() + expected.registration.lease_seconds as i64 * 1000;
        // The adapter verifies the original endpoint fingerprint and owner, then
        // tools/list capability. It neither discovers a new host nor sends a message.
        crate::adapters::probe(&probe).await?;
        let delivery = claim.delivery.id.clone();
        let token = claim.token.clone();
        let target = claim.delivery.target_id.clone();
        let updated = self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let mut current = read_session(&tx,&expected.key)?;
            ensure!(current.connected && current.endpoint_identity==expected.endpoint_identity
                && serde_json::to_value(&current.registration)?==serde_json::to_value(&expected.registration)?,
                DeliveryError(FailureKind::RebindRequired,"session changed during renewal; explicit recovery required"));
            ensure!(current.expires_at==expected.expires_at,
                DeliveryError(FailureKind::Temporary,"lease refreshed concurrently; retry with current registration"));
            let t=now();
            let owns:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM deliveries d JOIN targets t ON t.id=d.target_id
                WHERE d.id=? AND d.target_id=? AND d.status='leased' AND d.token=? AND d.lease_until>?
                AND d.expires>? AND t.active=1 AND t.session_key=?)",
                params![delivery,target,token,t,t,expected.key],|r|r.get(0))?;
            ensure!(owns,"delivery changed during renewal; no host message sent");
            current.expires_at=t+current.registration.lease_seconds as i64*1000;
            tx.execute("UPDATE sessions SET expires=? WHERE key=?",params![current.expires_at,current.key])?;
            let details=json!([{"action":"lease_renewal","delivery_id":delivery,
                "previous_expires_at":expected.expires_at,"expires_at":current.expires_at,
                "evidence":"same endpoint fingerprint, owner and tools/list capability probe"}]);
            tx.execute("INSERT INTO target_recoveries(id,at,from_target,to_target,session_key,details) VALUES(?,?,?,?,?,?)",
                params![id(),t,target,target,current.key,serde_json::to_string(&details)?])?;
            tx.commit()?;
            Ok(current)
        }).await?;
        claim.session = updated;
        Ok(())
    }
}
