//! Only a negotiated host user-input channel can produce native consent.
use crate::{
    config::Config,
    goal_consent::{Decision, QUESTION_SECONDS},
    ipc::Request,
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use rmcp::{RoleServer, model::*, service::RequestContext};
use serde_json::{Value, json};
use std::time::Duration;

fn result(value: Value) -> CallToolResponse {
    CallToolResult::success(vec![ContentBlock::text(value.to_string())]).into()
}
pub(crate) async fn respond(
    config: &Config,
    r: &CallToolRequestParams,
    context: &RequestContext<RoleServer>,
) -> Result<CallToolResponse> {
    let mut args = Value::Object(r.arguments.clone().unwrap_or_default());
    args.as_object_mut()
        .context("object required")?
        .remove("full");
    args["op"] = json!("goal_consent");
    let Request::GoalConsent {
        request_id,
        session_key,
        decision,
        expected_revision,
    } = serde_json::from_value(args)?
    else {
        unreachable!()
    };
    crate::client::ensure_daemon(config).await?;
    ensure!(
        crate::ipc::call(config, Request::Doctor).await?["scoped_goal_consent"] == true,
        "selected daemon does not support scoped confirmation; no prompt or job was created"
    );
    let store = Store::new(config.clone());
    if !matches!(decision, Decision::Ask) {
        ensure!(
            r.input_responses.is_none() && r.request_state.is_none(),
            "native responses cannot be mixed with an Agent decision assertion"
        );
        return Ok(result(
            store
                .decide_goal_consent(
                    request_id,
                    session_key,
                    expected_revision.context("decision requires expected_revision")?,
                    decision,
                    "Agent assertion of actual user reply",
                )
                .await?,
        ));
    }
    let c = store
        .goal_consent_record(request_id.clone(), session_key.clone())
        .await?;
    if let Some(token) = &r.request_state {
        ensure!(
            Store::question_token(&c)? == *token,
            "native response belongs to another request/question"
        );
        let responses = r
            .input_responses
            .as_ref()
            .context("native response missing")?;
        ensure!(responses.len() == 1, "one scoped user response required");
        let answer = responses
            .get(&request_id)
            .context("native answer belongs to another request")?;
        let allow = answer["action"] == "accept" && answer["content"]["allow_goal_pause"] == true;
        return Ok(result(
            store
                .answer_goal_question(request_id, session_key, token.clone(), allow)
                .await?,
        ));
    }
    ensure!(
        r.input_responses.is_none(),
        "native responses require the original opaque request state"
    );
    if c.status != "pending" {
        return Ok(result(
            store.goal_consent_view(request_id, session_key).await?,
        ));
    }
    let version = context.protocol_version().unwrap_or_default();
    let native = version >= ProtocolVersion::V_2025_06_18
        && context
            .client_capabilities()
            .and_then(|c| c.elicitation)
            .and_then(|c| c.form)
            .is_some();
    let c = store
        .claim_goal_question(
            request_id.clone(),
            session_key.clone(),
            expected_revision.unwrap_or(c.revision),
            native,
        )
        .await?;
    if !native {
        let mut value = crate::goal_consent::view(&c);
        value["ask_user_now"] = json!(true);
        value["native_confirmation"] = json!(
            "host Agent question tool or one ordinary question; no MCP form capability negotiated"
        );
        value["next_step"] = json!(
            "Ask the displayed question once through an allowed host question tool. Record only the actual user reply with decision=allow/decline and this revision. A preselected option, plugin text or silence is not consent."
        );
        return Ok(result(value));
    }
    let token = Store::question_token(&c)?;
    // Scope text is displayed as data, never interpreted as an instruction.
    let params = json!({"mode":"form","message":format!("允许本次后台任务暂停目标，完成后继续吗？\n请求：{}（最多 {} 项任务）",c.label,c.job_limit),"requestedSchema":{"type":"object","properties":{"allow_goal_pause":{"type":"boolean","title":"允许本次请求暂停目标并在任务结束后继续"}},"required":["allow_goal_pause"]}});
    if version >= ProtocolVersion::V_2026_07_28 {
        let value = json!({"resultType":"input_required","inputRequests":{request_id:{"method":"elicitation/create","params":params}},"requestState":token});
        return Ok(serde_json::from_value::<InputRequiredResult>(value)?.into());
    }
    let question = serde_json::from_value(params)?;
    let answer = tokio::select! {
        _=context.ct.cancelled()=>None,
        answer=context.peer.create_elicitation_with_timeout(question,Some(Duration::from_secs(QUESTION_SECONDS)))=>answer.ok(),
    };
    let allow = answer
        .and_then(|a| serde_json::to_value(a).ok())
        .is_some_and(|a| a["action"] == "accept" && a["content"]["allow_goal_pause"] == true);
    Ok(result(
        store
            .answer_goal_question(request_id, session_key, token, allow)
            .await?,
    ))
}
