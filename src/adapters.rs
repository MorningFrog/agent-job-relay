use crate::{config::Config, delivery::Claim, session::Adapter};
use anyhow::{Result, bail, ensure};
use serde_json::json;
/// Capability adapters never select or discover a substitute session.
pub async fn deliver(config: &Config, c: &Claim) -> Result<&'static str> {
    ensure!(
        c.session.connected && c.session.expires_at > crate::core::now(),
        "session lease expired/invalidated; refresh or explicitly rebind"
    );
    match c.session.registration.adapter {
        Adapter::Inbox => {
            let dir = config.root.join("inbox").join(&c.session.key);
            crate::platform::secure_dir(&dir)?;
            let path = dir.join(format!("{}.json", c.delivery.id));
            let message = json!({"delivery_id":c.delivery.id,"session_key":c.session.key,"session_id":c.session.registration.session_id,"instance_id":c.session.registration.instance_id,"event":c.event,"can_resume":false});
            let raw = serde_json::to_vec(&message)?;
            let temp = dir.join(format!(".{}.tmp", crate::core::id()));
            let f = crate::platform::private_file(&temp)?;
            let mut file = tokio::fs::File::from_std(f);
            use tokio::io::AsyncWriteExt;
            file.write_all(&raw).await?;
            file.sync_all().await?;
            drop(file);
            tokio::fs::rename(&temp, &path).await?;
            let d = std::fs::File::open(dir)?;
            tokio::task::spawn_blocking(move || d.sync_all()).await??;
            Ok("notification_recorded")
        }
        Adapter::DesktopExperimental => bail!(
            "experimental Desktop transport is not enabled in this increment; no CLI fallback was attempted"
        ),
    }
}
