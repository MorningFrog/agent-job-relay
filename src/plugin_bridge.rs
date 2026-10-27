//! Owner-checked plugin bridge; the plugin calls its host's public session API.
#[cfg(any(unix, windows))]
use crate::delivery::{DeliveryError, FailureKind};
use crate::{delivery::Claim, session::AgentSession};
use anyhow::Result;
#[cfg(any(unix, windows))]
use anyhow::{Context, ensure};
#[cfg(any(unix, windows))]
use serde_json::json;

pub async fn exchange(session: &AgentSession, claim: Option<&Claim>) -> Result<()> {
    #[cfg(any(unix, windows))]
    {
        let endpoint = session
            .registration
            .endpoint
            .as_ref()
            .context("plugin bridge endpoint missing")?;
        let unchanged = || {
            session.endpoint_identity.is_some()
                && crate::platform::endpoint_identity(endpoint).ok().as_ref()
                    == session.endpoint_identity.as_ref()
        };
        ensure!(
            unchanged(),
            DeliveryError(
                FailureKind::RebindRequired,
                "plugin endpoint changed or disappeared; explicitly rebind the same session"
            )
        );
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let mut socket = crate::platform::connect_host(endpoint).await?;
            ensure!(crate::platform::connected_endpoint_identity(&socket, endpoint).ok().as_ref()==session.endpoint_identity.as_ref(), DeliveryError(FailureKind::RebindRequired, "plugin endpoint changed while connecting"));
            let r = &session.registration;
            let mut params = json!({"protocol_version":1,"agent_kind":r.agent_kind,"session_id":r.session_id,"caller_session_id":r.caller_thread_id,"instance_id":r.instance_id,"host_id":r.host_id});
            if let Some(c) = claim {
                params["delivery_id"] = json!(c.delivery.id);
                params["event_id"] = c.event["event_id"].clone();
                params["job_id"] = c.event["job_id"].clone();
                params["kind"] = c.event["kind"].clone();
            }
            crate::ipc::write_frame(&mut socket, &json!({"jsonrpc":"2.0","id":1,"method":if claim.is_some(){"relay/deliver"}else{"relay/probe"},"params":params})).await?;
            let response = crate::ipc::read_frame(&mut socket, 16384).await?;
            ensure!(response["jsonrpc"] == "2.0" && response["id"] == 1, DeliveryError(FailureKind::Permanent, "invalid plugin bridge response"));
            ensure!(response.get("error").is_none(), "plugin bridge refused request; inspect its host log (details suppressed)");
            let result = &response["result"];
            ensure!(result["protocol_version"] == 1 && result["agent_kind"] == r.agent_kind && result["session_id"] == r.session_id && result["instance_id"] == r.instance_id && result["host_id"] == r.host_id,
                DeliveryError(FailureKind::RebindRequired, "plugin bridge returned a different session or instance"));
            ensure!(result["can_resume"] == true, DeliveryError(FailureKind::Permanent, "plugin bridge does not support session continuation"));
            if let Some(c) = claim {
                ensure!(result["delivery_id"] == c.delivery.id && result["event_id"] == c.event["event_id"] && result["job_id"] == c.event["job_id"], DeliveryError(FailureKind::Permanent, "plugin bridge acknowledged different delivery metadata"));
                ensure!(result["accepted"] == true, "plugin host did not accept continuation");
            }
            Ok(())
        }).await.context("plugin bridge deadline exceeded (3s)")?
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (session, claim);
        anyhow::bail!("plugin bridge requires verified Linux IPC");
    }
}
