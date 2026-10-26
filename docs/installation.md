# Install once, use from your Agent

This is a source-build Linux development milestone, with a local native/Plugin installation workflow. No system packages, public plugin listing or automatic online publication exist. Rust stable/C toolchain is needed to build once; the installed program needs no Python, Node, Rust runtime or external database.

## Personal Desktop Plugin installation

These are user-chosen writes to a personal installation/configuration. Start with the previews. Development automation uses only isolated repository paths.

```bash
cargo build --release --locked
AJR_INSTALL_ROOT="$HOME/.local/share/agent-job-relay"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --dry-run
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

The v0.1.3 source-built bundle contains `bin/agent-job-relay`, a relative `relay` launcher, `.codex-plugin/plugin.json`, `.mcp.json`, a background-jobs skill, icon and an owner-only install receipt. It is independent of the source checkout after installation. Updating source does not update an installed or host-cached copy.

The MCP command is the stable absolute executable path in this local install, with explicit state arguments. Codex CLI 0.164.0 did not discover the previous portable root MCP declaration or expand plugin-root command variables; retaining both manifests also failed. Therefore portable root templates are not installed. The 0.1.3 isolated real-loader check discovers all 20 tools including job_recover_session (ADR 0004). The cached terminal launcher remains relative, but **cached MCP still requires the original installation**. This local development solution is not a qualified public plugin bootstrap mechanism.

`setup plugin` preserves existing marketplace metadata and other plugins. For an absent personal file it creates `~/.agents/plugins/marketplace.json` with the marketplace label **AgentJobRelay Local**. Restart Desktop, select that local marketplace in Plugins, install AgentJobRelay and start a new local chat. An existing marketplace keeps its own name/label. Select AgentJobRelay when needed and describe a real task, for example:

> Run this project's release tests in the background and report the result.

The plugin skill resolves the bundled launcher itself, obtains a current-terminal binding, submits with MCP when available and ends its turn. If MCP is unavailable, it reports the installed CLI fallback explicitly. On completion it reads bounded logs and summarizes the result. No executable/state/session paths belong in everyday user prompts. The task's working directory remains explicit in the tool request, filled by the Agent from the chosen project.

The user's installed skill + CLI fallback release-test task completed and woke the correct chat, after a failed first submission and a separate-state workaround. It did not call MCP; see live-e2e.md. After the authorized 0.1.2 update, the user confirmed the Desktop MCP connectivity test passed following the host job_doctor procedure. That is user-attested, without a new independently reviewed transcript. MCP task submission/wake and browser discovery/install UI still need separate evidence. Plugin availability can depend on host version, surface and account policies. A local executor and current-chat terminal context are required for this Desktop wake workflow; no web/remote execution API is provided. See ADR 0003 and limitations.md.

## State and alternate configuration

Without `--root`, install uses `$XDG_DATA_HOME/agent-job-relay`, else `$HOME/.local/share/agent-job-relay`. Install and state directories must be separate and not nested. The install receipt stores the selected absolute state directory; `--state-dir` at installation can select an isolated or custom one. The original source default remains XDG state/HOME. A native binary uses explicit `--state-dir`, AJR_STATE_DIR, then its installed profile, then the source default.

The plugin's MCP configuration pins the installed state explicitly. Its `relay` launcher sets AJR_STATE_DIR to that same profile, so an unrelated exported demo state cannot split MCP and terminal binding across databases. A deliberate CLI `--state-dir` still overrides it. Cached plugin copies retain the profile, but never a socket path, host identity, session key or credential.

Personal marketplace sources must remain within HOME; use a HOME-contained `--root` if XDG_DATA_HOME is elsewhere. For repo-local authoring, install under that repository and pass `setup plugin --config /absolute/repo/.agents/plugins/marketplace.json`. Paths are relative to the marketplace root (repo or HOME), not its `.agents/plugins` subdirectory.

For standalone MCP instead of Plugin installation in the host, run the installed launcher's `setup codex` preview/apply. Generic mcpServers JSON still uses `setup json --config <absolute-file>`. Do not register both routes unnecessarily. Generated configurations use `mcp --shared-host`. Server instructions include a current-terminal binding command for an Agent that has no plugin skill.

Only the owned entry is edited, with adjacent private receipt/lock/backup files; foreign entries and TOML comments are retained, modified/unowned entries refused. Restart the MCP client after applying. No unsupported ChatGPT settings location is guessed. Source-level `cargo install --path . --root ...` remains supported for native CLI development; it does not generate this plugin bundle/profile.

## Diagnostics

- `doctor`: reports an absent daemon as stopped, without starting it or creating state. Inherited-context availability is not a host probe or live wake claim.
- `doctor --client plugin` (or codex/json): checks the owned configuration entry. Configuration match is not proof the host has loaded it.
- `doctor --check-mcp`: initializes a private stdio subprocess, discovers required tools and verifies clean EOF. It sends no Desktop message or workload and does not claim host connectivity.
- `job_doctor` invoked by the actual host: proves that tool call reached MCP; shared mode explains why a current-terminal binding is still needed for wake.

## Update and remove safely

Before replacing or removing binaries, inspect active jobs and let them complete or explicitly cancel selected jobs. Active daemon/startup locks and unfinished persisted jobs block replacement/removal. For early-version upgrades use the updated source command below; current installed launchers also support ordinary `daemon stop`. No unrelated process is stopped automatically.

Rebuild source, then stop the installation's recorded runtime using the **new source executable**, not an old launcher that may lack these options:

```bash
cargo build --release --locked
./target/release/agent-job-relay daemon stop --install-root "$AJR_INSTALL_ROOT"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --dry-run
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
```

`--install-root` selects the owned receipt's state, overriding the stop command's `--state-dir`/AJR_STATE_DIR. It errors if that installation is absent. The stop waits for daemon lock release, so immediate update does not race the shutdown ack. For the observed early 0.1.0 daemon without IPC `shutdown`, only that specific remote error enables compatibility handling: recheck the original peer PID and boot/start identity, owned socket/root, executable name, daemon arguments and healthy legacy Doctor response, then send SIGINT through a Linux pidfd and wait for exit. This invokes the old Ctrl-C handler and reports `legacy_sigint`; no process search, group signal or force kill is used. Unknown peers, changed identities, other errors and unavailable pidfd support fail explicitly. Renamed legacy executables require manual review rather than relaxed matching.

Rebuild source and repeat install at the same root to update. Without an explicit `--state-dir` or AJR_STATE_DIR override, the previous receipt's state profile is retained even when updating from a source binary. Unset unrelated exported state overrides before updating. Unchanged installs are no-ops. Updates stage a full private tree and retain the old one as the reported backup. These two renames are not a crash/power-loss transaction; keep the backup and inspect it after an interrupted update. Refresh/reinstall the host's cached plugin separately after updating its marketplace source.

Current Linux daemons spawn from their mapped image even if a rebuild unlinks the original file. An already-running legacy daemon does not gain that repair: Doctor reports `runner_readiness.status=restart_required` when its executable path is gone and it lacks the loaded-image capability. New submissions fail before creating another job. Finish active jobs and explicitly stop that state's daemon, then submit using the updated program. Never switch state directories and report that as an update; never silently replay the failed job.

For Plugin removal, uninstall/disable the installed plugin in Desktop so its host-managed cache and enabled state are handled by the host. Then preview/apply `relay integration remove plugin`. For standalone MCP use the corresponding codex/json removal. This changes only the owned entry and preserves all task data.

Finally, after the runtime is idle, preview/apply:

```bash
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

SHA-256 ownership checks refuse modified/missing files, unknown content or symlinks. Only unchanged owned install files and directories are removed. An interrupted removal can need manual inspection; data, config backups and prior installation backups remain. Host caches are not scanned or deleted by the CLI. There is no data purge command, OS service manager or destructive automatic cleanup. Independent OS installers and package repositories are cancelled. Claude Code/OpenCode integrations and future official plugin platforms are tracked in TODO.md; no submission or publication occurs during development. Never delete another project's state or unknown directories.

Official plugin/marketplace references:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://learn.chatgpt.com/docs/extend/mcp

The 0.1.3 source increment adds session recover preview/--apply and completion-time lease renewal. It does not upgrade an existing personal install or host cache by itself. Automatic binding requires the updated daemon capability flags; finish active work, then explicitly stop/update using the existing installed profile. State schema v6 is additive but older executables refuse to initialize it.
