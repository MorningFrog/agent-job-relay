# Windows native runtime

The native x86_64 runtime supports durable background execution, bounded logs, cancellation/timeouts, daemon/runner recovery, MCP, inbox notifications and offline plugin maintenance. Windows 11 VM behavior tests pass. v0.1.8 is verified with ChatGPT Desktop for actual MCP submission and same-chat continuation using Full access. Named-pipe recovery/renewal tests use simulated hosts. Ordinary Codex CLI integration has an inbox and does not automatically resume a chat.

## Install the latest published package

Use the [PowerShell quick-install steps](installation.md#chatgpt-desktop--codex) to resolve npm latest and register its matching released marketplace. OpenCode uses `agent-job-relay-opencode-v1@latest`. Both routes include the Windows x86_64 native payload with a static MSVC runtime; recipient machines need Node.js 20+/npm, without Rust or Visual Studio. The catalog MCP launcher uses Node plus the system command processor to invoke pinned npx arguments; terminal use has a `relay.cmd` entry point.

Default npm state lives in `%LOCALAPPDATA%\AgentJobRelay\profiles\codex` or `opencode`, outside host/npm caches. `AJR_STATE_DIR` may explicitly select an existing profile; no automatic profile migration or upgrade-driven daemon replacement occurs. OpenCode V1 targets 1.19.4. Its Node/Bun pipe is checked against the exact server process and restricted to a private DACL before sessions register.

Actual Windows npm installation, native execution, history after relocation, corrupt-native refusal, Codex 0.164.0 catalog/20-tool discovery and OpenCode package-name loading/MCP submission/matching-session agent-loop continuation are checked with `scripts/check_windows_packages.mjs`. OpenCode uses a local simulated model. These checks do not establish Windows ChatGPT Desktop continuation; live evidence remains version-specific.

## Alternative: a trusted Windows offline bundle

Open PowerShell in the received Windows bundle directory:

```powershell
.\relay.cmd plugin bootstrap --dry-run
.\relay.cmd plugin bootstrap --apply
$ajrInstall = Join-Path $env:LOCALAPPDATA 'AgentJobRelay\plugin'
& "$ajrInstall\relay.cmd" --version
& "$ajrInstall\relay.cmd" doctor --client plugin --check-mcp
```

Expected: the preview identifies the chosen installation/state and marketplace entry; apply installs the native executable and `.cmd` launcher. Doctor reports a successful 20-tool stdio check. A stopped daemon before the first submission is expected. The bundle needs no recipient Rust, Python or Node. Restart the host, install/enable the local plugin and separately verify host MCP connectivity.

The default native installation is `%LOCALAPPDATA%\AgentJobRelay\plugin`; state is `%LOCALAPPDATA%\AgentJobRelay\state`. Explicit `--state-dir`/AJR_STATE_DIR and existing installation profiles retain their recorded selection. Configuration/removal preserves other entries and task history. Do not move state into a plugin/npm cache.

## Current-chat binding permissions

Run the installed `relay.cmd session connect --brief` through the initiating ChatGPT Desktop chat's terminal. It needs access to that host's local named pipe and peer process identity, subject to the host's [Windows sandbox and approval rules](https://learn.chatgpt.com/docs/windows/windows-sandbox). A visible MCP tool does not establish terminal access. Pipe failures report whether opening the connection or authenticating its peer failed; access denied does not by itself prove that the endpoint is stale.

The verified Windows ChatGPT Desktop workflow uses user-selected Full access. Restricted terminal execution returned access denied before binding; single-command authorization in that mode remains unverified. No administrator elevation or pipe-permission change was needed for the verified workflow.

If access is denied, the Agent may request the host's single-command authorization for that same binding command when the active policy permits it. The command must retain the signed-in user, inherited chat identity and installed state profile. This is not Windows administrator elevation. Do not change pipe permissions or use another terminal's identity. If authorization is unavailable or binding still fails, stop before `job_start` and report the error. An external terminal can start a daemon but cannot supply the originating chat's binding.

## Build when no bundle is available

Install Node.js 24/npm, stable Rust MSVC and the Visual Studio C++ tools, then run from source:

```powershell
npm ci --prefix web --ignore-scripts --no-audit --no-fund
npm run build --prefix web
cargo build --release --locked
.\target\release\agent-job-relay.exe plugin pack --output "$PWD\.tmp\windows-bundle" --apply
```

Transfer the complete Windows bundle, then use its `relay.cmd` bootstrap. A Linux bundle is not a Windows payload.

## Run and inspect a task

```powershell
$ajr = Join-Path $env:LOCALAPPDATA 'AgentJobRelay\plugin\bin\agent-job-relay.exe'
$submitted = & $ajr start --cwd "$PWD" --shell 'Start-Sleep -Seconds 2; Write-Output background-ok' | ConvertFrom-Json
& $ajr wait $submitted.job.job_id --seconds 5
& $ajr logs $submitted.job.job_id
```

Expected: a single completed Job/Run with exit 0 and `background-ok` in the log. Explicit argv execution is available after `--`; batch files require an explicit cmd shell. The Windows `--shell` default is PowerShell. MCP uses the same state and behavior.

## Automatic startup

Normal use needs no manual `daemon start`, login startup registration or administrator setup. The first task/session request starts the selected daemon; existing daemons are reused. After a daemon stop/crash or Windows restart, the next submission/registration starts it again. This does not replay jobs or keep commands alive across an OS restart. Doctor remains read-only.

The runtime tries native detached creation first, keeping the new child suspended until its process handle confirms complete detachment. Partial escape from an inner Job Object is refused before runtime code or state access, so a remaining outer restriction cannot strand the next runner. If a cleanup Job Object (including Codex CLI 0.164.0 MCP) denies breakaway, it uses Windows PowerShell/local CIM to start a bounded helper of the same executable. Both ends verify process instance/image, SID, integrity level, privilege inventory and private pipe security before transferring the caller's Unicode environment/cwd. No environment file, remote target, alternate user, elevation, scheduled task, startup entry or service installation is used. Runners still use native detached creation and Job Object containment.

Use `doctor` / `job_doctor` to inspect `daemon_launch_method`: `windows_native`, `windows_cim` or `foreground`. CLI doctor also reports the current automatic-startup policy and `outside_terminal_command`. `AJR_WINDOWS_STARTUP=auto` is the default; `direct` disables CIM for future startup attempts. Restricted/AppContainer/low-integrity tokens and local policies that deny CIM are refused explicitly.

If automatic startup is blocked, copy the exact `outside_terminal_command` from diagnostics/errors into an external PowerShell terminal. It already contains the selected program and state path. For a default offline install, this equivalent command selects the recorded profile:

```powershell
& "$ajrInstall\relay.cmd" daemon start --install-root $ajrInstall
```

The command exits once ready; repeated calls reuse the daemon. A terminal opened inside the restrictive host can inherit its restriction and may not suffice. `daemon stop` preserves independent runners/history. There is no resident restart watchdog: recovery happens on the next submission/registration.

Native tests cover nested host jobs, direct-only refusal before state/workload execution, cold restricted-host startup, host job destruction, daemon-crash restart, environment/identity isolation and policy refusal. Real installed Codex model/MCP acceptance must start with no prestarted daemon. Native VM tests pass under guest-agent SYSTEM and the ordinary interactive user; the cold-profile installed MCP/model workflow also passes under both. Actual Windows ChatGPT Desktop continuation is verified separately; see [live evidence](live-e2e.md).

## Update, recover and remove

Use the new bundle's launcher, finish active jobs, explicitly stop the original installation's daemon, and preview/apply `plugin update --root <original-install> --from <new-bundle>`. Keep the recorded state. Close host MCP processes when replacing their Windows executable/cache files; Windows may retain an executable image lock. An interrupted operation retains its journal/backups; run `plugin repair --path <exact-directory>` from the new bundle to preview and finish forward after releasing the lock. Never delete a cache or switch state to hide an update failure.

`daemon stop --install-root <original-install>` leaves runners independent. Installation/cache/configuration operations remain separate and never stop daemons automatically. Normal removal keeps SQLite history/logs; independent Windows installers and WinGet distribution are not planned.

## Execution boundaries

Native independent daemon/runner creation requires breakaway. Daemon startup can instead use the verified same-user CIM helper when a cleanup job blocks it; the helper must still launch an independent native daemon. Local policy or provider job limits can refuse that route. Cargo's cleanup job is such a test environment, so native behavioral fixtures use local CIM to launch their owned test processes under the same SID outside Cargo. Normal direct startup needs no CIM; restricted-host automatic startup needs built-in Windows PowerShell/local CIM to be permitted.

Runner-owned Job Objects terminate contained descendants on cancellation, timeout or runner death. Console Ctrl-Break is best effort before force termination. Processes that cannot join the job do not execute the user command. State and pipe security require the current user's SID, private DACLs and non-reparse paths. Physical power loss, filesystem exhaustion and ChatGPT Desktop version compatibility remain distinct qualifications. See [architecture decision](adr/0011-windows-native-runtime.md) and [testing](testing.md).
