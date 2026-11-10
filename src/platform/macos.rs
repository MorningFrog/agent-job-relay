//! Experimental Darwin adapter. No native/runtime acceptance is claimed.
//! Unlike Linux, macOS provides neither PR_SET_CHILD_SUBREAPER nor PDEATHSIG.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsStr,
    io::Read,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

// Apple's public dyld ABI returns an image header. Keep the pointer opaque:
// libc's Mach header bindings are deprecated, and only its bytes are needed.
unsafe extern "C" {
    #[link_name = "_dyld_get_image_header"]
    fn dyld_image_header(image_index: u32) -> *const u8;
}

fn process_info(pid: u32) -> Option<libc::proc_bsdinfo> {
    let pid = i32::try_from(pid).ok().filter(|pid| *pid > 0)?;
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>();
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if read != size as i32 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    (info.pbi_pid == pid as u32 && info.pbi_status != libc::SZOMB).then_some(info)
}

pub(super) fn identity(pid: u32) -> Option<String> {
    let info = process_info(pid)?;
    let mut boot = [0u8; 128];
    let mut size = boot.len();
    let result = unsafe {
        libc::sysctlbyname(
            c"kern.bootsessionuuid".as_ptr(),
            boot.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || size == 0 || size > boot.len() {
        return None;
    }
    let boot = std::ffi::CStr::from_bytes_with_nul(&boot[..size])
        .ok()?
        .to_str()
        .ok()?;
    let boot = uuid::Uuid::parse_str(boot).ok()?;
    Some(format!(
        "macos:{boot}:{pid}:{}:{}",
        info.pbi_start_tvsec, info.pbi_start_tvusec
    ))
}

fn process_image_path(pid: u32) -> Option<PathBuf> {
    let pid = i32::try_from(pid).ok().filter(|pid| *pid > 0)?;
    let mut buffer = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let read = unsafe { libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if read <= 0 {
        return None;
    }
    let end = buffer.iter().position(|byte| *byte == 0)?;
    Some(PathBuf::from(OsStr::from_bytes(&buffer[..end])))
}

pub(super) fn process_image_path_available(pid: u32) -> Option<bool> {
    if pid == std::process::id() {
        return Some(self_image().is_ok());
    }
    let before = identity(pid)?;
    let path = process_image_path(pid)?;
    (identity(pid).as_deref() == Some(before.as_str())).then(|| path.is_file())
}

fn image_uuid(commands: &[u8], count: u32) -> Result<[u8; 16]> {
    let mut offset = 0usize;
    let mut uuid = None;
    for _ in 0..count {
        let header = commands
            .get(offset..offset + 8)
            .context("truncated Mach-O command")?;
        let kind = u32::from_le_bytes(header[..4].try_into()?);
        let size = u32::from_le_bytes(header[4..].try_into()?) as usize;
        ensure!(size >= 8 && size % 8 == 0, "invalid Mach-O command size");
        let command = commands
            .get(offset..offset + size)
            .context("truncated Mach-O command")?;
        if kind == 0x1b {
            ensure!(size == 24 && uuid.is_none(), "invalid Mach-O UUID");
            uuid = Some(command[8..24].try_into()?);
        }
        offset += size;
    }
    ensure!(offset == commands.len(), "inconsistent Mach-O commands");
    uuid.context("native Mach-O UUID missing")
}

/// Darwin cannot execute an unlinked image through /proc/self/exe. Refuse a
/// replaced pathname instead of launching a runner from a different build.
pub(super) fn self_image() -> Result<PathBuf> {
    let loaded = unsafe { dyld_image_header(0) };
    ensure!(!loaded.is_null(), "loaded Mach-O header is unavailable");
    let header = unsafe { std::ptr::read_unaligned(loaded.cast::<[u32; 8]>()) };
    let (magic, count, size) = (header[0], header[4], header[5]);
    ensure!(
        magic == libc::MH_MAGIC_64 && count <= 4096 && size <= 1048576,
        "invalid loaded Mach-O header"
    );
    // dyld has already mapped and validated this image's load commands.
    let commands =
        unsafe { std::slice::from_raw_parts(loaded.cast::<u8>().add(32), size as usize) };
    let expected = image_uuid(commands, count)?;
    let path = std::env::current_exe()?;
    let mut file = super::read_file(&path)?;
    let mut disk_header = [0u8; 32];
    file.read_exact(&mut disk_header)?;
    ensure!(
        u32::from_le_bytes(disk_header[..4].try_into()?) == libc::MH_MAGIC_64
            && u32::from_le_bytes(disk_header[16..20].try_into()?) == count
            && u32::from_le_bytes(disk_header[20..24].try_into()?) == size,
        "native executable was replaced; finish active tasks and restart the same profile"
    );
    let mut disk_commands = vec![0; size as usize];
    file.read_exact(&mut disk_commands)?;
    ensure!(
        image_uuid(&disk_commands, count)? == expected,
        "native executable was replaced; no runner was launched from the new build"
    );
    Ok(path)
}

pub(super) fn publish_new_directory(source: &Path, destination: &Path) -> Result<()> {
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())?;
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

pub(super) fn group_is_empty(pid: u32) -> Result<bool> {
    let pid = i32::try_from(pid)?;
    ensure!(pid > 1, "refusing invalid process group");
    if unsafe { libc::kill(-pid, 0) } == 0 {
        return Ok(false);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(true);
    }
    Err(error.into())
}
