#![allow(dead_code)]
use agent_job_relay::{
    config::Config,
    core::*,
    ipc::{self, Request},
};
use serde_json::Value;
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};
pub struct Rig {
    pub dir: tempfile::TempDir,
    pub config: Config,
    pub daemon: Option<Child>,
}

impl Rig {
    pub async fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("rt-")
            .tempdir_in(super::root())
            .unwrap();
        let config = Config::new(Some(dir.path().join("state"))).unwrap();
        let mut r = Self {
            dir,
            config,
            daemon: None,
        };
        r.boot().await;
        r
    }
    pub async fn boot(&mut self) {
        self.boot_with_env(&[]).await;
    }
    pub async fn boot_with_env(&mut self, env: &[(&str, &std::ffi::OsStr)]) {
        let c = Command::new(env!("CARGO_BIN_EXE_agent-job-relay"))
            .arg("--state-dir")
            .arg(&self.config.root)
            .arg("daemon")
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.daemon = Some(c);
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if ipc::call(&self.config, Request::Doctor).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    pub fn crash(&mut self) {
        if let Some(mut c) = self.daemon.take() {
            c.kill().unwrap();
            c.wait().unwrap();
        }
    }
    pub async fn call(&self, r: Request) -> Value {
        ipc::call(&self.config, r).await.unwrap()
    }
    pub fn spec(&self, script: &str) -> StartSpec {
        StartSpec {
            command: CommandSpec::Shell {
                interpreter: "/bin/sh".into(),
                script: script.into(),
            },
            cwd: self.dir.path().to_path_buf(),
            env: Default::default(),
            timeout_seconds: None,
            session: None,
            idempotency_key: None,
            monitor_rules: vec![],
        }
    }
    pub async fn start(&self, script: &str) -> String {
        self.call(Request::Start {
            spec: self.spec(script),
        })
        .await["job"]["job_id"]
            .as_str()
            .unwrap()
            .into()
    }
    pub async fn state(&self, j: &str, states: &[&str]) -> Value {
        let mut last = Value::Null;
        tokio::time::timeout(Duration::from_secs(12), async {
            loop {
                let v = self.call(Request::Status { job_id: j.into() }).await;
                if states.contains(&v["status"].as_str().unwrap()) {
                    break v;
                }
                last = v;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("job {j} did not enter {states:?}; last state: {last}"))
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(mut c) = self.daemon.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        // Cleanup only verified processes belonging to this isolated rig, including panic paths.
        if let Ok(db) = rusqlite::Connection::open(self.config.root.join("relay.sqlite3"))
            && let Ok(mut q) = db.prepare(
                "SELECT workload_pid,workload_identity,runner_pid,runner_identity FROM runs",
            )
            && let Ok(rows) = q.query_map([], |r| {
                Ok((
                    r.get::<_, Option<u32>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<u32>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
        {
            for (pid, identity, rpid, ridentity) in rows.flatten() {
                if let (Some(p), Some(i)) = (pid, identity)
                    && agent_job_relay::platform::identity(p).as_ref() == Some(&i)
                {
                    let _ = agent_job_relay::platform::signal_group(p, 9);
                }
                if let (Some(p), Some(i)) = (rpid, ridentity)
                    && agent_job_relay::platform::identity(p).as_ref() == Some(&i)
                {
                    unsafe {
                        libc::kill(p as i32, libc::SIGKILL);
                    }
                }
            }
        }
    }
}
