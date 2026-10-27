//! Local writes wake workers immediately; bounded fallback checks observe runners
//! and other processes without coupling their lifetimes to the daemon.
use std::time::Duration;
use tokio::{sync::Notify, time::Instant};

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
