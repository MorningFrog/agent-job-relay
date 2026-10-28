//! Ordered partial admission; execution and idempotency remain per job.
use crate::{core::StartSpec, storage::Store};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::HashSet;

pub const MAX_JOBS: usize = 16;
pub const MAX_BYTES: usize = 131072;

/// Reject an invalid envelope before admitting any item, including on raw IPC.
pub fn validate(specs: &[StartSpec]) -> Result<()> {
    ensure!(
        (1..=MAX_JOBS).contains(&specs.len()),
        "batch must contain 1..16 specifications"
    );
    let mut keys = HashSet::new();
    for spec in specs {
        let key = spec.idempotency_key.as_deref().unwrap_or_default();
        ensure!(
            !key.is_empty() && key.len() <= 128,
            "every batch item requires an idempotency_key of 1..128 bytes"
        );
        ensure!(keys.insert(key), "batch idempotency keys must be distinct");
    }
    ensure!(
        serde_json::to_vec(
            &json!({"protocol_version":1,"request":{"op":"start_batch","specs":specs}})
        )?
        .len()
            <= MAX_BYTES,
        "batch request exceeds 128 KiB"
    );
    Ok(())
}

pub async fn submit(store: &Store, specs: Vec<StartSpec>) -> Result<Value> {
    submit_for_goal(store, specs, None).await
}
pub async fn submit_for_goal(
    store: &Store,
    specs: Vec<StartSpec>,
    goal_wait: Option<String>,
) -> Result<Value> {
    validate(&specs)?;
    if let Some(id) = &goal_wait {
        crate::core::validate_id(id)?;
    }
    let mut results = Vec::with_capacity(specs.len());
    let mut accepted = 0;
    for (index, spec) in specs.into_iter().enumerate() {
        let key = spec.idempotency_key.clone();
        // Each start has its own IMMEDIATE transaction and existing quota fence.
        // A disconnected viewer cannot roll back or couple accepted jobs.
        match store.start_for_goal(spec, goal_wait.clone()).await {
            Ok(job) => {
                accepted += 1;
                let result = crate::ipc::start_result(store, job).await;
                results.push(json!({"index":index,"idempotency_key":key,"accepted":true,"result":result}));
            }
            Err(error) => results.push(json!({"index":index,"idempotency_key":key,"accepted":false,"error":format!("{error:#}")})),
        }
    }
    let mut value =
        json!({"accepted":accepted,"rejected":results.len()-accepted,"results":results});
    if let Some(id) = goal_wait {
        value["goal_wait"] = match crate::goal_wait::seal(store, &id).await {
            Ok(result) => result,
            Err(error) => {
                json!({"goal_wait_id":id,"status":"attention","detail":format!("accepted jobs were retained; goal control failed: {error:#}")})
            }
        };
        value["next_step"] = value["goal_wait"]["next_step"].clone();
    }
    Ok(value)
}
