use anyhow::{Result, ensure};
use std::{
    fs::{OpenOptions, Permissions},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        process::{CommandExt, ExitStatusExt},
    },
    path::Path,
    process::Command,
};
pub fn initialize() {
    unsafe {
        libc::umask(0o077);
    }
}
pub fn secure_options(o: &mut OpenOptions) {
    o.mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
}
pub fn secure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        std::fs::set_permissions(path, Permissions::from_mode(0o700))?;
    }
    let m = std::fs::symlink_metadata(path)?;
    ensure!(
        m.is_dir() && !m.file_type().is_symlink(),
        "state path must be a real directory"
    );
    ensure!(
        m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0,
        "state directory must be owned by you with mode 0700: {}",
        path.display()
    );
    Ok(())
}
pub fn detach(c: &mut Command) -> Result<()> {
    unsafe {
        c.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(())
}
pub fn workload(c: &mut Command) -> Result<()> {
    c.process_group(0);
    #[cfg(target_os = "linux")]
    unsafe {
        let parent = libc::getpid();
        c.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(std::io::Error::other(
                    "runner exited before workload launch",
                ));
            }
            Ok(())
        });
    }
    Ok(())
}
pub fn identity(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let fields = stat
            .rsplit_once(')')?
            .1
            .split_whitespace()
            .collect::<Vec<_>>();
        if fields.first() == Some(&"Z") {
            return None;
        }
        let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
        Some(format!("{}:{}:{}", boot.trim(), pid, fields.get(19)?))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}
pub fn signal_group(pid: u32, signal: i32) -> Result<()> {
    ensure!(pid > 1, "refusing invalid process group");
    let r = unsafe { libc::kill(-(pid as i32), signal) };
    if r < 0 {
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() != Some(libc::ESRCH) {
            return Err(e.into());
        }
    }
    Ok(())
}
pub fn exit_signal(s: &std::process::ExitStatus) -> Option<i32> {
    s.signal()
}

pub fn endpoint_identity(path: &Path) -> Result<String> {
    use std::os::unix::fs::FileTypeExt;
    let m = std::fs::symlink_metadata(path)?;
    ensure!(
        m.file_type().is_socket() && m.uid() == unsafe { libc::geteuid() },
        "Desktop endpoint must be an owned Unix socket"
    );
    Ok(format!(
        "{}:{}:{}:{}",
        m.dev(),
        m.ino(),
        m.ctime(),
        m.ctime_nsec()
    ))
}
pub async fn connect_host(path: &Path) -> Result<tokio::net::UnixStream> {
    let stream = tokio::net::UnixStream::connect(path).await?;
    ensure!(
        stream.peer_cred()?.uid() == unsafe { libc::geteuid() },
        "Desktop endpoint peer owner mismatch"
    );
    Ok(stream)
}

pub fn runner_initialize() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let result = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
/// Called only after the supervised Tokio Child has been waited on.
pub fn reap_descendants() -> Result<bool> {
    loop {
        let result = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        if result > 0 {
            continue;
        }
        if result == 0 {
            return Ok(false);
        }
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::ECHILD) {
            return Ok(true);
        }
        return Err(e.into());
    }
}

pub fn verify_peer(stream: &tokio::net::UnixStream) -> Result<()> {
    ensure!(
        stream.peer_cred()?.uid() == unsafe { libc::geteuid() },
        "IPC peer owner mismatch"
    );
    Ok(())
}
