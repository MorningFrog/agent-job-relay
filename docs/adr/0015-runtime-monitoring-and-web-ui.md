# ADR 0015: Runner monitoring and bundled local Web UI

## Decision

Expose inspect, durable monitor rules and configurable cancellation through the existing single Rust package/executable. CLI, MCP and browser controls share library/store behavior. React + Vite assets are built before native compilation and embedded into that executable; recipients and offline installs need no frontend server/toolchain. The daemon's authenticated loopback listener defaults on, while browser opening remains explicit. This revises the previous Web UI non-goal and monitoring deferral under the user's implementation request.

Run detection inside the independent runner at capture time, before raw-prefix truncation. Keep bounded per-stream tails/match windows, monotonic silence/elapsed clocks and one durable trigger per rule revision. A transaction fences rule revision, enabled state and job state before inserting evidence, optional outbox delivery and local stop intent. Revision-aware edits and applied acknowledgements separate saved configuration from actual runner adoption. Viewer/host/daemon loss does not own monitoring. Database write failure remains a supervision failure with existing uncertain-outcome limits.

Use schema v9 tables for rules, changes, observations and stop requests, including them in cold snapshots/restore. Rebuild events under exclusive migration to replace all-kind uniqueness with lifecycle uniqueness; preserve SQLite's event sequence high-water mark and delivery references. Old cold catalogs are extended with empty monitor tables when read, without changing original compressed bytes. Verified v8 backups can migrate forward only within a new restore profile.

Use a random local port and per-daemon bearer token conveyed only by explicit private IPC. Bootstrap through a URL fragment, clear it from navigation and use session storage/header authentication for API/SSE. Validate Host/Origin, serve embedded resources only, constrain API operations/bodies/concurrency and use CSP/text rendering. Normal health omits access credentials. Startup opt-out is explicit; listener failure is diagnostic and does not terminate workloads. No public exposure, service, profile discovery or silent upgrade restart is introduced.

## Regression rationale

Test raw-cap/split-output detection, independent process-tree cancellation, silence dedupe/rearm, stale revision/terminal fences, preserved archive/backup/audit data, monotonic migration, local HTTP access restrictions and real installed launchers. Browser checks establish UI behavior only. Windows native behavior and actual mid-run Agent continuation need separate evidence; existing terminal-notification acceptance does not establish them.
