use crate::{
    config::Config,
    delivery::{Claim, DeliveryError, FailureKind},
    session::{Adapter, AgentSession},
};
#[cfg(any(unix, windows))]
use anyhow::Context;
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
/// Capability adapters never select or discover a substitute session.
pub async fn probe(session: &AgentSession) -> Result<Value> {
    ensure!(
        session.connected && session.expires_at > crate::core::now(),
        DeliveryError(
            FailureKind::RebindRequired,
            "session lease expired/invalidated; refresh or explicitly rebind"
        )
    );
    match session.registration.adapter {
        Adapter::Inbox => {
            Ok(json!({"transport":"local inbox","available":true,"can_resume":false}))
        }
        Adapter::PluginBridge => {
            crate::plugin_bridge::exchange(session, None).await?;
            Ok(
                json!({"transport":"owned local plugin endpoint","available":true,"can_confirm_resume":false}),
            )
        }
        Adapter::DesktopExperimental => {
            desktop(session, DesktopOperation::Probe).await?;
            Ok(
                json!({"transport":"experimental local Desktop endpoint","available":true,"can_confirm_resume":false,"live_e2e":"not verified"}),
            )
        }
    }
}
pub async fn deliver(config: &Config, c: &Claim) -> Result<&'static str> {
    ensure!(
        c.session.connected && c.session.expires_at > crate::core::now(),
        DeliveryError(
            FailureKind::RebindRequired,
            "session lease expired/invalidated; refresh or explicitly rebind"
        )
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
            crate::platform::publish_file(&temp, &path)?;
            crate::platform::sync_directory(&dir)?;
            Ok("notification_recorded")
        }
        Adapter::PluginBridge => {
            crate::plugin_bridge::exchange(&c.session, Some(c)).await?;
            Ok("accepted_by_host")
        }
        Adapter::DesktopExperimental => {
            desktop(&c.session, DesktopOperation::Deliver(c)).await?;
            Ok("accepted_by_host")
        }
    }
}
enum DesktopOperation<'a> {
    Probe,
    Deliver(&'a Claim),
    ReadActivity,
}

/// Read the owning host, not a separate app-server's always-unloaded runtime.
/// A native pause acknowledgement does not mean its submitting turn has ended.
pub(crate) async fn desktop_idle(session: &AgentSession) -> Result<bool> {
    ensure!(
        session.registration.adapter == Adapter::DesktopExperimental,
        "goal restoration requires the original Desktop host"
    );
    let snapshot = desktop(session, DesktopOperation::ReadActivity).await?;
    ensure!(
        snapshot["schemaVersion"] == 1
            && snapshot["thread"]["id"] == session.registration.session_id,
        "Desktop activity snapshot has an unsupported schema or different chat"
    );
    let status = snapshot["thread"]["status"]["type"]
        .as_str()
        .context("Desktop activity snapshot has no runtime status")?;
    match status {
        "active" => Ok(false),
        "idle" | "notLoaded" => {
            let turns = snapshot["turns"]
                .as_array()
                .context("Desktop activity snapshot has no turn summaries")?;
            ensure!(turns.len() <= 1, "Desktop returned too many turn summaries");
            for turn in turns {
                match turn["status"].as_str() {
                    Some("inProgress") => return Ok(false),
                    Some("completed" | "interrupted" | "failed") => {}
                    _ => bail!("Desktop returned an unknown turn status"),
                }
            }
            Ok(true)
        }
        _ => bail!("Desktop returned an unknown runtime status"),
    }
}

async fn desktop(session: &AgentSession, operation: DesktopOperation<'_>) -> Result<Value> {
    #[cfg(any(unix, windows))]
    {
        let endpoint = session
            .registration
            .endpoint
            .as_ref()
            .context("Desktop endpoint missing")?;
        // Unix can inspect the socket without opening it. A Windows fingerprint
        // consumes a pipe instance; authenticate the actual connection below so
        // a temporarily busy instance gets the bounded asynchronous wait.
        #[cfg(unix)]
        {
            let current = crate::platform::endpoint_identity(endpoint).map_err(|_| {
                anyhow::anyhow!(
                    "Desktop endpoint unavailable; refresh registration after host restart"
                )
            })?;
            ensure!(
                session.endpoint_identity.as_ref() == Some(&current),
                DeliveryError(
                    FailureKind::RebindRequired,
                    "Desktop endpoint instance changed; explicitly refresh registration or rebind"
                )
            );
        }
        ensure!(
            session.endpoint_identity.is_some(),
            DeliveryError(
                FailureKind::RebindRequired,
                "Desktop endpoint identity missing; explicitly refresh registration"
            )
        );
        tokio::time::timeout(std::time::Duration::from_secs(3),async {
            let mut s=crate::platform::connect_host(endpoint).await.map_err(|_|anyhow::anyhow!("Desktop connection unavailable; no fallback was attempted"))?;
            ensure!(crate::platform::connected_endpoint_identity(&s, endpoint).ok().as_ref()==session.endpoint_identity.as_ref(),DeliveryError(FailureKind::RebindRequired,"Desktop endpoint changed while connecting; refresh registration"));
            let tools=rpc(&mut s,1,"tools/list",json!({"threadStartKind":"all"})).await?;
            let list=tools.get("tools").or_else(||tools.get("toolsList")).and_then(Value::as_array).ok_or(DeliveryError(FailureKind::Permanent,"malformed Desktop tools catalog"))?;
            ensure!(list.iter().any(|t|(t["namespace"]=="codex_app"&&t["name"]=="send_message_to_thread")||t["name"]=="codex_app/send_message_to_thread"),DeliveryError(FailureKind::Permanent,"Desktop does not advertise send_message_to_thread; no fallback was attempted"));
            if matches!(operation, DesktopOperation::ReadActivity) {
                ensure!(list.iter().any(|t|(t["namespace"]=="codex_app"&&t["name"]=="read_thread")||t["name"]=="codex_app/read_thread"),"Desktop does not advertise read_thread; goal restoration deferred without guessing turn completion");
                let call_id=crate::core::id();
                let result=rpc(&mut s,2,"tools/call",json!({"callerSource":"codex","namespace":"codex_app","tool":"read_thread","threadId":session.registration.caller_thread_id,"callId":call_id,"turnId":call_id,"arguments":{"threadId":session.registration.session_id,"turnLimit":1,"includeOutputs":false,"maxOutputCharsPerItem":0}})).await?;
                ensure!(result["success"]==true,"Desktop did not acknowledge activity read");
                let content=result["contentItems"].as_array().context("Desktop activity read has no content")?;
                // Native Desktop contentItems use inputText, while normalized
                // tool responses use text. Both carry the same JSON snapshot.
                ensure!(content.len()==1 && matches!(content[0]["type"].as_str(),Some("inputText" | "text")),"Desktop activity read has an unsupported content envelope");
                return serde_json::from_str(content[0]["text"].as_str().context("Desktop activity read has no snapshot")?).context("Desktop activity snapshot is not JSON");
            }
            if let DesktopOperation::Deliver(c)=operation {
                let prompt = notification_prompt(&c.event);
                let result=rpc(&mut s,2,"tools/call",json!({"callerSource":"codex","namespace":"codex_app","tool":"send_message_to_thread","threadId":session.registration.caller_thread_id,"callId":c.delivery.id,"turnId":c.delivery.id,"arguments":{"threadId":session.registration.session_id,"prompt":prompt}})).await?;
                ensure!(result["success"]==true,"Desktop did not acknowledge acceptance; agent resume is unconfirmed");
            }
            Ok(Value::Null)
        }).await.context("Desktop sequence deadline exceeded (3s)")?
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (session, operation);
        bail!("experimental Desktop transport requires a verified native platform implementation")
    }
}
#[cfg(any(unix, windows))]
fn notification_prompt(event: &Value) -> String {
    use std::fmt::Write;
    let job = event["job_id"].as_str().unwrap_or("unknown");
    let run = event["run_id"].as_str().unwrap_or("unknown");
    let wait = event["goal_wait_id"].as_str();
    if event["action"] == "report_then_pause" {
        let wait = wait.unwrap_or("unknown");
        let request = event["request_id"].as_str().unwrap_or("unscoped");
        let revision = &event["goal_wait_revision"];
        let coalesced = event["goal_wait_feedback"]["context"]["coalesced_count"]
            .as_u64()
            .unwrap_or(0);
        return format!(
            "AgentJobRelay goal feedback: job {job}, run {run}, request {request}, wait {wait}, revision {revision}. Report the alert ({} earlier combined); use job_goal_wait or job_inspect only for needed evidence. Native-pause this same Goal, then confirm with job_goal_wait(confirm_paused=true, expected_revision={revision}) even if jobs finished. Confirm an already acknowledged pause directly; preserve any uncertain handoff.",
            coalesced
        );
    }
    if event["action"] == "report_feedback" {
        return format!(
            "AgentJobRelay event: job {job}, run {run} emitted a monitor alert. Use job_inspect for the evidence, report requested feedback, then yield if still running."
        );
    }
    let mut prompt = if let Some(wait) = wait {
        let request = event["request_id"].as_str().unwrap_or("unscoped");
        format!("AgentJobRelay goal wait: final results for request {request}, wait {wait}.")
    } else {
        "AgentJobRelay event:".into()
    };
    if let Some(results) = event["results"].as_array() {
        for result in results {
            let job = result["job_id"].as_str().unwrap_or("unknown");
            let run = result["run_id"].as_str().unwrap_or("unknown");
            let status = result["status"].as_str().unwrap_or_else(|| {
                if wait.is_none() {
                    event["kind"].as_str().unwrap_or("unknown")
                } else {
                    "outcome unavailable"
                }
            });
            let separator = if wait.is_some() { "\n" } else { " " };
            let observation = if result["read_only_observation"] == true {
                " (read-only observation)"
            } else {
                ""
            };
            let _ = write!(
                prompt,
                "{separator}Job {job}, run {run}{observation}: {status}"
            );
            if let Some(code) = result["exit_code"].as_i64() {
                let _ = write!(prompt, ", exit {code}");
            }
            if let Some(signal) = result["signal"].as_i64() {
                let _ = write!(prompt, ", signal {signal}");
            }
            if let Some(duration) = result["duration_seconds"].as_u64() {
                let _ = write!(prompt, ", {duration}s");
            }
            prompt.push('.');
        }
    }
    prompt.push_str(" Summarize the outcome. Use job_inspect only for needed evidence.");
    prompt
}

#[cfg(any(unix, windows))]
async fn rpc(
    s: &mut crate::platform::ClientStream,
    id: u32,
    method: &str,
    params: Value,
) -> Result<Value> {
    crate::ipc::write_frame(
        s,
        &json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
    )
    .await?;
    for _ in 0..64 {
        let v = crate::ipc::read_frame(s, 1048576).await?;
        ensure!(v["jsonrpc"] == "2.0", "malformed Desktop JSON-RPC response");
        if v["id"] != id {
            continue;
        }
        ensure!(
            v.get("error").is_none(),
            "Desktop rejected request (host error details suppressed)"
        );
        return v
            .get("result")
            .cloned()
            .context("Desktop response has no result");
    }
    bail!("Desktop response correlation failed (too many unrelated frames)")
}
