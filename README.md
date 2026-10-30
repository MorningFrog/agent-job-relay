<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。

English | [简体中文](README.zh-CN.md)
</div>

Run long commands independently of the Agent connection, keep their results, and notify the initiating chat when the host supports it.

## What it does

| Capability | Use it for |
|---|---|
| Durable background jobs | Builds, tests, training and data processing that continue after a chat or viewer closes |
| Goal waiting (experimental) | One short native or Agent question when pause permission is missing; durable approval covers this request’s bounded rounds. Original-goal handoff, restoration and deletion safeguards remain. [Usage and host limits](docs/goal-waiting.md) |
| Batch submission (0.1.12+) | Submit 1–16 independent commands through CLI/MCP, with per-item results and safe retries; [usage](docs/usage.md#submit-independent-commands-together-0112) |
| Remote tasks (0.1.12+) | Launch/attach on a pinned remote runtime or observe a fixed GitHub Actions attempt. Source 0.1.13 adds job/step status and an attempt link; [setup and boundaries](docs/remote-execution.md) |
| Mid-run feedback (0.1.12+) | Structured stages/steps/metrics, repeated milestone or threshold alerts and a bounded checker recipe; [usage and limits](docs/progress.md) |
| Results and history | Bounded stdout/stderr, exit outcomes, events and retained task history |
| Session-bound notifications | Deliver results to the initiating session; resume the chat when supported by its host |
| Local task manager | Inspect tasks, configure persistent output/silence/elapsed rules and stop work through CLI or the bundled Web UI |
| Data maintenance | Automatic archival, rolling backups and explicit restore into a new profile |

**Runtime: Linux x86_64/glibc and Windows x86_64.** ChatGPT Desktop same-chat continuation is verified on both through an experimental private interface; Windows verification uses Full access. Codex CLI and the Claude Code scaffold provide an inbox without automatic continuation. OpenCode V1 is experimental and has actual-loader checks with a simulated model. macOS/ARM runtime is unsupported. See [features and support](docs/features.md) for capabilities and [live evidence](docs/live-e2e.md) for verified versions.

## Installation and updates

Online installation needs Node.js 20+/npm and network access. ChatGPT Desktop/Codex also needs Codex CLI on PATH and Git for catalog fetching; Desktop requires a local executor. No Rust compiler is needed. Commands below use Bash; [PowerShell and Release ZIP alternatives](docs/installation.md#chatgpt-desktop--codex) are available.

Retain the original profile when updating. Current source automatically replaces an older daemon on first runtime use and restarts an online daemon after a local runtime update; finish active jobs before native offline replacement, incompatible schema migration or Windows online package replacement. Compatible Linux runners can continue during online updates. Versions before 0.1.12 require a manual same-profile stop. See [upgrade preparation](docs/updating.md#before-updating).

Native/source plugins use stable instruction entries and verified current-runtime MCP forwarding. Historical entries need [one migration and reconnection](docs/updating.md#cache-inspection-and-refresh); later same-version native updates follow the current workflow without clearing plugin caches.

### Unified online installation/update (source 0.1.14)

Install the CLI once, with npm's global command directory on PATH:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
```

After 0.1.14+ is published and installed, run `ajr update` for both initial ChatGPT Desktop/Codex plugin installation and later CLI/plugin updates. `ajr update --dry-run` previews changes; `--cli-only` leaves the plugin untouched. New online plugin entries require the installed CLI. Reconnect/restart the host afterward; data/history stay in their original profiles. This command is currently source-only: published 0.1.12 still uses the manual host steps below. See [unified updates and limits](docs/updating.md#one-command-online-update-0114).

Use `ajr uninstall` for online removal: stop idle daemons, remove the plugin, its unshared marketplace and the global CLI, and retain history/logs/backups. Active tasks block removal; `ajr uninstall --force` stops them through the existing control path first. Use `--dry-run` to preview. `ajr update` automatically replaces older daemons; compatible active Linux runners continue under their original Job/Run without replay. Incompatible migration and active Windows package replacement remain blocked. See [uninstallation](docs/uninstalling.md).

### ChatGPT Desktop

**Online installation:** register the marketplace matching the latest published runtime:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

Restart ChatGPT Desktop, open **Plugins → AgentJobRelay**, install/enable the plugin, and select `agent-job-relay@agent-job-relay` in a new local chat.

**Online update:** replace the marketplace registration with the latest released tag:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

Refresh/reinstall AgentJobRelay in Desktop and reconnect MCP. [Complete online update procedure](docs/updating.md#update-the-online-marketplace).

**Local compilation:** [build and install the local plugin](docs/source-install.md) ([简体中文](docs/source-install.zh-CN.md)); [update it while preserving history](docs/updating.md#unified-install-and-update).

### Codex

**Online installation:** register the released marketplace and install its plugin entry:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION" &&
codex plugin add agent-job-relay@agent-job-relay
```

If the matching marketplace is already registered, run only the last command. Restart/reconnect Codex after installation.

**Online update:** resolve the latest release, replace the catalog registration and reinstall the entry:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION" &&
codex plugin add agent-job-relay@agent-job-relay
```

Restart/reconnect Codex afterward. CLI integration provides tools and an inbox without automatic model continuation. [Complete online update procedure](docs/updating.md#update-the-online-marketplace).

**Local compilation:** [build/install the native plugin](docs/source-install.md) and use its [local Codex plugin entry](docs/source-install.md#6-install-and-select-the-correct-plugin-in-chatgpt-desktop); [preserve-history updates](docs/updating.md#unified-install-and-update) use the same recorded installation.

### OpenCode

**Online installation (V1):** add the published plugin to `opencode.json`, preserving any other entries, then restart OpenCode:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Its matching native runtime is installed automatically. Do not enable a second `agent_job_relay` MCP entry.

**Online update:** keep the `@latest` entry, refresh the package cache using OpenCode's supported mechanism and restart after finishing jobs. See [OpenCode updates](docs/updating.md#update-opencode-v1). V1 is experimental; V2 is unsupported.

**Local compilation:** follow [OpenCode development setup](docs/host-plugins.md#opencode-v1-development-setup) and [local build/update instructions](docs/host-plugins.md#local-build-and-updates). This host uses its own plugin package with the shared native runtime.

## Everyday use

### In your Agent

Select the installed AgentJobRelay plugin in your host. On first use, ask it to call `job_doctor` to confirm MCP connectivity, then describe your task:

> Run this project's tests in the background and tell me when they finish.

Everyday prompts need no executable path or session ID. The plugin binds the actual initiating session, submits the task and keeps its results. ChatGPT Desktop can continue the same chat on verified versions; OpenCode V1 continuation remains experimental. With Codex CLI's inbox, ask the Agent to inspect the result when needed.

You can also ask:

> Open AgentJobRelay's task manager so I can inspect the running tasks.

The plugin can open the bundled Web UI; new online installations require the global CLI, while native offline plugins use their own launcher. The current source workflow automatically shows an accepted background task beside the chat and reuses one panel for that chat/profile when the host supports embedded-tab discovery and navigation; the panel supports narrow sidebars. Follow [daily usage](docs/usage.md) for MCP submission and same-chat recovery, or [monitoring and Web UI](docs/runtime-monitoring.md) for persistent rules and stopping.

### Command line and Web UI

With the CLI installed, open the task manager or submit work using the online ChatGPT/Codex profile:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
agent-job-relay codex start --timeout 60 -- cargo check
agent-job-relay codex list
agent-job-relay codex inspect <job-id>
```

Use the returned job ID in the last command; the first submission starts the daemon automatically. OpenCode uses `agent-job-relay opencode`; native offline installations use their recorded launcher/profile. npm 0.1.11 and newer also register the `ajr` shortcut. See [command reference](docs/commands.md) and [CLI updates](docs/updating.md#update-the-terminal-command).

Commands run with your OS permissions. Job completion, host acknowledgement (`accepted_by_host`) and observed Agent continuation are separate results. Removal retains task history and logs; see [uninstallation](docs/uninstalling.md) and [troubleshooting](docs/troubleshooting.md).

## Find documentation

The **[documentation index / 文档导航](docs/README.md)** groups all guides by task.

| I want to… | Read |
|---|---|
| Understand capabilities and host support | [Features](docs/features.md), [limitations](docs/limitations.md) |
| Install or connect a host | [Installation](docs/installation.md), [Windows](docs/windows.md), [host plugins](docs/host-plugins.md) |
| Try feature test prompts | [English](docs/test-prompts.md) · [简体中文](docs/test-prompts.zh-CN.md) |
| Run tasks or look up a command | [Usage](docs/usage.md), [command reference](docs/commands.md), [monitoring](docs/runtime-monitoring.md) |
| Update, remove, back up or fix an installation | [Updates](docs/updating.md), [uninstallation](docs/uninstalling.md), [data maintenance](docs/data-maintenance.md), [troubleshooting](docs/troubleshooting.md) |
| Check progress or verification | [Implementation status](docs/IMPLEMENTATION_STATUS.md), [live evidence](docs/live-e2e.md), [roadmap](docs/TODO.md), [changelog](CHANGELOG.md) |
| Contribute or understand the design | [Contributing](CONTRIBUTING.md), [testing](docs/testing.md), [architecture](docs/architecture.md), [protocol](docs/protocol.md), [security](docs/security.md) |
| Prepare a maintainer release | [Manual release guide](docs/release.md) |

Community testing and PRs are welcome, especially for Claude Code and additional host versions. Releases use npm and a self-hosted marketplace; official-directory approval is separate. Licensed under [MIT](LICENSE).
