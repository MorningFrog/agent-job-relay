# TODO / Roadmap

## Scope

Distribute through Agent plugins around one Rust/MCP runtime, targeting ChatGPT Desktop/Codex, Claude Code and OpenCode. Local development, easy installation/update and data-preserving removal come first. **Do not publish, submit, push or configure online credentials during development.** Official installation mechanisms and directory approval are separate future milestones.

## Current baseline

- Source **0.1.4**, Linux only: offline native plugin export/bootstrap/update, explicit checked cache refresh, pinned profiles, owned configuration, shared-MCP terminal binding, TTL-bounded retries, same-chat recovery and verified delivery-time renewal are implemented and tested.
- The isolated actual Codex CLI 0.164.0 loader bootstraps a relocated offline bundle, checks its actual cache and discovers **20 tools**. The latest read-only installed MCP health check reports runtime **0.1.4** with matching active/installed state.
- CLI live wake and installed MCP submission/same-chat continuation passed by user confirmation, with persisted task/log/delivery correlation. Plugin-browser discovery/install and current host-version compatibility remain open. Detailed evidence belongs in [implementation status](IMPLEMENTATION_STATUS.md) and [live E2E](live-e2e.md).

## Host integration and plugin delivery

- **Completed P0 acceptance:** real installed MCP submission, successful quality-gate workload and completion-triggered same-chat continuation, with no CLI fallback, are user-confirmed and correlated with persisted records. Do not repeat this test merely to refill evidence; see [live E2E](live-e2e.md).
- **P1:** qualify plugin-browser discovery/install and supported host versions during an appropriate installation. Keep UI, local stdio, tool connectivity and observed continuation separate; no artificial crash matrix is required.
- **Completed local increment:** offline `plugin pack/bootstrap/update`, version/profile preservation, managed cache launchers and explicit snapshot inspection/refresh. Recipients need no Rust. Installation/cache upgrade provenance is separate from live wake acceptance; see [ADR 0007](adr/0007-offline-plugin-runtime.md).
- **P1:** extend offline bootstrap to verified host-driven installation and portable local launch paths as loaders support them; design authenticated native artifact download/provenance before any online installer. The tested loader still requires a stable original installation, and legacy caches need host refresh first. Native artifacts inside plugins do not revive OS packages; no download/publication is implemented.
- **P1:** add Claude Code's actual manifest/MCP/skills layout and locally verify installation, exact caller identity and supported completion/continuation. Reuse the Rust runtime; the Desktop socket is not a Claude adapter.
- **P1:** choose and document OpenCode V1/V2, then implement its supported plugin mechanism, MCP bridge and owned-session routing. Verify notification versus continuation independently; ship no placeholder support claims.
- **P1:** generate host descriptors from shared metadata where useful, but test each real loader. The current Codex development bundle requires `.codex-plugin/plugin.json`, `.mcp.json` and a stable absolute native command.
- **P1:** improve interrupted setup/update/removal recovery while preserving modified/foreign files, other plugins, custom profiles and history. Distinguish marketplace-source removal, host-cache uninstall and user-data deletion.
- **P2, later authorization required:** prepare official platform eligibility, publisher identity, listing assets, permissions, update provenance and review materials. Recheck OpenAI local-MCP eligibility, Claude marketplace/directory requirements and OpenCode's supported package mechanism; package publication does not itself prove official approval.

## Testing and maintenance

- **Completed:** testing/artifact rules now prioritize behavioral regression coverage; remove README wording/structure assertions, retain required fixtures and keep only concise current verification in documentation. Generated outputs/reports/logs remain ignored.

## Runtime backlog

- **P1, deferred user request (2026-10-26): task output monitoring and interruption.** Log reads/`logs --follow` and manual `job_stop` do not implement it. Define monitored conditions, Agent/runtime responsibility, who decides to interrupt, bounded output handling, duplicate-trigger and false-positive protection, and acceptance scenarios in a later increment. Record only until implementation is requested.
- **P1:** qualify real Desktop restart/rebind and long-job renewal when an appropriate use case occurs. Current recovery/renewal evidence is installed-process/fake-host automation; multi-day expiry is injected. Host-version compatibility and receiver deduplication remain open.
- **P1:** strengthen Linux containment for escaped descendants/runner death; add longer soak and controlled disk/power-loss tests. Current process-group cancellation assumes trusted non-daemonizing workloads.
- **P1:** implement and natively verify Windows SID/DACL, pipes and Job Objects, plus macOS identity, lifecycle and IPC. Plugin delivery alone proves no OS support.
- **P1:** strengthen per-call identity authentication for shared hosts. Delivery-time renewal beyond 24 hours is implemented, but caller identity remains trusted input; never infer it from a latest-session record.
- **P1:** add backup/export, migration maintenance, retention previews and configurable budgets. Purge needs its own explicit design and is not plugin uninstall.
- **P2:** negotiated standard MCP Tasks, richer progress and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats.

## Cancelled work

Independent `.deb`/APT, `.rpm`/DNF, AppImage, Windows installers/WinGet, macOS `.dmg`/Homebrew, Flatpak/Flathub, PyPI/uv packaging and their repository/publication pipelines are cancelled. No such installer, signing/notarization or automatic publication workflow was implemented. Reopen only on a new user request.

## Reference links

These links informed the 2026-10-26 plugin direction; recheck current requirements before implementing a host or seeking publication:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://opencode.ai/v2/docs/plugins
