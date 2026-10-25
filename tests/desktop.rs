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
use serde_json::json;
use std::{path::PathBuf, time::Duration};
async fn store() -> (tempfile::TempDir, Store) {
    agent_job_relay::platform::initialize();
    let d = tempfile::tempdir_in(common::root()).unwrap();
    let c = Config::new(Some(d.path().join("state"))).unwrap();
    c.prepare().unwrap();
    let s = Store::new(c);
    s.init().await.unwrap();
    (d, s)
}
fn registration(endpoint: PathBuf, session: &str, instance: &str) -> Registration {
    Registration {
        agent_kind: "codex_desktop".into(),
        session_id: session.into(),
        instance_id: instance.into(),
        host_id: "fake-host".into(),
        adapter: Adapter::DesktopExperimental,
        endpoint: Some(endpoint),
        caller_thread_id: Some("explicit-caller".into()),
        lease_seconds: 3600,
        experimental: true,
    }
}
async fn claim(s: &Store, key: String, cwd: PathBuf) -> agent_job_relay::delivery::Claim {
    let j = s
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
        })
        .await
        .unwrap();
    s.finish(j.job_id, "completed", Some(0), None, "done".into())
        .await
        .unwrap();
    s.claim().await.unwrap().unwrap()
}
#[tokio::test]
async fn fake_desktop_fragmented_catalog_fields_routing_and_acceptance_only() {
    let (d, s) = store().await;
    let ep = d.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&ep).unwrap();
    let a = s
        .register(registration(ep.clone(), "thread-a", "instance-a"))
        .await
        .unwrap();
    let b = s
        .register(registration(ep, "thread-b", "instance-b"))
        .await
        .unwrap();
    assert_ne!(a.key, b.key);
    let c = claim(&s, a.key.clone(), d.path().into()).await;
    let expected = c.delivery.id.clone();
    let fake = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let list = ipc::read_frame(&mut stream, 65536).await.unwrap();
        assert_eq!(list["method"], "tools/list");
        let raw=serde_json::to_vec(&json!({"jsonrpc":"2.0","id":list["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).unwrap();
        use tokio::io::AsyncWriteExt;
        stream.write_u32_le(raw.len() as u32).await.unwrap();
        for chunk in raw.chunks(3) {
            stream.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
        let request = ipc::read_frame(&mut stream, 65536).await.unwrap();
        assert_eq!(request["params"]["callerSource"], "codex");
        assert_eq!(request["params"]["threadId"], "explicit-caller");
        assert_eq!(request["params"]["arguments"]["threadId"], "thread-a");
        assert_eq!(request["params"]["callId"], expected);
        assert!(
            !request["params"]["arguments"]["prompt"]
                .as_str()
                .unwrap()
                .contains("done")
        );
        ipc::write_frame(
            &mut stream,
            &json!({"jsonrpc":"2.0","id":request["id"],"result":{"success":true}}),
        )
        .await
        .unwrap();
    });
    let outcome = adapters::deliver(&s.config, &c).await;
    assert_eq!(outcome.as_ref().unwrap(), &"accepted_by_host");
    s.complete_claim(c, outcome).await.unwrap();
    fake.await.unwrap();
    let rows = s.deliveries(None, 0, 100).await.unwrap();
    assert_eq!(rows["deliveries"][0]["status"], "accepted_by_host");
    assert!(
        !agent_job_relay::session::capabilities(&Adapter::DesktopExperimental).can_confirm_resume
    );
}
#[tokio::test]
async fn host_replacement_requires_refresh_and_never_routes_to_cli() {
    let (d, s) = store().await;
    let ep = d.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&ep).unwrap();
    let a = s
        .register(registration(ep.clone(), "thread-a", "old"))
        .await
        .unwrap();
    let c = claim(&s, a.key, d.path().into()).await;
    drop(listener);
    std::fs::remove_file(&ep).unwrap();
    let replacement = tokio::net::UnixListener::bind(&ep).unwrap();
    assert!(
        adapters::deliver(&s.config, &c)
            .await
            .unwrap_err()
            .to_string()
            .contains("instance changed")
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(50), replacement.accept())
            .await
            .is_err()
    );
    assert!(!s.config.root.join("inbox").exists());
    let refreshed = s
        .register(registration(ep, "thread-a", "old"))
        .await
        .unwrap();
    assert_ne!(refreshed.endpoint_identity, c.session.endpoint_identity);
}
#[tokio::test]
async fn catalog_rejection_and_oversized_frame_fail_closed() {
    for large in [false, true] {
        let (d, s) = store().await;
        let ep = d.path().join("host.sock");
        let listener = tokio::net::UnixListener::bind(&ep).unwrap();
        let a = s.register(registration(ep, "thread", "i")).await.unwrap();
        let c = claim(&s, a.key, d.path().into()).await;
        let fake = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let v = ipc::read_frame(&mut stream, 65536).await.unwrap();
            if large {
                use tokio::io::AsyncWriteExt;
                stream.write_u32_le(1048577).await.unwrap();
            } else {
                ipc::write_frame(
                    &mut stream,
                    &json!({"jsonrpc":"2.0","id":v["id"],"result":{"tools":[]}}),
                )
                .await
                .unwrap();
            }
        });
        assert!(adapters::deliver(&s.config, &c).await.is_err());
        fake.await.unwrap();
        assert!(!s.config.root.join("inbox").exists());
    }
}
#[tokio::test]
async fn opt_in_and_caller_identity_are_required() {
    let (d, s) = store().await;
    let ep = d.path().join("host.sock");
    let _listener = tokio::net::UnixListener::bind(&ep).unwrap();
    let mut r = registration(ep, "thread", "i");
    r.experimental = false;
    assert!(s.register(r.clone()).await.is_err());
    r.experimental = true;
    r.caller_thread_id = None;
    assert!(s.register(r).await.is_err());
}
