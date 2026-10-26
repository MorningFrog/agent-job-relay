<div align="center"> 
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

English | [简体中文](README.zh-CN.md)

Run durable background commands and route completion events to explicitly registered AI agent sessions.

持久执行后台命令, 将完成事件可靠地路由到明确注册的 AI Agent 会话.
</div>

A native Rust CLI, independent runner and stdio MCP server for one trusted local
user. **v0.1.0 is a source-build development milestone, tested on Linux x86_64.**
No installer or package-manager release is available.

## What works today

| Capability | Current evidence |
|---|---|
| Explicit argv or opt-in shell, cwd, timeout and idempotent submission | Linux process tests |
| Independent daemon/runner, SQLite jobs/runs/events, bounded stdout/stderr | Linux process tests, including daemon crash/restart |
| Process-group cancellation and adopted child reaping | Linux tests; escaping daemon processes are outside containment |
| Standard stdio MCP tools; client exit does not stop jobs | Actual MCP handshake, discovery and execution tests |
| Sessions, leases, exact routing, transactional outbox, retry/TTL/dead letters | Automated persistence and failure tests |
| Codex CLI integration | MCP configuration and notification inbox; **no automatic resume** |
| Desktop wake | Experimental Unix socket adapter; automated fake-host tests and **user-confirmed basic live E2E** on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04 |
| Setup, doctor and owned MCP removal | Isolated configuration tests; no real user settings changed |
| Windows/macOS runtime | **Not implemented/verified**; requests fail explicitly |

Job completion, `notification_recorded`, `accepted_by_host` and observed Agent
execution are distinct. No current adapter automatically confirms that an Agent resumed.
A configured Desktop `can_resume` capability describes the request mechanism;
it is not live E2E evidence. The user-confirmed basic live test and artifact checks
are recorded in [live E2E evidence](docs/live-e2e.md); restart/rebind scenarios
remain outside that result.

## One-minute source quick start

Requires Rust stable (package MSRV 1.89), Cargo and a C toolchain for bundled
SQLite. Runtime needs no Python, Node, Go, Docker or external database.
From the repository root:

```bash
cargo build --release --locked
export AJR_STATE_DIR="$PWD/.tmp/demo/state"
./target/release/agent-job-relay start -- /bin/echo "hello from the runner"
./target/release/agent-job-relay doctor
```

**The first submission automatically starts a detached daemon.** No extra terminal
or OS service is required. The response contains job.job_id and job.run_id. Set
JOB_ID to the returned job ID, then inspect it:

```bash
./target/release/agent-job-relay status "$JOB_ID"
./target/release/agent-job-relay logs "$JOB_ID"
./target/release/agent-job-relay events "$JOB_ID"
```

Inside a Desktop Agent's terminal tool, opt into same-session wake with one flag:

```bash
./target/release/agent-job-relay start --wake -- cargo check
```

This reads only the current process's inherited CODEX_THREAD_ID and
CODEX_APP_TOOLS_PIPE_PATH, registers/refreshes that exact session, probes the host,
and binds the job before execution. No manually copied thread ID, socket or key
is needed. `--wake` explicitly opts into the experimental Desktop adapter.
If identity or capability is unavailable, the job is not started and the error
explains how to use the correct host context; there is no inbox/session fallback.

```bash
./target/release/agent-job-relay session connect  # optional connection diagnostic
./target/release/agent-job-relay daemon stop     # leaves runners and data intact
```

`daemon` still runs in the foreground for diagnosis; Ctrl-C leaves runners alive.
A subsequent submission restarts a stopped daemon. No login autostart is installed.
Without an override, state defaults to `$XDG_STATE_HOME/agent-job-relay` or
`$HOME/.local/state/agent-job-relay`. `--state-dir /absolute/path` overrides it.
CLI and MCP must use the same directory. Existing state must be owned by you
with mode 0700; keep socket paths shorter than 104 bytes.

## CLI examples

```bash
./target/release/agent-job-relay start --timeout 60 --idempotency-key build-001 -- cargo check
./target/release/agent-job-relay start --shell 'printf "out\n"; printf "err\n" >&2'
./target/release/agent-job-relay logs "$JOB_ID" --stream stderr --cursor 0 --limit 8192
./target/release/agent-job-relay logs "$JOB_ID" --follow
./target/release/agent-job-relay wait "$JOB_ID" --after 0 --seconds 10
./target/release/agent-job-relay stop "$JOB_ID"
```

`--` separates CLI options from executable arguments; shell execution is always
explicit. CLI resolves omitted cwd to your current directory; MCP requires an
absolute cwd. Jobs inherit the **daemon's environment**, with explicit MCP/request
`env` overrides if needed. Values of overrides are persisted privately, not echoed.

Lists/events/deliveries return `next_cursor`; pass it as `--after`. Log cursors
are byte offsets, not character counts. Log queries use lossy UTF-8; original
bytes are retained up to 16 MiB per stream, then drained/discarded. `--follow`
is a human viewer; Ctrl-C stops the viewer only. Wait returns new events (including
`started`), a terminal state or the deadline; it may return before completion.
For long tasks, use a capable target or inspect later; avoid Agent polling loops.

Eight active jobs are allowed. Reusing an idempotency key with identical settings
returns the original job; different settings conflict. An intentional rerun needs
a new key. `stop` is idempotent; a completed job stays completed.

## MCP setup and safe removal

Preview is the default. `--apply` explicitly authorizes a write. Start with
isolated files; these commands do not register your real Codex configuration:

```bash
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --apply
./target/release/agent-job-relay doctor --client codex --config "$PWD/.tmp/demo/codex.toml"
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --apply
```

For a real Codex client, choose its actual `--config` path, or omit the option to
use `$CODEX_HOME/config.toml` / `$HOME/.codex/config.toml`. Review `setup codex
--dry-run` before choosing `--apply`. Restart the MCP client after applying.
The generated entry points to the **current executable** and state directory;
re-run setup after moving the binary. Setup does not install a service; task
submissions start the daemon automatically.

Generic clients using `mcpServers` JSON can use:

```bash
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --dry-run
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --apply
./target/release/agent-job-relay integration remove json --config "$PWD/.tmp/demo/mcp.json" --apply
```

No ChatGPT Desktop configuration location or unsupported format is guessed.
For a client with manual configuration, the generated entry has this shape:

```json
{
  "command": "/absolute/path/to/agent-job-relay",
  "args": ["--state-dir", "/absolute/private/state", "mcp"]
}
```

Only the owned `agent-job-relay` entry is changed. Private adjacent ownership
receipts and timestamp-independent unique backups support review and rollback.
Foreign entries are preserved; modified/unowned entries are refused. Backups can
contain secrets: keep them private. Restore a backup only after reviewing newer
client changes. Integration removal retains all job history/logs/config state.
Config and receipt writes are individually atomic; interrupted setup fails closed
and may require reviewing the receipt and backup.

`agent-job-relay mcp` serves stdout exclusively as MCP; diagnostics use stderr.
Tools: `job_start/status/list/logs/events/stop/wait/doctor`, automatic connection, session registration,
probe/invalidation/inbox, target add/rebind, delivery list/attempts/retry.
`job_start` takes `{"spec":{"command":{"kind":"exec","program":"/bin/echo",
"args":["hello"]},"cwd":"/absolute/workdir"}}`. See [protocol](docs/protocol.md)
for exact schemas and bounded results.

## Sessions and wake behavior

For normal MCP use, enable wake directly in the start call:

```json
{
  "wake_me": true,
  "spec": {
    "command": { "kind": "exec", "program": "cargo", "args": ["check"] },
    "cwd": "/absolute/workdir"
  }
}
```

`job_start` auto-starts the daemon; wake_me=true also registers, refreshes and
probes the MCP process's inherited Desktop context. Then the Agent can end its
turn and wait for the completion message. `job_connect_session` is an optional
no-argument diagnostic; routine starts do not require it. Do not combine wake_me
with explicit spec.session (`--wake` and `--session` also conflict).

Automatic binding requires a per-session MCP process that actually receives the
two host variables. A shared MCP process cannot infer which chat called it. If
those variables are absent, let the Agent use `start --wake` from its current
Desktop terminal tool, or explicitly supply an authorized spec.session. No
human needs to copy IDs in the normal terminal-tool path.

Automatic registration uses a 24-hour lease and a socket fingerprint as host
instance identity. Each wake-enabled submission refreshes it; a replacement
endpoint produces a new binding without moving existing jobs. Jobs exceeding
that lease need explicit refresh/rebind. A host acknowledgement still does not
confirm Agent execution. The new shortcuts have automated fake-host coverage;
the separately recorded live pass used the explicit registration procedure.

Advanced manual registration remains available for other hosts and diagnosis.
For a local notification demonstration, register explicit identities:

```bash
./target/release/agent-job-relay session register --agent codex_cli --session-id demo-thread --instance-id demo-instance --host-id local
```

Use the returned `session.key` as `SESSION_KEY` (it is different from logical
session_id). For a real integration, supply the actual host/thread identities:

```bash
./target/release/agent-job-relay session probe "$SESSION_KEY"
./target/release/agent-job-relay start --session "$SESSION_KEY" -- /bin/echo notified
./target/release/agent-job-relay inbox "$SESSION_KEY"
./target/release/agent-job-relay delivery list --job-id "$JOB_ID"
./target/release/agent-job-relay delivery attempts "$DELIVERY_ID"
./target/release/agent-job-relay delivery retry "$DELIVERY_ID"
```

The inbox is durable notification storage, **not automatic Codex resume**.
Registrations expire after one hour by default (`--lease-seconds`, max 86400).
Repeat registration with the same explicit identities to refresh. New host
instances get different keys; existing jobs never follow the latest session.
Use `target add JOB_ID SESSION_KEY --replay` for explicit terminal-event replay,
or `target rebind TARGET_ID NEW_SESSION_KEY` after host replacement.

Desktop registration additionally requires an explicitly supplied `--endpoint`,
`--caller-thread-id` and `--experimental`. Use only identities/endpoints you are
authorized to control. The private API may change; Windows named pipes are not
implemented. [Manual E2E procedure](docs/testing.md) describes validation without
claiming that a fake host proves live Agent execution. A failed Desktop delivery
never opens another CLI session. Stable delivery IDs support receiver deduplication;
lost acknowledgements can still cause repeated host effects.

## Recovery, safety and uninstall

SQLite records execution intent before launch. Each runner holds an exclusive
run lock and claims once; uncertain launches become `orphaned` rather than being
silently repeated. Runners persist terminal state/events/outbox while the daemon
is down. A missing runner/boot identity is not success. Machine shutdown stops
processes; state survives for classification, not execution across power loss.

Commands execute with your OS permissions. No sandbox, CPU/memory quota, secret
masking or containment of deliberately escaped process groups is provided.
Do not feed untrusted output back as higher-priority Agent instructions.
[Security](docs/security.md) and [limitations](docs/limitations.md) describe the
remaining failure boundaries.

Optional isolated development installation (no global PATH changes):

```bash
cargo install --path . --locked --root "$PWD/.tmp/install"
./.tmp/install/bin/agent-job-relay --version
cargo uninstall --root "$PWD/.tmp/install" agent-job-relay
```

Before removing/replacing binaries, inspect running jobs, stop them if desired,
and run `agent-job-relay daemon stop` for the same state directory (or stop a
foreground daemon with Ctrl-C). Removing software, removing MCP integration,
removing a service and deleting user data are different operations. For a binary
you manually copied, remove only that binary after checking processes. **Normal
removal preserves state.** There is no `data purge` command or system uninstaller;
do not delete unknown directories or another project's data. Future installers
must also preserve history/logs/user config by default.

## Troubleshooting and development

- **Daemon unavailable:** submissions auto-start it using the same state root.
  `doctor` intentionally checks without starting it. For startup failures run
  foreground `daemon` to see diagnostics; no OS service is installed.
- **MCP missing:** preview/apply the correct client config and restart the client.
  `doctor --client codex --config ...` checks owned config, not a live MCP handshake.
- **No Agent wake:** use `start --wake` / wake_me=true in the current Desktop
  context; `session connect` probes it. For existing jobs inspect session lease,
  deliveries and attempts.
  Inbox has no resume capability; Desktop acceptance is not confirmed execution.
- **orphaned:** inspect external effects before deliberately submitting a new job.
  No automatic replay is performed. Escaped grandchildren may need manual review.
- **Permission/socket error:** choose an owned private state directory with 0700
  permissions and a shorter path. Do not loosen it or reuse Vanth data.
- **Config removal refused:** the entry was changed or has no ownership receipt;
  review it and the backup. Other MCP entries and job data remain intact.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

Tests create isolated state under `.tmp/`, exercise real processes on Linux,
and simulate private Desktop transport. CI defines Linux full tests plus
Windows/macOS compile/library checks; remote CI and those platforms are not
locally verified. See [architecture](docs/architecture.md),
[testing](docs/testing.md), [implementation status](docs/IMPLEMENTATION_STATUS.md),
[reference audit](docs/reference-audit.md) and [changelog](CHANGELOG.md).

## Roadmap / Deferred distribution

`.deb`/APT, `.rpm`/DNF, `.AppImage`, Windows `.exe`/WinGet, macOS `.dmg`/Homebrew,
Flatpak/Flathub, GitHub Releases binaries, optional PyPI wheels/`uv tool`, signing,
notarization and GitHub Actions package publication are **planned, not shipped**.
No fabricated install links or published package commands are provided.
[Distribution & Packaging TODO](docs/TODO.md) specifies priorities, lifecycle
semantics, constraints, tests and separately authorized repository publication.

MIT licensed. Independently implemented; Vanth was studied read-only for behavioral
requirements, not copied. No remote repository was created or pushed by this work.
