//! Fresh local capacity evidence and admission-only disk pressure policy.
use super::*;
const MIB: u64 = 1024 * 1024;

fn describe(policy: &Policy, available: Option<u64>, error: Option<String>) -> Value {
    let status = match available {
        None => "unavailable",
        Some(bytes)
            if policy.disk_stop_new_jobs_mib > 0
                && bytes < u64::from(policy.disk_stop_new_jobs_mib) * MIB =>
        {
            "critical"
        }
        Some(bytes)
            if policy.disk_warn_mib > 0 && bytes < u64::from(policy.disk_warn_mib) * MIB =>
        {
            "warning"
        }
        Some(_) => "ok",
    };
    json!({"status":status,"available_bytes":available,"sampled_at":now(),
    "warn_below_bytes":u64::from(policy.disk_warn_mib)*MIB,
    "stop_new_jobs_below_bytes":u64::from(policy.disk_stop_new_jobs_mib)*MIB,
    "new_jobs_allowed":match available {Some(bytes)=>policy.disk_stop_new_jobs_mib==0 || bytes>=u64::from(policy.disk_stop_new_jobs_mib)*MIB,None=>policy.disk_stop_new_jobs_mib==0},
    "error":error,"active_workloads":"preserved","disk_reservation":false,
    "reason":match status {
        "critical"=>"Low disk space: new admissions are refused; existing workloads, terminal persistence, stop and unchanged-key retries remain available. Inspect data status and free space explicitly without removing protected history or deliveries.",
        "warning"=>"Disk space is below the warning threshold. Inspect retained history, backup health and archive deferrals; keep active work and recoverable deliveries.",
        "unavailable"=>"Disk capacity could not be sampled. New admissions fail closed when a refusal threshold is enabled; inspection and existing work remain available.",
        _=>"Available disk space is above configured thresholds. This sample does not reserve space or guarantee future writes.",
    }})
}

/// This runs in the existing short IMMEDIATE admission transaction, after both
/// hot/cold idempotency returns and before recording any new execution intent.
pub(crate) fn admit(db: &Connection, root: &Path) -> Result<()> {
    let policy = policy(db)?;
    if policy.disk_stop_new_jobs_mib == 0 {
        return Ok(());
    }
    let available = crate::platform::available_space(root)
        .context("disk capacity unavailable; no new job admitted; inspect data status")?;
    ensure!(
        available >= u64::from(policy.disk_stop_new_jobs_mib) * MIB,
        "disk pressure: {available} bytes available, below new-job threshold {} MiB; no new job admitted; inspect data status, preserve active work and retry with the same original key",
        policy.disk_stop_new_jobs_mib
    );
    Ok(())
}

impl Store {
    pub async fn disk_pressure(&self) -> Result<Value> {
        let policy = self.read(|db| policy(db)).await?;
        let root = self.config.root.clone();
        let available =
            tokio::task::spawn_blocking(move || crate::platform::available_space(&root)).await?;
        Ok(match available {
            Ok(bytes) => describe(&policy, Some(bytes), None),
            Err(error) => describe(&policy, None, Some(format!("{error:#}"))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_thresholds_are_explicit_and_missing_samples_do_not_invent_capacity() {
        let mut policy = Policy::default();
        for (bytes, status, allowed) in [
            (513 * MIB, "ok", true),
            (64 * MIB, "warning", true),
            (63 * MIB, "critical", false),
            (0, "critical", false),
        ] {
            let value = describe(&policy, Some(bytes), None);
            assert_eq!(value["status"], status);
            assert_eq!(value["new_jobs_allowed"], allowed);
        }
        let missing = describe(&policy, None, Some("fixture".into()));
        assert_eq!(missing["status"], "unavailable");
        assert_eq!(missing["new_jobs_allowed"], false);
        policy.disk_stop_new_jobs_mib = 0;
        assert_eq!(describe(&policy, None, None)["new_jobs_allowed"], true);
        policy.disk_warn_mib = 0;
        assert_eq!(describe(&policy, Some(0), None)["status"], "ok");
        policy.disk_warn_mib = 10;
        policy.disk_stop_new_jobs_mib = 11;
        assert!(policy.validate().is_err());
        let old:Policy=serde_json::from_value(json!({"hot_jobs":1000,"archive_after_days":30,"hot_log_mib":256,"backup_hours":24,"backup_keep":3})).unwrap();
        assert_eq!(old.disk_warn_mib, 512);
        assert_eq!(old.disk_stop_new_jobs_mib, 64);
    }
}
