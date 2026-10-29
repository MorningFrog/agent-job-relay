# Installation

[Documentation index](README.md) · [Features](features.md) · [Updates](updating.md) · [Troubleshooting](troubleshooting.md)

Install the latest published npm packages and matching [GitHub Release](https://github.com/MorningFrog/agent-job-relay/releases/latest). Supported runtime platforms are Linux x86_64/glibc and Windows x86_64. Online installation requires Node.js 20+/npm and network access, without Rust, Python or a source build. macOS/ARM and official plugin-directory listing remain unsupported.

| Installation route | Start here |
|---|---|
| Published global CLI and task manager | [Terminal and Web UI](#terminal-and-web-ui) |
| Published ChatGPT Desktop/Codex plugin | [Marketplace installation](#chatgpt-desktop--codex) |
| Published OpenCode V1 plugin | [OpenCode V1](#opencode-v1) |
| Native offline bundle | [Local plugin installation](#quick-install-from-an-offline-bundle) |
| Build locally or configure standalone MCP | [Alternative routes](#alternative-installation-routes) |

Already installed? Follow [updating and repair](updating.md) or [uninstallation](uninstalling.md). See [profiles](#profiles-and-existing-installations) before selecting an existing state directory.

## Quick install the latest release

### Terminal and Web UI

Use npm's standard global command registration:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
```

Expected: npm registers `agent-job-relay`; `ui` opens the embedded task manager for the online ChatGPT/Codex profile. No shell function, source checkout or separate frontend installation is needed. Node.js 20+/npm and npm's global command directory must be on PATH; use a user-writable npm installation. In PowerShell, use `npm.cmd` and `agent-job-relay.cmd` if script policy blocks npm's `.ps1` wrappers.

npm 0.1.11 and newer also register `ajr`. With that version installed:

```bash
ajr list
ajr start --shell 'echo hello'
ajr ui
```

`ajr` defaults to the same `codex` profile as the online plugin; `ajr opencode <command>` selects OpenCode. `AJR_STATE_DIR` overrides the host default, and explicit `--state-dir` takes priority. The full command retains its existing default: use `agent-job-relay codex <command>` to select the plugin profile. A previous offline installation keeps its own recorded profile; use its installed launcher rather than assuming `ajr` selects it. Installing a host plugin alone does not register global terminal commands. For browser access without installing the CLI, ask the selected plugin to open AgentJobRelay's task manager.

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

Expected: Codex reports the AgentJobRelay marketplace registration at the selected release tag. Stop if version lookup or registration fails. An already registered source with a different tag needs the [online update steps](updating.md#update-the-online-marketplace). If Codex was installed as a standalone executable, use that installed executable instead of `codex.cmd`.

Restart ChatGPT Desktop, open **Plugins**, select the **AgentJobRelay** marketplace, install/enable **AgentJobRelay**, then select `agent-job-relay@agent-job-relay` in a new local chat. Codex CLI users can install the catalog entry with `codex plugin add agent-job-relay@agent-job-relay`; CLI tools/inbox do not imply automatic CLI model continuation.

Ask the selected plugin to call `job_doctor` without submitting a task. Expected: an actual MCP call succeeds. Shared-host mode can report no bound chat before the first terminal binding; that is normal. Then submit a harmless background command and verify its matching result using [manual acceptance](manual-testing.md). Marketplace discovery, tool connectivity and observed same-chat continuation remain separate checks.

### OpenCode V1

Add the latest published plugin to `opencode.json` and restart OpenCode:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Expected: the host loads the plugin and its exact matching native runtime dependency. Do not enable another `agent_job_relay` MCP entry alongside it. For later maintenance, follow [OpenCode updates](updating.md#update-opencode-v1). OpenCode V1 is experimental; real-provider/V2/restart-recovery evidence is separate from simulated-model loader checks.

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

## Profiles and existing installations

The new persistent host profiles are `$XDG_STATE_HOME/agent-job-relay/codex` and `.../opencode`, falling back to `$HOME/.local/state/...`; Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. AJR_STATE_DIR may explicitly select a reviewed existing profile. They live outside npm/host caches and do not migrate old offline history automatically. Do not enable old and new MCP integrations together or switch profiles to hide a failure. Finish jobs before file replacement or incompatible schema migration. Current source replaces older daemons on new runtime use and restarts online daemons during applied local updates; published versions still require a manual same-profile stop. Uninstall preserves data and never stops a daemon automatically.

## Quick install from an offline bundle

Open a terminal in the received bundle directory:

```bash
./relay plugin install --dry-run
./relay plugin install --apply
```

Current bundles use `plugin install`; older bundles retain the compatible `plugin bootstrap --dry-run` / `--apply` entry. The preview shows the native installation and local marketplace entry. Apply installs the runtime and registers that entry. It preserves other marketplace entries and refuses modified or unowned content. The bundle directory can be moved before installation and is no longer needed afterward.

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

After building, the native development entry remains available:

```bash
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --dry-run
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
```

Direct `install` preserves an existing profile only when no explicit state override is supplied; unset unrelated AJR_STATE_DIR values first. Use the [profile-preserving native update route](updating.md#unified-install-and-update) for an existing installation. `cargo install --path . --root ...` is CLI development tooling and does not generate the plugin bundle/profile.

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
