//! Windows ownership, local pipes and crash-fenced Job Object supervision.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, c_void},
    fs::{File, OpenOptions},
    mem::{size_of, size_of_val, zeroed},
    os::windows::{
        ffi::OsStrExt,
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    ptr::null_mut,
};
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use windows_sys::Win32::{
    Foundation::*,
    Globalization::CompareStringOrdinal,
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::{Console::*, JobObjects::*, Pipes::*, SystemServices::*, Threading::*},
    UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
};
fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
fn check(ok: i32) -> Result<()> {
    if ok == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
fn owned(raw: HANDLE) -> Result<OwnedHandle> {
    ensure!(
        !raw.is_null() && raw != INVALID_HANDLE_VALUE,
        "Windows handle: {}",
        std::io::Error::last_os_error()
    );
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}
struct LocalMemory(*mut c_void);
impl Drop for LocalMemory {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}
fn sid_text(sid: PSID) -> Result<String> {
    let mut text = null_mut();
    unsafe {
        check(ConvertSidToStringSidW(sid, &mut text))?;
    }
    let _memory = LocalMemory(text.cast());
    let mut len = 0;
    unsafe {
        while *text.add(len) != 0 {
            len += 1;
        }
        Ok(String::from_utf16(std::slice::from_raw_parts(text, len))?)
    }
}
fn token_sid(process: HANDLE, owner: bool) -> Result<String> {
    let class = if owner { TokenOwner } else { TokenUser };
    let mut token = null_mut();
    unsafe {
        check(OpenProcessToken(process, TOKEN_QUERY, &mut token))?;
    }
    let token = owned(token)?;
    let mut size = 0;
    unsafe {
        GetTokenInformation(token.as_raw_handle(), class, null_mut(), 0, &mut size);
    }
    ensure!(size > 0 && size < 65536, "invalid Windows token size");
    let mut data = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    unsafe {
        check(GetTokenInformation(
            token.as_raw_handle(),
            class,
            data.as_mut_ptr().cast(),
            size,
            &mut size,
        ))?;
        sid_text(if owner {
            (*(data.as_ptr().cast::<TOKEN_OWNER>())).Owner
        } else {
            (*(data.as_ptr().cast::<TOKEN_USER>())).User.Sid
        })
    }
}
fn process_sid(process: HANDLE) -> Result<String> {
    token_sid(process, false)
}
pub fn current_sid() -> Result<String> {
    process_sid(unsafe { GetCurrentProcess() })
}
struct Security {
    _descriptor: LocalMemory,
    attributes: SECURITY_ATTRIBUTES,
}
impl Security {
    fn new(directory: bool) -> Result<Self> {
        let sid = current_sid()?;
        let inherit = if directory { "OICI" } else { "" };
        let sddl = wide(format!(
            "O:{sid}D:P(A;{inherit};GA;;;{sid})(A;{inherit};GA;;;SY)"
        ));
        let mut descriptor = null_mut();
        unsafe {
            check(ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                null_mut(),
            ))?;
        }
        Ok(Self {
            _descriptor: LocalMemory(descriptor),
            attributes: SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            },
        })
    }
}
fn verify_security(handle: HANDLE) -> Result<()> {
    let mut owner = null_mut();
    let mut dacl = null_mut();
    let mut descriptor = null_mut();
    let code = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    ensure!(
        code == 0,
        "cannot inspect Windows file security: {}",
        std::io::Error::from_raw_os_error(code as i32)
    );
    let _memory = LocalMemory(descriptor);
    let sid = current_sid()?;
    ensure!(
        !owner.is_null() && sid_text(owner)? == sid,
        "state/install owner SID differs from current user"
    );
    ensure!(!dacl.is_null(), "state/install must have a private DACL");
    unsafe {
        for index in 0..(*dacl).AceCount as u32 {
            let mut raw = null_mut();
            check(GetAce(dacl, index, &mut raw))?;
            let header = &*raw.cast::<ACE_HEADER>();
            if header.AceType as u32 == ACCESS_DENIED_ACE_TYPE {
                continue;
            }
            ensure!(
                header.AceType as u32 == ACCESS_ALLOWED_ACE_TYPE,
                "unsupported state/install access rule"
            );
            let ace = &*raw.cast::<ACCESS_ALLOWED_ACE>();
            let allowed = sid_text((&ace.SidStart as *const u32).cast_mut().cast())?;
            ensure!(
                allowed == sid || allowed == "S-1-5-18",
                "state/install DACL grants another principal access"
            );
        }
    }
    Ok(())
}
pub fn secure_options(options: &mut OpenOptions) {
    options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}
pub fn open_private_file(path: &Path, exclusive: bool, append: bool) -> Result<File> {
    check_path(path)?;
    let security = Security::new(false)?;
    let access = if append {
        FILE_APPEND_DATA | FILE_GENERIC_READ
    } else {
        GENERIC_READ | GENERIC_WRITE
    };
    let handle = owned(unsafe {
        CreateFileW(
            wide(path).as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &security.attributes,
            if exclusive { CREATE_NEW } else { OPEN_ALWAYS },
            FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    })?;
    let file = File::from(handle);
    verify_private_file(&file)?;
    Ok(file)
}
pub fn verify_private_file(file: &File) -> Result<()> {
    let mut info = unsafe { zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    unsafe {
        check(GetFileInformationByHandle(file.as_raw_handle(), &mut info))?;
    }
    ensure!(
        info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "refusing a reparse point"
    );
    verify_security(file.as_raw_handle())
}
pub fn check_path(path: &Path) -> Result<()> {
    let mut walked = PathBuf::new();
    for component in path.components() {
        ensure!(
            !matches!(component, std::path::Component::ParentDir),
            "path must not contain .."
        );
        walked.push(component);
        if let Ok(metadata) = std::fs::symlink_metadata(&walked) {
            ensure!(
                !super::is_link(&metadata),
                "refusing reparse point in path: {}",
                walked.display()
            );
        }
    }
    Ok(())
}
pub fn secure_dir(path: &Path) -> Result<()> {
    check_path(path)?;
    if !path.exists() {
        let parent = path.parent().context("private directory needs a parent")?;
        if !parent.exists() {
            secure_dir(parent)?;
        }
        let security = Security::new(true)?;
        let result = unsafe { CreateDirectoryW(wide(path).as_ptr(), &security.attributes) };
        if result == 0
            && std::io::Error::last_os_error().raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32)
        {
            check(result)?;
        }
    }
    ensure!(
        std::fs::symlink_metadata(path)?.is_dir(),
        "state path must be a real directory"
    );
    let mut options = OpenOptions::new();
    options
        .access_mode(READ_CONTROL | FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    verify_private_file(&options.open(path)?)
}
pub fn initialize() {}
pub fn detach(command: &mut std::process::Command) -> Result<()> {
    // Refuse a non-breakaway host job instead of silently sharing its lifetime.
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB);
    Ok(())
}
fn quote(value: &OsStr) -> Vec<u16> {
    let mut out = vec![b'"' as u16];
    let mut slashes = 0;
    for value in value.encode_wide() {
        if value == b'\\' as u16 {
            slashes += 1;
            continue;
        }
        out.extend(std::iter::repeat_n(
            b'\\' as u16,
            if value == b'"' as u16 {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        slashes = 0;
        out.push(value);
    }
    out.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    out.push(b'"' as u16);
    out
}

/// Spawn daemon/runner modes without inheritable handles, including host capture pipes.
/// Closing process/thread handles requires no Windows zombie reaping.
pub fn spawn_background(command: std::process::Command) -> Result<()> {
    spawn_with_environment(command, std::env::vars_os().collect())
}
fn environment_order(a: &OsStr, b: &OsStr) -> std::cmp::Ordering {
    let a: Vec<_> = a.encode_wide().collect();
    let b: Vec<_> = b.encode_wide().collect();
    unsafe { CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1) }
        .cmp(&2)
}

fn spawn_with_environment(
    command: std::process::Command,
    mut environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
) -> Result<()> {
    let program = Path::new(command.get_program());
    ensure!(
        program.is_absolute(),
        "background runtime image must be absolute"
    );
    let mut line = quote(command.get_program());
    for arg in command.get_args() {
        line.push(b' ' as u16);
        line.extend(quote(arg));
    }
    line.push(0);
    for (key, value) in command.get_envs() {
        environment.retain(|(old, _)| !environment_order(old, key).is_eq());
        if let Some(value) = value {
            environment.push((key.into(), value.into()));
        }
    }
    environment.sort_by(|(a, _), (b, _)| environment_order(a, b));
    let mut block = Vec::<u16>::new();
    for (key, value) in environment {
        block.extend(key.encode_wide());
        block.push(b'=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    let cwd = command.get_current_dir().map(wide);
    let mut startup = unsafe { zeroed::<STARTUPINFOW>() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    let mut process = unsafe { zeroed::<PROCESS_INFORMATION>() };
    unsafe {
        check(CreateProcessW(
            wide(program).as_ptr(),
            line.as_mut_ptr(),
            null_mut(),
            null_mut(),
            0,
            DETACHED_PROCESS
                | CREATE_NEW_PROCESS_GROUP
                | CREATE_BREAKAWAY_FROM_JOB
                | CREATE_UNICODE_ENVIRONMENT,
            block.as_ptr().cast(),
            cwd.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            &startup,
            &mut process,
        ))
        .context(
            "independent Windows launch failed; an enclosing host Job Object may forbid breakaway",
        )?;
    }
    let _thread = owned(process.hThread)?;
    let _process = owned(process.hProcess)?;
    Ok(())
}
fn process_handle(pid: u32, access: u32) -> Result<OwnedHandle> {
    owned(unsafe { OpenProcess(access, 0, pid) })
}
fn handle_identity(handle: HANDLE, pid: u32) -> Result<String> {
    let mut created = unsafe { zeroed::<FILETIME>() };
    let mut ended = created;
    let mut kernel = created;
    let mut user = created;
    unsafe {
        check(GetProcessTimes(
            handle,
            &mut created,
            &mut ended,
            &mut kernel,
            &mut user,
        ))?;
    }
    let mut code = 0;
    unsafe {
        check(GetExitCodeProcess(handle, &mut code))?;
    }
    if code != STILL_ACTIVE as u32 {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "process has exited").into());
    }
    Ok(format!(
        "windows:{pid}:{}",
        ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64
    ))
}
pub fn identity(pid: u32) -> Option<String> {
    let handle = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
    handle_identity(handle.as_raw_handle(), pid).ok()
}
/// Pin and verify the original instance before a diagnostic/test force stop.
pub fn terminate_instance(pid: u32, expected: &str) -> Result<()> {
    let handle = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE)?;
    ensure!(
        process_sid(handle.as_raw_handle())? == current_sid()?,
        "process owner mismatch"
    );
    ensure!(
        handle_identity(handle.as_raw_handle(), pid)? == expected,
        "process identity changed; no termination was sent"
    );
    unsafe { check(TerminateProcess(handle.as_raw_handle(), 1)) }
}
pub fn process_image_path_available(pid: u32) -> Option<bool> {
    let handle = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
    let mut path = vec![0u16; 32768];
    let mut size = path.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(handle.as_raw_handle(), 0, path.as_mut_ptr(), &mut size)
    } == 0
    {
        return None;
    }
    Some(PathBuf::from(String::from_utf16(&path[..size as usize]).ok()?).is_file())
}
pub fn exit_signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}
pub fn runner_initialize() -> Result<()> {
    if unsafe { GetConsoleWindow() }.is_null() {
        unsafe {
            check(AllocConsole())?;
        }
    }
    unsafe {
        ShowWindow(GetConsoleWindow(), SW_HIDE);
    }
    Ok(())
}
pub fn reap_descendants() -> Result<bool> {
    Ok(true)
}
pub struct Interrupt(Option<tokio::signal::windows::CtrlC>);
impl Interrupt {
    pub async fn recv(&mut self) -> Option<()> {
        match &mut self.0 {
            Some(signal) => signal.recv().await,
            None => std::future::pending().await,
        }
    }
}
pub fn interrupt_signal() -> Result<Interrupt> {
    // Detached daemons have no console; their authenticated IPC shutdown remains available.
    if unsafe { GetConsoleWindow() }.is_null() {
        return Ok(Interrupt(None));
    }
    Ok(Interrupt(Some(tokio::signal::windows::ctrl_c()?)))
}
pub struct GroupGuard {
    job: OwnedHandle,
    process: Option<OwnedHandle>,
    pid: u32,
}
impl GroupGuard {
    fn create(name: &str) -> Result<Self> {
        let security = Security::new(false)?;
        let raw = unsafe { CreateJobObjectW(&security.attributes, wide(name).as_ptr()) };
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let job = owned(raw)?;
        ensure!(!existed, "workload Job Object already exists");
        let mut limits = unsafe { zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            check(SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of_val(&limits) as u32,
            ))?;
        }
        Ok(Self {
            job,
            process: None,
            pid: 0,
        })
    }
    pub fn signal(&self, signal: i32) -> Result<()> {
        if signal == 9 {
            return unsafe { check(TerminateJobObject(self.job.as_raw_handle(), 1)) };
        }
        if let Some(process) = &self.process
            && handle_identity(process.as_raw_handle(), self.pid).is_ok()
        {
            // Console Ctrl-Break is best effort; force termination is always job-scoped.
            unsafe {
                GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.pid);
            }
        }
        Ok(())
    }
    pub fn disarm(&mut self) {}
}
impl Drop for GroupGuard {
    fn drop(&mut self) {
        let _ = self.signal(9);
    }
}
/// The helper joins the owner's job before it reads/executes the user command.
/// If the owner dies before/during join, opening fails or last-close kills the
/// helper. There is no suspended user process outside a Job Object.
pub fn spawn_workload(
    config: &crate::config::Config,
    job: &crate::core::Job,
) -> Result<(tokio::process::Child, GroupGuard)> {
    let name = format!("Local\\AgentJobRelay-workload-{}", crate::core::id());
    let mut guard = GroupGuard::create(&name)?;
    let mut command = std::process::Command::new(super::self_image()?);
    command
        .args(["--state-dir"])
        .arg(&config.root)
        .arg("windows-workload")
        .arg(&job.job_id)
        .arg(&name)
        .current_dir(&job.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(CREATE_NEW_PROCESS_GROUP);
    let child = tokio::process::Command::from(command)
        .kill_on_drop(true)
        .spawn()?;
    guard.pid = child.id().context("missing workload helper PID")?;
    guard.process = Some(process_handle(
        guard.pid,
        PROCESS_QUERY_LIMITED_INFORMATION,
    )?);
    Ok((child, guard))
}
pub fn join_workload(name: &str) -> Result<()> {
    let uuid = name
        .strip_prefix("Local\\AgentJobRelay-workload-")
        .context("invalid workload Job Object name")?;
    crate::core::validate_id(uuid)?;
    let handle = owned(unsafe {
        OpenJobObjectW(
            JOB_OBJECT_ASSIGN_PROCESS | JOB_OBJECT_QUERY,
            0,
            wide(name).as_ptr(),
        )
    })?;
    unsafe {
        check(AssignProcessToJobObject(
            handle.as_raw_handle(),
            GetCurrentProcess(),
        ))?;
    }
    drop(handle); // Only the independent runner retains a kill-on-close handle.
    Ok(())
}
pub fn shell_command(interpreter: &str, script: &str) -> std::process::Command {
    let mut command = std::process::Command::new(interpreter);
    let name = Path::new(interpreter)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    match name.as_str() {
        "cmd" => {
            command
                .args(["/D", "/S", "/C"])
                .raw_arg(format!("\"{script}\""));
        }
        "pwsh" | "powershell" => {
            command.args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ]);
        }
        _ => {
            command.args(["-c", script]);
        }
    }
    command
}
pub fn local_endpoint(root: &Path) -> PathBuf {
    let mut digest = Sha256::new();
    digest.update(current_sid().unwrap_or_default());
    for value in root.as_os_str().encode_wide() {
        digest.update(
            (if (65..=90).contains(&value) {
                value + 32
            } else {
                value
            })
            .to_le_bytes(),
        );
    }
    PathBuf::from(format!(
        "\\\\.\\pipe\\AgentJobRelay-{:x}",
        digest.finalize()
    ))
}
pub fn validate_pipe_path(path: &Path) -> Result<()> {
    let text = path.to_str().context("pipe name must be valid Unicode")?;
    let tail = text
        .strip_prefix("\\\\.\\pipe\\")
        .context("endpoint must be a local \\\\.\\pipe\\ name; remote pipes are refused")?;
    ensure!(
        !tail.is_empty() && tail.len() <= 180 && !tail.contains(['\\', '/', '\0']),
        "invalid local pipe name"
    );
    Ok(())
}
pub type ClientStream = NamedPipeClient;
pub type ServerStream = NamedPipeServer;
pub struct Listener {
    server: NamedPipeServer,
    path: PathBuf,
}
impl Listener {
    fn create(path: &Path, first: bool) -> Result<NamedPipeServer> {
        validate_pipe_path(path)?;
        let security = Security::new(false)?;
        Ok(unsafe {
            ServerOptions::new()
                .first_pipe_instance(first)
                .reject_remote_clients(true)
                .max_instances(64)
                .create_with_security_attributes_raw(
                    path,
                    (&security.attributes as *const SECURITY_ATTRIBUTES)
                        .cast_mut()
                        .cast(),
                )?
        })
    }
    pub fn bind(path: &Path) -> Result<Self> {
        Ok(Self {
            server: Self::create(path, true)?,
            path: path.into(),
        })
    }
    pub async fn accept(&mut self) -> Result<NamedPipeServer> {
        self.server.connect().await?;
        let next = Self::create(&self.path, false)?;
        Ok(std::mem::replace(&mut self.server, next))
    }
}
fn checked_peer(handle: HANDLE, server: bool) -> Result<(u32, String)> {
    let mut pid = 0;
    unsafe {
        check(if server {
            GetNamedPipeServerProcessId(handle, &mut pid)
        } else {
            GetNamedPipeClientProcessId(handle, &mut pid)
        })
        .context("cannot identify named-pipe peer process")?;
    }
    let process = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION)
        .context("cannot open named-pipe peer process for authentication")?;
    ensure!(
        process_sid(process.as_raw_handle()).context("cannot read named-pipe peer user SID")?
            == current_sid().context("cannot read current process user SID")?,
        "pipe peer owner SID mismatch"
    );
    let identity = handle_identity(process.as_raw_handle(), pid)
        .context("cannot read named-pipe peer process identity")?;
    let mut rechecked = 0;
    unsafe {
        check(if server {
            GetNamedPipeServerProcessId(handle, &mut rechecked)
        } else {
            GetNamedPipeClientProcessId(handle, &mut rechecked)
        })
        .context("cannot recheck named-pipe peer process")?;
    }
    ensure!(pid == rechecked, "pipe peer changed during authentication");
    Ok((pid, identity))
}
pub fn verify_peer(stream: &NamedPipeServer) -> Result<()> {
    checked_peer(stream.as_raw_handle(), false)?;
    Ok(())
}
pub fn peer_pid(stream: &ClientStream) -> Result<Option<u32>> {
    Ok(Some(checked_peer(stream.as_raw_handle(), true)?.0))
}
fn open_pipe(path: &Path) -> Result<ClientStream> {
    validate_pipe_path(path)?;
    let stream = ClientOptions::new()
        .security_qos_flags(SECURITY_IDENTIFICATION)
        .open(path)
        .context("cannot open named pipe with current process permissions")?;
    checked_peer(stream.as_raw_handle(), true)?;
    Ok(stream)
}
pub async fn connect_host(path: &Path) -> Result<ClientStream> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match open_pipe(path) {
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32))
                    && tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await
            }
            result => return result,
        }
    }
}
pub fn endpoint_identity(path: &Path) -> Result<String> {
    let stream = open_pipe(path)?;
    let (_, identity) = checked_peer(stream.as_raw_handle(), true)?;
    Ok(format!("{identity}:{}", path.display()))
}
pub fn cleanup_endpoint(_: &Path) -> Result<()> {
    Ok(())
}
pub fn rename_write_through(source: &Path, destination: &Path, replace: bool) -> Result<()> {
    unsafe {
        check(MoveFileExW(
            wide(source).as_ptr(),
            wide(destination).as_ptr(),
            MOVEFILE_WRITE_THROUGH
                | if replace {
                    MOVEFILE_REPLACE_EXISTING
                } else {
                    0
                },
        ))
    }
}
pub fn connected_endpoint_identity(stream: &ClientStream, path: &Path) -> Result<String> {
    let (_, identity) = checked_peer(stream.as_raw_handle(), true)?;
    Ok(format!("{identity}:{}", path.display()))
}

/// Node's public pipe API does not expose a DACL option. The host plugin calls
/// this once after listen; the native core pins/checks its exact server before
/// restricting the named pipe's security descriptor to the current SID.
pub fn secure_bridge_pipe(path: &Path, expected_pid: u32) -> Result<serde_json::Value> {
    validate_pipe_path(path)?;
    let handle = owned(unsafe {
        CreateFileW(
            wide(path).as_ptr(),
            GENERIC_READ | GENERIC_WRITE | WRITE_DAC | WRITE_OWNER,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            null_mut(),
        )
    })?;
    let (pid, identity) = checked_peer(handle.as_raw_handle(), true)?;
    ensure!(
        pid == expected_pid,
        "plugin pipe server PID differs; no ACL was changed"
    );
    let mut owner = null_mut();
    let mut descriptor = null_mut();
    let code = unsafe {
        GetSecurityInfo(
            handle.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    };
    ensure!(code == 0, "cannot inspect plugin pipe owner");
    let _owner_memory = LocalMemory(descriptor);
    let server = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    ensure!(
        !owner.is_null()
            && (sid_text(owner)? == current_sid()?
                || sid_text(owner)? == token_sid(server.as_raw_handle(), true)?),
        "plugin pipe owner differs from its verified creator token; no ACL was changed"
    );
    let security = Security::new(false)?;
    let mut dacl = null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    unsafe {
        check(GetSecurityDescriptorDacl(
            security.attributes.lpSecurityDescriptor,
            &mut present,
            &mut dacl,
            &mut defaulted,
        ))?;
    }
    ensure!(present != 0 && !dacl.is_null(), "private pipe DACL missing");
    let mut assigned_owner = null_mut();
    let mut owner_defaulted = 0;
    unsafe {
        check(GetSecurityDescriptorOwner(
            security.attributes.lpSecurityDescriptor,
            &mut assigned_owner,
            &mut owner_defaulted,
        ))?;
    }
    let code = unsafe {
        SetSecurityInfo(
            handle.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION
                | DACL_SECURITY_INFORMATION
                | PROTECTED_DACL_SECURITY_INFORMATION,
            assigned_owner,
            null_mut(),
            dacl,
            null_mut(),
        )
    };
    ensure!(
        code == 0,
        "cannot restrict plugin pipe DACL: {}",
        std::io::Error::from_raw_os_error(code as i32)
    );
    verify_security(handle.as_raw_handle())?;
    ensure!(
        checked_peer(handle.as_raw_handle(), true)?.1 == identity,
        "plugin server changed during security setup"
    );
    Ok(serde_json::json!({"secured":true,"server_pid":pid,"private_acl":true}))
}
pub fn prepare_bridge(config: &crate::config::Config) -> Result<serde_json::Value> {
    config.prepare()?;
    let parent = config.root.join("bridges");
    secure_dir(&parent)?;
    let unique = crate::core::id();
    let root = parent.join(format!("oc-{unique}"));
    secure_dir(&root)?;
    Ok(
        serde_json::json!({"root":root,"endpoint":format!("\\\\.\\pipe\\AgentJobRelay-opencode-{unique}")}),
    )
}
pub fn verify_bridge_endpoint(stream: &ClientStream) -> Result<()> {
    verify_security(stream.as_raw_handle())
}

mod startup;
pub(super) use startup::launch_daemon;
pub use startup::{bootstrap, outside_start_command};
