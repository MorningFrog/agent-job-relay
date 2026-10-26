<div align="center"> 
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

English | [简体中文](README.zh-CN.md)

Run durable background commands and route completion events to explicitly registered AI agent sessions.

持久执行后台命令, 将完成事件可靠地路由到明确注册的 AI Agent 会话.
</div>

A native Rust CLI, independent runner and stdio MCP server for one trusted local user. **v0.1.4 is an unreleased development milestone, tested on Linux x86_64.** Delivery is moving to Agent plugins. Independent OS installers and package repositories are cancelled; no official plugin publication is planned during development.

## What works today

| Capability | Current evidence |
|---|---|
| Explicit argv or opt-in shell, cwd, timeout and idempotent submission | Linux process tests |
| Independent daemon/runner, SQLite jobs/runs/events, bounded stdout/stderr | Linux process tests, including daemon crash/restart |
| Process-group cancellation and adopted child reaping | Linux tests; escaping daemon processes are outside containment |
| Standard stdio MCP tools; client exit does not stop jobs | Actual MCP handshake, discovery and execution tests |
| Sessions, leases, exact routing, transactional outbox, retry/TTL/dead letters | Automated persistence and failure tests |
| Codex CLI integration | MCP configuration and notification inbox; **no automatic resume** |
| Desktop wake | Experimental Unix socket; fake-host tests and user-confirmed explicit-registration/CLI same-chat wake |
| Setup, doctor and owned MCP removal | Isolated configuration tests; no real user settings changed |
| Local Desktop Plugin + MCP | Source 0.1.4: Codex CLI 0.164.0 loader discovers 20 tools in isolation; installed MCP task/wake and plugin-browser evidence pending |
| Plugin runtime management | Offline native bundle, profile-preserving update, explicit cache inspection/refresh; recipients need no Rust compiler |
| Windows/macOS runtime | **Not implemented/verified**; requests fail explicitly |

Job completion, `notification_recorded`, `accepted_by_host` and observed Agent execution are distinct. No adapter automatically confirms resume; Desktop `can_resume` describes the request mechanism. On ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04, the user confirmed explicit-registration and CLI shortcut same-chat wake, including a quiet control chat and idle-daemon restart. An installed skill/CLI fallback task also resumed, and v0.1.2 MCP connectivity was user-confirmed without a new reviewed transcript. These results do not qualify installed MCP task/wake, Desktop restart/rebind or browser installation; see [live evidence](docs/live-e2e.md).

## Install once, use from chat

Build locally, then install the native program and Desktop plugin to a stable private location. The following is an explicit **personal installation**; commands without `--apply` only preview. Development tests use isolated `.tmp/` paths.

```bash
cargo build --release --locked
AJR_INSTALL_ROOT="$HOME/.local/share/agent-job-relay"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Restart Desktop, open Plugins and select your personal local marketplace (`AgentJobRelay Local` if newly created), then install AgentJobRelay. Start a new local chat, select the plugin if needed, and ask:

> Run this project's tests in the background and tell me when they finish.

Everyday prompts need no tool path, state directory or session ID. The skill binds the current chat, submits through MCP when available and explicitly reports CLI fallback. It uses the chosen project's working directory, ends the turn after submission and reads results on completion.

The local bundle uses `.codex-plugin/plugin.json` and `.mcp.json`. Source 0.1.4 includes `job_recover_session` among its 20 tools; updating source does not update a personal install or cache. `doctor --check-mcp` checks local stdio, while an actual host `job_doctor` call checks MCP connectivity. See [installation](docs/installation.md).

Installation leaves PATH and services unchanged. State is separate from binaries and pinned across plugin-cache copies; exported AJR_STATE_DIR is ignored by the plugin, while explicit CLI `--state-dir` can override it. The default root is `$XDG_DATA_HOME/agent-job-relay` or the path above; use a HOME-contained root for a personal marketplace. Cached MCP and terminal launchers both use the managed native installation, which must remain present.

For a standalone MCP configuration, use the installed launcher's `setup codex` instead of `setup plugin`. The server advertises the same terminal-binding workflow. Avoid registering both routes for the same host/state unnecessarily.

To provide a compatible Linux user with an offline bundle, build once and export it locally:

```bash
AJR_BUNDLE="$PWD/.tmp/agent-job-relay-offline"
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --apply
"$AJR_BUNDLE/relay" plugin bootstrap --dry-run
"$AJR_BUNDLE/relay" plugin bootstrap --apply
```

The received directory can be moved before bootstrap; its `relay plugin bootstrap` installs the native runtime and owned local marketplace entry without Rust, Python or Node. It validates file hashes and the OS/architecture/ABI label, uses the recipient's state profile and preserves existing history. Use a trusted bundle: local hashes check integrity, not publisher identity. The export is a local development artifact, not a directly installable marketplace ZIP or published release.

## Source-only development quick start

Requires Rust stable (package MSRV 1.89), Cargo and a C toolchain for bundled SQLite. Runtime needs no Python, Node, Go, Docker or external database. From the repository root:

```bash
cargo build --release --locked
export AJR_STATE_DIR="$PWD/.tmp/demo/state"
./target/release/agent-job-relay start -- /bin/echo "hello from the runner"
./target/release/agent-job-relay doctor
```

**The first submission automatically starts a detached daemon.** No extra terminal or OS service is required. The response contains job.job_id and job.run_id. Set JOB_ID to the returned job ID, then inspect it:

```bash
./target/release/agent-job-relay status "$JOB_ID"
./target/release/agent-job-relay logs "$JOB_ID"
./target/release/agent-job-relay events "$JOB_ID"
```

Inside a Desktop Agent's terminal tool, opt into same-session wake with one flag:

```bash
./target/release/agent-job-relay start --wake -- cargo check
```

This reads only the current process's inherited CODEX_THREAD_ID and CODEX_APP_TOOLS_PIPE_PATH, registers/refreshes that exact session, probes the host, and binds the job before execution. No manually copied thread ID, socket or key is needed. `--wake` explicitly opts into the experimental Desktop adapter. If identity or capability is unavailable, the job is not started and the error explains how to use the correct host context; there is no inbox/session fallback.

```bash
./target/release/agent-job-relay session connect  # optional connection diagnostic
./target/release/agent-job-relay daemon stop     # leaves runners and data intact
```

`daemon` still runs in the foreground for diagnosis; Ctrl-C leaves runners alive. A subsequent submission restarts a stopped daemon. No login autostart is installed. Without an override, state defaults to `$XDG_STATE_HOME/agent-job-relay` or `$HOME/.local/state/agent-job-relay`. `--state-dir /absolute/path` overrides it. CLI and MCP must use the same directory. Existing state must be owned by you with mode 0700; keep socket paths shorter than 104 bytes.

## CLI examples

```bash
./target/release/agent-job-relay start --timeout 60 --idempotency-key build-001 -- cargo check
./target/release/agent-job-relay start --shell 'printf "out\n"; printf "err\n" >&2'
./target/release/agent-job-relay logs "$JOB_ID" --stream stderr --cursor 0 --limit 8192
./target/release/agent-job-relay logs "$JOB_ID" --follow
./target/release/agent-job-relay wait "$JOB_ID" --after 0 --seconds 10
./target/release/agent-job-relay stop "$JOB_ID"
```

`--` separates CLI options from executable arguments; shell execution is always explicit. CLI resolves omitted cwd to your current directory; MCP requires an absolute cwd. Jobs inherit the **daemon's environment**, with explicit MCP/request `env` overrides if needed. Values of overrides are persisted privately, not echoed.

Lists/events/deliveries return `next_cursor`; pass it as `--after`. Log cursors are byte offsets, not character counts. Log queries use lossy UTF-8; original bytes are retained up to 16 MiB per stream, then drained/discarded. `--follow` is a human viewer; Ctrl-C stops the viewer only. Wait returns new events (including `started`), a terminal state or the deadline; it may return before completion. For long tasks, use a capable target or inspect later; avoid Agent polling loops.

Eight active jobs are allowed. Reusing an idempotency key with identical settings returns the original job; different settings conflict. An intentional rerun needs a new key. `stop` is idempotent; a completed job stays completed.

Temporary notification failures retry with bounded backoff until the 24-hour event TTL. Identity/lease failures requiring recovery pause; unsupported host capabilities stop immediately. `delivery list` exposes `failure_kind` and `next_attempt_at`; `delivery retry` preserves the ID/history and never reruns a job.

## Recover notifications after Desktop restart

From the same chat's terminal, use the installed launcher:

```bash
"$AJR_INSTALL_ROOT/relay" session recover
"$AJR_INSTALL_ROOT/relay" session recover --apply
```

The first command refreshes/probes the current binding and previews eligible jobs; it does not move targets or send a chat message. `--apply` restores only that logical chat/caller/host, preserving original delivery IDs, expiry and attempt history. Accepted or expired events are excluded; in-flight attempts are skipped. Commands are never rerun. Responses include a bounded recovery audit and `next_cursor`; use `--after` for more pages. Shared MCP offers `job_recover_session(session_key, apply)` using a fresh current-terminal binding. Preview/apply requires the updated runtime; older live daemons are refused before a wake-dependent submission, with explicit same-profile update guidance. This has isolated installed CLI/MCP and fake-host coverage; real Desktop restart and continuation remain unverified. Private host identity is trusted caller input.

## MCP setup and safe removal

Preview is the default. `--apply` explicitly authorizes a write. Start with isolated files; these commands do not register your real Codex configuration:

```bash
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --apply
./target/release/agent-job-relay doctor --client codex --config "$PWD/.tmp/demo/codex.toml"
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --apply
```

For a real Codex client, choose its actual `--config` path, or omit the option to use `$CODEX_HOME/config.toml` / `$HOME/.codex/config.toml`. Review `setup codex --dry-run` before choosing `--apply`. Restart the MCP client after applying. The generated entry points to the **current executable** and state directory; use the installed launcher so these stay stable. Setup does not install a service; task submissions start the daemon automatically.

Generic clients using `mcpServers` JSON can use:

```bash
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --dry-run
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --apply
./target/release/agent-job-relay integration remove json --config "$PWD/.tmp/demo/mcp.json" --apply
```

No ChatGPT Desktop configuration location or unsupported format is guessed. For a client with manual configuration, the generated entry has this shape:

```json
{
  "command": "/absolute/path/to/agent-job-relay",
  "args": ["--state-dir", "/absolute/private/state", "mcp", "--shared-host"]
}
```

Only the owned `agent-job-relay` entry is changed. Private adjacent ownership receipts and timestamp-independent unique backups support review and rollback. Foreign entries are preserved; modified/unowned entries are refused. Backups can contain secrets: keep them private. Restore a backup only after reviewing newer client changes. Integration removal retains all job history/logs/config state. Config and receipt writes are individually atomic; interrupted setup fails closed and may require reviewing the receipt and backup.

`agent-job-relay mcp` serves stdout exclusively as MCP; diagnostics use stderr. Tools: `job_start/status/list/logs/events/stop/wait/doctor`, automatic connection, session registration, probe/invalidation/inbox/recovery, target add/rebind, delivery list/attempts/retry. `job_start` takes `{"spec":{"command":{"kind":"exec","program":"/bin/echo", "args":["hello"]},"cwd":"/absolute/workdir"}}`. See [protocol](docs/protocol.md) for exact schemas and bounded results.

## Sessions and wake behavior

Installed plugins and generated MCP configurations use `mcp --shared-host`. The Agent runs the bundled launcher with `session connect --brief` from the **current chat's terminal tool**, then uses its fresh `session_key` in the MCP submission. This happens within the plugin workflow; the user copies no IDs.

```json
{
  "spec": {
    "command": { "kind": "exec", "program": "cargo", "args": ["check"] },
    "cwd": "/absolute/workdir",
    "session": "<fresh key returned to the Agent by the terminal binding>"
  }
}
```

Shared mode rejects `wake_me=true` and `job_connect_session`: its environment cannot establish which chat called it. The binding command probes actual current-terminal context before any wake-dependent workload is submitted. If MCP is unavailable, the skill can use its installed launcher with `start --wake` in that same terminal. Context failure is explicit; no other session is selected.

For an explicitly dedicated per-session MCP process, manual `mcp` without `--shared-host` retains `wake_me=true` / `job_connect_session` using its inherited context. Do not combine wake_me with spec.session (`--wake` and `--session` also conflict). This advanced path is not the default installed plugin workflow.

Automatic registration uses a 24-hour lease and a socket fingerprint as host instance identity. Each wake-enabled submission refreshes it; a replacement endpoint produces a new binding without moving existing jobs. Automatic bindings opt into renewal when a result is due: near-expired/expired leases renew only after a same-endpoint owner/fingerprint and capability probe. No periodic host polling is added. Changed endpoints and invalidated registrations need explicit recovery; manual registrations default to no renewal (`--renew-on-delivery` opts in for an owned Desktop caller/destination). A host acknowledgement still does not confirm Agent execution.

Advanced manual registration remains available for other hosts and diagnosis. For a local notification demonstration, register explicit identities:

```bash
./target/release/agent-job-relay session register --agent codex_cli --session-id demo-thread --instance-id demo-instance --host-id local
```

Use the returned `session.key` as `SESSION_KEY` (it is different from logical session_id). For a real integration, supply the actual host/thread identities:

```bash
./target/release/agent-job-relay session probe "$SESSION_KEY"
./target/release/agent-job-relay start --session "$SESSION_KEY" -- /bin/echo notified
./target/release/agent-job-relay inbox "$SESSION_KEY"
./target/release/agent-job-relay delivery list --job-id "$JOB_ID"
./target/release/agent-job-relay delivery attempts "$DELIVERY_ID"
./target/release/agent-job-relay delivery retry "$DELIVERY_ID"
```

The inbox is durable notification storage, **not automatic Codex resume**. Registrations expire after one hour by default (`--lease-seconds`, max 86400). Repeat registration with the same explicit identities to refresh. New host instances get different keys; existing jobs never follow the latest session. Use `target add JOB_ID SESSION_KEY --replay` for explicit terminal-event replay, or `target rebind TARGET_ID NEW_SESSION_KEY` after host replacement.

Desktop registration additionally requires an explicitly supplied `--endpoint`, `--caller-thread-id` and `--experimental`. Use only identities/endpoints you are authorized to control. The private API may change; Windows named pipes are not implemented. [Manual E2E procedure](docs/testing.md) describes validation without claiming that a fake host proves live Agent execution. A failed Desktop delivery never opens another CLI session. Stable delivery IDs support receiver deduplication; lost acknowledgements can still cause repeated host effects.

## Recovery, safety and uninstall

For the managed local install, first remove/disable the plugin through Desktop and preview/apply `relay integration remove plugin` (or codex/json for standalone MCP). Inspect active jobs, finish/cancel them as appropriate, then `relay daemon stop`. Preview/apply `relay uninstall --root <same-install-root>` removes only unchanged owned install files. Task data and backups remain; host-managed plugin cache removal belongs to the host. Modified or unknown install contents are refused.

SQLite records execution intent before launch. Each runner holds an exclusive run lock and claims once; uncertain launches become `orphaned` rather than being silently repeated. Runners persist terminal state/events/outbox while the daemon is down. A missing runner/boot identity is not success. Machine shutdown stops processes; state survives for classification, not execution across power loss.

Commands execute with your OS permissions. No sandbox, CPU/memory quota, secret masking or containment of deliberately escaped process groups is provided. Do not feed untrusted output back as higher-priority Agent instructions. [Security](docs/security.md) and [limitations](docs/limitations.md) describe the remaining failure boundaries.

Optional isolated development installation (no global PATH changes):

```bash
cargo install --path . --locked --root "$PWD/.tmp/install"
./.tmp/install/bin/agent-job-relay --version
cargo uninstall --root "$PWD/.tmp/install" agent-job-relay
```

Before removing/replacing binaries, inspect running jobs, stop them if desired, and run `agent-job-relay daemon stop` for the same state directory (or stop a foreground daemon with Ctrl-C). Removing software, removing MCP integration, removing a service and deleting user data are different operations. For a binary you manually copied, remove only that binary after checking processes. **Normal removal preserves state.** There is no `data purge` command or system-package uninstaller; do not delete unknown directories or another project's data. Plugin updates and removal must preserve history/logs/user config by default.

When updating at the same install root without a state override, the existing state profile is retained. Finish active jobs, rebuild source, stop that state's daemon, repeat install and refresh the host's cached plugin. The updated source executable can stop the correct daemon directly from the installation profile, regardless of an unrelated exported AJR_STATE_DIR:

```bash
cargo build --release --locked
./target/release/agent-job-relay daemon stop --install-root "$AJR_INSTALL_ROOT"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
```

The stop command waits for the daemon lock to be released. For a verified early 0.1.0 daemon that rejects `shutdown`, it uses that exact process's existing Ctrl-C handler through a Linux pidfd and reports `method=legacy_sigint`. It never signals a process group or force-kills a process. Install itself does not stop daemons. Unset unrelated AJR_STATE_DIR exports before updating; an explicit install state override intentionally selects another profile. Current Linux daemons launch runners from their loaded image even after a source rebuild. An already-running legacy daemon with a removed executable still needs an explicit idle restart; it is never silently restarted or given a new state.

For later offline updates, use `relay plugin update --from "$AJR_BUNDLE" --root "$AJR_INSTALL_ROOT" --dry-run`, then `--apply` after the same-profile runtime is idle. This path always retains the recorded state, rejects downgrades and does not stop daemons. Add repeated `--cache <absolute-cache-directory>` arguments to preflight and refresh selected snapshots. `plugin status --root ... --cache ...` reports staleness; `plugin cache refresh --root ... --path ... --apply` refreshes separately. Modified/foreign caches are refused, backups retained, and legacy caches without the new runtime profile must first be refreshed through the host. No cache scan or deletion occurs; reconnect MCP after updating.

## Troubleshooting and development

- **Daemon unavailable:** submissions auto-start it using the same state root. `doctor` intentionally checks without starting it. For startup failures run foreground `daemon` to see diagnostics; no OS service is installed.
- **MCP missing:** update the local bundle, refresh the cached plugin and restart the client. A loaded skill alone does not prove MCP loaded. The repaired Codex manifest is tested against CLI 0.164.0; `doctor --check-mcp` is a local self-check, and `doctor --client ...` checks owned config. Neither proves Desktop connectivity.
- **Runner restart required / missing executable:** `doctor` detects an affected legacy daemon. Let active jobs finish, then explicitly stop the same state's daemon and resubmit with the updated runtime. Preserve the failed job; changing state directories bypasses the issue rather than repairing it.
- **No Agent wake:** the plugin binds through the current terminal with `session connect --brief`, then passes spec.session to shared MCP. CLI `start --wake` uses current-terminal context; wake_me=true is only for per-session MCP. For existing jobs inspect session lease, deliveries and attempts. Inbox has no resume capability; Desktop acceptance is not confirmed execution.
- **orphaned:** inspect external effects before deliberately submitting a new job. No automatic replay is performed. Escaped grandchildren may need manual review.
- **Permission/socket error:** choose an owned private state directory with 0700 permissions and a shorter path. Do not loosen it or reuse Vanth data.
- **Config removal refused:** the entry was changed or has no ownership receipt; review it and the backup. Other MCP entries and job data remain intact.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

Tests create isolated state under `.tmp/`, exercise real processes on Linux, and simulate private Desktop transport. CI defines Linux full tests plus Windows/macOS compile/library checks; remote CI and those platforms are not locally verified. See [architecture](docs/architecture.md), [testing](docs/testing.md), [implementation status](docs/IMPLEMENTATION_STATUS.md), [reference audit](docs/reference-audit.md) and [changelog](CHANGELOG.md).

## Plugin roadmap

Use one Rust/MCP core with host-specific plugins. Codex/Desktop has a local Linux development bundle with offline bootstrap/update and explicit cache refresh. Automatic download, host-driven bootstrap, Claude Code/OpenCode and task output monitoring/interruption remain future work. Output monitoring is recorded only: current log reads/follow and manual stop do not implement it.

Future delivery should use each Agent's officially supported plugin platform or installation mechanism. The project remains in development: **no platform submission, package upload or public release now**. Independent OS installers, package repositories and their publication pipelines are cancelled. Native runtime artifacts inside plugins may still be needed per OS/architecture. See the [Plugin roadmap](docs/TODO.md) for actual support and future work.

MIT licensed. Independently implemented; Vanth was studied read-only for behavioral requirements, not copied. No remote repository was created or pushed by this work.
