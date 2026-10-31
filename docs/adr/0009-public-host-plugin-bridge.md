# ADR 0009: caller-owned bridge to public host session APIs

Accepted, 2026-10-26, source 0.1.6.

## Decision

Keep command execution, durable delivery and recovery in the Rust core. Add `plugin_bridge` registrations for host plugins that expose an owner-only Unix socket or private Windows pipe and call a documented host session API. The core verifies endpoint identity/peer ownership, bounded versioned framing, echoed caller/instance and exact acknowledged delivery metadata. Host acceptance remains distinct from model execution.

OpenCode V1 obtains the caller from `tool.execute.before`, supplies the binding to shared MCP, and uses the public session `promptAsync` API. Its bridge instance owns only sessions observed in tool contexts, verifies their project directory, sends only stable completion metadata, and has bounded in-process delivery dedupe. Existing fenced renewal/scoped recovery is generalized by exact adapter identity; it never crosses ChatGPT Desktop/plugin adapters or guesses the latest session. Explicit registration now starts an absent per-state daemon; read-only queries still do not.

The selected runnable target is OpenCode/SDK/plugin 1.19.4. V2's Plugin.define/transforms are a separate future entry, not a compatibility guess. The account-free check uses the actual V1 loader, MCP, session API and agent loop with a local simulated model and an unrelated control session. This does not qualify a real model provider, V2 or OpenCode restart recovery.

Claude Code receives its documented manifest/MCP/skill layout and a helper using the skill's explicit host-substituted session ID. It is inbox-only. Synthetic caller/helper and actual MCP-entry checks do not establish Claude plugin loading or model execution. The maintainer has no account; community tests and PRs are requested before a public hook/channel continuation adapter is implemented.

## Limits

The bridge uses this project's local protocol on Linux and Windows, not a public host socket or cryptographic caller attestation. Plugin reload/host replacement requires explicit caller-owned recovery; receiver dedupe across restart is not established. IPC v1 retains its registration vocabulary, but older binaries that predate `plugin_bridge` cannot interpret those records. Source updates do not change personal installations or running daemons. Current platform/schema support, package availability and host qualification are tracked in [implementation status](../IMPLEMENTATION_STATUS.md) and [host packages](../host-plugins.md).
