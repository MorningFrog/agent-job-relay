use anyhow::{Result, bail};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
#[cfg(not(unix))]
pub fn secure_options(_: &mut OpenOptions) {}
#[cfg(not(unix))]
pub fn secure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}
#[cfg(not(unix))]
pub fn initialize() {}
#[cfg(not(unix))]
pub fn detach(_: &mut std::process::Command) -> Result<()> {
    bail!("native background runtime is not implemented on this platform")
}
#[cfg(not(unix))]
pub fn workload(_: &mut std::process::Command) -> Result<()> {
    bail!("native process supervision is not implemented on this platform")
}
#[cfg(not(unix))]
pub fn identity(_: u32) -> Option<String> {
    None
}
#[cfg(not(unix))]
pub fn signal_group(_: u32, _: i32) -> Result<()> {
    bail!("process groups unavailable")
}
#[cfg(not(unix))]
pub fn exit_signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}
pub fn private_file(path: &Path) -> Result<File> {
    let mut o = OpenOptions::new();
    o.read(true).write(true).create(true).truncate(false);
    secure_options(&mut o);
    Ok(o.open(path)?)
}
pub fn require_runtime() -> Result<()> {
    if !cfg!(target_os = "linux") {
        bail!(
            "background runtime currently requires Linux; native IPC/identity/security adapters are not verified on this OS"
        );
    }
    Ok(())
}
#[cfg(not(unix))]
pub fn endpoint_identity(_: &Path) -> Result<String> {
    bail!("native host endpoint identity is not implemented on this OS")
}

/// Ensures ordinary supervision errors terminate the entire owned process group.
pub struct GroupGuard {
    pid: u32,
    armed: bool,
}
impl GroupGuard {
    pub fn new(pid: u32) -> Self {
        Self { pid, armed: true }
    }
    pub fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for GroupGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = signal_group(self.pid, 9);
        }
    }
}
#[cfg(not(unix))]
pub fn runner_initialize() -> Result<()> {
    bail!("native runner initialization unavailable")
}
#[cfg(not(unix))]
pub fn reap_descendants() -> Result<bool> {
    bail!("native descendant reaping unavailable")
}
