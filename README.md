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
| Goal waiting (experimental) | One scoped pause approval, original-goal restoration and deletion safeguards. Source 0.1.17 adds requested intermediate restoration/report/native re-pause cycles; [usage and host limits](docs/goal-waiting.md) |
| Batch submission | Submit 1–16 independent commands through CLI/MCP, with per-item results and safe retries; [usage](docs/usage.md#submit-independent-commands-together-0112) |
| Remote tasks | Launch/attach on a pinned remote runtime or observe a fixed GitHub Actions attempt, including job/step status and an attempt link; [setup and boundaries](docs/remote-execution.md) |
| Mid-run feedback | Structured stages/steps/metrics, repeated milestone or threshold alerts and a bounded checker recipe; [usage and limits](docs/progress.md) |
| Results and history | Bounded stdout/stderr, exit outcomes, events and retained task history |
| Session-bound notifications | Deliver results to the initiating session; resume the chat when supported by its host |
| Local task manager | Inspect tasks, configure persistent output/silence/elapsed rules and stop work through CLI or the bundled Web UI |
| Data maintenance | Automatic archival, rolling backups and explicit restore into a new profile |

**Runtime: Linux x86_64/glibc and Windows x86_64.** ChatGPT Desktop same-chat continuation is verified on both through an experimental private interface; Windows verification uses Full access. Codex CLI and the Claude Code scaffold provide an inbox without automatic continuation. OpenCode V1 is experimental and has actual-loader checks with a simulated model. macOS/ARM runtime is unsupported. See [features and support](docs/features.md) for capabilities and [live evidence](docs/live-e2e.md) for verified versions.

## Install

Online installation needs **Node.js 20+/npm**, network access and npm's global command directory on PATH. ChatGPT Desktop/Codex also needs **Codex CLI and Git**; Desktop needs a local executor. No Rust compiler is needed. Use a user-writable npm installation. [PowerShell commands and other installation routes](docs/installation.md).

### ChatGPT Desktop / Codex

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr update
```

This installs the global CLI and matching marketplace/plugin. Restart or reconnect Desktop/Codex, enable **AgentJobRelay** in Plugins and select `agent-job-relay@agent-job-relay` in a local chat. Ask it to call `job_doctor` to check MCP connectivity. See [installation](docs/installation.md#chatgpt-desktop--codex) for expected results and host requirements.

### Terminal and Web UI only

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr ui
```

`ajr` uses the online ChatGPT/Codex profile; `ajr opencode` selects OpenCode. Native offline installations use their recorded launcher and profile.

### OpenCode V1

Add this entry to `opencode.json`, preserving existing plugins, then restart OpenCode:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

The matching native runtime is included. Do not enable a second `agent_job_relay` MCP entry. [OpenCode setup and support limits](docs/installation.md#opencode-v1).

For a native local plugin, follow the [source walkthrough](docs/source-install.md) ([简体中文](docs/source-install.zh-CN.md)) or [offline bundle steps](docs/installation.md#quick-install-from-an-offline-bundle).

## Update or uninstall

For the online ChatGPT Desktop/Codex installation:

| Task | Preview | Apply |
|---|---|---|
| Update CLI and plugin | `ajr update --dry-run` | `ajr update` |
| Remove CLI and plugin | `ajr uninstall --dry-run` | `ajr uninstall` |

Restart or reconnect the host afterward. Uninstall retains history, logs and backups; active tasks block removal unless you explicitly use `--force` to stop them first. These maintenance commands require CLI 0.1.14 or newer; [older CLIs](docs/updating.md#update-the-terminal-command) first need the npm installation command above.

Use the route matching your installation: [online and native updates](docs/updating.md), [OpenCode updates](docs/updating.md#update-opencode-v1), [uninstallation and configuration removal](docs/uninstalling.md). Updates keep the original profile; finish jobs before native offline/Windows package replacement or incompatible schema migration. Compatible Linux runners can continue during online updates.

## Everyday use

### In your Agent

Select the installed AgentJobRelay plugin in your host. On first use, ask it to call `job_doctor` to confirm MCP connectivity, then describe your task:

> Run this project's tests in the background and tell me when they finish.

Everyday prompts need no executable path or session ID. The plugin binds the actual initiating session, submits the task and keeps its results. ChatGPT Desktop can continue the same chat on verified versions; OpenCode V1 continuation remains experimental. With Codex CLI's inbox, ask the Agent to inspect the result when needed.

You can also ask:

> Open AgentJobRelay's task manager so I can inspect the running tasks.

The plugin opens the bundled task manager beside the chat when the host supports it. Follow [daily usage](docs/usage.md) for MCP submission and same-chat recovery, or [monitoring and Web UI](docs/runtime-monitoring.md) for persistent rules and stopping.

### Command line and Web UI

With the CLI installed, open the task manager or submit work using the online ChatGPT/Codex profile:

```bash
ajr ui
ajr start --timeout 60 -- cargo check
ajr list
ajr inspect <job-id>
```

Use the returned job ID in the last command; the first submission starts the daemon automatically. OpenCode uses `ajr opencode`; native offline installations use their recorded launcher/profile. See the [command reference](docs/commands.md).

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
