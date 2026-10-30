//! Per-listener, demand-driven samples. Cached responses never serve controls.
use crate::{ipc::Request, storage::Store};
use anyhow::Result;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::time::Instant;

pub(super) const MAX_VIEWERS: usize = 8;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const HEALTH_INTERVAL: Duration = Duration::from_secs(10);

pub(super) struct Sampler {
    store: Store,
    overview: Option<(Instant, Value)>,
    health: Option<(Instant, Value)>,
    frames: BTreeMap<Option<String>, Arc<str>>,
}
impl Sampler {
    pub(super) fn new(store: Store) -> Self {
        Self {
            store,
            overview: None,
            health: None,
            frames: BTreeMap::new(),
        }
    }
    pub(super) async fn sample(&mut self, job: Option<String>) -> Result<Arc<str>> {
        self.sample_at(job, Instant::now()).await
    }
    async fn sample_at(&mut self, job: Option<String>, at: Instant) -> Result<Arc<str>> {
        if self
            .overview
            .as_ref()
            .is_none_or(|(sampled, _)| at.duration_since(*sampled) >= SAMPLE_INTERVAL)
        {
            self.frames.clear();
            // Publish only complete successful samples. An expired/erroring
            // query must never fall back to an earlier success.
            let mut overview = self.store.inspect(None, None, 50).await?;
            if self
                .health
                .as_ref()
                .is_none_or(|(sampled, _)| at.duration_since(*sampled) >= HEALTH_INTERVAL)
            {
                let mut health = crate::ipc::dispatch(&self.store, Request::Doctor).await?;
                health["sampled_at"] = json!(crate::core::now());
                self.health = Some((at, health));
            }
            overview["health"] = self.health.as_ref().expect("health sampled").1.clone();
            self.overview = Some((at, overview));
        }
        if let Some(frame) = self.frames.get(&job) {
            return Ok(frame.clone());
        }
        let detail = if let Some(id) = &job {
            self.store.inspect(Some(id.clone()), None, 20).await?
        } else {
            Value::Null
        };
        let overview = &self.overview.as_ref().expect("overview sampled").1;
        let frame: Arc<str> =
            serde_json::to_string(&json!({"overview":overview,"detail":detail}))?.into();
        // Navigation churn must not retain an unbounded number of task tails.
        if self.frames.len() == MAX_VIEWERS {
            self.frames.pop_first();
        }
        self.frames.insert(job, frame.clone());
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, core::*};

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
            title: None,
            command: CommandSpec::Exec {
                program: "never-execute".into(),
                args: vec![],
            },
            cwd: dir.path().into(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
            monitor_rules: vec![],
        };
        (dir, store, spec)
    }
    fn decode(frame: &str) -> Value {
        serde_json::from_str(frame).unwrap()
    }

    #[tokio::test]
    async fn viewers_share_samples_but_controls_and_profiles_stay_fresh() {
        let (_dir, store, spec) = fixture().await;
        let first = store.start(spec.clone()).await.unwrap();
        let sampler = Arc::new(tokio::sync::Mutex::new(Sampler::new(store.clone())));
        let at = Instant::now();
        let mut viewers = tokio::task::JoinSet::new();
        for _ in 0..MAX_VIEWERS {
            let sampler = sampler.clone();
            let key = first.job_id.clone();
            viewers
                .spawn(async move { sampler.lock().await.sample_at(Some(key), at).await.unwrap() });
        }
        let frame = viewers.join_next().await.unwrap().unwrap();
        while let Some(view) = viewers.join_next().await {
            assert!(
                Arc::ptr_eq(&frame, &view.unwrap()),
                "same-task viewers reuse the serialized frame"
            );
        }
        let second = store.start(spec).await.unwrap();
        let mut sampler = sampler.lock().await;
        let same = sampler
            .sample_at(Some(first.job_id.clone()), at + Duration::from_millis(999))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&frame, &same));
        let other = decode(
            &sampler
                .sample_at(Some(second.job_id.clone()), at)
                .await
                .unwrap(),
        );
        assert_eq!(other["detail"]["job"]["job_id"], second.job_id);
        assert_eq!(other["overview"]["jobs"].as_array().unwrap().len(), 1);
        let overview = decode(&sampler.sample_at(None, at + SAMPLE_INTERVAL).await.unwrap());
        assert_eq!(overview["detail"], Value::Null);
        assert_eq!(overview["overview"]["jobs"].as_array().unwrap().len(), 2);
        assert_eq!(overview["overview"]["health"]["active_jobs"], 1);
        let doctor = crate::ipc::dispatch(&store, Request::Doctor).await.unwrap();
        assert_eq!(
            doctor["active_jobs"], 2,
            "explicit diagnosis bypasses viewer cache"
        );
        assert!(
            overview["overview"]["health"]["web_ui"]
                .get("access_url")
                .is_none()
        );
        let refreshed = decode(&sampler.sample_at(None, at + HEALTH_INTERVAL).await.unwrap());
        assert_eq!(refreshed["overview"]["health"]["active_jobs"], 2);
        let (_other_dir, other_store, _) = fixture().await;
        let isolated = decode(&Sampler::new(other_store).sample_at(None, at).await.unwrap());
        assert!(isolated["overview"]["jobs"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn expired_errors_do_not_replay_old_details_and_navigation_is_bounded() {
        let (_dir, store, spec) = fixture().await;
        let mut sampler = Sampler::new(store.clone());
        let at = Instant::now();
        let mut ids = vec![];
        for _ in 0..MAX_VIEWERS + 2 {
            let job = store.start(spec.clone()).await.unwrap();
            store
                .finish(
                    job.job_id.clone(),
                    "completed",
                    Some(0),
                    None,
                    "fixture".into(),
                )
                .await
                .unwrap();
            ids.push(job.job_id);
        }
        for id in &ids {
            sampler.sample_at(Some(id.clone()), at).await.unwrap();
        }
        assert_eq!(sampler.frames.len(), MAX_VIEWERS);
        let id = ids.last().unwrap().clone();
        let key = id.clone();
        store
            .call(move |db| {
                db.execute(
                    "INSERT INTO runtime_observations(job_id,data) VALUES(?,'invalid JSON')",
                    [key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(
            sampler
                .sample_at(Some(id.clone()), at + SAMPLE_INTERVAL)
                .await
                .is_err()
        );
        let key = id.clone();
        store
            .call(move |db| {
                db.execute(
                    r#"UPDATE runtime_observations SET data='{"recovered":true}' WHERE job_id=?"#,
                    [key],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let recovered = decode(
            &sampler
                .sample_at(Some(id), at + SAMPLE_INTERVAL)
                .await
                .unwrap(),
        );
        assert_eq!(recovered["detail"]["observation"]["recovered"], true);
    }
}
