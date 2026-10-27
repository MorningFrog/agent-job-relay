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
        renew_on_delivery: false,
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
async fn dispatcher_runs_four_chats_concurrently_without_overlapping_one_chat() {
    let (dir, store) = store().await;
    let endpoint = dir.path().join("parallel.sock");
    let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let mut jobs = Vec::new();
    for (chat, instance) in [
        ("slow", "one"),
        ("slow", "two"),
        ("fast-1", "one"),
        ("fast-2", "one"),
        ("fast-3", "one"),
        ("fast-4", "one"),
    ] {
        let session = store
            .register(registration(endpoint.clone(), chat, instance))
            .await
            .unwrap();
        let job = store
            .start(StartSpec {
                command: CommandSpec::Exec {
                    program: "true".into(),
                    args: vec![],
                },
                cwd: dir.path().into(),
                env: Default::default(),
                timeout_seconds: None,
                session: Some(session.key),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .finish(
                job.job_id.clone(),
                "completed",
                Some(0),
                None,
                "done".into(),
            )
            .await
            .unwrap();
        jobs.push(job.job_id);
    }
    let rows = store.deliveries(None, 0, 100).await.unwrap();
    let ids: Vec<_> = rows["deliveries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap().to_owned())
        .collect();
    let (sent, mut received) = tokio::sync::mpsc::channel(8);
    // Dropping either JoinSet aborts its children, including on assertion failure.
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                result=listener.accept()=>{
                    let (mut stream, _)=result.unwrap();
                    let sent=sent.clone();
                    connections.spawn(async move {
                        let list=ipc::read_frame(&mut stream,65536).await.unwrap();
                        ipc::write_frame(&mut stream,&json!({"jsonrpc":"2.0","id":list["id"],
                            "result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).await.unwrap();
                        let request=ipc::read_frame(&mut stream,65536).await.unwrap();
                        let (release, wait)=tokio::sync::oneshot::channel::<()>();
                        sent.send((request["params"]["callId"].as_str().unwrap().to_owned(),release)).await.unwrap();
                        wait.await.unwrap();
                        ipc::write_frame(&mut stream,&json!({"jsonrpc":"2.0","id":request["id"],"result":{"success":true}})).await.unwrap();
                    });
                },
                Some(result)=connections.join_next()=>{result.unwrap();},
            }
        }
    });
    tasks.spawn(agent_job_relay::delivery::worker(store.clone()));
    let mut held = std::collections::HashMap::new();
    for _ in 0..4 {
        let (id, release) = tokio::time::timeout(Duration::from_secs(2), received.recv())
            .await
            .unwrap()
            .unwrap();
        held.insert(id, release);
    }
    for index in [0, 2, 3, 4] {
        assert!(held.contains_key(&ids[index]));
    }
    let rows = store.deliveries(None, 0, 100).await.unwrap();
    assert_eq!(rows["deliveries"][1]["status"], "pending");
    assert_eq!(rows["deliveries"][5]["status"], "pending");
    assert!(received.try_recv().is_err());
    // A fast destination frees a slot for another chat while the first is held.
    held.remove(&ids[2]).unwrap().send(()).unwrap();
    let (id, release) = tokio::time::timeout(Duration::from_secs(2), received.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(id, ids[5]);
    held.insert(id, release);
    // The second registration of the slow chat starts only after its first ack.
    held.remove(&ids[0]).unwrap().send(()).unwrap();
    let (id, release) = tokio::time::timeout(Duration::from_secs(2), received.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(id, ids[1]);
    held.insert(id, release);
    for (_, release) in held {
        release.send(()).unwrap();
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let rows = store.deliveries(None, 0, 100).await.unwrap();
            if rows["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|d| d["status"] == "accepted_by_host")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tasks.shutdown().await;
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

#[tokio::test]
async fn expired_owned_binding_renews_after_probe_then_delivers_with_original_id() {
    let (d, s) = store().await;
    let ep = d.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&ep).unwrap();
    let mut r = registration(ep, "thread-a", "original");
    r.caller_thread_id = Some(r.session_id.clone());
    r.renew_on_delivery = true;
    let a = s.register(r).await.unwrap();
    let mut c = claim(&s, a.key.clone(), d.path().into()).await;
    s.call(|db| {
        db.execute("UPDATE sessions SET expires=0", [])?;
        Ok(())
    })
    .await
    .unwrap();
    c.session = s.session(a.key.clone()).await.unwrap();
    let expected = c.delivery.id.clone();
    let fake = tokio::spawn(async move {
        for renewal in [true, false] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let v = ipc::read_frame(&mut stream, 65536).await.unwrap();
            assert_eq!(v["method"], "tools/list");
            ipc::write_frame(&mut stream,&json!({"jsonrpc":"2.0","id":v["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).await.unwrap();
            if renewal {
                assert!(ipc::read_frame(&mut stream, 65536).await.is_err());
            } else {
                let v = ipc::read_frame(&mut stream, 65536).await.unwrap();
                assert_eq!(v["params"]["callId"], expected);
                ipc::write_frame(
                    &mut stream,
                    &json!({"jsonrpc":"2.0","id":v["id"],"result":{"success":true}}),
                )
                .await
                .unwrap();
            }
        }
    });
    s.renew_delivery_session(&mut c).await.unwrap();
    assert_eq!(c.session.key, a.key);
    assert_eq!(c.session.endpoint_identity, a.endpoint_identity);
    assert!(c.session.expires_at > now());
    let outcome = adapters::deliver(&s.config, &c).await;
    s.complete_claim(c, outcome).await.unwrap();
    fake.await.unwrap();
    assert_eq!(
        s.deliveries(None, 0, 100).await.unwrap()["deliveries"][0]["status"],
        "accepted_by_host"
    );
    let audits = s
        .call(|db| {
            Ok(
                db.query_row("SELECT count(*) FROM target_recoveries", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn renewal_never_revives_opted_out_invalidated_or_replaced_bindings() {
    for mode in ["opted_out", "invalidated", "replaced"] {
        let (d, s) = store().await;
        let ep = d.path().join("host.sock");
        let listener = tokio::net::UnixListener::bind(&ep).unwrap();
        let mut r = registration(ep.clone(), "thread-a", "original");
        r.caller_thread_id = Some(r.session_id.clone());
        r.renew_on_delivery = mode != "opted_out";
        let a = s.register(r).await.unwrap();
        let mut c = claim(&s, a.key.clone(), d.path().into()).await;
        s.call(|db| {
            db.execute("UPDATE sessions SET expires=0", [])?;
            Ok(())
        })
        .await
        .unwrap();
        if mode == "invalidated" {
            s.invalidate(a.key.clone()).await.unwrap();
        }
        c.session = s.session(a.key.clone()).await.unwrap();
        drop(listener);
        std::fs::remove_file(&ep).unwrap();
        let replacement = tokio::net::UnixListener::bind(ep).unwrap();
        let result = s.renew_delivery_session(&mut c).await;
        if mode == "opted_out" {
            result.unwrap();
            assert!(adapters::deliver(&s.config, &c).await.is_err());
        } else {
            assert!(result.is_err());
        }
        assert_eq!(s.session(a.key).await.unwrap().expires_at, 0);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), replacement.accept())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn renewal_fences_changes_during_probe_and_rolls_back_if_audit_fails() {
    for mode in ["invalidate", "revoke_claim", "audit_failure"] {
        let (d, s) = store().await;
        let ep = d.path().join("host.sock");
        let listener = tokio::net::UnixListener::bind(&ep).unwrap();
        let mut r = registration(ep, "thread-a", "original");
        r.caller_thread_id = Some(r.session_id.clone());
        r.renew_on_delivery = true;
        let a = s.register(r).await.unwrap();
        let mut c = claim(&s, a.key.clone(), d.path().into()).await;
        s.call(|db| {
            db.execute("UPDATE sessions SET expires=0", [])?;
            Ok(())
        })
        .await
        .unwrap();
        c.session = s.session(a.key.clone()).await.unwrap();
        let store = s.clone();
        let key = a.key.clone();
        let fake = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let v = ipc::read_frame(&mut stream, 65536).await.unwrap();
            if mode == "invalidate" {
                store.invalidate(key).await.unwrap();
            } else {
                store.call(move|db| {
                if mode=="revoke_claim" {db.execute("UPDATE deliveries SET status='cancelled',token=NULL",[])?;}
                else {db.execute_batch("CREATE TRIGGER fail_renewal BEFORE INSERT ON target_recoveries BEGIN SELECT RAISE(ABORT,'audit failure'); END;")?;}
                Ok(())
            }).await.unwrap();
            }
            ipc::write_frame(&mut stream,&json!({"jsonrpc":"2.0","id":v["id"],"result":{"tools":[{"namespace":"codex_app","name":"send_message_to_thread"}]}})).await.unwrap();
            assert!(ipc::read_frame(&mut stream, 65536).await.is_err());
        });
        assert!(s.renew_delivery_session(&mut c).await.is_err());
        fake.await.unwrap();
        assert_eq!(s.session(a.key).await.unwrap().expires_at, 0);
        assert_eq!(
            s.call(|db| Ok(db
                .query_row("SELECT count(*) FROM target_recoveries", [], |r| r
                    .get::<_, i64>(0))?))
                .await
                .unwrap(),
            0
        );
    }
}
