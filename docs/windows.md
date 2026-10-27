# Windows native runtime

The native x86_64 runtime supports durable background execution, bounded logs, cancellation/timeouts, daemon/runner recovery, MCP, inbox notifications and offline plugin maintenance. Windows 11 VM behavior tests pass. Desktop named-pipe transport/recovery tests use simulated hosts; Windows Desktop UI installation and observed same-chat continuation still need real-host acceptance. Ordinary Codex CLI integration has an inbox and does not automatically resume a chat.

## Install a trusted Windows offline bundle

Open PowerShell in the received Windows bundle directory:

```powershell
.\relay.cmd plugin bootstrap --dry-run
.\relay.cmd plugin bootstrap --apply
$ajrInstall = Join-Path $env:LOCALAPPDATA 'AgentJobRelay\plugin'
& "$ajrInstall\relay.cmd" --version
& "$ajrInstall\relay.cmd" doctor --client plugin --check-mcp
```

Expected: the preview identifies the chosen installation/state and marketplace entry; apply installs the native executable and `.cmd` launcher. Doctor reports a successful 20-tool stdio check. A stopped daemon before the first submission is expected. The bundle needs no recipient Rust, Python or Node. Restart the host, install/enable the local plugin and separately verify host MCP connectivity. The shared 0.1.7 npm candidate also carries Windows bytes; the normal registry route remains unavailable until publication.

The default native installation is `%LOCALAPPDATA%\AgentJobRelay\plugin`; state is `%LOCALAPPDATA%\AgentJobRelay\state`. Explicit `--state-dir`/AJR_STATE_DIR and existing installation profiles retain their recorded selection. Configuration/removal preserves other entries and task history. Do not move state into a plugin/npm cache.

## npm and host plugins

After publication, the self-hosted catalog and OpenCode use the same Windows x86_64 native payload with a static MSVC runtime; recipient machines need Node.js/npm, without Rust or Visual Studio. Follow [release installation](release.md#consumer-install-after-publication). The Windows catalog MCP launcher uses Node plus the system command processor to invoke pinned npx arguments. Terminal use has a `relay.cmd` entry point.

Default npm state lives in `%LOCALAPPDATA%\AgentJobRelay\profiles\codex` or `opencode`, outside host/npm caches. `AJR_STATE_DIR` may explicitly select an existing profile; no automatic profile migration or daemon restart occurs. OpenCode V1 requires 1.19.4 and native >=0.1.7. Its Node/Bun pipe is checked against the exact server process and restricted to a private DACL before sessions register.

Actual Windows npm installation, native execution, history after relocation, corrupt-native refusal, Codex 0.164.0 catalog/20-tool discovery and OpenCode package-name loading/MCP submission/matching-session agent-loop continuation are checked with `scripts/check_windows_packages.mjs`. OpenCode uses a local simulated model. These checks do not establish Windows Desktop UI installation or continuation.

## Build when no bundle is available

Install stable Rust MSVC and the Visual Studio C++ tools, then run from source:

```powershell
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

## Start before a restrictive host

Codex CLI 0.164.0 places its MCP process in a non-breakaway cleanup Job Object. A cold `job_start` therefore refuses independent daemon creation instead of pretending the workload can survive host cleanup. Before starting Codex, run the following in a normal external PowerShell terminal:

```powershell
& "$ajrInstall\relay.cmd" daemon start --install-root $ajrInstall
```

Expected: `daemon` is `reachable`, with no submitted job or session. The command exits after starting the independent daemon, and repeated calls reuse it. `--install-root` selects the installation's recorded state even if the shell exports another state path. For the npm catalog after publication, use `npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@0.1.7 agent-job-relay codex daemon start` outside Codex instead. Use the same profile as MCP; repeat after a machine restart or explicit daemon stop. A terminal opened by the restrictive host may inherit its job restriction and is not sufficient.

The prestarted daemon launches independent runners and allows MCP job submission from the restricted client. An actual installed Codex MCP/model check through Codex-Manager (`gpt-6.1-sol`/`low`) passed start, one bounded wait, status and logs with a single completed native job/exit 0; it ran as guest-agent SYSTEM, with no logged-in interactive user. This requirement does not grant Codex automatic chat resume; the CLI still offers tools/inbox only. Desktop behavior must be checked separately.

## Update, recover and remove

Use the new bundle's launcher, finish active jobs, explicitly stop the original installation's daemon, and preview/apply `plugin update --root <original-install> --from <new-bundle>`. Keep the recorded state. Close host MCP processes when replacing their Windows executable/cache files; Windows may retain an executable image lock. An interrupted operation retains its journal/backups; run `plugin repair --path <exact-directory>` from the new bundle to preview and finish forward after releasing the lock. Never delete a cache or switch state to hide an update failure.

`daemon stop --install-root <original-install>` leaves runners independent. Installation/cache/configuration operations remain separate and never stop daemons automatically. Normal removal keeps SQLite history/logs; independent Windows installers and WinGet distribution are not planned.

## Execution boundaries

An enclosing host Job Object must permit breakaway for independent daemon/runner launch. A restricted host that forbids it receives a launch error; the runtime does not escape through a Windows service. Cargo's cleanup job is such a test environment, so native behavioral fixtures use local CIM to launch their owned test processes under the same SID outside Cargo. Runtime users need no CIM dependency.

Runner-owned Job Objects terminate contained descendants on cancellation, timeout or runner death. Console Ctrl-Break is best effort before force termination. Processes that cannot join the job do not execute the user command. State and pipe security require the current user's SID, private DACLs and non-reparse paths. Physical power loss, filesystem exhaustion and Desktop version compatibility remain distinct qualifications. See [architecture decision](adr/0011-windows-native-runtime.md) and [testing](testing.md).
