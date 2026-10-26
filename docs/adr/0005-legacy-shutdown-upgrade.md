# ADR 0005: stop early daemons by verified peer and installed profile

Accepted, 2026-10-26, v0.1.2. The user hit an active-install lock and then an unknown `shutdown` operation. The owned install profile selected a still-running early 0.1.0 daemon from before the shutdown API; a bare source command can select a different state. Upgrading source alone cannot add an operation to a live peer.

## Decision

Add `daemon stop --install-root` on the updated source executable. It selects the receipt profile even when the shell exports another state, errors if the install is absent, and does not replace any files. Install --apply still refuses active runtimes; it never stops one implicitly. Successful stop waits for lock release so an immediate upgrade does not race an accepted shutdown request.

Keep the original same-UID IPC peer PID and boot/start identity in a typed remote error. Only an exact unknown-shutdown rejection from that peer enables legacy handling. Reconnect to the same peer, require owned private root/socket, recheck identity, native executable name and daemon arguments, and obtain a healthy known 0.1.0 Doctor response. Open a Linux pidfd before validation/signal, invoke the old Ctrl-C handler with SIGINT through that pinned handle, and wait for process exit. PID reuse cannot redirect the signal. No process-name lookup, process-group signal, force kill, silent new-state selection or workload replay is added.

Renamed legacy binaries, inaccessible proc, missing pidfd support, changed peers and other errors fail explicitly. This stays in platform code and the trusted local-user model; executable-name checks are not cryptographic product attestation. Modern shutdown still uses IPC. No shutdown tool is exposed through MCP.

## Evidence

Three isolated upgrade tests exercise an old-protocol fixture, actual signal/exit and lock release, immediate owned replacement, history/profile retention despite wrong exports, stale identity refusal, unrelated peer refusal and no signal on other errors. Existing real modern Runner tests cover surviving daemon stop. All 50 tests, formatting, strict Clippy and release build passed. Actual isolated Codex CLI loading found 19 tools again.

After explicit user installation authorization, the actual idle early daemon was stopped through this path, the personal native install/cache upgraded to 0.1.2, and the original tables/profile were verified unchanged. Installed/cache MCP self-checks found 19 tools. The user declined recovery backups: only newly created copies/old-install backup were removed after checking ownership, preserving user data and other plugins. Desktop refresh/live MCP behavior remains a separate gate.
