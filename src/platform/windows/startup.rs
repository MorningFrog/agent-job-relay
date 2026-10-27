//! Same-user, on-demand daemon bootstrap outside a restrictive host cleanup job.
//! CIM launches only this executable's bounded helper. No task/service is installed.
use super::*;
use crate::{config::Config, ipc};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, os::windows::ffi::OsStringExt, process::Stdio, time::Duration};

const LIMIT: usize = 2 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Transfer {
    version: String,
    environment: Vec<(Vec<u16>, Vec<u16>)>,
    cwd: Option<PathBuf>,
}
fn in_job() -> Result<bool> {
    let mut result = 0;
    unsafe {
        check(IsProcessInJob(GetCurrentProcess(), null_mut(), &mut result))?;
    }
    Ok(result != 0)
}
fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub async fn launch_daemon(mut command: std::process::Command, config: &Config) -> Result<()> {
    let mode = std::env::var("AJR_WINDOWS_STARTUP").unwrap_or_else(|_| "auto".into());
    ensure!(
        ["auto", "direct"].contains(&mode.as_str()),
        "AJR_WINDOWS_STARTUP must be auto or direct"
    );
    command.env("AJR_DAEMON_LAUNCH_METHOD", "windows_native");
    let mut direct = std::process::Command::new(command.get_program());
    direct.args(command.get_args());
    for (key, value) in command.get_envs() {
        match value {
            Some(v) => {
                direct.env(key, v);
            }
            None => {
                direct.env_remove(key);
            }
        }
    }
    if let Some(cwd) = command.get_current_dir() {
        direct.current_dir(cwd);
    }
    match spawn_background(direct) {
        Ok(()) => return Ok(()),
        Err(error) => {
            let denied = error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32));
            if mode != "auto" || !denied || !in_job()? {
                return Err(error.context(format!("Windows startup mode={mode}")));
            }
        }
    }
    tokio::time::timeout(Duration::from_secs(14), broker(command, config)).await
        .context("Windows CIM bootstrap timed out; use same-profile daemon start in an outside terminal")?
        .context("automatic Windows startup could not use local CIM; check Windows PowerShell/CIM policy or start the same profile in an outside terminal")
}
async fn broker(command: std::process::Command, config: &Config) -> Result<()> {
    let executable = crate::platform::self_image()?;
    let caller_token = startup_token(unsafe { GetCurrentProcess() })?;
    ensure!(
        Path::new(command.get_program()) == executable
            && command.get_args().eq([
                OsStr::new("--state-dir"),
                config.root.as_os_str(),
                OsStr::new("daemon")
            ]),
        "CIM bootstrap is restricted to this executable's daemon mode"
    );
    let endpoint = PathBuf::from(format!(
        r"\\.\pipe\AgentJobRelay-startup-{}",
        crate::core::id()
    ));
    let mut listener = Listener::bind(&endpoint)?;
    let parent = identity(std::process::id()).context("cannot identify startup caller")?;
    let args = [
        OsString::from("--state-dir"),
        config.root.as_os_str().into(),
        OsString::from("windows-bootstrap"),
        endpoint.as_os_str().into(),
        parent.into(),
    ];
    let mut line = quote(executable.as_os_str());
    for arg in args {
        line.push(b' ' as u16);
        line.extend(quote(&arg));
    }
    let line = String::from_utf16(&line).context("bootstrap path must be Unicode")?;
    // All interpolated values are single-quoted literals, never script fragments.
    // No environment values, commands from a job, credentials or remote target enter CIM.
    let script = format!(
        r#"$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';[Console]::OutputEncoding=[Text.UTF8Encoding]::new();$startup=New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly -Property @{{ShowWindow=[uint16]0;CreateFlags=[uint32]0x10}};$result=Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine={};ProcessStartupInformation=$startup}};if($result.ReturnValue -ne 0){{throw ('local CIM creation failed: '+$result.ReturnValue)}};[Console]::WriteLine($result.ProcessId)"#,
        literal(&line)
    );
    let powershell =
        PathBuf::from(std::env::var_os("SystemRoot").context("SystemRoot unavailable")?)
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    ensure!(powershell.is_absolute(), "SystemRoot must be absolute");
    let mut launcher = tokio::process::Command::new(powershell);
    launcher
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    launcher.creation_flags(CREATE_NO_WINDOW);
    let output = tokio::time::timeout(Duration::from_secs(8), launcher.output())
        .await
        .context("local CIM command timed out")??;
    ensure!(
        output.status.success(),
        "local CIM refused startup: {}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(2048)
            .collect::<String>()
    );
    let pid: u32 = std::str::from_utf8(&output.stdout)?
        .trim()
        .parse()
        .context("invalid local CIM process response")?;
    let process = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION)
        .context("CIM startup helper exited before authentication")?;
    ensure!(
        process_sid(process.as_raw_handle())? == current_sid()?,
        "bootstrap process has another SID; no environment was sent"
    );
    ensure!(
        startup_token(process.as_raw_handle())? == caller_token,
        "bootstrap token differs from the caller; no environment was sent"
    );
    verify_image(process.as_raw_handle(), &executable)?;
    let expected = handle_identity(process.as_raw_handle(), pid)?;
    let mut socket = listener.accept().await?;
    ensure!(
        checked_peer(socket.as_raw_handle(), false)?.1 == expected,
        "bootstrap pipe client differs from the recorded process; no environment was sent"
    );
    let mut environment: Vec<_> = std::env::vars_os().collect();
    for (key, value) in command.get_envs() {
        environment.retain(|(k, _)| !environment_order(k, key).is_eq());
        if let Some(v) = value {
            environment.push((key.into(), v.into()));
        }
    }
    let transfer = Transfer {
        version: env!("CARGO_PKG_VERSION").into(),
        environment: environment
            .into_iter()
            .map(|(k, v)| (k.encode_wide().collect(), v.encode_wide().collect()))
            .collect(),
        cwd: Some(
            command
                .get_current_dir()
                .map(Path::to_path_buf)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?,
        ),
    };
    let bytes = serde_json::to_value(transfer)?;
    ensure!(
        serde_json::to_vec(&bytes)?.len() <= LIMIT,
        "startup environment exceeds transfer budget; no daemon was launched"
    );
    ipc::write_frame(&mut socket, &bytes).await?;
    let ack = ipc::read_frame(&mut socket, 4096).await?;
    ensure!(
        ack["started"] == true,
        "bootstrap refused daemon launch: {}",
        ack["error"].as_str().unwrap_or("invalid response")
    );
    Ok(())
}

pub async fn bootstrap(config: Config, endpoint: PathBuf, parent_identity: String) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        ensure!(
            endpoint
                .to_str()
                .is_some_and(|s| s.starts_with(r"\\.\pipe\AgentJobRelay-startup-")),
            "invalid startup endpoint"
        );
        validate_pipe_path(&endpoint)?;
        let mut socket = connect_host(&endpoint).await?;
        verify_security(socket.as_raw_handle())?;
        ensure!(
            checked_peer(socket.as_raw_handle(), true)?.1 == parent_identity,
            "startup server instance differs; no environment was received"
        );
        let (parent_pid, _) = checked_peer(socket.as_raw_handle(), true)?;
        let parent = process_handle(parent_pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        ensure!(
            startup_token(parent.as_raw_handle())?
                == startup_token(unsafe { GetCurrentProcess() })?,
            "startup caller token differs"
        );
        verify_image(parent.as_raw_handle(), &crate::platform::self_image()?)?;
        let transfer: Transfer =
            serde_json::from_value(ipc::read_frame(&mut socket, LIMIT).await?)?;
        let launch = (|| -> Result<()> {
            ensure!(
                transfer.version == env!("CARGO_PKG_VERSION"),
                "startup executable version changed"
            );
            let mut environment = Vec::new();
            for (key, value) in transfer.environment {
                ensure!(
                    !key.is_empty() && !key.contains(&0) && !value.contains(&0),
                    "invalid startup environment"
                );
                // Hidden drive-current-directory entries begin with '=' on Windows.
                ensure!(
                    !key.iter().skip(1).any(|&x| x == b'=' as u16),
                    "invalid environment key"
                );
                environment.push((OsString::from_wide(&key), OsString::from_wide(&value)));
            }
            let mut command = std::process::Command::new(crate::platform::self_image()?);
            command
                .arg("--state-dir")
                .arg(&config.root)
                .arg("daemon")
                .env_remove("CODEX_THREAD_ID")
                .env_remove("CODEX_APP_TOOLS_PIPE_PATH")
                .env("AJR_DAEMON_LAUNCH_METHOD", "windows_cim");
            if let Some(cwd) = transfer.cwd {
                ensure!(cwd.is_absolute(), "startup cwd must be absolute");
                command.current_dir(cwd);
            }
            spawn_with_environment(command, environment)
        })();
        let response = match &launch {
            Ok(()) => serde_json::json!({"started":true}),
            Err(e) => serde_json::json!({"started":false,"error":format!("{e:#}")}),
        };
        ipc::write_frame(&mut socket, &response).await?;
        launch
    })
    .await
    .context("startup helper expired without a valid caller; no job was submitted")?
}

fn verify_image(process: HANDLE, expected: &Path) -> Result<()> {
    let mut image = vec![0u16; 32768];
    let mut size = image.len() as u32;
    unsafe {
        check(QueryFullProcessImageNameW(
            process,
            0,
            image.as_mut_ptr(),
            &mut size,
        ))?;
    }
    let actual = crate::platform::normalized_path(PathBuf::from(OsString::from_wide(
        &image[..size as usize],
    )));
    ensure!(
        environment_order(actual.as_os_str(), expected.as_os_str()).is_eq(),
        "startup peer executable differs"
    );
    Ok(())
}
// A cleanup Job Object is a lifetime boundary. Do not turn a restricted sandbox
// token into an unrestricted/elevated broker process with the same account SID.
#[derive(Debug, PartialEq, Eq)]
struct StartupToken {
    sid: String,
    integrity: u32,
    privileges: Vec<u64>,
}
fn startup_token(process: HANDLE) -> Result<StartupToken> {
    let mut raw = null_mut();
    unsafe {
        check(OpenProcessToken(process, TOKEN_QUERY, &mut raw))?;
    }
    let token = owned(raw)?;
    let integrity = token_integrity(token.as_raw_handle())?;
    Ok(StartupToken {
        sid: process_sid(process)?,
        integrity,
        privileges: token_privileges(token.as_raw_handle())?,
    })
}
// Compare privilege presence, not enabled bits: present privileges can already
// be enabled by the caller. A filtered token must not regain removed privileges.
fn token_privileges(token: HANDLE) -> Result<Vec<u64>> {
    let mut size = 0;
    unsafe {
        GetTokenInformation(token, TokenPrivileges, null_mut(), 0, &mut size);
    }
    ensure!((4..65536).contains(&size), "invalid privilege token size");
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    unsafe {
        check(GetTokenInformation(
            token,
            TokenPrivileges,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        ))?;
        let info = buffer.as_ptr().cast::<TOKEN_PRIVILEGES>();
        let count = (*info).PrivilegeCount as usize;
        ensure!(
            count <= (size as usize - 4) / size_of::<LUID_AND_ATTRIBUTES>(),
            "invalid privilege count"
        );
        let values = std::slice::from_raw_parts(
            std::ptr::addr_of!((*info).Privileges).cast::<LUID_AND_ATTRIBUTES>(),
            count,
        );
        let mut values: Vec<_> = values
            .iter()
            .map(|v| ((v.Luid.HighPart as u32 as u64) << 32) | u64::from(v.Luid.LowPart))
            .collect();
        values.sort_unstable();
        Ok(values)
    }
}
fn token_integrity(token: HANDLE) -> Result<u32> {
    ensure!(
        unsafe { IsTokenRestricted(token) } == 0,
        "restricted Windows tokens cannot use automatic CIM bootstrap"
    );
    let mut container = 0u32;
    let mut size = 0;
    unsafe {
        check(GetTokenInformation(
            token,
            TokenIsAppContainer,
            (&mut container as *mut u32).cast(),
            4,
            &mut size,
        ))?;
    }
    ensure!(
        container == 0,
        "AppContainer tokens cannot use automatic CIM bootstrap"
    );
    unsafe {
        GetTokenInformation(token, TokenIntegrityLevel, null_mut(), 0, &mut size);
    }
    ensure!(size > 0 && size < 65536, "invalid integrity token size");
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    let integrity = unsafe {
        check(GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        ))?;
        let sid = (*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()))
            .Label
            .Sid;
        let count = *GetSidSubAuthorityCount(sid);
        ensure!(count > 0, "integrity SID missing");
        *GetSidSubAuthority(sid, u32::from(count - 1))
    };
    ensure!(
        integrity >= SECURITY_MANDATORY_MEDIUM_RID as u32,
        "low-integrity callers cannot use automatic CIM bootstrap"
    );
    Ok(integrity)
}

/// A copyable same-profile fallback; paths are literal PowerShell arguments.
pub fn outside_start_command(config: &Config) -> Result<String> {
    let image = crate::platform::self_image()?;
    Ok(format!(
        "& {} --state-dir {} daemon start",
        literal(image.to_str().context("executable path is not Unicode")?),
        literal(config.root.to_str().context("state path is not Unicode")?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bootstrap_rejects_restricted_and_low_integrity_tokens() {
        let mut raw = null_mut();
        unsafe {
            check(OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_DUPLICATE,
                &mut raw,
            ))
            .unwrap();
        }
        let original = owned(raw).unwrap();
        assert!(token_integrity(original.as_raw_handle()).is_ok());
        let mut sid = null_mut();
        unsafe {
            check(ConvertStringSidToSidW(
                wide(current_sid().unwrap()).as_ptr(),
                &mut sid,
            ))
            .unwrap();
        }
        let _memory = LocalMemory(sid);
        let restricting = SID_AND_ATTRIBUTES {
            Sid: sid,
            Attributes: 0,
        };
        let mut raw = null_mut();
        unsafe {
            check(CreateRestrictedToken(
                original.as_raw_handle(),
                DISABLE_MAX_PRIVILEGE,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &restricting,
                &mut raw,
            ))
            .unwrap();
        }
        let restricted = owned(raw).unwrap();
        let mut raw = null_mut();
        unsafe {
            check(CreateRestrictedToken(
                original.as_raw_handle(),
                DISABLE_MAX_PRIVILEGE,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                &mut raw,
            ))
            .unwrap();
        }
        let privilege_filtered = owned(raw).unwrap();
        assert_ne!(
            token_privileges(original.as_raw_handle()).unwrap(),
            token_privileges(privilege_filtered.as_raw_handle()).unwrap()
        );

        assert!(
            token_integrity(restricted.as_raw_handle())
                .unwrap_err()
                .to_string()
                .contains("restricted Windows tokens")
        );
        let mut raw = null_mut();
        unsafe {
            check(DuplicateTokenEx(
                original.as_raw_handle(),
                TOKEN_QUERY | TOKEN_ADJUST_DEFAULT,
                std::ptr::null(),
                SecurityImpersonation,
                TokenPrimary,
                &mut raw,
            ))
            .unwrap();
        }
        let low = owned(raw).unwrap();
        let mut sid = null_mut();
        unsafe {
            check(ConvertStringSidToSidW(
                wide("S-1-16-4096").as_ptr(),
                &mut sid,
            ))
            .unwrap();
        }
        let _memory = LocalMemory(sid);
        let label = TOKEN_MANDATORY_LABEL {
            Label: SID_AND_ATTRIBUTES {
                Sid: sid,
                Attributes: SE_GROUP_INTEGRITY as u32,
            },
        };
        unsafe {
            check(SetTokenInformation(
                low.as_raw_handle(),
                TokenIntegrityLevel,
                (&label as *const TOKEN_MANDATORY_LABEL).cast(),
                (size_of_val(&label) + GetLengthSid(sid) as usize) as u32,
            ))
            .unwrap();
        }
        assert!(
            token_integrity(low.as_raw_handle())
                .unwrap_err()
                .to_string()
                .contains("low-integrity callers")
        );
    }
}
