use anyhow::{Context, Result, ensure};
use std::{fs::File, path::PathBuf};
#[derive(Clone, Debug)]
pub struct Config {
    pub root: PathBuf,
}
impl Config {
    pub fn new(root: Option<PathBuf>) -> Result<Self> {
        let explicit = root.or_else(|| std::env::var_os("AJR_STATE_DIR").map(PathBuf::from));
        let root = match explicit {
            Some(path) => path,
            None => crate::deployment::runtime_state()?.unwrap_or_else(|| {
                if let Some(p) = std::env::var_os("XDG_STATE_HOME") {
                    PathBuf::from(p).join("agent-job-relay")
                } else if let Some(p) = std::env::var_os("LOCALAPPDATA") {
                    PathBuf::from(p).join("AgentJobRelay").join("state")
                } else {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                        .join(".local/state/agent-job-relay")
                }
            }),
        };
        let root = crate::platform::normalized_path(root);
        #[cfg(windows)]
        crate::platform::check_path(&root)?;
        ensure!(
            root.is_absolute(),
            "state directory must be absolute (use --state-dir)"
        );
        Ok(Self { root })
    }
    pub fn prepare(&self) -> Result<()> {
        crate::platform::secure_dir(&self.root)?;
        crate::platform::secure_dir(&self.root.join("runs"))?;
        Ok(())
    }
    pub fn socket(&self) -> PathBuf {
        crate::platform::local_endpoint(&self.root)
    }
    pub fn run_dir(&self, run: &str) -> PathBuf {
        self.root.join("runs").join(run)
    }
    pub fn run_lock(&self, run: &str) -> Result<File> {
        crate::core::validate_id(run)?;
        let dir = self.run_dir(run);
        crate::platform::secure_dir(&dir)?;
        crate::platform::private_file(&dir.join("runner.lock"))
    }
    pub fn daemon_lock(&self) -> Result<File> {
        let f = crate::platform::private_file(&self.root.join("daemon.lock"))?;
        f.try_lock()
            .context("daemon is already running for this state directory")?;
        Ok(f)
    }
}
pub fn open_append(path: &std::path::Path) -> Result<File> {
    #[cfg(windows)]
    return crate::platform::open_private_file(path, false, true);
    #[cfg(not(windows))]
    {
        let mut o = std::fs::OpenOptions::new();
        o.create(true).append(true);
        crate::platform::secure_options(&mut o);
        Ok(o.open(path)?)
    }
}
