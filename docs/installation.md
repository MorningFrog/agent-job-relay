# Installation and maintenance

Install the latest published npm packages and matching [GitHub Release](https://github.com/MorningFrog/agent-job-relay/releases/latest). Supported runtime platforms are Linux x86_64/glibc and Windows x86_64. Online installation requires Node.js 20+/npm and network access, without Rust, Python or a source build. macOS/ARM and official plugin-directory listing remain unsupported.

For existing native local installations, retain the recorded profile and follow [offline updates](#updating-an-existing-installation). Source builds and recipient-toolchain-free native bundles remain [alternative routes](#alternative-installation-routes).

## Quick install the latest release

### Terminal and Web UI

Use npm's standard global command registration:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
```

Expected: npm registers `agent-job-relay`; `ui` opens the embedded task manager for the online ChatGPT/Codex profile. No shell function, source checkout or separate frontend installation is needed. Node.js 20+/npm and npm's global command directory must be on PATH; use a user-writable npm installation. In PowerShell, use `npm.cmd` and `agent-job-relay.cmd` if script policy blocks npm's `.ps1` wrappers.

The 0.1.11 source also registers `ajr`, awaiting publication. With that version installed:

```bash
ajr list
ajr start --shell 'echo hello'
ajr ui
```

`ajr` defaults to the same `codex` profile as the online plugin; `ajr opencode <command>` selects OpenCode. `AJR_STATE_DIR` overrides the host default, and explicit `--state-dir` takes priority. The full command retains its existing default: use `agent-job-relay codex <command>` to select the plugin profile. A previous offline installation keeps its own recorded profile; use its installed launcher rather than assuming `ajr` selects it. Installing a host plugin alone does not register global terminal commands. For browser access without installing the CLI, ask the selected plugin to open AgentJobRelay's task manager.

Update the CLI by repeating its npm installation with the version matching the selected plugin. This does not update the host's installed plugin or replace an already running daemon. If `ui` reports an older daemon, finish its jobs, explicitly run `agent-job-relay codex daemon stop` (or `ajr daemon stop` with 0.1.11), then retry `ui` for that same profile. Removal uses `npm uninstall --global --ignore-scripts agent-job-relay-runtime`; task history and running processes remain intact.

### ChatGPT Desktop / Codex

Have Codex CLI and Git available on PATH and use a ChatGPT Desktop local executor. Query npm's latest runtime and register its matching GitHub release tag, so installation follows published versions rather than development main. Without Git, use the [Release ZIP](#install-from-a-github-release-zip).

Bash:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

PowerShell with npm-installed CLIs:

```powershell
$AjrVersion = npm.cmd view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org
if ($LASTEXITCODE -ne 0) { throw "Could not resolve the latest published runtime" }
codex.cmd plugin marketplace add MorningFrog/agent-job-relay --ref "v$($AjrVersion.Trim())"
```

Expected: Codex reports the AgentJobRelay marketplace registration at the selected release tag. Stop if version lookup or registration fails. An already registered source with a different tag needs the [online update steps](#update-the-online-marketplace). If Codex was installed as a standalone executable, use that installed executable instead of `codex.cmd`.

Restart ChatGPT Desktop, open **Plugins**, select the **AgentJobRelay** marketplace, install/enable **AgentJobRelay**, then select `agent-job-relay@agent-job-relay` in a new local chat. Codex CLI users can install the catalog entry with `codex plugin add agent-job-relay@agent-job-relay`; CLI tools/inbox do not imply automatic CLI model continuation.

Ask the selected plugin to call `job_doctor` without submitting a task. Expected: an actual MCP call succeeds. Shared-host mode can report no bound chat before the first terminal binding; that is normal. Then submit a harmless background command and verify its matching result using [manual acceptance](manual-testing.md). Marketplace discovery, tool connectivity and observed same-chat continuation remain separate checks.

### OpenCode V1

Add the latest published plugin to `opencode.json` and restart OpenCode:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Expected: the host loads the plugin and its exact matching native runtime dependency. Do not enable another `agent_job_relay` MCP entry alongside it. Finish jobs before upgrades and restart the host after refreshing its package cache with the host-supported mechanism. OpenCode V1 is experimental; real-provider/V2/restart-recovery evidence is separate from simulated-model loader checks.

### Install from a GitHub Release ZIP

If using a downloaded marketplace instead of a Git fetch, open [the latest GitHub Release](https://github.com/MorningFrog/agent-job-relay/releases/latest), download its `agent-job-relay-marketplace-*.zip` and `SHA256SUMS`, verify the ZIP's SHA-256, and extract it to a stable directory. Register that directory:

```bash
codex plugin marketplace add <absolute-extracted-marketplace-directory>
```

Use the directory containing `.agents/plugins/marketplace.json`, then follow the host installation/enablement steps above. Keep this directory for future marketplace refreshes. The marketplace ZIP uses npm on first invocation; it is not a native offline bundle. Each npm runtime tarball in the same Release includes its native executables; no recipient Rust build or secondary binary downloader is needed.

### Check the npm runtime

For a standalone version/integrity smoke check without starting a task:

```bash
npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@latest agent-job-relay --version
```

Expected: the latest published runtime version is printed. The host plugin's installed snapshot pins its corresponding runtime version; this check does not upgrade that snapshot.

### Update the online marketplace

Finish active jobs, then resolve the latest version before replacing the registered catalog source. Codex refuses to add a different tag over an existing source, and marketplace upgrade retains the original tag. For an existing npm-backed marketplace, use Bash:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

Or PowerShell:

```powershell
$AjrVersion = npm.cmd view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org
if ($LASTEXITCODE -ne 0) { throw "Could not resolve the latest published runtime" }
codex.cmd plugin marketplace remove agent-job-relay
if ($LASTEXITCODE -ne 0) { throw "Could not remove the old marketplace registration" }
codex.cmd plugin marketplace add MorningFrog/agent-job-relay --ref "v$($AjrVersion.Trim())"
```

Expected: the catalog is registered at the newly resolved tag. Refresh/reinstall AgentJobRelay through the host and reconnect MCP; CLI users can run `codex plugin add agent-job-relay@agent-job-relay` again. Removing this catalog registration preserves the external runtime profile/history. Already running processes retain their loaded version; explicitly stop the same profile before an incompatible upgrade. For OpenCode, keep `@latest`, refresh its package cache using the host-supported mechanism and restart it.

## Profiles and existing installations

The new persistent host profiles are `$XDG_STATE_HOME/agent-job-relay/codex` and `.../opencode`, falling back to `$HOME/.local/state/...`; Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. AJR_STATE_DIR may explicitly select a reviewed existing profile. They live outside npm/host caches and do not migrate old offline history automatically. Do not enable old and new MCP integrations together or switch profiles to hide a failure. Finish jobs and explicitly stop the original profile before an incompatible native upgrade; no plugin install/update/uninstall stops daemons or purges data.

## Quick install from an offline bundle

Open a terminal in the received bundle directory:

```bash
./relay plugin bootstrap --dry-run
./relay plugin bootstrap --apply
```

The preview shows the native installation and local marketplace entry. Apply installs the runtime and registers that entry. It preserves other marketplace entries and refuses modified or unowned content. The bundle directory can be moved before bootstrap and is no longer needed afterward.

Restart ChatGPT Desktop, open Plugins, select the local marketplace and install/enable AgentJobRelay. A new marketplace is named `agent-job-relay-local`, displayed as **AgentJobRelay Local**; an existing one retains its name/label. Open a new local chat and select `agent-job-relay@agent-job-relay-local` (substitute an existing catalog's name when different). The repository catalog's `agent-job-relay@agent-job-relay` entry uses the published npm runtime and a separate profile; selecting it does not test this native local installation. Availability and UI behavior depend on the host version/account; use the [manual acceptance procedure](manual-testing.md) to verify them.

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

## Unified install and update

Current native bundles provide `plugin install` for both first installation and updates. The compatible bootstrap/update commands above remain available for existing bundles.

From a received native bundle that provides this entry:

```bash
./relay plugin install
./relay plugin install --apply
```

For an existing custom installation, select its original `--root`; the recorded state profile is retained even when the shell exports a different state directory. From its installed launcher, update from a new trusted bundle with:

```bash
"$AJR_INSTALL_ROOT/relay" plugin install --from "$AJR_BUNDLE"
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" plugin install --from "$AJR_BUNDLE" --apply
```

Review the preview and finish active jobs before the explicit same-profile stop. The command validates bundle ownership/version and every explicitly selected cache (`--cache` may repeat), then maintains the runtime and local marketplace entry. `--config` selects a custom marketplace file; reuse that selection on subsequent maintenance. Default output keeps the changed destinations, preserved profile, backups and next action; `--full` shows complete operation reports. Marketplace writes and directory replacements remain separate operations with their existing recovery limits. No cache scan, automatic daemon stop or profile migration is added. Refresh the plugin through the host and reconnect MCP afterward. Existing `bootstrap` and `update` commands stay compatible.

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

Retry the original bootstrap/update afterward to finish marketplace registration or other selected caches. Repair neither changes the database nor rolls back an installed runtime. Cross-directory/configuration operations and uninstall remain separate and are not crash-atomic. Before journal publication, interruption leaves the original untouched and may leave unreferenced staging files; these and old pre-0.1.5 interruptions require inspection, not automatic scanning/deletion. Physical power-loss/ENOSPC testing remains pending. Version 0.1.9 uses schema v6; 0.1.10 and newer use schema v9 and refuse migration until the old daemon stops and old launching/running jobs finish. Older executables refuse newer schemas. See [data maintenance](data-maintenance.md). See [implementation status](IMPLEMENTATION_STATUS.md), [live evidence](live-e2e.md) and [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Other host development packages

OpenCode V1 and Claude Code use separate source packages with the same >=0.1.6 core (Windows OpenCode requires >=0.1.7); they are not exported by the Codex offline bundle. Configure their native path/state once using [host-plugin setup](host-plugins.md). OpenCode 1.19.4 has an actual-loader/MCP/agent-loop check with a simulated model. Claude is inbox-only and needs account-backed community testing; [PRs are welcome](../CONTRIBUTING.md). No personal host configuration is changed automatically.
