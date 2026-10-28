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
| Results and history | Bounded stdout/stderr, exit outcomes, events and retained task history |
| Session-bound notifications | Deliver results to the initiating session; resume the chat when supported by its host |
| Local task manager | Inspect tasks, configure persistent output/silence/elapsed rules and stop work through CLI or the bundled Web UI |
| Data maintenance | Automatic archival, rolling backups and explicit restore into a new profile |

**Runtime: Linux x86_64/glibc and Windows x86_64.** ChatGPT Desktop same-chat continuation is verified on both through an experimental private interface; Windows verification uses Full access. Codex CLI and the Claude Code scaffold provide an inbox without automatic continuation. OpenCode V1 is experimental and has actual-loader checks with a simulated model. macOS/ARM runtime is unsupported. See [features and support](docs/features.md) for capabilities and [live evidence](docs/live-e2e.md) for verified versions.

## Get started

Online installation needs Node.js 20+/npm and network access. ChatGPT Desktop/Codex also needs a local executor, Codex CLI on PATH and Git for catalog fetching. No Rust compiler is needed.

### ChatGPT Desktop / Codex

In Bash, register the marketplace matching the latest published runtime:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

Restart ChatGPT Desktop, open **Plugins → AgentJobRelay**, install/enable the plugin, and select `agent-job-relay@agent-job-relay` in a new local chat. Ask it to call `job_doctor`, then give it a task:

> Run this project's tests in the background and tell me when they finish.

Codex CLI users install the entry with `codex plugin add agent-job-relay@agent-job-relay`; CLI integration provides tools/inbox. For PowerShell, Release ZIPs, OpenCode and native offline bundles, follow [installation](docs/installation.md). For local development, use [source installation](docs/source-install.md) ([简体中文](docs/source-install.zh-CN.md)).

### Terminal and Web UI

Install the CLI separately if you want global terminal commands:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
```

This opens the bundled task manager using the online ChatGPT/Codex profile. You can also ask the selected plugin to open it. The source adds an `ajr` shortcut that awaits publication; use the full commands here with the published package. See [command reference](docs/commands.md) for entry points and profile selection.

## Everyday use

The first submission starts the daemon automatically. From a terminal:

```bash
agent-job-relay codex start --timeout 60 -- cargo check
agent-job-relay codex list
agent-job-relay codex inspect <job-id>
```

Use the returned job ID in the last command. Follow [daily usage](docs/usage.md) for logs, MCP submission and same-chat recovery, or [monitoring and Web UI](docs/runtime-monitoring.md) for persistent rules and stopping.

Commands run with your OS permissions. Job completion, host acknowledgement (`accepted_by_host`) and observed Agent continuation are separate results. Updates require preserving the original profile; removal retains task history and logs. See [updates](docs/updating.md), [uninstallation](docs/uninstalling.md) and [troubleshooting](docs/troubleshooting.md).

## Find documentation

The **[documentation index / 文档导航](docs/README.md)** groups all guides by task.

| I want to… | Read |
|---|---|
| Understand capabilities and host support | [Features](docs/features.md), [limitations](docs/limitations.md) |
| Install or connect a host | [Installation](docs/installation.md), [Windows](docs/windows.md), [host plugins](docs/host-plugins.md) |
| Run tasks or look up a command | [Usage](docs/usage.md), [command reference](docs/commands.md), [monitoring](docs/runtime-monitoring.md) |
| Update, remove, back up or fix an installation | [Updates](docs/updating.md), [uninstallation](docs/uninstalling.md), [data maintenance](docs/data-maintenance.md), [troubleshooting](docs/troubleshooting.md) |
| Check progress or verification | [Implementation status](docs/IMPLEMENTATION_STATUS.md), [live evidence](docs/live-e2e.md), [roadmap](docs/TODO.md), [changelog](CHANGELOG.md) |
| Contribute or understand the design | [Contributing](CONTRIBUTING.md), [testing](docs/testing.md), [architecture](docs/architecture.md), [protocol](docs/protocol.md), [security](docs/security.md) |
| Prepare a maintainer release | [Manual release guide](docs/release.md) |

Community testing and PRs are welcome, especially for Claude Code and additional host versions. Releases use npm and a self-hosted marketplace; official-directory approval is separate. Licensed under [MIT](LICENSE).
