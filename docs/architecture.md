# Architecture

One package and executable; CLI, stdio MCP, daemon and runner modes.

CLI/MCP → bounded owner-only Unix socket → daemon → SQLite intent.
Daemon → detached runner → separate workload process group → capped byte logs.
Runner → SQLite final state + event + delivery outbox in one transaction.
Daemon → capability adapter → explicit session endpoint → host acceptance.

SQLite WAL, FULL synchronous, foreign keys, busy timeout, versioned schema.
Each operation runs on a bounded blocking worker and uses its own connection;
short IMMEDIATE write transactions serialize across daemon and runner processes.
No SQLite lock crosses await. Stable UUID job/run/event/target IDs, event sequence
is an SQLite monotonic integer. No external broker, HTTP listener or telemetry.

Job: queued → launching → running → completed/failed/cancelled/timeout/orphaned.
Run records attempt=1, runner identity, workload identity, exit code/signal.
Retrying a submission with the same key and identical spec returns the original;
a changed spec conflicts. Execution is never replayed after launching uncertainty.

Launch intent commits before spawn. Runner takes a per-run OS lock and atomically
claims the launching intent before starting any workload. Duplicate runners lose
the CAS. A daemon crash before/after spawning cannot duplicate execution. A launch
that remains unclaimed after grace becomes orphaned, never automatically retried.
The same lock fences recovery against a delayed runner. PID is diagnostic only;
Linux boot ID + /proc start ticks identify processes. Recovery checks lock ownership,
not kill(pid,0). Each terminal transition atomically inserts one event and outbox.

Sessions bind agent kind, logical session ID, host instance and lease. Targets bind
the immutable registration key; refresh requires the same identity. Rebinding a
new host instance requires an explicit target change, never a latest-session guess.
Delivery: pending → leased → accepted_by_host OR retry_wait → dead_letter.
Expired claims can be reclaimed. Stable event/target IDs provide receiver dedupe;
there is no exactly-once promise. Host ack is not proof of agent execution.

Linux is the verified runtime target. Other OS execution and IPC must fail clearly
until their native security/lifecycle adapters are implemented and tested.

Client-side convenience is in client.rs (ADR 0002). Submissions launch a detached
per-state daemon on demand. Opted-in wake resolves only the CLI/MCP process's
current inherited host identity, registers/refreshes and probes before submitting.
No latest-session lookup or daemon-owned caller inference is used. `daemon stop`
stops dispatch only; runner and result persistence lifetimes remain independent.

Installed entry points (ADR 0003) are handled by deployment.rs and diagnostics.rs.
A private native/plugin bundle carries a persistent state profile, never a chat
identity. The relative terminal launcher retains the profile across cache copies.
The local Codex bundle now uses `.codex-plugin/plugin.json` plus `.mcp.json`, with
a stable absolute installed executable path: the tested Codex CLI 0.164.0 loader
did not discover portable-root MCP or expand plugin-root variables (ADR 0004).
The original installation must remain present for cached MCP. Setup/removal also
owns one local marketplace entry. Shared MCP rejects
implicit inherited identity; the plugin obtains a fresh session key from the
calling chat's terminal and supplies it to job_start explicitly. Runtime/outbox
semantics stay unchanged. Configuration, stdio self-check, actual host tool
connectivity and observed continuation are separate evidence levels.

Linux daemon/Runner spawning uses `/proc/self/exe`, the mapped image, so rebuilding
source or unlinking its pathname cannot break a current daemon's Runner launch.
A peer-derived Doctor field plus a capability flag identifies an affected legacy
daemon before submission; it is not restarted automatically. Updating an install
without a state override retains its receipt's profile. One Rust/MCP core is the
basis for host-specific plugins; Claude Code and OpenCode are future adapters,
not current implementations. Independent OS installers/repositories are cancelled;
official plugin platforms are future work, with no publication during development.

Explicit stop can select the installation receipt via `--install-root`, independently
of source defaults or shell state exports. For early 0.1.0 peers lacking IPC
shutdown, compatibility code retains the original peer identity and uses Linux
pidfds to invoke only that daemon's Ctrl-C handler after revalidation. It waits
for process exit/lock release; install never stops a daemon on its own. The legacy
path requires proc/pidfd support and refuses unknown/renamed peers. Modern IPC
shutdown also waits for lock release before returning success.

Same-chat recovery (recovery.rs) probes a fresh current-terminal registration,
checks the unchanged session snapshot inside a transaction, and scopes old targets
to identical agent/logical/host/caller identities. It transfers only unexpired
unresolved deliveries, retaining IDs/history/TTL; accepted events and live claims
are excluded. New immutable targets and the recovery audit commit atomically.

Automatic bindings opt into delivery-time renewal (renewal.rs). Only an actual
result claim can trigger a same-endpoint owner/fingerprint/capability probe; no
periodic host polling is needed. Near-expired/expired sessions extend only after
an unchanged-snapshot/current-claim check, atomically with a renewal audit.
Invalidation or host replacement requires explicit recovery. Manual/old bindings
remain opted out. Capability preflight rejects old daemons before automatic binding.
