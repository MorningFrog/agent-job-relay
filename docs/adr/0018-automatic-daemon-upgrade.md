# ADR 0018: use the new daemon after runtime updates

Accepted, 2026-10-29. Supersedes the manual-only daemon upgrade policy in ADRs 0002, 0005 and 0010; their identity, delivery and installation safety constraints remain.

## Problem

Installing a newer executable left the old per-profile daemon running. Client startup accepted any reachable daemon, so new CLI/MCP/UI entry points could keep using old behavior indefinitely. Local managed updates also required a separate manual stop.

## Decision

Runtime-starting client operations compare the connected daemon's numeric package version with their own. Equal/newer peers are reused; older peers are rechecked under the private startup lock. Capture the authenticated process instance from Doctor and require the same instance on the shutdown connection before transmitting anything. Use existing graceful IPC shutdown, or the verified Linux legacy pidfd path, and wait for daemon-lock release before launching the new image. Serialize starters through readiness so concurrent clients converge on one owner. Doctor and other ordinary read-only queries retain their non-starting behavior.

Compatible independent runners retain execution, IDs, cursors and results. Before stopping for a schema upgrade, refuse launching/running old work; storage migration retains its own exclusivity and active-run checks. No downgrade, workload replay, profile switch, process-name search or force kill is introduced.

Applied managed installation updates validate ownership, version, selected caches and idle jobs before stopping an online daemon. Hold startup/daemon locks through file replacement, preserve the existing journal/backups, then invoke the installed CLI to launch its own daemon and verify readiness/version. This retains Windows CIM's same-image authentication when the updater is another executable. Offline profiles stay offline. Unchanged updates, previews, cache-only operations, repair and removal retain their existing behavior. A replacement or startup failure reports the remaining recovery action and never rolls back a used schema. Concurrent admission detected after shutdown refuses file replacement and directs the user to restart the unchanged runtime and finish that work.

## Verification and limits

Linux behavior tests exercise real native daemon/runner handover with an older-version fixture, concurrent starts/UI, compatible running-work survival, launch-once/log preservation, no downgrade, migration refusal, pinned-peer mismatch, managed update preview/no-op/active-job checks and the real legacy signal path. The older-version native fixture changes only package-version bytes; it does not establish compatibility with every historical release. Current results and Windows/host qualification boundaries belong in [implementation status](../IMPLEMENTATION_STATUS.md).

Package installation itself has no lifecycle script. An npm/host update takes effect when a new entry starts or binds work; an already loaded MCP process must reconnect to load its new executable. A manual foreground daemon can bypass the startup lock and is still fenced by its daemon lock. Directory replacement, subsequent startup, marketplace writes and cache updates are not one crash-atomic transaction; interrupted installations retain the explicit recorded repair workflow.
