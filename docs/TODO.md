# TODO / Roadmap

## Scope and baseline

Distribute one Rust/MCP runtime through ChatGPT Desktop/Codex, Claude Code and OpenCode plugins. Prepare and verify locally; pushing, publishing and official submissions require final confirmation. Release operations must be manually triggered. The npm account is awaiting verification and package names remain provisional.

Source 0.1.8 implements Linux/Windows native execution, offline plugin maintenance, checked forward repair, shared-MCP caller binding, same-chat recovery and delivery-time renewal. Dual-platform npm/marketplace candidates and the manual release workflow are prepared. OpenCode V1 has actual-loader checks with a simulated model; Claude remains an inbox-only scaffold. Current results and completed acceptance belong in [implementation status](IMPLEMENTATION_STATUS.md) and [live evidence](live-e2e.md), rather than repeated run histories here.

## Host integration and distribution

- [ ] **P1:** finalize npm account/ownership, package names and publisher provenance; review the tested artifacts before approved publication, then verify public-registry installation. See [release preparation](release.md).
- [ ] **P1:** after approved push, run the updated native Windows CI and manually triggered dual-platform release workflow. Earlier remote CI does not verify these updated jobs.
- [ ] **P1:** verify clean Desktop UI installation, additional host versions, results completed while Desktop is closed, real multi-day renewal and receiver deduplication. Keep the passed active-job restart/rebind/continuation scenario complete; do not repeat it merely to refill evidence.
- [ ] **P1:** obtain OpenCode real-provider, server restart/recovery and broader version evidence. Keep V2 as a separate future entry; consolidate shared descriptor metadata only where useful and test each actual loader.
- [ ] **P1, community help:** verify Claude's actual loader, skill session substitution and account-backed MCP workflow. Research public asyncRewake/channels before implementing continuation; keep the scaffold inbox-only until verified. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- [ ] **P1:** qualify host-driven bootstrap and authenticated artifact provenance. Offline hashes and loopback registry checks do not establish online publisher identity.
- [ ] **P2, later authorization:** prepare official-platform eligibility, publisher identity, listing assets, permissions and review materials. Recheck OpenAI local-MCP eligibility, Claude directory requirements and OpenCode's supported package mechanism; publication alone does not establish official approval.

## Runtime and maintenance

- [ ] **P1:** verify Windows ordinary interactive-user execution and real Desktop continuation. Existing native/model checks ran under guest-agent SYSTEM; consult the [Windows guide](windows.md) for the verified daemon-only CIM route and policy/token fallback.
- [ ] **P1:** implement and natively verify macOS process identity, lifecycle and IPC. Plugin packaging and portable compilation do not establish runtime support.
- [ ] **P1:** strengthen Linux containment for escaped descendants/runner death and add longer soak plus controlled disk/power-loss tests. Current process-group cancellation assumes trusted non-daemonizing workloads.
- [ ] **P1:** strengthen per-call identity authentication for shared hosts. Current caller identity is trusted input; never infer it from a latest-session record.
- [ ] **P1:** add backup/export, migration maintenance, retention previews and configurable budgets. Purge needs its own explicit design and is separate from uninstall.
- [ ] **P1:** extend interrupted-operation handling to setup/marketplace cross-file writes, uninstall and pre-journal/legacy remnants. Existing per-directory install/cache repair finishes recorded replacements forward and preserves backups/history.
- [ ] **P1:** independently verify the declared Rust 1.89 minimum or update it from build evidence.
- [ ] **P2:** negotiated standard MCP Tasks, richer progress and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats.

## Deferred by user request

**Task output monitoring and interruption (2026-10-26)** remains deferred until explicitly requested for implementation. Existing log reads/`logs --follow` and manual `job_stop` do not implement it. A future design must define monitored conditions, Agent/runtime responsibility, interruption decisions, bounded output handling, duplicate-trigger/false-positive protection and acceptance scenarios.

## Cancelled work

Independent `.deb`/APT, `.rpm`/DNF, AppImage, Windows installers/WinGet, macOS `.dmg`/Homebrew, Flatpak/Flathub, PyPI/uv packaging and their repository/publication pipelines are cancelled. Reopen only on a new user request. Native payloads inside plugins do not revive independent OS packages.

## Reference links

Recheck current requirements before implementing a host or seeking publication:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://opencode.ai/v2/docs/plugins
