#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{
    adapters,
    config::Config,
    core::*,
    ipc,
    session::{Adapter, Registration},
    storage::Store,
};
use serde_json::{Value, json};
use std::path::PathBuf;
async fn store() -> (tempfile::TempDir, Store) {
    agent_job_relay::platform::initialize();
    let dir = tempfile::Builder::new()
        .prefix("bridge-")
        .tempdir_in(common::root())
        .unwrap();
    let config = Config {
        root: dir.path().join("state"),
    };
    config.prepare().unwrap();
    let store = Store::new(config);
    store.init().await.unwrap();
    (dir, store)
}
fn registration(endpoint: PathBuf, session: &str, instance: &str) -> Registration {
    Registration {
        agent_kind: "opencode".into(),
        session_id: session.into(),
        instance_id: instance.into(),
        host_id: "owned-plugin".into(),
        adapter: Adapter::PluginBridge,
        endpoint: Some(endpoint),
        caller_thread_id: Some(session.into()),
        lease_seconds: 86400,
        experimental: true,
        renew_on_delivery: true,
    }
}
async fn claim(store: &Store, key: String, cwd: PathBuf) -> agent_job_relay::delivery::Claim {
    let job = store
        .start(StartSpec {
            command: CommandSpec::Exec {
                program: "true".into(),
                args: vec![],
            },
            cwd,
            env: Default::default(),
            timeout_seconds: None,
            session: Some(key),
            idempotency_key: None,
            monitor_rules: vec![],
        })
        .await
        .unwrap();
    store
        .finish(
            job.job_id,
            "completed",
            Some(0),
            None,
            "untrusted output must not reach host".into(),
        )
        .await
        .unwrap();
    store.claim().await.unwrap().unwrap()
}
fn reply(request: &Value) -> Value {
    let p = &request["params"];
    json!({"jsonrpc":"2.0","id":request["id"],"result":{"protocol_version":1,"agent_kind":p["agent_kind"],"session_id":p["session_id"],"instance_id":p["instance_id"],"host_id":p["host_id"],"can_resume":true,"accepted":true,"delivery_id":p["delivery_id"],"event_id":p["event_id"],"job_id":p["job_id"]}})
}
#[tokio::test]
async fn bridge_renews_same_endpoint_and_delivers_only_exact_metadata_to_owned_session() {
    let (dir, store) = store().await;
    let endpoint = dir.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let a = store
        .register(registration(endpoint.clone(), "a", "one"))
        .await
        .unwrap();
    let b = store
        .register(registration(endpoint, "b", "one"))
        .await
        .unwrap();
    assert_ne!(a.key, b.key);
    let mut c = claim(&store, a.key.clone(), dir.path().into()).await;
    let key = a.key.clone();
    let expired = now() - 172_800_000;
    store
        .call(move |db| {
            db.execute(
                "UPDATE sessions SET expires=? WHERE key=?",
                rusqlite::params![expired, key],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    c.session.expires_at = expired;
    let expected = c.delivery.id.clone();
    let fake = tokio::spawn(async move {
        for method in ["relay/probe", "relay/deliver"] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = ipc::read_frame(&mut stream, 16384).await.unwrap();
            assert_eq!(request["method"], method);
            assert_eq!(request["params"]["session_id"], "a");
            assert_eq!(request["params"]["caller_session_id"], "a");
            assert!(request["params"].get("command").is_none());
            assert!(!request.to_string().contains("untrusted output"));
            if method == "relay/deliver" {
                assert_eq!(request["params"]["delivery_id"], expected);
                assert!(request["params"]["event_id"].is_string());
            }
            ipc::write_frame(&mut stream, &reply(&request))
                .await
                .unwrap();
        }
    });
    store.renew_delivery_session(&mut c).await.unwrap();
    assert!(c.session.expires_at > now());
    let result = adapters::deliver(&store.config, &c).await;
    assert_eq!(result.as_ref().unwrap(), &"accepted_by_host");
    store.complete_claim(c, result).await.unwrap();
    fake.await.unwrap();
    assert_eq!(
        store.deliveries(None, 0, 10).await.unwrap()["deliveries"][0]["status"],
        "accepted_by_host"
    );
    assert!(!agent_job_relay::session::capabilities(&Adapter::PluginBridge).can_confirm_resume);
}
#[tokio::test]
async fn bridge_identity_refusal_and_endpoint_replacement_require_scoped_recovery() {
    let (dir, store) = store().await;
    let endpoint = dir.path().join("host.sock");
    let old_listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let old = store
        .register(registration(endpoint.clone(), "a", "one"))
        .await
        .unwrap();
    let c = claim(&store, old.key.clone(), dir.path().into()).await;
    let delivery = c.delivery.id.clone();
    let fake = tokio::spawn(async move {
        let (mut stream, _) = old_listener.accept().await.unwrap();
        let request = ipc::read_frame(&mut stream, 16384).await.unwrap();
        let mut response = reply(&request);
        response["result"]["session_id"] = json!("wrong-session");
        ipc::write_frame(&mut stream, &response).await.unwrap();
        old_listener
    });
    let result = adapters::deliver(&store.config, &c).await;
    assert!(result.is_err());
    store.complete_claim(c, result).await.unwrap();
    let old_listener = fake.await.unwrap();
    std::fs::remove_file(&endpoint).unwrap();
    let _new_listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    assert!(adapters::probe(&old).await.is_err());
    let new = store
        .register(registration(endpoint, "a", "two"))
        .await
        .unwrap();
    let preview = store
        .recover_session(new.clone(), false, 0, 20)
        .await
        .unwrap();
    assert_eq!(preview["targets"].as_array().unwrap().len(), 1);
    let recovered = store.recover_session(new, true, 0, 20).await.unwrap();
    assert_eq!(recovered["targets"][0]["applied"], true);
    let after = store.deliveries(None, 0, 10).await.unwrap();
    assert_eq!(after["deliveries"][0]["id"], delivery);
    assert_eq!(after["deliveries"][0]["attempts"], 1);
    drop(old_listener);
}
#[tokio::test]
async fn bridge_rejects_redirected_callers_and_oversized_responses() {
    let (dir, store) = store().await;
    let endpoint = dir.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let mut r = registration(endpoint, "a", "one");
    r.caller_thread_id = Some("b".into());
    assert!(store.register(r.clone()).await.is_err());
    r.caller_thread_id = Some("a".into());
    let session = store.register(r).await.unwrap();
    let fake = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        ipc::read_frame(&mut stream, 16384).await.unwrap();
        use tokio::io::AsyncWriteExt;
        stream.write_u32_le(16385).await.unwrap();
    });
    assert!(adapters::probe(&session).await.is_err());
    fake.await.unwrap();
}

#[tokio::test]
async fn bridge_never_accepts_an_ack_for_another_delivery() {
    let (dir, store) = store().await;
    let endpoint = dir.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let session = store
        .register(registration(endpoint, "a", "one"))
        .await
        .unwrap();
    let c = claim(&store, session.key, dir.path().into()).await;
    let fake = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = ipc::read_frame(&mut stream, 16384).await.unwrap();
        let mut response = reply(&request);
        response["result"]["delivery_id"] = json!(id());
        ipc::write_frame(&mut stream, &response).await.unwrap();
    });
    let result = adapters::deliver(&store.config, &c).await;
    assert!(result.is_err());
    store.complete_claim(c, result).await.unwrap();
    assert_eq!(
        store.deliveries(None, 0, 10).await.unwrap()["deliveries"][0]["status"],
        "dead_letter"
    );
    fake.await.unwrap();
}
