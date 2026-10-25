# Reference audit (2026-10-26)

Read-only reference: `/data/Projects/Agents/vanth-fork`, HEAD
`ea5333e1d23076c3e1a2707e2c4b68b60794a43f`. Inspected README, Apache-2.0
LICENSE, daemon.py, runner.py, server.py, codex_pipe.py, commit ea5333e,
stop HTTP and delivery/runner hardening tests. No reference files modified.

The local fork includes Linux AF_UNIX connection handling (a Unix socket is not
an ordinary file), `callerSource: codex`, separate caller/destination thread IDs,
MCP-independent daemon session creation, and job_stop idempotency parameter
regressions. These are observed local fixes, not assumptions about upstream.

Borrowed behavioral requirements: bound frames and logs; deadline shorter than
claim lease; preserve execution after MCP cleanup; never route a failed Desktop
wake into another CLI thread; replay with stable delivery IDs; cancellation must
cross the actual public boundary. No implementation fragments copied.

Avoid inheriting artifact registries, SSH beta, metrics, broad schedulers,
HTTP command execution and Python/Go split. New architecture: Rust modular
monolith; explicit argv vs shell; owner-only local socket; SQLite job/run/session
entities; independent runner owning execution lock and writing final transaction
directly even while daemon is absent. Four lifetimes remain separate: workload,
MCP connection, agent session registration, durable delivery.

Official SDK README/API inspected from downloaded rmcp 3.7.0 (transport-io,
ServerHandler); crate resolution/build validates versions in Cargo.lock.
Queried tokio 1.54.1, clap 4.6.7, rusqlite 0.41.0, serde 1.0.229,
tracing 0.1.44, rustix 1.1.5, windows-sys 0.61.2, interprocess 2.4.7,
thiserror 2.0.21. Tokio native Unix sockets suffice; interprocess is unnecessary
for the Linux milestone. Narrow libc calls are confined to platform code.
anyhow supplies contextual errors; no unused error framework is added.

Public Codex documentation checked:
https://learn.chatgpt.com/docs/app-server (via official developers.openai.com).
`thread/resume` loads stored context; `turn/start` begins generation;
`turn/steer` requires the active turn identity. Owning a new app-server is not
proof of permission or ability to inject into another Desktop/CLI writer.
v0.1 therefore uses notification-only explicit registrations for Codex CLI,
plus an opt-in experimental Desktop transport. No automatic second writer,
credential reading, implicit session discovery or claim of live E2E.

License: independent original implementation under MIT. Reference is Apache-2.0;
no reference code, assets or packaging copied. Behavioral observations and
independently written tests do not relicense the reference repository.
