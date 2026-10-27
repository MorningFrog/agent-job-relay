<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

English | [简体中文](README.zh-CN.md)

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。
</div>

Run long commands independently of the Agent connection, keep their results, and notify the initiating chat when the host supports it. **v0.1.8 is an unreleased Linux/Windows x86_64 development version.** ChatGPT Desktop same-chat continuation is verified on Linux and Windows using an experimental interface; Windows verification uses Full access. macOS runtime is unsupported. OpenCode V1 has an experimental plugin package candidate; Claude Code has an inbox-only development framework that needs community verification.

For OpenCode/Claude development setup and evidence boundaries, see [host plugins](docs/host-plugins.md).

## Quick install

For ChatGPT Desktop, use a trusted offline bundle for your Linux system. For an existing installation, go directly to [Updating](#updating). For a first installation, open a terminal in the received bundle directory and run:

```bash
./relay plugin bootstrap --dry-run
./relay plugin bootstrap --apply
```

This installs the native runtime and registers its local plugin marketplace. No Rust, Python or Node is needed. Preview is the default; `--apply` writes the installation and owned configuration. There is no public download or automatic online installer yet. If you only have source, follow [Install from source](#install-from-source).

Restart ChatGPT Desktop, open Plugins, select your local marketplace (**AgentJobRelay Local** for a new marketplace), and install/enable AgentJobRelay. Start a new local chat and select `agent-job-relay@agent-job-relay-local` (or your existing personal marketplace's name). The separate `agent-job-relay@agent-job-relay` entry uses the unpublished npm runtime and is not the working local installation. Host UI discovery still requires manual acceptance on your host version.

Check the default installation:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Use your original path for a custom installation. Keep that directory: both MCP and cached terminal launchers use its managed runtime. State and history are stored separately; PATH and system services are unchanged. Already installed? Follow [Updating](#updating). Custom paths and standalone MCP setup are in the [installation guide](docs/installation.md).

## Windows

Use a trusted Windows offline bundle and its `relay.cmd` launcher. Native execution, process-tree supervision, MCP, offline maintenance and installed npm/host-loader behavior are verified on Windows 11. The 0.1.8 candidate carries both Linux and Windows executables; publication is pending. Windows now starts the daemon on demand, including Codex MCP cleanup jobs, through a verified same-user local CIM bootstrap when needed. If local policy blocks CIM, diagnostics provide a copyable outside-terminal fallback. Follow the [Windows guide](docs/windows.md) for installation, task examples and execution boundaries.

## Use from chat

Ask in a local ChatGPT Desktop chat with the plugin selected:

> Run this project's tests in the background and tell me when they finish.

The Agent binds the current chat, submits the task, ends its turn and reads the result after notification. Everyday prompts need no executable path or session ID. If MCP is unavailable, the workflow reports its CLI fallback. A local terminal is required for binding; ordinary Codex CLI integration provides a notification inbox, without automatic chat continuation.

The first submission or explicit session registration starts the daemon automatically. Commands run with your OS permissions. Job completion, host acknowledgement (`accepted_by_host`) and observed Agent continuation are separate results. For a first acceptance run, follow [manual testing](docs/manual-testing.md); detailed CLI commands and recovery are in [usage](docs/usage.md).

## Updating

Finish active tasks first. Use the **new bundle's launcher**, which also works when the installed version predates plugin-management commands. Set AJR_BUNDLE to the received directory's absolute path and AJR_INSTALL_ROOT to the existing installation:

```bash
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
"$AJR_INSTALL_ROOT/relay" --version
```

The update retains the recorded state/history and a backup, refuses downgrades and never stops a daemon implicitly. Refresh/reinstall the plugin through the host and reconnect MCP afterward. Legacy caches need this host refresh; explicit checked cache repair is described in [installation](docs/installation.md#cache-inspection-and-refresh).

If an install/update or cache replacement was interrupted, use the new bundle's launcher to preview and finish the recorded replacement:

```bash
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --apply
```

For a cache, pass that exact cache directory instead. Repair preserves history and backups, refuses modified files and never stops a daemon. Retry the original bootstrap/update afterward to finish any remaining steps. See [interrupted operations](docs/installation.md#compatibility-and-interrupted-operations) for limits.

## Uninstalling

Remove/disable the plugin in the host, finish active jobs, then use the same installed launcher:

```bash
"$AJR_INSTALL_ROOT/relay" integration remove plugin --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove plugin --apply
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

Uninstall preserves task data and backups. Modified or unknown install files are refused. Host cache removal belongs to the host; there is no data purge command. Standalone MCP removal is covered in the [installation guide](docs/installation.md#uninstalling).

## Troubleshooting

| Symptom | Next step |
|---|---|
| Plugin or MCP tools missing | Refresh the plugin in the host and reconnect. A loaded skill alone does not prove MCP loaded. |
| `npm E404` for `agent-job-relay-runtime` | The online candidate is selected. Follow [local source installation](docs/source-install.md), then select `agent-job-relay@agent-job-relay-local`. |
| Local self-check passes, but chat tools fail | Invoke `job_doctor` from the host; a local stdio check does not establish host connectivity. |
| Update reports an active runtime | Finish jobs, then explicitly stop the daemon for the original installation. |
| Notification does not arrive | Check session/delivery state; use same-chat recovery when needed. See [usage](docs/usage.md). |
| Ownership check refuses changes | Preserve the files and inspect the reported changes/backup. |

## Plugin release candidates

Source is public on [GitHub](https://github.com/MorningFrog/agent-job-relay). Candidate implementations provide the OpenCode npm plugin, shared runtime package and ChatGPT self-hosted marketplace; **the two main npm packages remain unpublished, so normal online installation is not available yet.** Maintain existing installations through the [offline route](#quick-install).

The GitHub **Release** workflow runs manually only: dry-run builds and verifies a Cargo-versioned candidate, then a separate manual run promotes those exact files to npm and GitHub. First publication uses stage-only npm access and maintainer 2FA approval; later releases use OIDC. Release Notes come from CHANGELOG. Pushes, PRs and tags never publish. See the [release walkthrough](docs/release.md) for setup, approval, retries and future online installation. A self-hosted marketplace is separate from OpenAI's official directory.

## Install from source

Follow the [step-by-step local source installation](docs/source-install.md) ([简体中文](docs/source-install.zh-CN.md)). It covers prerequisites, checkout, compilation, bundle export, first installation or history-preserving update, ChatGPT Desktop selection and an actual MCP check, with expected results at each step.

The result is the native local plugin in **AgentJobRelay Local**, normally referenced as `agent-job-relay@agent-job-relay-local`. No npm publication or Node.js installation is required. Building alone does not install it; complete the bootstrap/update and ChatGPT Desktop steps. Windows source commands are in the [Windows guide](docs/windows.md#build-when-no-bundle-is-available); developer gates are in [testing](docs/testing.md).

## Implementation and verification

| Area | Implementation | Verification and remaining scope |
|---|---|---|
| Linux runtime and offline maintenance | Implemented | Automated execution, lifecycle, recovery and installation tests pass. |
| Windows runtime and automatic startup | Implemented | Native Windows 11 tests and actual Codex MCP cold startup under an ordinary interactive user pass. |
| ChatGPT Desktop same-chat continuation | Verified on Linux and Windows | v0.1.8 continues the initiating chat after task completion. Windows verification uses Full access; see [live evidence](docs/live-e2e.md). |
| OpenCode V1 | Experimental plugin candidate | Actual loader, MCP and session API pass with a simulated model; real-provider sessions, restart recovery and V2 remain open. |
| Claude Code | Inbox-only scaffold | Synthetic caller and MCP launcher checks pass; actual Claude loader/account/model verification needs community help. |
| npm runtime and self-hosted marketplace | Candidates prepared, unpublished | Installed package and loader checks pass on Linux/Windows using a loopback registry; public-registry installation remains pending. |
| CI and release | Native Linux/Windows checks and manual release workflow defined | Earlier remote CI passed; updated Windows native/release jobs await a remote run. The unified release workflow awaits its first manual remote run. |
| macOS runtime | Unimplemented | Portable compilation checks only; native execution remains unsupported. |
| Output monitoring/interruption | Deferred | Awaits an explicit implementation request; log reads and manual stop are available. |

Current results and version-specific limits are maintained in [implementation status](docs/IMPLEMENTATION_STATUS.md) and [live evidence](docs/live-e2e.md). Official directory submission requires separate approval; independent OS installers/package repositories are cancelled. **Community testing and PRs are welcome, especially for Claude Code and container-based version checks.**

## Documentation and support

| Topic | Guide |
|---|---|
| Step-by-step local installation from source | [Source installation](docs/source-install.md), [简体中文](docs/source-install.zh-CN.md) |
| Installation, profiles, updates, caches and removal | [Installation](docs/installation.md) |
| CLI, MCP, notifications and recovery | [Usage](docs/usage.md) |
| Human acceptance and expected results | [Manual testing](docs/manual-testing.md) |
| Automated tests and verification scope | [Testing](docs/testing.md) |
| Current support and evidence | [Implementation status](docs/IMPLEMENTATION_STATUS.md), [live evidence](docs/live-e2e.md) |
| Safety and operational limits | [Security](docs/security.md), [limitations](docs/limitations.md) |
| Design and API details | [Architecture](docs/architecture.md), [protocol](docs/protocol.md) |
| Editable icon and SVG/PNG export | [Icon artwork](assets/icons/README.md) |
| Host frameworks and community testing | [Host plugins](docs/host-plugins.md), [Contributing](CONTRIBUTING.md) |
| Release candidates and manual Actions | [Release](docs/release.md) |
| Planned work and changes | [Roadmap](docs/TODO.md), [changelog](CHANGELOG.md) |

Licensed under [MIT](LICENSE).
