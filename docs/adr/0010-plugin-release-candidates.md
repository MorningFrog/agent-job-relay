# ADR 0010: shared npm runtime and self-hosted plugin delivery

Accepted, 2026-10-26, source 0.1.6 candidate. This extends local delivery; it does not replace the owned offline installation lifecycle. The original publication gating is extended by [ADR 0013](0013-unified-manual-release.md), which fixes the two main npm identities and defines reviewed manual candidate preparation and exact-byte OIDC publication.

## Problem

The OpenCode source entry required a developer binary path. The tested Codex 0.164.0 loader did not expand plugin-root placeholders, so copying the existing unrendered plugin into a public Git marketplace would expose a catalog without a working MCP server. One Rust/MCP core must remain shared while recipients avoid a compiler, author paths and cached user profiles.

## Decisions

- Package the native Linux x86_64/glibc executable in `agent-job-relay-runtime`, alongside a checked version/platform/hash/minimum-glibc manifest. There are no install scripts or secondary native downloads. A staging script produces reviewed npm tarballs and a marketplace archive without publishing. Release builds use an Ubuntu 22.04 baseline; other builds report their actual required glibc symbols.
- The OpenCode package depends on the exact runtime version, resolves its executable directly and retains explicit development overrides. The ChatGPT repository catalog points to a separate portable/compatibility plugin; MCP and its terminal launcher use the same exact npm version through `npx --ignore-scripts` with an explicit registry. This avoids unsupported root-variable expansion. First use requires Node/npm/network and trust in that npm publisher.
- Stable `codex`/`opencode` default profiles live outside caches and may be explicitly overridden with AJR_STATE_DIR. They contain no caller identity and do not migrate existing offline installations. Host bindings still come from the real current terminal/tool caller. Raw npm operations have no lifecycle hooks. Coordinated online maintenance follows [ADR 0021](0021-unified-online-update.md), including checked daemon shutdown/restart; it never replays workloads, deletes history or guesses caller identity.
- Keep offline managed roots, receipts, snapshot repair, retained backups and downgrade rules separate. npm/cache management is host-owned; do not apply offline receipt repair to an npm cache. Already loaded daemon images remain independent of package pathname lifetime. Older same-profile daemons follow the checked handover in [ADR 0018](0018-automatic-daemon-upgrade.md); incompatible migrations require completed jobs.
- Only manual `workflow_dispatch` prepares release artifacts or optionally creates a prerelease draft. Ordinary CI verifies changes without publishing. Preserve existing releases/assets and never move a mismatched tag. npm publishing/account configuration and official directory submission are separate actions requiring approval.

## Verification boundary

Candidate checks install the actual tarballs, exercise native execution/history after package relocation, and refuse corrupted native bytes. An isolated loopback registry serves unpublished candidate versions to the actual OpenCode Bun name/version installer and Codex marketplace loader; public dependency metadata stays on npm. OpenCode uses a simulated model and checks the completed response in the owned session plus an untouched control session. Codex checks cached terminal-launcher execution and the current MCP tool catalog without a model turn or ChatGPT Desktop contact. Neither check proves public registry availability, clean ChatGPT Desktop UI installation, real-provider behavior, publisher authenticity or official approval. Current results belong in implementation status, not this decision record.
