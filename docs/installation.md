# Installation and maintenance

Use the offline bundle for the quickest installation. It includes the native executable and needs no Rust, Python or Node on the recipient's machine. Current support is Linux x86_64 with a compatible libc and a POSIX shell, plus Windows x86_64 with a `.cmd` launcher (see the [Windows guide](windows.md)); there is no public download, online installer or official plugin listing yet. Use a trusted build: its hashes check integrity, not publisher identity.

Already installed? Follow [Updating an existing installation](#updating-an-existing-installation). For a first install, use the quick path below. Without a bundle, follow the complete [local source installation walkthrough](source-install.md) ([简体中文](source-install.zh-CN.md)); standalone MCP is under [Alternative installation routes](#alternative-installation-routes).

## npm and self-hosted marketplace candidates

Candidate delivery uses a shared Linux x86_64/glibc and Windows x86_64 npm runtime for OpenCode V1 and a ChatGPT Desktop/Codex self-hosted marketplace. The packages are not published; names and npm account verification are pending. Use [release preparation](release.md) to build/test the actual tarballs locally. Its consumer commands are explicitly for use after publication. First online use will require Node.js 20+/npm and network access, but no Rust compiler. Existing offline installs remain supported below.

The new persistent host profiles are `$XDG_STATE_HOME/agent-job-relay/codex` and `.../opencode`, falling back to `$HOME/.local/state/...`; Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. AJR_STATE_DIR may explicitly select a reviewed existing profile. They live outside npm/host caches and do not migrate old offline history automatically. Do not enable old and new MCP integrations together or switch profiles to hide a failure. Finish jobs and explicitly stop the original profile before an incompatible native upgrade; no plugin install/update/uninstall stops daemons or purges data.

## Quick install from an offline bundle

Open a terminal in the received bundle directory:

```bash
./relay plugin bootstrap --dry-run
./relay plugin bootstrap --apply
```

The preview shows the native installation and local marketplace entry. Apply installs the runtime and registers that entry. It preserves other marketplace entries and refuses modified or unowned content. The bundle directory can be moved before bootstrap and is no longer needed afterward.

Restart Desktop, open Plugins, select the local marketplace and install/enable AgentJobRelay. A new marketplace is named `agent-job-relay-local`, displayed as **AgentJobRelay Local**; an existing one retains its name/label. Open a new local chat and select `agent-job-relay@agent-job-relay-local` (substitute an existing catalog's name when different). The repository catalog's `agent-job-relay@agent-job-relay` entry instead needs the unpublished npm runtime; selecting it can expose the skill while MCP startup fails with `E404`. Availability and UI behavior depend on the host version/account; use the [manual acceptance procedure](manual-testing.md) to verify them.

Check the default installation:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

For a custom installation, use its actual root in all following commands. The local check verifies configuration and stdio, not the actual host connection or chat continuation. Both cached terminal launchers and MCP use this managed installation; keep it present.

## Updating an existing installation

Use the new offline bundle's launcher, including when upgrading a version too old to recognize `plugin update`. Set AJR_BUNDLE to the new bundle's absolute directory and AJR_INSTALL_ROOT to the original installation root. Finish active jobs before applying the update.

```bash
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Review the preview first. Update retains the installation's recorded state despite unrelated shell overrides, refuses downgrades, and preserves the previous installation as the reported backup. An unchanged update is a no-op. The explicit stop selects the original profile and waits for daemon-lock release; update itself never stops a daemon, and active locks/jobs block replacement.

Refresh/reinstall the plugin through the host and reconnect MCP afterward. Already loaded processes keep their old image until reconnected/restarted. Source rebuilds do not update installed binaries or host caches. Do not switch state directories to work around an upgrade failure. Early legacy shutdown handling is described in [ADR 0005](adr/0005-legacy-shutdown-upgrade.md).

## Cache inspection and refresh

Prefer the host's refresh/reinstall for normal plugin updates: the host owns its version registry, cache selection and enabled state. Legacy caches without `.agent-job-relay-runtime.json` need this host refresh before using managed cache repair.

For a known cache directory belonging to this plugin, set AJR_CACHE to that exact absolute directory:

```bash
"$AJR_INSTALL_ROOT/relay" plugin status --cache "$AJR_CACHE"
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --dry-run
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --apply
```

Commands run through a custom installed launcher default to its own root. Refresh validates the original installation/state/platform, complete file set and hashes. It supports host copies with or without the management receipt, refuses altered/unknown/missing/symlinked/foreign files, retains the previous snapshot as a backup and is idempotent. It does not scan/delete caches or change the host's registry/enabled state. Do not pass a parent containing other plugins. Reconnect MCP after a change.

To preflight and refresh several known snapshots during runtime update, repeat `--cache <absolute-directory>` on `plugin update`. All selected caches are checked before runtime replacement. Cache refresh alone does not stop the daemon. Each directory replacement records a durable recovery operation; multiple directories and marketplace writes remain separate. On interruption use `plugin repair` for the exact affected directory, then retry the original update. No runtime/schema rollback occurs.

## Custom directories and stored data

| Setting | Behavior |
|---|---|
| Installation root | Default `$XDG_DATA_HOME/agent-job-relay`, else `$HOME/.local/share/agent-job-relay`; select another with bootstrap `--root` |
| Fresh bootstrap state | Select global `--state-dir`; otherwise AJR_STATE_DIR, then XDG state/HOME defaults |
| Existing bootstrap/update state | Always retain the recorded installation profile |
| Ordinary CLI state | Explicit `--state-dir`, then AJR_STATE_DIR, then installed profile, then XDG state/HOME default |
| Plugin CLI/MCP state | Both pin the installed profile; deliberate ordinary CLI `--state-dir` can override it |
| Marketplace | Default `$HOME/.agents/plugins/marketplace.json`; bootstrap `--config` may select `<marketplace-root>/.agents/plugins/marketplace.json` |

Installation and state must be separate, not nested. Keep installation/state/configuration outside the delivery bundle, and marketplace configuration outside the installation. Personal marketplace sources must be inside HOME; if XDG_DATA_HOME is elsewhere, pass a HOME-contained `--root`. Repo-local authoring may use a repo-contained root and marketplace config. State directories require owner-only permissions; keep Unix socket paths shorter than 104 bytes.

The installation contains the native program, launcher, Codex manifest/MCP configuration, workflow skill, icon, license, runtime profile and ownership receipt. Profiles contain state/installation paths, not host sockets, chat identities, session keys or credentials. Keep backups private. See [security](security.md) for data and command-output boundaries.

## Alternative installation routes

### Build a bundle from source

Follow [Install the local plugin from source](source-install.md) ([简体中文](source-install.zh-CN.md)) for prerequisites, checkout, build/export commands, first installation or update, exact marketplace selection and MCP verification. The resulting native bundle needs no recipient Node/npm or compiler and can be transferred while preserving executable permissions. It is a local delivery artifact, not the online marketplace ZIP; no download, signature verification or automatic host installation hook is implemented.

### Install directly from a source build

After building, the native development entry remains available:

```bash
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --dry-run
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
```

Direct `install` preserves an existing profile only when no explicit state override is supplied; unset unrelated AJR_STATE_DIR values first. `plugin update` is the recommended profile-preserving update route. `cargo install --path . --root ...` is CLI development tooling and does not generate the plugin bundle/profile.

### Configure standalone MCP

Use this instead of enabling the plugin's MCP in the same host/state:

```bash
"$AJR_INSTALL_ROOT/relay" setup codex --dry-run
"$AJR_INSTALL_ROOT/relay" setup codex --apply
```

Codex uses `$CODEX_HOME/config.toml`, else `$HOME/.codex/config.toml`; `--config` selects another absolute file. Generic `mcpServers` JSON uses `setup json --config <absolute-file>`. Generated entries use the stable installed executable and `mcp --shared-host`. Only the owned AgentJobRelay entry changes; other entries/comments are preserved. Restart the client after applying. No unsupported ChatGPT settings path is guessed.

## Diagnostics

| Check | What it establishes |
|---|---|
| `relay --version` | Executable version |
| `relay doctor` | Daemon/store health when available; a stopped daemon is reported without starting it |
| `relay doctor --client plugin` | Owned marketplace entry matches this installation |
| `relay doctor --check-mcp` | Local stdio initialization/tool discovery/EOF |
| Host `job_doctor` | This actual tool call reached MCP |
| Observed same-chat continuation | The Agent actually resumed to handle this task's result |

Configuration, local stdio, host connectivity and continuation are separate checks. Missing tools require host refresh/reconnection; a loaded skill alone does not establish MCP availability.

## Uninstalling

Remove/disable AgentJobRelay through the host first. Finish active jobs, then preview/apply owned configuration removal:

```bash
"$AJR_INSTALL_ROOT/relay" integration remove plugin --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove plugin --apply
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

For standalone MCP, replace `plugin` in integration removal with `codex`, or `json --config <same-file>`. Uninstall removes unchanged owned installation files only; modified/unknown files are refused. Task history, logs and backups remain. The host handles cache removal. There is no data purge or system-service uninstall operation.

## Compatibility and interrupted operations

The tested Codex CLI 0.164.0 loader uses `.codex-plugin/plugin.json` and `.mcp.json` with a stable absolute native command. Its portable root and PLUGIN_ROOT behavior differed from the documented format; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). Packaging does not establish other host/OS support.

Installation and cache replacements durably stage files and record a private sibling operation journal before renaming the original directory. A pending operation blocks another lifecycle change. If the installation path is missing, use the new offline bundle or source executable, not the missing installed launcher:

```bash
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --apply
```

Use an explicitly selected cache directory for a cache operation. Preview reports `prepared`, `destination_missing` or `installed`; apply finishes the recorded replacement and retains the original backup. Repeating a completed repair reports `changed=false`. Repair verifies the complete staged/original/backup snapshots, profiles and hashes; modified, unknown, missing or symlinked contents are refused. Installation repair requires idle old/new profiles and never stops a daemon. Cache repair leaves the managed runtime running.

Retry the original bootstrap/update afterward to finish marketplace registration or other selected caches. Repair neither changes the database nor rolls back an installed runtime. Cross-directory/configuration operations and uninstall remain separate and are not crash-atomic. Before journal publication, interruption leaves the original untouched and may leave unreferenced staging files; these and old pre-0.1.5 interruptions require inspection, not automatic scanning/deletion. Physical power-loss/ENOSPC testing remains pending. Current schema is v6; older executables refuse newer schemas. See [implementation status](IMPLEMENTATION_STATUS.md), [live evidence](live-e2e.md) and [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Other host development packages

OpenCode V1 and Claude Code use separate source packages with the same >=0.1.6 core (Windows OpenCode requires >=0.1.7); they are not exported by the Codex offline bundle. Configure their native path/state once using [host-plugin setup](host-plugins.md). OpenCode 1.19.4 has an actual-loader/MCP/agent-loop check with a simulated model. Claude is inbox-only and needs account-backed community testing; [PRs are welcome](../CONTRIBUTING.md). No personal host configuration is changed automatically.
