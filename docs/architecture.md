# Architecture

One package and executable; CLI, stdio MCP, daemon and runner modes.

## Source map

| Responsibility | Modules |
|---|---|
| Process initialization | `src/main.rs` |
| CLI syntax and dispatch | `src/cli/args.rs`, `src/cli/mod.rs` (binary-private) |
| MCP tools and client convenience | `src/mcp.rs`, `src/client.rs`, `src/diagnostics.rs`, `src/output.rs` |
| Job intent, execution and persistence | `src/core.rs`, `src/daemon.rs`, `src/runner.rs`, `src/storage.rs` |
| Session binding and notifications | `src/session.rs`, `src/adapters.rs`, `src/plugin_bridge.rs`, `src/delivery.rs`, `src/recovery.rs`, `src/renewal.rs` |
| Idle scheduling and local wake signals | `src/scheduling.rs`, `src/storage.rs` |
| Local transport and OS primitives | `src/ipc.rs`, `src/platform/` |
| Configuration and plugin lifecycle | `src/config.rs`, `src/integration.rs`, `src/deployment.rs`, `src/plugin.rs`, `src/lifecycle.rs` |

The binary owns argument parsing, profile selection and dispatch. CLI and MCP share library behavior; OS-specific operations belong in `platform/`. Host entry points live in `plugins/`, the npm launcher in `packages/runtime/`, and developer verification scripts in `scripts/`.

Icon artwork lives in `assets/icons/agent-job-relay.vectorcraft`; the README/offline SVG, PNG and marketplace SVG are exported from that saved source using VectorCraft. The offline SVG is embedded by `src/deployment.rs`; see [artwork maintenance](../assets/icons/README.md).

## Runtime and storage

- CLI/MCP → bounded private Unix socket / Windows named pipe → daemon → SQLite intent.
- Daemon → detached runner → separate workload process group / Windows Job Object → capped byte logs.
- Runner → SQLite final state + event + delivery outbox in one transaction.
- Daemon → capability adapter → explicit session endpoint → host acceptance.

SQLite WAL, FULL synchronous, foreign keys, busy timeout, versioned schema. Store clones share a cache of up to four idle connections; a process-wide semaphore bounds blocking database operations to four. Connections are checked out exclusively, and failed operations or unfinished transactions discard their connection. Short IMMEDIATE write transactions serialize across daemon and runner processes. No SQLite lock crosses await. Stable UUID job/run/event/target IDs, event sequence is an SQLite monotonic integer. No external message broker, HTTP listener or telemetry. The daemon retains its SIGINT receiver across reconciliation/database awaits, so a graceful-stop request is not lost between select-loop polls.

In-process writes signal reconciliation and delivery immediately. Idle fallback checks back off from 100 ms to one second; reconciliation keeps its 100 ms cadence while jobs are active. Independent runners still persist directly to SQLite, and cross-process changes are discovered by the bounded fallback without requiring a live daemon. An empty delivery queue uses read-only checks instead of opening a write transaction.

Job: queued → launching → running → completed/failed/cancelled/timeout/orphaned. Run records attempt=1, runner identity, workload identity, exit code/signal. Retrying a submission with the same key and identical spec returns the original; a changed spec conflicts. Execution is never replayed after launching uncertainty.

Launch intent commits before spawn. Runner takes a per-run OS lock and atomically claims the launching intent before starting any workload. Duplicate runners lose the CAS. A daemon crash before/after spawning cannot duplicate execution. A launch that remains unclaimed after grace becomes orphaned, never automatically retried. The same lock fences recovery against a delayed runner. PID is diagnostic only; Linux boot ID + procfs start ticks identify processes. Recovery checks lock ownership, not kill(pid,0). Each terminal transition atomically inserts one event and outbox.

## Sessions and delivery

Sessions bind agent kind, logical session ID, host instance and lease. Targets bind the immutable registration key; refresh requires the same identity. Rebinding a new host instance requires an explicit target change, never a latest-session guess. Delivery: pending → leased → accepted_by_host OR retry_wait → dead_letter. Expired claims can be reclaimed. Up to four attempts run concurrently across chats. Transactional claims exclude any other live claim for the same adapter/agent/host/logical session, including another registration instance, and select ready records in insertion order. Retry backoff retains its existing semantics; a waiting retry does not block newer ready notifications. Stable event/target IDs provide receiver dedupe; there is no exactly-once promise. Host ack is not proof of agent execution.

Linux and Windows x86_64 have native runtime verification. Windows uses private SID/DACL named pipes, peer process-token checks and runner-owned Job Objects with a pre-execution helper join. Daemon/runner spawning disables handle inheritance so host capture pipes cannot leak. Background Windows processes remain suspended until their pinned child handle confirms full Job Object detachment; a partial inner-job breakaway is discarded before code/state access so automatic daemon startup can use its checked CIM path. Runners still require native detachment. The Windows daemon-only bootstrap uses private bounded environment transfer and rejects broader/restricted token changes (ADR 0012). Other OS execution/IPC still fail explicitly; see [ADR 0011](adr/0011-windows-native-runtime.md).

## Client and installation boundaries

Client-side convenience is in client.rs (ADR 0002). Submissions and explicit session registrations launch a detached per-state daemon on demand. Opted-in wake resolves only the CLI/MCP process's current inherited host identity, registers/refreshes and probes before submitting. No latest-session lookup or daemon-owned caller inference is used. `daemon start` explicitly starts/reuses a selected daemon without creating a job/session; Windows normally starts on demand and can use a verified same-user local CIM helper when host cleanup blocks direct launch. `daemon stop` stops dispatch only; runner and result persistence lifetimes remain independent.

Current-source convenience keeps the raw IPC/storage contract intact. `connect [--recover [--apply]]` refreshes the actual terminal binding and captures its workspace; MCP accepts this explicit object for submission/recovery without remembering a shared current caller. Common MCP results use compact views, with `full=true` restoring complete records. `plugin install` reuses the existing checked bundle/render/replacement/cache/marketplace implementations for first installation and profile-preserving update; bootstrap/update remain compatible. Preview remains the default and replacement never stops a daemon implicitly.

Installed entry points (ADR 0003) are handled by deployment.rs and diagnostics.rs. A private native/plugin bundle carries a persistent state profile, never a chat identity. The terminal launcher retains the profile across cache copies and invokes the same managed native executable as MCP, avoiding stale cached binaries. The local Codex bundle now uses `.codex-plugin/plugin.json` plus `.mcp.json`, with a stable absolute installed executable path: the tested Codex CLI 0.164.0 loader did not discover portable-root MCP or expand plugin-root variables (ADR 0004). The original installation must remain present for cached MCP. Setup/removal also owns one local marketplace entry. Shared MCP rejects implicit inherited identity. ChatGPT Desktop binds from the current terminal; OpenCode V1 binds from its actual tool context and supplies the key to MCP. The Claude scaffold uses explicit skill session substitution and an inbox-only registration. Runtime/outbox semantics stay unchanged. Configuration, stdio self-check, actual host tool connectivity and observed continuation are separate evidence levels.

Linux daemon/Runner spawning uses the procfs self-executable link to the mapped image, so rebuilding source or unlinking its pathname cannot break a current daemon's Runner launch. A peer-derived Doctor field plus a capability flag identifies an affected legacy daemon before submission; it is not restarted automatically. Updating an install without a state override retains its receipt's profile. One Rust/MCP core is the basis for host-specific plugins; OpenCode V1 has a public-session-API bridge; Claude Code has a community-verification scaffold with inbox-only capability. V2 is a separate future entry (ADR 0009). Independent OS installers/repositories are cancelled; official directory submissions remain future work. Published plugin distribution uses the shared npm runtime and a self-hosted marketplace.

## Offline plugin lifecycle

`plugin.rs` exports native bytes and unbound templates with a version/platform/hash manifest. Explicit recipient bootstrap validates the package and installation/marketplace ownership before installing; no recipient compiler or language runtime is required. Bundle paths and author profiles do not become installed state. Version metadata follows the payload; bootstrap/update preserve existing profiles and new management paths reject downgrades. Installation launched from a custom root defaults management operations to that root.

Runtime snapshot profiles retain installation/state/platform and hashes independently of management receipts, which hosts may omit when copying a plugin. Status and refresh operate only on supplied cache paths. They validate unchanged complete snapshots, fence installation/cache changes with locks, stage replacements and retain backups. Unknown/modified/legacy snapshots fail explicitly; no host enablement change or global cache scan is added. Update preflights all supplied caches before runtime replacement. `lifecycle.rs` durably stages each replacement and publishes an immutable sibling journal before renames; explicit `plugin repair --path` validates snapshots and finishes that operation forward, retaining backups. Pending journals fence further lifecycle changes. Install/config/cache operations remain separate; uninstall has no recovery journal (ADR 0008). No daemon is stopped, schema rolled back or state purged automatically. Offline delivery does not establish online provenance, host-driven bootstrap or portable launch support (ADR 0007).

## Shared npm runtime and self-hosted marketplace

`packages/runtime` distributes the same native executable with a version/platform/SHA-256/minimum-glibc manifest, no install scripts and no selected user profile. The CLI checks compatibility/integrity before spawning. OpenCode resolves that package directly; its explicit development executable override remains available. Default persistent `codex`/`opencode` profiles live outside npm/host caches, remain stable across versions and allow an explicit AJR_STATE_DIR override. They do not automatically migrate an offline installation or capture caller identity. Packages can carry Linux x86_64/glibc and Windows x86_64 targets; the launcher selects and validates only its own target. Windows state defaults to `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`.

`.agents/plugins/marketplace.json` points to `plugins/chatgpt`. Portable and compatibility MCP declarations plus the terminal launcher pin the same npm runtime and codex profile. Using `npx` avoids plugin-root expansion and developer absolute paths on the tested loader; scripts are disabled and the registry is explicit. First use needs Node/npm/network. Installation resolves npm latest and fetches its matching released Git tag, avoiding development main's possibly unpublished pins. Existing offline managed installs/cache repair remain separate and unchanged. See [installation](installation.md) and [release preparation](release.md).

Windows Desktop/plugin exchanges wait asynchronously for a busy pipe and authenticate the actual connected server against the stored identity before sending any request. They do not open an extra fingerprint connection before that bounded wait; Unix retains its non-consuming socket identity check.

Windows OpenCode bridges create their Node/Bun pipe, then invoke the native helper to pin its exact server process and restrict its owner/DACL before registering any session. The core verifies the pipe DACL on every bridge exchange.

Only maintainers manually start release workflows. **Prepare Release** (`release.yml`) validates Cargo-derived versions/pins/CHANGELOG, runs native and installed-package/loader gates, then seals source/run/channel and hashes without npm credentials. **Publish Release** (`release-publish.yml`) previews the actual OIDC identity by default; an explicit non-dry run submits the sealed bytes, retains immutable per-upload acceptance receipts and publishes GitHub without polling registry visibility. **Check Release** (`release-check.yml`) separately verifies public bytes/channel/installability and GitHub using read-only permissions; only this checker may wait for propagation. **Release All** (`release-all.yml`) coordinates the manual workflows through exact Run IDs and stable step keys, retaining the configured publisher filename and fencing main changes before dispatch and in each child. Distinct parent/child concurrency groups avoid deadlock. Recovery keeps original candidates, accepted uploads and matching assets; source/payload changes, foreign receipts and conflicting tags stop. First-package bootstrap remains maintainer-operated; official-directory approval is separate. See [ADR 0013](adr/0013-unified-manual-release.md) and [release operations](release.md).

## Upgrade and notification recovery

Explicit stop can select the installation receipt via `--install-root`, independently of source defaults or shell state exports. For early 0.1.0 peers lacking IPC shutdown, compatibility code retains the original peer identity and uses Linux pidfds to invoke only that daemon's Ctrl-C handler after revalidation. It waits for process exit/lock release; install never stops a daemon on its own. The legacy path requires proc/pidfd support and refuses unknown/renamed peers. Modern IPC shutdown also waits for lock release before returning success.

Same-chat recovery (recovery.rs) probes a fresh current-terminal registration, checks the unchanged session snapshot inside a transaction, and scopes old targets to identical agent/logical/host/caller identities. It transfers only unexpired unresolved deliveries, retaining IDs/history/TTL; accepted events and live claims are excluded. New immutable targets and the recovery audit commit atomically.

Automatic bindings opt into delivery-time renewal (renewal.rs). Only an actual result claim can trigger a same-endpoint owner/fingerprint/capability probe; no periodic host polling is needed. Near-expired/expired sessions extend only after an unchanged-snapshot/current-claim check, atomically with a renewal audit. Invalidation or host replacement requires explicit recovery. Manual/old bindings remain opted out. Capability preflight rejects old daemons before automatic binding.

## Public host plugin bridge

`plugin_bridge.rs` uses the existing owner-checked Unix socket / private Windows pipe transport with project protocol v1, a 16 KiB response cap and three-second deadline. Probe/delivery echoes must match the exact agent/session/instance/host; acceptance also echoes delivery/event/job IDs. Only static completion metadata crosses into the host. The plugin admits only callers observed from tool contexts and uses its public session API; no latest-session discovery or ChatGPT Desktop fallback is involved.

Scoped recovery includes the exact adapter discriminator, so ChatGPT Desktop and plugin registrations cannot coalesce. The existing claim-fenced delivery-time renewal checks the same endpoint/caller. OpenCode V1 supports explicit recovery through its caller-owned helper, with actual OpenCode restart still needing community verification. Inbox-only Claude registrations cannot claim resume. See [ADR 0009](adr/0009-public-host-plugin-bridge.md) and [host packages](host-plugins.md).
