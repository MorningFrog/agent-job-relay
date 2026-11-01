//! Local writes wake workers immediately; bounded fallback checks observe runners
//! and other processes without coupling their lifetimes to the daemon.
use std::time::Duration;
use tokio::{sync::Notify, time::Instant};

pub(crate) const EXECUTION_LIMIT: u32 = 8;
pub(crate) const WAITING_LIMIT: u32 = 64;

impl crate::storage::Store {
    /// Reserve a slot and launch nonce in one write transaction. Competing ticks,
    /// cancellation and deadline expiry cannot bypass FIFO or execute twice.
    pub(crate) async fn claim_queued(&self, job_id: String) -> anyhow::Result<Option<String>> {
        use crate::{
            core::*,
            storage::{finish_tx, read_job},
        };
        use rusqlite::params;
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let job = read_job(&tx, &job_id)?;
            if job.status != JobState::Queued { return Ok(None); }
            if job.run.started_at.is_none() && job.queue.as_ref().is_some_and(|q| q.expires_at <= now()) {
                finish_tx(&tx, &job_id, JobState::Timeout, None, None,
                    "queue deadline expired before first launch; workload/observation was not started; external outcomes remain unobserved")?;
                tx.commit()?;
                return Ok(None);
            }
            let occupied: u32 = tx.query_row("SELECT count(*) FROM jobs WHERE status IN ('launching','running')", [], |r| r.get(0))?;
            let older: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE status='queued' AND rowid<?)", [job.sequence], |r| r.get(0))?;
            if occupied >= EXECUTION_LIMIT || older { return Ok(None); }
            tx.execute("UPDATE jobs SET status='launching',updated=? WHERE id=? AND status='queued'", params![now(),job_id])?;
            tx.execute("UPDATE runs SET status='launching' WHERE job_id=?", [&job_id])?;
            let nonce = tx.query_row("SELECT nonce FROM runs WHERE job_id=?", [&job_id], |r| r.get(0))?;
            tx.commit()?;
            Ok(Some(nonce))
        }).await
    }
}

pub(crate) struct IdleBackoff {
    delay: Duration,
}
impl IdleBackoff {
    pub(crate) fn new() -> Self {
        Self {
            delay: Duration::from_millis(100),
        }
    }
    pub(crate) fn next(&mut self, active: bool) -> Duration {
        let delay = if active {
            Duration::from_millis(100)
        } else {
            self.delay
        };
        self.delay = if active {
            delay
        } else {
            (delay * 2).min(Duration::from_secs(1))
        };
        delay
    }
}

pub(crate) async fn wait_for_work(changes: &Notify, deadline: Instant) {
    tokio::select! {
        _ = changes.notified() => {},
        _ = tokio::time::sleep_until(deadline) => {},
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, core::*, storage::Store};

    async fn fixture() -> (tempfile::TempDir, Store, StartSpec) {
        crate::platform::initialize();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        config.prepare().unwrap();
        let store = Store::new(config);
        store.init().await.unwrap();
        let spec = StartSpec {
            command: CommandSpec::Exec {
                program: "never-execute".into(),
                args: vec![],
            },
            cwd: dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            queue_timeout_seconds: Some(60),
            session: None,
            idempotency_key: None,
            monitor_rules: vec![],
            title: None,
        };
        (dir, store, spec)
    }

    #[tokio::test]
    async fn concurrent_admission_is_bounded_and_retries_keep_deadlines_at_capacity() {
        let (_dir, store, spec) = fixture().await;
        let mut requests = tokio::task::JoinSet::new();
        for index in 0..80 {
            let mut spec = spec.clone();
            spec.idempotency_key = Some(format!("queue-{index}"));
            let store = store.clone();
            requests.spawn(async move { (spec.clone(), store.start(spec).await) });
        }
        let mut accepted = Vec::new();
        let mut rejected = 0;
        while let Some(result) = requests.join_next().await {
            match result.unwrap() {
                (spec, Ok(job)) => accepted.push((spec, job)),
                (_, Err(error)) => {
                    assert!(error.to_string().contains("waiting job quota"));
                    rejected += 1;
                }
            }
        }
        assert_eq!((accepted.len(), rejected), (72, 8));
        let (spec, first) = &accepted[0];
        let retried = store.start(spec.clone()).await.unwrap();
        assert_eq!(retried.job_id, first.job_id);
        assert_eq!(retried.run_id, first.run_id);
        assert_eq!(
            retried.queue.as_ref().unwrap().expires_at,
            first.queue.as_ref().unwrap().expires_at
        );
        let mut changed = spec.clone();
        changed.queue_timeout_seconds = Some(120);
        assert!(
            store
                .start(changed)
                .await
                .unwrap_err()
                .to_string()
                .contains("different specification")
        );
        let mut legacy = spec.clone();
        legacy.idempotency_key = None;
        legacy.queue_timeout_seconds = None;
        assert!(
            store
                .start(legacy)
                .await
                .unwrap_err()
                .to_string()
                .contains("active job quota")
        );
        for seconds in [0, 604801] {
            let mut invalid = spec.clone();
            invalid.queue_timeout_seconds = Some(seconds);
            assert!(
                store
                    .start(invalid)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("queue timeout")
            );
        }
    }

    #[tokio::test]
    async fn slot_claims_fence_fifo_duplicate_claims_cancellation_and_expiry() {
        let (_dir, store, spec) = fixture().await;
        let mut jobs = Vec::new();
        for _ in 0..10 {
            jobs.push(store.start(spec.clone()).await.unwrap());
        }
        assert!(
            store
                .claim_queued(jobs[1].job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        let mut claims = tokio::task::JoinSet::new();
        for _ in 0..16 {
            let store = store.clone();
            let job = jobs[0].job_id.clone();
            claims.spawn(async move { store.claim_queued(job).await.unwrap() });
        }
        let mut claimed = 0;
        while let Some(result) = claims.join_next().await {
            claimed += usize::from(result.unwrap().is_some());
        }
        assert_eq!(claimed, 1);
        for job in &jobs[1..8] {
            assert!(
                store
                    .claim_queued(job.job_id.clone())
                    .await
                    .unwrap()
                    .is_some()
            );
        }
        assert!(
            store
                .claim_queued(jobs[8].job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        store.stop(jobs[8].job_id.clone()).await.unwrap();
        assert!(
            store
                .claim_queued(jobs[8].job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        let expired = jobs[9].job_id.clone();
        store
            .call(move |db| {
                db.execute(
                    "UPDATE jobs SET created=created-61000 WHERE id=?",
                    [expired],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        // Expiration does not need a free execution slot.
        assert!(
            store
                .claim_queued(jobs[9].job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        let expired = store.status(jobs[9].job_id.clone()).await.unwrap();
        assert_eq!(expired.status, JobState::Timeout);
        assert!(expired.run.started_at.is_none());
        assert!(expired.detail.unwrap().contains("queue deadline"));
        assert_eq!(
            store
                .events(expired.job_id.clone(), 0, 20)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            store
                .claim_queued(expired.job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(store.events(expired.job_id, 0, 20).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn queued_remote_cancel_is_local_and_observer_recovery_keeps_its_run() {
        let (_dir, store, mut spec) = fixture().await;
        spec.command = CommandSpec::Remote {
            remote: Box::new(crate::remote::RemoteSpec {
                target_id: id(),
                transport: crate::remote::Transport {
                    program: "never-contact".into(),
                    args: vec![],
                },
                intent: crate::remote::Intent::Attach {
                    run: crate::remote::RunRef {
                        job_id: id(),
                        run_id: id(),
                        attempt: 1,
                    },
                    allow_stop: true,
                },
            }),
        };
        let cancelled = store.start(spec.clone()).await.unwrap();
        assert_eq!(
            store.stop(cancelled.job_id.clone()).await.unwrap().status,
            JobState::Cancelled
        );
        assert!(
            store
                .claim_queued(cancelled.job_id)
                .await
                .unwrap()
                .is_none()
        );
        let recovery = store.start(spec).await.unwrap();
        let key = recovery.job_id.clone();
        store
            .call(move |db| {
                db.execute("UPDATE runs SET started=1 WHERE job_id=?", [&key])?;
                db.execute("UPDATE jobs SET created=created-61000 WHERE id=?", [&key])?;
                Ok(())
            })
            .await
            .unwrap();
        // A recovered observer has already started once; first-launch expiry
        // cannot replace its remote outcome or prevent exact-run reconnect.
        assert!(
            store
                .claim_queued(recovery.job_id.clone())
                .await
                .unwrap()
                .is_some()
        );
        let claimed = store.status(recovery.job_id).await.unwrap();
        assert_eq!(claimed.status, JobState::Launching);
        assert_eq!(claimed.run_id, recovery.run_id);
        assert_eq!(claimed.run.attempt, 1);
    }

    #[tokio::test]
    async fn restored_queue_is_orphaned_and_archived_retry_preserves_original_deadline() {
        let (dir, store, mut spec) = fixture().await;
        spec.idempotency_key = Some("restore-queue".into());
        let job = store.start(spec.clone()).await.unwrap();
        let backup = store.backup().await.unwrap();
        let destination = dir.path().join("restored");
        crate::maintenance::restore(
            std::path::PathBuf::from(backup["path"].as_str().unwrap()),
            destination.clone(),
        )
        .await
        .unwrap();
        let restored = Store::new(Config::new(Some(destination)).unwrap());
        restored.init().await.unwrap();
        let result = restored.status(job.job_id.clone()).await.unwrap();
        assert_eq!(result.status, JobState::Orphaned);
        assert_eq!(result.run_id, job.run_id);
        assert!(
            restored
                .claim_queued(job.job_id.clone())
                .await
                .unwrap()
                .is_none()
        );
        store.stop(job.job_id.clone()).await.unwrap();
        store.archive_job(job.job_id.clone()).await.unwrap();
        let retry = store.start(spec).await.unwrap();
        assert_eq!(retry.job_id, job.job_id);
        assert_eq!(retry.status, JobState::Cancelled);
        assert_eq!(
            retry.queue.unwrap().expires_at,
            job.queue.unwrap().expires_at
        );
    }
}
