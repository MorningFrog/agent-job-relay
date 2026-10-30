# Installation

[Documentation index](README.md) · [Features](features.md) · [Updates](updating.md) · [Troubleshooting](troubleshooting.md)

Install the latest published npm packages and matching [GitHub Release](https://github.com/MorningFrog/agent-job-relay/releases/latest). Supported runtime platforms are Linux x86_64/glibc and Windows x86_64. Online installation requires Node.js 20+/npm and network access, without Rust, Python or a source build. macOS/ARM and official plugin-directory listing remain unsupported.

| Installation route | Start here |
|---|---|
| Published ChatGPT Desktop/Codex plugin | [Marketplace installation](#chatgpt-desktop--codex) |
| Published global CLI and task manager | [Terminal and Web UI](#terminal-and-web-ui) |
| Published OpenCode V1 plugin | [OpenCode V1](#opencode-v1) |
| Native offline bundle | [Local plugin installation](#quick-install-from-an-offline-bundle) |
| Build locally or configure standalone MCP | [Alternative routes](#alternative-installation-routes) |

Already installed? Follow [updating and repair](updating.md) or [uninstallation](uninstalling.md). See [profiles](#profiles-and-existing-installations) before selecting an existing state directory.

## Quick install the latest release

### ChatGPT Desktop / Codex

Requires Node.js 20+/npm, Codex CLI and Git on PATH. Use a user-writable npm installation; Desktop also needs a local executor.

Bash:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr update
```

PowerShell with npm-installed CLIs:

```powershell
npm.cmd install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
if ($LASTEXITCODE -ne 0) { throw "CLI installation failed" }
ajr.cmd update
if ($LASTEXITCODE -ne 0) { throw "Plugin installation failed" }
```

Expected: npm registers `ajr` and `agent-job-relay`; `ajr update` installs the matching released marketplace and `agent-job-relay@agent-job-relay` plugin. It also serves as the [later update command](updating.md#one-command-online-update-0114). Use `ajr update --dry-run` for a preview. Unified maintenance requires CLI 0.1.14 or newer; the npm command installs the latest release. Existing local/ZIP/modified marketplaces use their original route and are preserved, not replaced by the online updater.

1. Restart or reconnect ChatGPT Desktop/Codex.
2. In Desktop, open **Plugins**, enable **AgentJobRelay** and select `agent-job-relay@agent-job-relay` in a local chat. UI availability depends on the host/account.
3. Ask the selected plugin to call `job_doctor` without submitting a task. Expected: the actual MCP call succeeds; an initially unbound chat is normal in shared-host mode.
4. Try a harmless background command using [manual acceptance](manual-testing.md). Tool connectivity, host acceptance and observed same-chat continuation are separate results; Codex CLI provides tools/inbox without automatic model continuation.

The host does not install global commands automatically. Keep npm's global command directory on PATH. In PowerShell, `.cmd` wrappers avoid script-policy restrictions; a standalone Codex executable can be used in place of `codex.cmd` for manual host commands.

### Terminal and Web UI

For CLI/UI use without a host plugin:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr --version
ajr ui
```

Expected: the CLI prints the installed version and opens the bundled task manager. No source checkout or separate frontend installation is needed. In PowerShell, use `npm.cmd` and `ajr.cmd` if needed.

`ajr` defaults to the online `codex` profile; `ajr opencode <command>` selects OpenCode. The full command uses `agent-job-relay codex <command>` for the same profile. Native offline installations use their recorded launcher/profile. See [profiles](#profiles-and-existing-installations) before selecting existing data.

### OpenCode V1

Add the published plugin to `opencode.json`, preserving any existing plugin entries:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Restart OpenCode. Expected: the host loads the plugin and its exact matching native runtime dependency. Do not enable another `agent_job_relay` MCP entry alongside it. `ajr update` does not manage this host package; use [OpenCode updates](updating.md#update-opencode-v1). V1 is experimental; real-provider/V2/restart-recovery evidence remains separate from simulated-model loader checks.

### Install from a GitHub Release ZIP

Use this alternative when Git is unavailable. Install the global CLI using the npm command above, then download `agent-job-relay-marketplace-*.zip` and `SHA256SUMS` from [the latest GitHub Release](https://github.com/MorningFrog/agent-job-relay/releases/latest). Verify the ZIP's SHA-256 and extract it to a stable directory. Register the directory containing `.agents/plugins/marketplace.json`:

```bash
codex plugin marketplace add <absolute-extracted-marketplace-directory>
codex plugin add agent-job-relay@agent-job-relay
```

Expected: the host installs the plugin from that local marketplace. Restart/reconnect and verify `job_doctor` as above. Keep the extracted directory for [later ZIP updates](updating.md#update-a-release-zip-marketplace); `ajr update` does not replace a local marketplace. This online catalog uses npm/network on first invocation and requires the global CLI; it is distinct from the native offline bundle below.

### Check the npm runtime without installing globally

```bash
npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@latest agent-job-relay --version
```

Expected: the latest published runtime version is printed without starting a task. This does not install the host plugin or satisfy its global CLI prerequisite.

## Profiles and existing installations

Persistent online profiles use `$XDG_STATE_HOME/agent-job-relay/codex` and `.../opencode`, falling back to `$HOME/.local/state/...`; Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. They stay outside npm/host caches. `--state-dir` takes priority over AJR_STATE_DIR and the default. Native offline installations retain their recorded profiles; online installation does not migrate that history automatically.

Keep the same launcher/profile for installation checks and later maintenance. Do not enable duplicate MCP integrations or switch data directories to hide a failure. See [update requirements](updating.md#before-updating) for active-job and schema boundaries, and [uninstallation](uninstalling.md) for daemon shutdown and retained data.

## Quick install from an offline bundle

Open a terminal in the received bundle directory:

```bash
./relay plugin install --dry-run
./relay plugin install --apply
```

The preview shows the native installation and local marketplace entry. Apply installs the runtime and registers that entry. It preserves other marketplace entries and refuses modified or unowned content. The bundle directory can be moved before installation and is no longer needed afterward.

Restart ChatGPT Desktop, open Plugins, select the local marketplace and install/enable AgentJobRelay. A new marketplace is named `agent-job-relay-local`, displayed as **AgentJobRelay Local**; an existing one retains its name/label. Open a new local chat and select `agent-job-relay@agent-job-relay-local` (substitute an existing catalog's name when different). The repository catalog's `agent-job-relay@agent-job-relay` entry uses the published npm runtime and a separate profile; selecting it does not test this native local installation. Availability and UI behavior depend on the host version/account; use the [manual acceptance procedure](manual-testing.md) to verify them.

Check the default installation:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

For a custom installation, use its actual root in all following commands. The local check verifies configuration and stdio, not the actual host connection or chat continuation. Both cached terminal launchers and MCP use this managed installation; keep it present.

## Custom directories and stored data

| Setting | Behavior |
|---|---|
| Installation root | Default `$XDG_DATA_HOME/agent-job-relay`, else `$HOME/.local/share/agent-job-relay`; select another with `plugin install --root` |
| Fresh native installation state | Select global `--state-dir`; otherwise AJR_STATE_DIR, then XDG state/HOME defaults |
| Existing native installation/update state | Always retain the recorded installation profile |
| Ordinary CLI state | Explicit `--state-dir`, then AJR_STATE_DIR, then installed profile, then XDG state/HOME default |
| Plugin CLI/MCP state | Both pin the installed profile; deliberate ordinary CLI `--state-dir` can override it |
| Marketplace | Default `$HOME/.agents/plugins/marketplace.json`; `plugin install --config` may select `<marketplace-root>/.agents/plugins/marketplace.json` |

Installation and state must be separate, not nested. Keep installation/state/configuration outside the delivery bundle, and marketplace configuration outside the installation. Personal marketplace sources must be inside HOME; if XDG_DATA_HOME is elsewhere, pass a HOME-contained `--root`. Repo-local authoring may use a repo-contained root and marketplace config. State directories require owner-only permissions; keep Unix socket paths shorter than 104 bytes.

The installation contains the native program, launcher, Codex manifest/MCP configuration, workflow skill, icon, license, runtime profile and ownership receipt. Profiles contain state/installation paths, not host sockets, chat identities, session keys or credentials. Keep backups private. See [security](security.md) for data and command-output boundaries.

## Alternative installation routes

### Build a bundle from source

Follow [Install the local plugin from source](source-install.md) ([简体中文](source-install.zh-CN.md)) for prerequisites, checkout, build/export commands, first installation or update, exact marketplace selection and MCP verification. The resulting native bundle needs no recipient Node/npm or compiler and can be transferred while preserving executable permissions. It is a local delivery artifact, not the online marketplace ZIP; no download, signature verification or automatic host installation hook is implemented.

### Install directly from a source build

After building, choose a stable installation directory and use the native development entry. For a first Linux installation:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --dry-run
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
```

Direct `install` preserves an existing profile only when no explicit state override is supplied; unset unrelated AJR_STATE_DIR values first. Use the [profile-preserving native update route](updating.md#update-a-native-local-or-offline-plugin) for an existing installation. `cargo install --path . --root ...` is CLI development tooling and does not generate the plugin bundle/profile.

### Configure standalone MCP

Use this instead of enabling the plugin's MCP in the same host/state:

```bash
"$AJR_INSTALL_ROOT/relay" setup codex --dry-run
"$AJR_INSTALL_ROOT/relay" setup codex --apply
```

Codex uses `$CODEX_HOME/config.toml`, else `$HOME/.codex/config.toml`; `--config` selects another absolute file. Generic `mcpServers` JSON uses `setup json --config <absolute-file>`. Generated entries use the stable installed executable and `mcp --shared-host`. Only the owned AgentJobRelay entry changes; other entries/comments are preserved. Restart the client after applying. No unsupported ChatGPT settings path is guessed.

## Other host development packages

The Codex offline bundle does not contain OpenCode or Claude source packages. Use [host-plugin setup](host-plugins.md) for their development configuration and evidence boundaries. Claude remains inbox-only and needs account-backed community verification; no personal host configuration is changed automatically.

## Verify and continue

Call `job_doctor` from the selected host to confirm actual MCP connectivity; a local stdio self-check establishes only local tool discovery. Use [manual acceptance](manual-testing.md) for a first harmless task and [daily usage](usage.md) for regular operation. Find exact commands in the [command reference](commands.md), and diagnose missing tools with [troubleshooting](troubleshooting.md).
