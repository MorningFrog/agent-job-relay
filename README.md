<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

English | [简体中文](README.zh-CN.md)

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。
</div>

Run long commands independently of the Agent connection, keep their results, and notify the initiating chat when the host supports it. **Published packages support Linux/Windows x86_64.** ChatGPT Desktop same-chat continuation is verified on Linux and Windows using an experimental interface; Windows verification uses Full access. macOS runtime is unsupported. OpenCode V1 has an experimental plugin package candidate; Claude Code has an inbox-only development framework that needs community verification.

For OpenCode/Claude development setup and evidence boundaries, see [host plugins](docs/host-plugins.md).

## Quick install

Use the latest published npm packages and matching GitHub Release. No Rust compiler or source build is needed. Online installation requires Node.js 20+/npm and network access; ChatGPT Desktop/Codex also needs a local executor, Codex CLI on PATH and Git for catalog fetching (or use the Release ZIP).

### ChatGPT Desktop / Codex

In Bash, select the latest npm runtime and its matching released marketplace:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

After it succeeds, restart ChatGPT Desktop, open **Plugins → AgentJobRelay**, install/enable the plugin, and select `agent-job-relay@agent-job-relay` in a new local chat. Codex CLI users can instead run `codex plugin add agent-job-relay@agent-job-relay`; CLI integration provides tools/inbox, without automatic model continuation.

Ask the selected plugin to call `job_doctor` before submitting a task. A skill appearing in the chat does not prove MCP connectivity. PowerShell commands, GitHub ZIP installation and native offline maintenance are in the [installation guide](docs/installation.md).

### OpenCode V1

Add the latest published plugin to `opencode.json`, then restart OpenCode:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

The plugin installs its matching native runtime automatically. OpenCode V1 remains experimental; see [host integration and evidence](docs/host-plugins.md). Already using a native local installation? Preserve its profile and follow [maintenance](docs/installation.md#updating-an-existing-installation).

## Windows

The same published-package route supports Windows x86_64; use the [PowerShell quick-install commands](docs/installation.md#chatgpt-desktop--codex). Windows starts the daemon on demand and can use a verified same-user local CIM bootstrap when host cleanup blocks native creation. If local policy blocks CIM, diagnostics provide a copyable outside-terminal fallback. Native runtime verification and live Desktop acceptance have version-specific boundaries; see the [Windows guide](docs/windows.md) and [live evidence](docs/live-e2e.md).

## Use from chat

Ask in a local ChatGPT Desktop chat with the plugin selected:

> Run this project's tests in the background and tell me when they finish.

The Agent binds the current chat, submits the task, ends its turn and reads the result after notification. Everyday prompts need no executable path or session ID. If MCP is unavailable, the workflow reports its CLI fallback. A local terminal is required for binding; ordinary Codex CLI integration provides a notification inbox, without automatic chat continuation.

The first submission or explicit session registration starts the daemon automatically. Commands run with your OS permissions. Job completion, host acknowledgement (`accepted_by_host`) and observed Agent continuation are separate results. For a first acceptance run, follow [manual testing](docs/manual-testing.md); detailed CLI commands and recovery are in [usage](docs/usage.md).

## Updating

Finish active jobs first. For the npm-backed ChatGPT Desktop/Codex marketplace, follow the [online update commands](docs/installation.md#update-the-online-marketplace): resolve latest, replace the old catalog registration, refresh/reinstall the plugin in the host and reconnect MCP. OpenCode's `@latest` entry follows the published channel; refresh its package cache using the host's supported mechanism and restart it. A running daemon keeps its loaded executable; an incompatible upgrade requires an explicit stop of the same profile.

Native local installations use a separate recorded profile. Follow [profile-preserving offline updates](docs/installation.md#updating-an-existing-installation), [unified maintenance](docs/installation.md#unified-install-and-update) or [interruption repair](docs/installation.md#compatibility-and-interrupted-operations). No update automatically stops a daemon, switches profiles or replays a workload.

## Uninstalling

Disable/remove the plugin through its host, then remove its marketplace/config entry if no longer wanted. History remains outside npm/plugin caches; removal does not stop existing daemons or erase task data. Native local installation removal is covered in the [installation guide](docs/installation.md#uninstalling), including preview/apply and preservation of modified or foreign files.

## Troubleshooting

| Symptom | Next step |
|---|---|
| Plugin or MCP tools missing | Refresh the plugin in the host and reconnect. A loaded skill alone does not prove MCP loaded. |
| `npm E404` for `agent-job-relay-runtime` | Check the pinned version, registry and network for the online entry; the separate native route is [local source installation](docs/source-install.md). |
| Local self-check passes, but chat tools fail | Invoke `job_doctor` from the host; a local stdio check does not establish host connectivity. |
| Update reports an active runtime | Finish jobs, then explicitly stop the daemon for the original installation. |
| Notification does not arrive | Check session/delivery state; use same-chat recovery when needed. See [usage](docs/usage.md). |
| Ownership check refuses changes | Preserve the files and inspect the reported changes/backup. |

## Plugin releases

Source and release assets are public on [GitHub](https://github.com/MorningFrog/agent-job-relay). Install using npm `latest` and its matching GitHub Release; package-internal runtime dependencies stay pinned together. Online marketplace/OpenCode installation is described in the [release guide](docs/release.md#consumer-installation). Existing native installations keep the [offline maintenance route](docs/installation.md#updating-an-existing-installation).

Only maintainers publish. **Prepare Release** builds/tests a Cargo-versioned candidate; **Publish Release** previews OIDC or submits those exact files; **Check Release** independently verifies public availability. **Release All** provides one manual entry for the entire sequence and defaults to dry-run. Formal releases use `latest`; notes come from CHANGELOG. Pushes, PRs and tags never publish. See [contributor conventions](CONTRIBUTING.md) and the [maintainer release guide](docs/release.md). A self-hosted marketplace is separate from OpenAI's official directory.

## Install from source

Follow the [step-by-step local source installation](docs/source-install.md) ([简体中文](docs/source-install.zh-CN.md)). It covers prerequisites, checkout, compilation, bundle export, first installation or history-preserving update, ChatGPT Desktop selection and an actual MCP check, with expected results at each step.

The result is the native local plugin in **AgentJobRelay Local**, normally referenced as `agent-job-relay@agent-job-relay-local`. No npm publication or Node.js installation is required. Building alone does not install it; complete the bundle installation/update and ChatGPT Desktop steps. Windows source commands are in the [Windows guide](docs/windows.md#build-when-no-bundle-is-available); developer gates are in [testing](docs/testing.md).

## Implementation and verification

| Area | Implementation | Verification and remaining scope |
|---|---|---|
| Linux runtime and offline maintenance | Implemented | Automated execution, lifecycle, recovery and installation tests pass. |
| Windows runtime and automatic startup | Implemented | Native Windows 11 tests and actual Codex MCP cold startup under an ordinary interactive user pass. |
| ChatGPT Desktop same-chat continuation | Verified on Linux and Windows | The verified published baseline continues the initiating chat after task completion. Windows verification uses Full access; see version-specific [live evidence](docs/live-e2e.md). |
| OpenCode V1 | Experimental plugin candidate | Actual loader, MCP and session API pass with a simulated model; real-provider sessions, restart recovery and V2 remain open. |
| Claude Code | Inbox-only scaffold | Synthetic caller and MCP launcher checks pass; actual Claude loader/account/model verification needs community help. |
| npm runtime and self-hosted marketplace | Published | Linux/Windows package/loader checks and clean public-registry installation pass; official directory approval is separate. |
| CI and release | Manual prepare/publish/check plus Release All | Native/package CI, both npm OIDC publications and GitHub Release completion pass; public installation passes locally, and the repaired independent Actions check awaits a maintainer run. |
| macOS runtime | Unimplemented | Portable compilation checks only; native execution remains unsupported. |
| Output monitoring/interruption | Deferred | Awaits an explicit implementation request; log reads and manual stop are available. |

The current release adds unified `plugin install` and `connect [--recover [--apply]]` entries, caller/workspace binding objects and compact MCP results (`full=true` for complete records); see [daily usage](docs/usage.md) and [unified maintenance](docs/installation.md#unified-install-and-update). It also reduces idle database work and permits notifications to up to four chats concurrently. Current results and version-specific limits are maintained in [implementation status](docs/IMPLEMENTATION_STATUS.md) and [live evidence](docs/live-e2e.md). Official directory submission requires separate approval; independent OS installers/package repositories are cancelled. **Community testing and PRs are welcome, especially for Claude Code and container-based version checks.**

Unreleased source adds automatic data maintenance: 30-day/1,000-job/256-MiB hot-data budgets, daily rolling backups and transparent archived-history reads. It also reserves database reads for control requests and keeps reconciliation off the IPC acceptance loop. These changes use schema v8 and require finishing old workloads and explicitly stopping the old daemon before upgrade; the published package remains unchanged. See [automatic maintenance and restore](docs/data-maintenance.md).

## Documentation and support

| Topic | Guide |
|---|---|
| Step-by-step local installation from source | [Source installation](docs/source-install.md), [简体中文](docs/source-install.zh-CN.md) |
| Installation, profiles, updates, caches and removal | [Installation](docs/installation.md) |
| Automatic data budgets, backups and restore (unreleased source) | [Data maintenance](docs/data-maintenance.md) |
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
