#![cfg(target_os = "linux")]
mod common;
use agent_job_relay::{core::*, ipc::Request, storage::Store};
use common::runtime::Rig;
use serde_json::json;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::OnceLock,
    time::Duration,
};

fn fault_library() -> &'static std::path::Path {
    static LIBRARY: OnceLock<PathBuf> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let root = common::root().join(format!("storage-fault-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let library = root.join("fault.so");
        let result = Command::new("cc")
            .args(["-shared", "-fPIC", "-O2", "-Wall", "-Wextra", "-Werror"])
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/storage-fault.c"),
            )
            .args(["-ldl", "-o"])
            .arg(&library)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        library
    })
}
struct Fault {
    flag: PathBuf,
    evidence: PathBuf,
}
impl Fault {
    async fn boot(r: &mut Rig, mode: &str, active: bool) -> Self {
        r.crash();
        let fault = Self {
            flag: r.dir.path().join("fault-on"),
            evidence: r.dir.path().join("fault-evidence"),
        };
        if active {
            std::fs::write(&fault.flag, "enabled").unwrap();
        }
        let root = r.config.root.clone();
        r.boot_with_env(&[
            ("LD_PRELOAD", fault_library().as_os_str()),
            ("AJR_TEST_FAULT_ROOT", root.as_os_str()),
            ("AJR_TEST_FAULT_FLAG", fault.flag.as_os_str()),
            ("AJR_TEST_FAULT_EVIDENCE", fault.evidence.as_os_str()),
            ("AJR_TEST_FAULT_MODE", std::ffi::OsStr::new(mode)),
        ])
        .await;
        fault
    }
    fn observed(&self) {
        assert!(
            std::fs::read_to_string(&self.evidence)
                .unwrap()
                .contains("ENOSPC")
        );
    }
}
async fn until(mut check: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn log_write_and_sync_enospc_never_report_success_or_replay() {
    for mode in ["log_write", "log_sync"] {
        let mut r = Rig::new().await;
        let fault = Fault::boot(&mut r, mode, true).await;
        let mut spec = r.spec("echo once >> launches; printf persisted; sleep 0.1");
        spec.idempotency_key = Some("faulted-output".into());
        let accepted = r.call(Request::Start { spec: spec.clone() }).await;
        let id = accepted["job"]["job_id"].as_str().unwrap();
        let done = r.state(id, &["failed"]).await;
        fault.observed();
        assert!(
            done["detail"]
                .as_str()
                .unwrap()
                .contains("output persistence failed"),
            "{done}"
        );
        assert!(
            done["detail"]
                .as_str()
                .unwrap()
                .contains("No space left on device"),
            "{done}"
        );
        if let Some(pid) = done["workload_pid"].as_u64() {
            assert!(agent_job_relay::platform::identity(pid as u32).is_none());
        }
        let again = r.call(Request::Start { spec }).await;
        assert_eq!(again["job"]["job_id"], id);
        assert_eq!(again["job"]["run_id"], accepted["job"]["run_id"]);
        assert_eq!(
            std::fs::read_to_string(r.dir.path().join("launches")).unwrap(),
            "once\n"
        );
        let events = r
            .call(Request::Events {
                job_id: id.into(),
                after: 0,
                limit: 100,
            })
            .await;
        assert!(
            !events["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "completed")
        );
    }
}

#[tokio::test]
async fn wal_enospc_loses_no_ids_and_recovers_unknown_without_execution_replay() {
    let mut r = Rig::new().await;
    let fault = Fault::boot(&mut r, "wal_write", false).await;
    let mut spec = r.spec(
        "echo once >> launches; echo ready > ready; while [ ! -f release ]; do sleep 0.02; done",
    );
    spec.idempotency_key = Some("wal-fault".into());
    let accepted = r.call(Request::Start { spec: spec.clone() }).await;
    let id = accepted["job"]["job_id"].as_str().unwrap();
    until(|| r.dir.path().join("ready").exists()).await;
    let running = r.state(id, &["running"]).await;
    let runner = running["runner_pid"].as_u64().unwrap() as u32;
    let identity = agent_job_relay::platform::identity(runner).unwrap();
    r.crash();
    std::fs::write(&fault.flag, "enabled").unwrap();
    // A runner observation attempts a real WAL write while the workload lives.
    until(|| fault.evidence.exists()).await;
    until(|| agent_job_relay::platform::identity(runner).as_ref() != Some(&identity)).await;
    fault.observed();
    std::fs::remove_file(&fault.flag).unwrap();
    r.boot().await;
    let recovered = r.state(id, &["orphaned"]).await;
    assert_eq!(recovered["run_id"], accepted["job"]["run_id"]);
    assert!(recovered["exit_code"].is_null());
    assert!(
        recovered["detail"]
            .as_str()
            .unwrap()
            .contains("not replayed")
    );
    assert_eq!(r.call(Request::Start { spec }).await["job"]["job_id"], id);
    assert_eq!(
        std::fs::read_to_string(r.dir.path().join("launches")).unwrap(),
        "once\n"
    );
    let events = r
        .call(Request::Events {
            job_id: id.into(),
            after: 0,
            limit: 100,
        })
        .await;
    assert!(
        !events["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "completed")
    );
    Store::new(r.config.clone()).data_check().await.unwrap();
}

struct PinnedChildren(Vec<(std::os::fd::OwnedFd, u32)>);
struct OwnedControl(std::process::Child);
impl Drop for OwnedControl {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl PinnedChildren {
    fn read(r: &Rig, names: &[&str]) -> Self {
        use std::os::fd::FromRawFd;
        Self(
            names
                .iter()
                .map(|name| {
                    let pid = std::fs::read_to_string(r.dir.path().join(name))
                        .unwrap()
                        .trim()
                        .parse::<u32>()
                        .unwrap();
                    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
                    assert!(fd >= 0);
                    (unsafe { std::os::fd::OwnedFd::from_raw_fd(fd as i32) }, pid)
                })
                .collect(),
        )
    }
}
impl Drop for PinnedChildren {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        for (fd, _) in &self.0 {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
        }
    }
}

#[tokio::test]
async fn live_runner_cleans_adopted_session_escape_on_completion_and_cancellation() {
    for cancel in [false, true] {
        let r = Rig::new().await;
        let mut unrelated = OwnedControl(
            Command::new("sleep")
                .arg("30")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let id = r.start("setsid /bin/sh -c 'echo $$ > escaped; sleep 60 & echo $! > escaped-child; wait' & while [ ! -f escaped-child ]; do sleep 0.01; done; echo ready > ready; while [ ! -f release ]; do sleep 0.02; done").await;
        until(|| r.dir.path().join("ready").exists()).await;
        let children = PinnedChildren::read(&r, &["escaped", "escaped-child"]);
        if cancel {
            r.call(Request::Stop {
                job_id: id.clone(),
                grace_seconds: 0,
                wait_seconds: 0,
            })
            .await;
        } else {
            std::fs::write(r.dir.path().join("release"), "finish").unwrap();
        }
        let expected = if cancel { "cancelled" } else { "completed" };
        let done = r.state(&id, &[expected]).await;
        for (_, pid) in &children.0 {
            assert!(
                agent_job_relay::platform::identity(*pid).is_none(),
                "escaped descendant survived: {done}"
            );
        }
        let survived = unrelated.0.try_wait().unwrap().is_none();
        assert!(survived, "cleanup reached an unrelated process");
    }
}

#[tokio::test]
async fn native_soak_bounds_resources_and_survives_restarts_backup_and_restore() {
    use agent_job_relay::{
        maintenance,
        session::{Adapter, Registration},
    };
    let seconds = std::env::var("AJR_RELIABILITY_SECONDS")
        .map(|v| v.parse::<u64>().expect("invalid soak duration"))
        .unwrap_or(3);
    assert!((3..=3600).contains(&seconds));
    let mut r = Rig::new().await;
    let store = Store::new(r.config.clone());
    let session = store
        .register(Registration {
            agent_kind: "codex_cli".into(),
            session_id: "soak".into(),
            instance_id: "fixture".into(),
            host_id: "fixture".into(),
            adapter: Adapter::Inbox,
            endpoint: None,
            caller_thread_id: None,
            lease_seconds: 7200,
            experimental: false,
            renew_on_delivery: false,
        })
        .await
        .unwrap();
    let burst = if seconds >= 30 {
        agent_job_relay::runner::LOG_CAP + 8192
    } else {
        8192
    };
    let mut jobs = vec![];
    for index in 0..8 {
        let cwd = r.dir.path().join(format!("work-{index}"));
        std::fs::create_dir(&cwd).unwrap();
        let mut spec = r.spec("");
        spec.command = CommandSpec::Exec {
            program: "python3".into(),
            args: vec![
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/reliability-workload.py")
                    .to_string_lossy()
                    .into(),
                burst.to_string(),
            ],
        };
        spec.cwd = cwd;
        spec.session = Some(session.key.clone());
        spec.idempotency_key = Some(format!("soak-{index}"));
        spec.timeout_seconds = Some(seconds + 60);
        spec.monitor_rules = vec![serde_json::from_value(json!({"condition":{"kind":"progress","source":"soak","trigger":{"kind":"step","every":25},"cooldown_seconds":1},"action":"alert","notify_agent":true,"grace_seconds":0})).unwrap()];
        let result = r.call(Request::Start { spec: spec.clone() }).await;
        jobs.push((
            result["job"]["job_id"].as_str().unwrap().to_owned(),
            result["job"]["run_id"].as_str().unwrap().to_owned(),
            spec,
        ));
    }
    for (_, _, spec) in &jobs {
        until(|| spec.cwd.join("ready").exists()).await;
    }
    let started = tokio::time::Instant::now();
    let mut baseline = std::collections::BTreeMap::new();
    let mut peak = std::collections::BTreeMap::new();
    let mut restarts = 0;
    let cadence = if seconds >= 30 { 60 } else { 1 };
    let mut next_restart = Duration::from_secs(cadence);
    let mut backup = None;
    while started.elapsed() < Duration::from_secs(seconds) {
        for (id, run, spec) in &jobs {
            let job = store.status(id.clone()).await.unwrap();
            assert_eq!(&job.run_id, run);
            assert_eq!(job.status, JobState::Running);
            let pid = job.run.runner_pid.unwrap();
            let stat = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
            let rss = stat
                .lines()
                .find_map(|line| line.strip_prefix("VmRSS:"))
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let fds = std::fs::read_dir(format!("/proc/{pid}/fd"))
                .unwrap()
                .count();
            let initial = baseline.entry(pid).or_insert((rss, fds));
            let max = peak.entry(pid).or_insert((rss, fds));
            max.0 = max.0.max(rss);
            max.1 = max.1.max(fds);
            assert!(
                rss <= initial.0 + 65536,
                "runner RSS grew beyond 64 MiB: {pid}"
            );
            assert!(
                fds <= initial.1 + 32,
                "runner descriptors grew without bound: {pid}"
            );
            for stream in ["stdout", "stderr"] {
                assert!(
                    std::fs::metadata(r.config.run_dir(run).join(format!("{stream}.log")))
                        .unwrap()
                        .len()
                        <= agent_job_relay::runner::LOG_CAP
                );
            }
            assert_eq!(
                std::fs::read_to_string(spec.cwd.join("launches")).unwrap(),
                "once\n"
            );
        }
        store
            .inspect(Some(jobs[0].0.clone()), None, 20)
            .await
            .unwrap();
        if started.elapsed() >= next_restart && started.elapsed().as_secs() + 1 < seconds {
            r.crash();
            r.boot().await;
            restarts += 1;
            next_restart += Duration::from_secs(cadence);
        }
        if backup.is_none() && started.elapsed() >= Duration::from_secs(seconds / 2) {
            match store.backup().await {
                Ok(snapshot) => backup = Some(snapshot),
                Err(error) => assert!(
                    error
                        .to_string()
                        .contains("data maintenance is already running"),
                    "{error:#}"
                ),
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        while backup.is_none() {
            match store.backup().await {
                Ok(snapshot) => backup = Some(snapshot),
                Err(error) => assert!(
                    error
                        .to_string()
                        .contains("data maintenance is already running"),
                    "{error:#}"
                ),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    for (_, _, spec) in &jobs {
        std::fs::write(spec.cwd.join("release"), "finish").unwrap();
    }
    for (id, run, spec) in &jobs {
        let done = r.state(id, &["completed"]).await;
        assert_eq!(&done["run_id"], run);
        let detail = store.inspect(Some(id.clone()), None, 20).await.unwrap();
        assert!(
            detail["observation"]["progress"]["accepted"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
        assert_eq!(detail["observation"]["progress"]["rejected"], 0);
        assert_eq!(
            r.call(Request::Start { spec: spec.clone() }).await["job"]["job_id"],
            id.as_str()
        );
        let events = store.events(id.clone(), 0, 100).await.unwrap();
        // Terminal may follow the first bounded event page; query its exact count.
        let key = id.clone();
        store
            .read(move |db| {
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM events WHERE job_id=? AND kind='completed'",
                        [key],
                        |r| r.get::<_, u32>(0)
                    )?,
                    1
                );
                Ok(())
            })
            .await
            .unwrap();
        assert!(!events.is_empty());
    }
    store.data_check().await.unwrap();
    let restored_root = r.dir.path().join("restored");
    maintenance::restore(
        PathBuf::from(backup.unwrap()["path"].as_str().unwrap()),
        restored_root.clone(),
    )
    .await
    .unwrap();
    let restored = Store::new(agent_job_relay::config::Config::new(Some(restored_root)).unwrap());
    for (id, run, _) in &jobs {
        let old = restored.status(id.clone()).await.unwrap();
        assert_eq!(&old.run_id, run);
        assert_eq!(old.status, JobState::Orphaned);
    }
    let report = json!({"seconds":seconds,"jobs":jobs.len(),"daemon_restarts":restarts,"burst_bytes_per_stream":burst,"runner_baseline_kib_fds":baseline,"runner_peak_kib_fds":peak,"exact_run_completion":true,"one_launch_each":true,"backup_restore_without_replay":true});
    if let Some(path) = std::env::var_os("AJR_RELIABILITY_REPORT") {
        let path = PathBuf::from(path);
        let parent = path.parent().unwrap().canonicalize().unwrap();
        assert!(
            parent.starts_with(common::root().canonicalize().unwrap()),
            "soak report must stay under repository .tmp"
        );
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    eprintln!("{report}");
}
