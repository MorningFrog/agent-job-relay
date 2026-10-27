use anyhow::{Result, bail};
#[cfg(not(windows))]
use std::fs::OpenOptions;
use std::{fs::File, path::Path};
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
#[cfg(not(any(unix, windows)))]
pub fn secure_options(_: &mut OpenOptions) {}
#[cfg(not(any(unix, windows)))]
pub fn secure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}
#[cfg(not(any(unix, windows)))]
pub fn initialize() {}
#[cfg(not(any(unix, windows)))]
pub fn detach(_: &mut std::process::Command) -> Result<()> {
    bail!("native background runtime is not implemented on this platform")
}
#[cfg(not(any(unix, windows)))]
pub fn workload(_: &mut std::process::Command) -> Result<()> {
    bail!("native process supervision is not implemented on this platform")
}
#[cfg(not(any(unix, windows)))]
pub fn identity(_: u32) -> Option<String> {
    None
}
#[cfg(not(any(unix, windows)))]
pub fn signal_group(_: u32, _: i32) -> Result<()> {
    bail!("process groups unavailable")
}
#[cfg(not(any(unix, windows)))]
pub fn exit_signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}
pub fn private_file(path: &Path) -> Result<File> {
    #[cfg(windows)]
    return open_private_file(path, false, false);
    #[cfg(not(windows))]
    {
        let mut o = OpenOptions::new();
        o.read(true).write(true).create(true).truncate(false);
        secure_options(&mut o);
        Ok(o.open(path)?)
    }
}
pub fn create_new_file(path: &Path) -> Result<File> {
    #[cfg(windows)]
    return open_private_file(path, true, false);
    #[cfg(not(windows))]
    {
        let mut o = OpenOptions::new();
        o.write(true).create_new(true);
        secure_options(&mut o);
        Ok(o.open(path)?)
    }
}
pub fn normalized_path(path: std::path::PathBuf) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if let Some(Component::Prefix(prefix)) = path.components().next()
            && let Prefix::VerbatimDisk(letter) = prefix.kind()
        {
            let mut normal = std::path::PathBuf::from(format!("{}:\\", letter as char));
            normal.extend(path.components().skip(2));
            return normal;
        }
    }
    path
}
pub fn require_runtime() -> Result<()> {
    if !cfg!(any(target_os = "linux", windows)) {
        bail!(
            "background runtime requires Linux or Windows; native IPC/identity/security adapters are not verified on this OS"
        );
    }
    Ok(())
}
/// Bundle compatibility is explicit; a plugin does not add native OS support.
pub fn runtime_target() -> String {
    let abi = if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_env = "gnu") {
        "gnu"
    } else {
        "native"
    };
    format!("{}-{}-{abi}", std::env::consts::ARCH, std::env::consts::OS)
}
/// Execute the mapped image even after a rebuild or cache refresh unlinks its path.
pub fn self_image() -> Result<std::path::PathBuf> {
    require_runtime()?;
    #[cfg(windows)]
    return Ok(std::env::current_exe()?);
    #[cfg(not(windows))]
    {
        let path = std::path::PathBuf::from("/proc/self/exe");
        anyhow::ensure!(
            path.is_file(),
            "loaded executable is unavailable through /proc/self/exe"
        );
        Ok(path)
    }
}
#[cfg(not(any(unix, windows)))]
pub fn endpoint_identity(_: &Path) -> Result<String> {
    bail!("native host endpoint identity is not implemented on this OS")
}
#[cfg(not(unix))]
pub async fn stop_legacy_daemon(_: &Path, _: u32, _: &str) -> Result<()> {
    bail!("verified legacy daemon shutdown is only implemented on Linux")
}

/// Ensures ordinary supervision errors terminate the entire owned process group.
#[cfg(not(windows))]
pub struct GroupGuard {
    pid: u32,
    armed: bool,
}
#[cfg(not(windows))]
impl GroupGuard {
    pub fn new(pid: u32) -> Self {
        Self { pid, armed: true }
    }
    pub fn disarm(&mut self) {
        self.armed = false;
    }
    pub fn signal(&self, signal: i32) -> Result<()> {
        signal_group(self.pid, signal)
    }
}
#[cfg(not(windows))]
impl Drop for GroupGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = signal_group(self.pid, 9);
        }
    }
}
#[cfg(not(any(unix, windows)))]
pub fn runner_initialize() -> Result<()> {
    bail!("native runner initialization unavailable")
}
#[cfg(not(any(unix, windows)))]
pub fn reap_descendants() -> Result<bool> {
    bail!("native descendant reaping unavailable")
}

pub fn is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    metadata.file_type().is_symlink()
}
pub fn validate_endpoint(path: &Path) -> Result<()> {
    #[cfg(windows)]
    return validate_pipe_path(path);
    #[cfg(not(windows))]
    {
        anyhow::ensure!(
            path.is_absolute() && path.as_os_str().len() < 104,
            "host endpoint must be an absolute Unix socket path shorter than 104 bytes"
        );
        Ok(())
    }
}
pub fn validate_local_endpoint(path: &Path) -> Result<()> {
    validate_endpoint(path)
}
pub fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(not(windows))]
    std::fs::File::open(path)?.sync_all()?;
    // Windows has no directory fsync. File publication uses write-through rename.
    #[cfg(windows)]
    let _ = path;
    Ok(())
}
pub fn publish_file(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(windows)]
    return rename_write_through(source, destination, true);
    #[cfg(not(windows))]
    {
        std::fs::rename(source, destination)?;
        Ok(())
    }
}
pub fn rename_directory(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(windows)]
    return rename_write_through(source, destination, false);
    #[cfg(not(windows))]
    {
        std::fs::rename(source, destination)?;
        Ok(())
    }
}
pub const NATIVE_FILE: &str = if cfg!(windows) {
    "bin/agent-job-relay.exe"
} else {
    "bin/agent-job-relay"
};
pub const LAUNCHER_FILE: &str = if cfg!(windows) { "relay.cmd" } else { "relay" };

pub const DEFAULT_SHELL: &str = if cfg!(windows) {
    "powershell.exe"
} else {
    "/bin/sh"
};

/// Automatic daemon startup preserves the selected profile and caller environment.
pub async fn launch_daemon(
    command: std::process::Command,
    config: &crate::config::Config,
) -> Result<()> {
    #[cfg(windows)]
    return windows::launch_daemon(command, config).await;
    #[cfg(not(windows))]
    {
        let _ = config;
        let mut command = command;
        command.env("AJR_DAEMON_LAUNCH_METHOD", "unix_native");
        spawn_background(command)
    }
}
