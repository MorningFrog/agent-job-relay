use agent_job_relay::platform;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

// Cargo owns a non-breakaway Job Object on Windows. Launch fixture daemons
// through local CIM, under the same SID and outside Cargo's cleanup job, so the
// tests exercise real process independence rather than weakening the runtime.
pub(crate) struct DaemonProcess {
    pub(crate) pid: u32,
    pub(crate) identity: String,
}
impl DaemonProcess {
    pub(crate) fn kill(&self) -> anyhow::Result<()> {
        if platform::identity(self.pid).is_none() {
            return Ok(());
        }
        platform::terminate_instance(self.pid, &self.identity)
    }
    pub(crate) fn wait(&self) -> anyhow::Result<()> {
        for _ in 0..200 {
            if platform::identity(self.pid).as_ref() != Some(&self.identity) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        anyhow::bail!("fixture daemon did not exit")
    }
    pub(crate) fn try_wait(&self) -> anyhow::Result<Option<&'static str>> {
        Ok((platform::identity(self.pid).as_ref() != Some(&self.identity)).then_some("exited"))
    }
}
pub(crate) fn launch_daemon(dir: &Path, state: &Path, disable_web_ui: bool) -> DaemonProcess {
    let literal = |path: &Path| path.to_str().unwrap().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue'
$pinfo=[Diagnostics.ProcessStartInfo]::new()
$pinfo.FileName='{}'
$pinfo.Arguments='--state-dir "{}" daemon'
$pinfo.UseShellExecute=$false;$pinfo.RedirectStandardError=$true;$pinfo.CreateNoWindow=$false
{web_ui}
$p=[Diagnostics.Process]::Start($pinfo)
$errorText=$p.StandardError.ReadToEndAsync()
[IO.File]::WriteAllText('{}',$p.Id.ToString())
if(!$p.WaitForExit(180000)){{$p.Kill();$p.WaitForExit()}}
[IO.File]::WriteAllText('{}',$errorText.GetAwaiter().GetResult())
"#,
        literal(Path::new(env!("CARGO_BIN_EXE_agent-job-relay"))),
        literal(state),
        literal(&dir.join("daemon.pid")),
        literal(&dir.join("daemon.log")),
        web_ui = if disable_web_ui {
            "$pinfo.EnvironmentVariables['AJR_WEB_UI']='off'"
        } else {
            ""
        },
    );
    let script_path = dir.join("daemon-launch.ps1");
    std::fs::write(&script_path, script).unwrap();
    launch_fixture_script(&script_path);
    for _ in 0..200 {
        if let Ok(text) = std::fs::read_to_string(dir.join("daemon.pid"))
            && let Ok(pid) = text.parse()
            && let Some(identity) = platform::identity(pid)
        {
            return DaemonProcess { pid, identity };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!(
        "fixture launcher failed: {}",
        std::fs::read_to_string(dir.join("daemon.log")).unwrap_or_default()
    );
}
pub(crate) fn launch_fixture_script(script_path: &Path) {
    let ps = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let command_line = format!(
        "\"{}\" -NoLogo -NoProfile -NonInteractive -File \"{}\"",
        ps.display(),
        script_path.display()
    )
    .replace('\'', "''");
    let request = format!(
        "$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';$startup=New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly -Property @{{ShowWindow=[uint16]0;CreateFlags=[uint32]0x10}};$result=Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine='{command_line}';ProcessStartupInformation=$startup}};if($result.ReturnValue -ne 0){{throw ('fixture process creation failed: '+$result.ReturnValue)}}"
    );
    let out = Command::new(ps)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &request,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
