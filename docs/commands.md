# Command reference

[Documentation index](README.md) · [Usage walkthrough](usage.md) · [Troubleshooting](troubleshooting.md)

This reference groups public commands by purpose: [tasks](#start-and-read-tasks), [monitoring](#inspect-monitor-and-open-the-task-manager), [sessions/delivery](#bind-sessions-and-recover-notifications), [daemon/diagnostics](#manage-the-daemon-and-diagnose), [installation](#install-update-and-remove-integrations), [data](#maintain-data) and [MCP tools](#mcp-tools-by-purpose). Use `<command> --help` for exact flags supported by your installed version. Step-by-step procedures live in the linked guides; hidden runner and Windows bootstrap modes are internal implementation details.

## Choose the entry point and profile

| Installation | Command prefix | Selected state |
|---|---|---|
| Published npm runtime, ChatGPT/Codex | `agent-job-relay codex` | Persistent online `codex` profile |
| Published npm runtime, OpenCode | `agent-job-relay opencode` | Persistent online `opencode` profile |
| npm 0.1.11 and newer | `ajr` or `ajr opencode` | `codex` by default, or `opencode` explicitly |
| Native offline installation | `"$AJR_INSTALL_ROOT/relay"` | That installation's recorded profile |
| Windows native installation | `& "$ajrInstall\relay.cmd"` in PowerShell | That installation's recorded profile |
| Source/developer binary | `./target/release/agent-job-relay` | Explicit state, environment or XDG/HOME default |

The full npm command without a host selector retains its original native default, which may differ from the online plugin's profile. Use `codex` or `opencode` explicitly. Ordinary selection prioritizes global `--state-dir`, then `AJR_STATE_DIR`, then the installed/host default. `daemon start/stop --install-root` and `ui --install-root` deliberately select the recorded installation profile. Native plugin updates retain the recorded profile even when unrelated shell overrides exist.

See [installation profiles](installation.md#profiles-and-existing-installations) and [custom directories](installation.md#custom-directories-and-stored-data). Windows npm wrappers can be invoked as `agent-job-relay.cmd` / `ajr.cmd` when PowerShell policy blocks `.ps1` wrappers.

Examples below use the published ChatGPT/Codex prefix. Substitute the appropriate prefix from the table for your installation.

```bash
agent-job-relay codex --help
agent-job-relay codex start --help
agent-job-relay codex monitor add --help
```

## Start and read tasks

| Command after the prefix | Purpose and useful options |
|---|---|
| `start -- <program> <args...>` | Submit explicit argv; `--title <purpose>` (0.1.17+); `--cwd`, `--timeout <seconds>`, `--idempotency-key`, `--session` or current-terminal `--wake` |
| `start --shell '<script>'` | Explicit shell execution; `--interpreter` selects the shell |
| `start-batch --file <json-file>` (0.1.12+) | Submit a JSON array of 1–16 specs with distinct stable keys; `--file -` reads stdin, `--wake` binds every item to the actual caller; [partial-admission/retry semantics](usage.md#submit-independent-commands-together-0112) |
| `start-spec --file <json-file>` (0.1.12+) | Submit one bounded JSON StartSpec, including remote launch/attach or GitHub Actions observation; `--file -` for stdin and `--wake` for the current caller |
| `remote init` / `remote serve` (0.1.12+) | Explicit remote-profile provisioning / one authenticated stdio request; [setup and protocol](remote-execution.md) |
| `list` | Bounded history; `--after`, `--limit` |
| `status <job-id>` | Durable job/run state and exit outcome |
| `logs <job-id>` | Bounded stdout/stderr; `--stream`, byte `--cursor`, `--limit`, optional human-viewer `--follow`; 0.1.18+ `--export <new-file>` exports retained text with gap markers |
| `events <job-id>` | Durable events; `--after`, `--limit` |
| `wait <job-id>` | Bounded event wait; `--after`, `--seconds` (at most 30) |
| `stop <job-id>` | Idempotent cancellation; `--grace` before force stop, `--wait` for bounded observation. Read-only remote/workflow jobs stop observation only |

```bash
agent-job-relay codex start --timeout 60 --idempotency-key build-001 -- cargo check
agent-job-relay codex logs <job-id> --stream stderr --cursor 0 --limit 8192
agent-job-relay codex stop <job-id> --grace 30s --wait 10s
```

Use returned IDs and `next_cursor`; do not use PIDs as job IDs. Reusing a key with an identical specification returns the original job; a deliberate rerun needs a new key. Long Agent jobs should end the turn after a capable-host submission rather than poll. See [execution and result workflow](usage.md#run-and-inspect-commands-through-cli).

## Inspect, monitor and open the task manager

| Command after the prefix | Purpose |
|---|---|
| `inspect [job-id]` | Overview or bounded task snapshot; `--watch`, `--json`, `--before`, `--limit` |
| `ui [job-id]` | Open the embedded task manager; `--no-open` prints access information; `--install-root` selects an installation |
| `monitor list <job-id>` | Inspect rule revisions, application and trigger evidence |
| `monitor add <job-id>` | Add one rule |
| `monitor update <job-id> <rule-id> --revision <n>` | Replace the observed rule revision |
| `monitor enable/disable/remove <job-id> <rule-id> --revision <n>` | Rearm, disable or remove a rule while retaining its audit |

`--progress '{"source":"training","trigger":{"kind":"stage"},"cooldown_seconds":30}'` adds repeated structured feedback; see [progress/checker usage and limits](progress.md). The same option is accepted by start and monitor add/update, and the Web editor supports stage/step/metric rules.

Rules use one condition: `--no-output <duration>`, `--elapsed <duration>` or `--contains <literal>`. Options include `--stream stdout/stderr/both`, `--action alert/stop`, `--grace <duration>` and `--notify-agent` for an already bound job. Start accepts one initial rule with the same flags. Durations accept seconds or `s`, `m`, `h`, `d`; stop grace is bounded to 0–300 seconds and wait to 0–30. See [monitoring procedures and expected results](runtime-monitoring.md).

## Bind sessions and recover notifications

| Command after the prefix | Purpose |
|---|---|
| `connect [--cwd <absolute-project>]` | Obtain a fresh current-chat/workspace binding from its actual terminal |
| `connect [--request-label <label>] [--job-count <1..16>] [--request-id <id>]` (experimental) | Prepare a scoped request or reconnect after approval; pending input creates no wait/job |
| `connect --wait-goal` (experimental) | Assert actual user pause permission; scoped options record it for this request, otherwise use single-handoff compatibility |
| `instructions` | Read the current embedded plugin workflow, policy and digest without starting a daemon/job; cached skills load this at every invocation |
| `goal-wait <id> [--confirm-paused\|--cancel] [--expected-revision <n>]` (experimental) | Inspect, confirm this chat's successful native goal pause, or cancel future control/notification; mutations require the observed revision and jobs keep running; [handoff and interruption](goal-waiting.md) |
| `connect --recover [--apply]` | Preview/apply same-chat recovery; `--after`, `--limit`, `--full` |
| `session connect [--brief]` | Compatible current-terminal binding entry |
| `session recover [--apply]` | Compatible same-chat recovery entry |
| `session register --agent <kind> --session-id <id> --instance-id <id> --host-id <id>` | Advanced explicit registration; `--lease-seconds`, endpoint/caller fields and experimental/renewal opt-ins |
| `session list` | List registrations; `--after`, `--limit` |
| `session probe/invalidate <session-key>` | Probe or invalidate an exact registration |
| `inbox <session-key>` | Inspect durable inbox notifications; `--after`, `--limit` |
| `delivery list [--job-id <job-id>]` | Inspect deliveries; `--after`, `--limit` |
| `delivery attempts <delivery-id>` | Attempt history; `--after`, `--limit` |
| `delivery retry <delivery-id>` | Explicit retry with renewed TTL, preserving ID/history; never rerun the job |
| `target add <job-id> <session-key> [--replay]` | Advanced exact-session target, optionally replaying eligible existing terminal events |
| `target rebind <target-id> <session-key>` | Advanced explicit target replacement |

`connect` and wake-enabled start routes return successful `needs_confirmation` before wait/job creation for an active goal without recorded permission. Use `job_goal_consent` for one native form or Agent-question fallback, then `connect --request-id <id>` for this request’s remaining rounds. `--request-label` and `--job-count` fix the scope; `--wait-goal` may assert actual permission already present in the initial user request. Doctor separates scoped consent/native handoff from unavailable consent-free pause. See [goal waiting](goal-waiting.md).

Recovery preserves original expiry and excludes accepted/expired events and live claims. Explicit delivery retry has different TTL semantics. Never guess caller IDs or reuse a binding from another chat. See [MCP submission](usage.md#submit-through-mcp-with-same-chat-notification), [recovery](usage.md#recover-notifications) and [protocol](protocol.md) before advanced target operations.

## Manage the daemon and diagnose

| Command after the prefix | Purpose |
|---|---|
| `daemon start [--install-root <root>]` | Start/reuse this profile without submitting work |
| `daemon stop [--install-root <root>]` | Stop dispatch; independent runners and results remain |
| `daemon` | Run the daemon in the foreground for diagnosis |
| `doctor [--client plugin/codex/json] [--check-mcp]` | Health/configuration diagnosis and optional local stdio check; `--config` requires `--client` |
| `mcp [--shared-host]` | Serve MCP on stdio; shared mode requires actual caller binding |
| `request '<request-json>'` | Advanced raw IPC request; the CLI wraps protocol v1 |
| `--version` | Print this executable's version |

`doctor`, `status` and `inspect` do not start a daemon. A local MCP self-check is separate from host tool connectivity. See [diagnostics](troubleshooting.md#understand-the-checks) and [Windows startup](windows.md#automatic-startup).

## Install, update and remove integrations

Native configuration and directory changes preview by default; `--apply` writes the reviewed change. Pack export follows the same pattern. Online npm `update` and `uninstall` apply by default; `--dry-run` selects its preview.

| Command after the prefix | Purpose and guide |
|---|---|
| `update [--dry-run] [--to <x.y.z>] [--cli-only]` | npm 0.1.14+: coordinate global CLI and matching online ChatGPT Desktop/Codex plugin installation/update; [procedure and limits](updating.md#one-command-online-update-0114) |
| `plugin install [--root <root>] [--from <bundle>]` | Native bundle installation/update and marketplace registration; `--config`, repeated `--cache`, `--full`; [install](installation.md#quick-install-from-an-offline-bundle), [update](updating.md#update-a-native-local-or-offline-plugin) |
| `plugin bootstrap [--root <root>] [--config <file>]` | Compatible first-install entry from a bundle; [installation](installation.md) |
| `plugin update --from <bundle> [--root <root>] [--cache <path>]` | Compatible profile-preserving native update; [updating](updating.md#update-a-native-local-or-offline-plugin) |
| `plugin pack --output <unused-directory>` | Export a native bundle; [source installation](source-install.md#3-build-and-export-a-local-bundle) |
| `plugin status [--root <root>] [--cache <path>]` | Inspect installation and explicitly selected caches |
| `plugin cache refresh --path <cache> [--root <root>]` | Replace a verified known cache; [cache maintenance](updating.md#cache-inspection-and-refresh) |
| `plugin repair --path <exact-directory>` | Finish a recorded interrupted replacement; [repair](updating.md#compatibility-and-interrupted-operations) |
| `install [--root <root>]` | Low-level native development install; [alternative routes](installation.md#install-directly-from-a-source-build) |
| `setup plugin/codex/json [--config <file>]` | Configure an owned marketplace or MCP entry; [standalone MCP](installation.md#configure-standalone-mcp) |
| `integration remove plugin/codex/json [--config <file>]` | Remove unchanged owned configuration; [uninstallation](uninstalling.md) |
| `uninstall [--dry-run] [--force] [--cli-only]` | npm 0.1.14+: remove online plugin/unshared marketplace/global CLI, stop idle daemons by default; force uses normal task stops, always retain data; [online removal](uninstalling.md#one-command-online-uninstall-0114) |
| `uninstall --root <root>` | Remove unchanged owned native installation files with preview/`--apply` and retain state; [uninstallation](uninstalling.md) |

Online npm installation and host catalog commands are documented in [installation](installation.md) and [updating](updating.md); they are separate from native offline management commands.

## Maintain data

| Command after the prefix | Purpose |
|---|---|
| `data status` | Capacity, policy, latest scan, backup IDs and errors |
| `data policy` | Read policy; options preview changes until `--apply` |
| `data maintain [--apply]` | Preview or run an immediate maintenance batch and due backup |
| `data backup` | Create an additional consistent local snapshot |
| `data restore --from <backup> --to <new-state-directory>` | Verify and restore into a new profile without command replay |
| `data check` | Explicit SQLite integrity and foreign-key validation |

Policy flags are `--hot-jobs`, `--archive-after-days`, `--hot-log-mib`, `--backup-hours` and `--backup-keep`. Automatic defaults, restore requirements and interruption behavior are in [data maintenance](data-maintenance.md).

## MCP tools by purpose

| Purpose | Tools |
|---|---|
| Execution and results | `job_start`, `job_status`, `job_list`, `job_logs`, `job_events`, `job_wait`, `job_stop` |
| Batch and experimental goal control | `job_start_batch`, `job_goal_consent`, `job_goal_wait`; scoped consent and native handoff use the actual terminal binding |
| Inspection and rules | `job_inspect`, `job_monitor` |
| Health | `job_doctor` |
| Session binding and inbox | `job_connect_session`, `job_register_session`, `job_sessions`, `job_probe_session`, `job_recover_session`, `job_invalidate_session`, `job_inbox` |
| Targets and delivery | `job_add_wake_target`, `job_rebind_target`, `job_deliveries`, `job_retry_delivery`, `job_delivery_attempts` |

Tools accept optional `full=true` for complete diagnostic records. In shared-host mode, `job_connect_session` and `wake_me=true` are rejected; pass the actual host caller's binding instead. Web access URLs, daemon shutdown, installation and data maintenance have no corresponding MCP tools. Use [protocol](protocol.md) for schemas, states, byte limits and compatibility details.

## Inspection and log additions (0.1.18+)

`inspect <job-id> --json --compact` uses the same bounded result summary as default MCP `job_inspect`; MCP `full=true` and ordinary CLI JSON retain complete detail. Task details include independent notification diagnostics and original-chat recovery steps. `logs <job-id> --export <new-file>` exports retained UTF-8-lossy text and explicit missing-byte markers, refuses overwrite and is exclusive with `--follow`, `--cursor` and `--limit`. Native runs retain the original 16-MiB prefix plus four recent 1-MiB segments per stream. See [inspection, recovery and retained logs](runtime-monitoring.md).
