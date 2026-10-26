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
identity. The relative launcher and `${PLUGIN_ROOT}` manifest survive plugin-cache
copying. Setup/removal also owns one local marketplace entry. Shared MCP rejects
implicit inherited identity; the plugin obtains a fresh session key from the
calling chat's terminal and supplies it to job_start explicitly. Runtime/outbox
semantics stay unchanged. Configuration, stdio self-check, actual host tool
connectivity and observed continuation are separate evidence levels.
