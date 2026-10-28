# Features and support

[Documentation index](README.md) · [Install](installation.md) · [Use](usage.md)

AgentJobRelay is a single-user local runtime for long commands initiated by an Agent or terminal. One Rust executable provides CLI, MCP, daemon and runner modes; plugins supply the host-specific caller binding and notification path.

## Background execution and results

| Capability | Behavior |
|---|---|
| Explicit commands | Execute argv or an explicitly selected shell with a working directory, timeout and idempotency key |
| Batch submission (unreleased source) | CLI/MCP accepts 1–16 independent specs with per-item results, stable-key retries and the existing eight-active-job quota; see [batch usage](usage.md#submit-independent-commands-together-unreleased-source) |
| Remote execution/observation (unreleased source) | Pinned native remote launch/attach, reconnect and exact-run stop; existing GitHub Actions attempt observation through `gh`; [requirements and limits](remote-execution.md) |
| Independent lifetime | A detached runner owns the workload; closing the submitting chat, MCP client, terminal viewer or browser does not stop it |
| Durable results | Separate job/run IDs, exit outcomes, events, stdout/stderr and byte cursors remain available after completion |
| Controlled stopping | Request graceful then forced process-tree stopping with bounded observation; terminal outcomes are preserved |
| Conservative recovery | Daemon restart reconciles stored work; uncertain execution is diagnosed rather than automatically replayed |

See [daily usage](usage.md) and the [command reference](commands.md). Commands execute with your OS permissions. Eight active jobs are allowed; raw stdout/stderr retain up to 16 MiB per stream. This runtime is not a command sandbox and does not resume processes across machine reboot.

## Inspection, rules and task manager

CLI, MCP and the bundled Web UI share the same inspection, monitor and stop behavior. Inspect task state and recent output; add persistent literal-output, silence or elapsed-time rules that record an alert or stop the workload. Rules run in the runner without an Agent, browser or daemon remaining connected. A rule fires once per revision and can be rearmed explicitly.

The task manager is embedded in the native executable and enabled by default with the daemon. It uses authenticated loopback access and opens only on request. See [runtime monitoring](runtime-monitoring.md) for rule limits, application evidence, credentials and offline snapshots.

## Session-bound notifications

Jobs, sessions and deliveries have independent lifetimes. Each notification targets the actual initiating session; shared MCP does not infer a caller from its process environment or choose a recent chat. Durable deliveries retain IDs, attempts and expiry through retries and same-chat recovery. Recovery never reruns a command.

Job completion, inbox recording (`notification_recorded`), host acceptance (`accepted_by_host`) and observed Agent continuation are different results. No adapter automatically records `confirmed_resumed`. See [notification usage and recovery](usage.md#recover-notifications), [host integration](host-plugins.md) and [delivery protocol](protocol.md#delivery).

## History, backups and local maintenance

Automatic maintenance archives eligible terminal history according to age/count/log budgets and keeps bounded rolling local backups. Existing task/log queries include archived records. Explicit restore verifies a backup and creates a new profile without replaying execution intents. Installation/update/removal preserve the recorded state profile and refuse modified or foreign owned content.

See [data maintenance](data-maintenance.md), [updating](updating.md) and [uninstalling](uninstalling.md). Archival retains history indefinitely; backups on the same disk do not protect against loss of that disk.

## Platform and host support

| Platform or host | Available behavior | Evidence boundary |
|---|---|---|
| Linux x86_64/glibc | Native runtime, CLI/MCP/Web and npm/native bundles | Native behavior and installed-package checks |
| Windows x86_64 | Native runtime, private IPC, process trees and on-demand daemon startup | Native/package checks; policy can block the daemon-only CIM fallback |
| ChatGPT Desktop on Linux/Windows | Experimental same-chat completion notification and continuation | Verified for the version in [live evidence](live-e2e.md); Windows acceptance uses Full access and a private interface |
| Codex CLI | Tools and notification inbox | No automatic model continuation |
| OpenCode V1 | Experimental npm plugin using actual caller context and the public session API | Actual loader/MCP/agent loop with a simulated model; real-provider and restart-recovery qualification remain open |
| Claude Code | Inbox-only source scaffold | Synthetic caller/MCP checks; actual loader/account/model verification needs community help |
| macOS / ARM | No native runtime support | Portable compilation does not establish execution support |

Online distribution uses npm and a self-hosted marketplace. Official-directory approval is separate; independent OS installers/package repositories are cancelled. Published and source versions may differ: [implementation status](IMPLEMENTATION_STATUS.md) records the current baseline, [live evidence](live-e2e.md) records real-host acceptance, and [TODO](TODO.md) tracks remaining work. Detailed operational limits are in [limitations](limitations.md) and [security](security.md).
