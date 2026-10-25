# Implementation status

Version 0.1.0 under construction, 2026-10-26, Linux x86_64.
Stage 0: specification and reference read; architecture/ADR recorded; Cargo check
passed (Rust 1.99.1; resolved SDK rmcp 3.7.0). Stages 1–4 in progress. No live Agent wake has been verified.
Next: implement durable execution, recovery and behavior tests, then MCP/delivery
and onboarding. Distribution and release automation remain deferred.

Stage 1 increment: implemented detached runner, Linux process groups, bounded
binary-safe capture (UTF-8-lossy read), timeout/cancel, file-lock/CAS recovery,
SQLite schema v1, local length-prefixed owner-only IPC, CLI and pagination.
`cargo test --test runtime -- --test-threads=2`: 7 passed, 0 failed on Linux.
`cargo clippy --all-targets -- -D warnings`: initial style warnings repaired;
strict rerun required at final gate. No live host integration tested.

Stage 2 increment: rmcp 3.7 stdio tools and schemas, explicit leased sessions,
transactional targets/outbox, bounded retry+jitter/TTL/dead letters, attempt
history, CAS-fenced claims, notification inbox, explicit rebind, Codex TOML and
JSON MCP setup/removal with receipts, backups, dry-run and --apply.
`cargo test --all-targets`: 18 passed (1 schema + 5 delivery + 4 setup + 8
runtime), 0 failed. Includes real stdio initialization/discovery/job execution
and MCP process exit while background job completes. `cargo clippy --all-targets
-- -D warnings`: passed. Private Desktop transport is not enabled yet.

Stage 3 increment: Codex CLI notification inbox; explicit session/target/delivery
CLI commands; experimental Desktop AF_UNIX JSON-RPC adapter with owner credential
check, endpoint inode/ctime binding, capability preflight, required callerSource
and caller/destination IDs, bounded fragmented frames, shared sequence deadline,
no fallback, acceptance-only outcome. Schema v3 adds endpoint instance identity.
`cargo test --all-targets`: 22 passed, 0 failed (Desktop 4 simulated tests).
`cargo clippy --all-targets -- -D warnings`: passed. Desktop live E2E and public
Codex app-server writer integration NOT verified. Windows pipe/Job Objects and
macOS runtime remain unimplemented; Stage 3 is a scoped Linux milestone only.

Reliability review increment: output write failures terminate the workload;
ordinary supervision errors have group cleanup guards; Linux runner acts as a
subreaper and reaps adopted descendants; bounded rmcp input lines (128 KiB);
fixed repeat initialization of schema v3; reject symlink databases; recheck
Desktop endpoint identity after connecting; target idempotence at quota;
CLI log following and optional integration diagnostics; two Tokio workers per
process. Added recovery/outbox restart, stale PID, quota, write failure and
secret non-echo regressions. Latest suite: 29 passed, 0 failed on Linux.
