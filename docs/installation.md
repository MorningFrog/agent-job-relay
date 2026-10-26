# Install once, use from your Agent

This Linux development milestone supports local native/Plugin installation and an offline runtime bundle. The bundle author needs Rust stable/C tools once; recipients need a compatible Linux system and `/bin/sh`, with no Rust, Python, Node or external database. No system packages, public plugin listing or online downloader exist.

## Offline bundle and bootstrap

The author exports a new directory containing the native executable, unbound plugin templates, license, launcher and SHA-256 manifest. No selected state, host identity, personal configuration or credentials are exported. Transfer the complete directory, preserving executable permissions; it may be renamed or moved before use.

```bash
cargo build --release --locked
AJR_BUNDLE="$PWD/.tmp/agent-job-relay-offline"
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --dry-run
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --apply
"$AJR_BUNDLE/relay" plugin bootstrap --dry-run
"$AJR_BUNDLE/relay" plugin bootstrap --apply
```

On the recipient's machine, run that received directory's `relay plugin bootstrap`. It validates the package, previews installation and marketplace ownership before writing, then installs the runtime and registers the local marketplace. Optional `--root` and `--config` select the stable installation and `<marketplace-root>/.agents/plugins/marketplace.json`; a fresh bootstrap accepts global `--state-dir`. An existing installation always keeps its recorded state, even with an unrelated AJR_STATE_DIR export or `--state-dir` override. Management launched through a custom installed launcher defaults to that installation. Install and state must remain separate; the delivery directory cannot contain installation/state/config destinations, and marketplace configuration stays outside the installation. A personal marketplace requires an installation inside HOME. After bootstrap, install/enable the plugin through the host as described below. The delivery directory is no longer needed.

Bundles declare the authoring version and OS/architecture/ABI label; mismatches, unsafe/missing/extra paths, changed bytes, unsupported formats and downgrades are refused. Same-version development rebuilds may differ and are updated by hash. Hashes detect local changes, not publisher authenticity or minimum glibc compatibility: use a trusted compatible build. The exported directory is an offline delivery artifact, not a directly installable marketplace ZIP. Bootstrap is explicit; this milestone does not implement an online download or automatic host install hook. Install and marketplace writes are separate operations; if registration fails after install, the error says so and a checked retry can finish it.

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

The v0.1.4 installation contains `bin/agent-job-relay`, a `relay` launcher, `.codex-plugin/plugin.json`, `.mcp.json`, a background-jobs skill, icon, license, runtime profile and an owner-only install receipt. It is independent of the source checkout and delivery bundle after installation. Updating source does not update an installed or host-cached copy.

Both MCP and terminal launchers use the stable managed native executable, with the same pinned state; cached terminal workflows no longer execute their historical bundled native copy. Codex CLI 0.164.0 did not discover the previous portable root MCP declaration or expand plugin-root variables; this increment also reproduced failures in args/cwd. Therefore portable root templates are not installed and the stable installation must remain present. The 0.1.4 isolated real-loader check bootstraps a relocated offline bundle, checks the actual host-created cache and discovers all 20 tools including job_recover_session. Official documentation lists the placeholder, but this tested loader still needs the local compatibility layout (ADRs 0004/0007). No public directory approval or arbitrary host-version compatibility is claimed.

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

For a received new offline bundle, use the installed manager without rebuilding source. AJR_BUNDLE below is the absolute received bundle directory; select only cache paths obtained from the host for this plugin.

```bash
"$AJR_INSTALL_ROOT/relay" plugin status --root "$AJR_INSTALL_ROOT"
"$AJR_INSTALL_ROOT/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_INSTALL_ROOT/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"
"$AJR_INSTALL_ROOT/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

First finish active jobs. The manager never stops a daemon and replacement still refuses active daemon/startup locks or jobs. Add repeated `--cache <absolute-path>` arguments to update to preflight all selected caches before runtime replacement. A source/bundle executable with the new management commands can also update an older installation; do not expect an old launcher to know new CLI options. Updates retain the receipt's state regardless of shell overrides, preserve configuration/history, refuse version downgrades and retain the prior installation as a backup. Running MCP processes keep their loaded image: reconnect after updating.

Inspect or refresh a particular cache separately:

```bash
"$AJR_INSTALL_ROOT/relay" plugin status --root "$AJR_INSTALL_ROOT" --cache "$AJR_CACHE"
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --root "$AJR_INSTALL_ROOT" --path "$AJR_CACHE" --dry-run
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --root "$AJR_INSTALL_ROOT" --path "$AJR_CACHE" --apply
```

AJR_CACHE is an explicit absolute directory for this plugin, never a parent containing other plugins. Refresh validates its original runtime profile, installation/state/platform and every owned hash; it supports host copies with or without the management receipt. It refuses modified, missing, extra, symlinked, foreign or overlapping content, stages a complete snapshot, retains the previous cache as a backup and is idempotent. A valid cache can be refreshed without stopping a daemon; both entry points already use the managed runtime. Legacy caches without `.agent-job-relay-runtime.json` require a first refresh/reinstall through the host. This replaces a checked file snapshot only; host registry versions/paths and enablement remain host-owned, so prefer the host's refresh/reinstall for its installation bookkeeping. No cache scanning, deletion or data migration occurs. Multi-directory updates and two-rename replacements are not crash-atomic; an error identifies possible completed runtime changes and the cache refresh to retry. Do not automatically roll back a runtime that may already have opened newer data.

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

SHA-256 ownership checks refuse modified/missing files, unknown content or symlinks. Only unchanged owned install files and directories are removed. An interrupted removal can need manual inspection; data, config backups and prior installation/cache backups remain. Host caches are only refreshed at explicitly selected paths, never scanned or deleted. There is no data purge command, OS service manager or destructive automatic cleanup. Independent OS installers and package repositories are cancelled. Claude Code/OpenCode integrations and future official plugin platforms are tracked in TODO.md; no submission or publication occurs during development. Never delete another project's state or unknown directories.

Official plugin/marketplace references:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://learn.chatgpt.com/docs/extend/mcp

The 0.1.4 increment retains session recovery/renewal and schema v6. It does not upgrade an existing personal install or host cache by itself. Automatic binding requires the updated daemon capability flags; finish active work, then explicitly stop/update using the existing installed profile. Older executables refuse to initialize newer schemas.
