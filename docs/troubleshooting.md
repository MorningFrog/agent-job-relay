# Troubleshooting

[Documentation index](README.md) · [Command reference](commands.md) · [Updating and repair](updating.md)

Use the same launcher and state profile as the affected plugin or task. Keep the original job/result when diagnosing failures. A different profile can make history appear missing without fixing the underlying issue.

## Find the symptom

| Symptom | Next step |
|---|---|
| Plugin or MCP tools missing | Refresh/reinstall the selected plugin and reconnect MCP; check [tool connectivity](#plugin-visible-but-tools-missing). |
| Terminal command missing | Install the [global npm CLI](installation.md#terminal-and-web-ui) and ensure npm's global command directory is on PATH. Plugin installation alone does not register it. |
| `ajr` missing | Install/update the global CLI using the [npm command](updating.md#update-the-terminal-command) and check its command directory on PATH. |
| `npm E404` / `ETARGET` | Check the selected version, registry and network. Use the latest published runtime and matching release catalog rather than an unpublished source pin. |
| Web UI reports an older daemon | Use the new runtime; runtime 0.1.12+ replaces older daemons on `ui`. Versions before 0.1.12 require finished jobs and a manual same-profile stop; see [updating](updating.md#before-updating). |
| UI no longer authenticates | Reopen `ui` after daemon replacement; its access credential changes. See [Web access](runtime-monitoring.md#open-the-bundled-web-ui). |
| Windows startup/binding fails | Follow [Windows startup](windows.md#automatic-startup) and [binding permissions](windows.md#current-chat-binding-permissions); use only the reported same-profile fallback. |
| Update reports an active runtime | Finish jobs and retry the reviewed [update](updating.md); native apply restarts an online daemon automatically; online updates allow compatible active Linux runners only. With older update executables, stop that same daemon manually first. |
| `update` is an unknown command | [Update an older global CLI through npm](updating.md#update-the-terminal-command), then retry `ajr update`. |
| `Marketplace has local changes` | Inspect the exact registered checkout; 0.1.15 fixes the false refusal of Codex's generated installation record while preserving other changes. See [marketplace checks and workaround](updating.md#marketplace-reports-local-changes). |
| Partial online update/removal | Retry with the original profile/options; preserve the recorded journal. See [online update retry](updating.md#online-update-retry) or [uninstall recovery](uninstalling.md#recover-a-partial-online-uninstall). |
| Interrupted install/cache replacement | Use the new trusted bundle to inspect and [repair the exact recorded destination](updating.md#compatibility-and-interrupted-operations). |
| Ownership check refuses changes | Preserve the files and inspect the reported changes/backup. Never remove a parent directory containing foreign content. |
| History appears missing | Compare explicit state arguments, AJR_STATE_DIR and the launcher's [recorded/default profile](commands.md#choose-the-entry-point-and-profile). |
| Notification does not arrive | Inspect session/delivery state and preview [same-chat recovery](usage.md#recover-notifications). |
| Monitor rule has not fired | Check saved/applied revision, condition and trigger evidence. Rules fire once per revision; see [monitoring](runtime-monitoring.md#install-persistent-rules). |
| Storage quota or backup error | Inspect `data status`; use [data maintenance](data-maintenance.md) for protected records, budgets and backup diagnostics. |

## Understand the checks

| Check | What it establishes |
|---|---|
| `relay --version` | Executable version |
| `relay doctor` | Daemon/store health when available; a stopped daemon is reported without starting it |
| `relay doctor --client plugin` | Owned marketplace entry matches this installation |
| `relay doctor --check-mcp` | Local stdio initialization/tool discovery/EOF |
| Host `job_doctor` | This actual tool call reached MCP |
| Observed same-chat continuation | The Agent actually resumed to handle this task's result |

Configuration, local stdio, host connectivity and continuation are separate checks. Missing tools require host refresh/reconnection; a loaded skill alone does not establish MCP availability.

## Plugin visible but tools missing

First identify the selected marketplace entry. The online `agent-job-relay@agent-job-relay` entry uses a published npm runtime; native local installation normally uses `agent-job-relay@agent-job-relay-local` from **AgentJobRelay Local**. A source checkout can pin an unpublished npm version. Use [published installation](installation.md) for online use or complete the [local source walkthrough](source-install.md) ([中文](source-install.zh-CN.md)) for a native plugin.

Ask the selected plugin for an actual `job_doctor` call. If a local `doctor --check-mcp` passes but the host call fails, refresh/reinstall that plugin entry and reconnect the host. A loaded skill alone is insufficient. Stop an MCP acceptance test when its tools are missing; a CLI submission is a separate workflow and cannot establish that MCP passed. Do not edit cached manifests or switch profiles to conceal the failure.

## Missing or failed notifications

Inspect the job's terminal state, `delivery list --job-id <job-id>` and `delivery attempts <delivery-id>` with the original profile. Inbox recording, host admission and observed Agent execution are distinct. A host with inbox-only capability will not automatically continue a chat.

After host replacement, obtain a fresh binding from the same chat and preview `connect --recover`; apply only for that actual caller. Recovery preserves delivery IDs, attempt history and original expiry, excluding accepted/expired events and live claims. Explicit `delivery retry` renews TTL and has different semantics. Neither action reruns the command. Follow [usage](usage.md#recover-notifications) for the complete procedure and [live evidence](live-e2e.md) for qualified scenarios.
