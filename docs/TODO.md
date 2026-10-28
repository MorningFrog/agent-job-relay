# TODO / Roadmap

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Live verification](live-e2e.md)

## Scope and baseline

Distribute one Rust/MCP runtime through ChatGPT Desktop/Codex, Claude Code and OpenCode plugins. Prepare and verify locally; pushing, publishing and official submissions require final confirmation. Release operations must be manually triggered. Both npm packages and a formal GitHub Release are public; OIDC publication completed and both packages expose provenance.

This is the sole project backlog. Current functionality, completed acceptance and version-specific evidence are maintained in [implementation status](IMPLEMENTATION_STATUS.md) and [live verification](live-e2e.md); changes by version are in [CHANGELOG](../CHANGELOG.md). Keep future work and explicit deferrals here.

## Host integration and distribution

- [x] Use standard npm global command registration for terminal/Web access; add `ajr` with the online Codex profile by default, explicit profile overrides and plugin task-manager guidance, without installation hooks or custom setup.
- [x] Qualify the 0.1.11 runtime/packages through Linux and Windows native CI, installed npm commands including Windows `ajr.cmd`, authenticated Web access and actual Codex/OpenCode loaders; OpenCode uses a simulated model.
- [x] Pass 0.1.11 release-revision CI, manual Prepare Release and actual OIDC publication dry-run; review sealed candidate 40715744368 and preserve its exact bytes.
- [ ] **P1, maintainer:** confirm publication and run the public checker for original 0.1.11 candidate 40715744368. Use recover_candidate for the documentation-only handoff update; never rebuild or replace the candidate. Revoke the unused bootstrap token. See [release handoff](release.md#current-release-handoff) and [implementation status](IMPLEMENTATION_STATUS.md).
- [ ] **P1:** verify additional ChatGPT Desktop versions, Windows single-command authorization under restricted permissions, results completed while ChatGPT Desktop is closed, real multi-day renewal and receiver deduplication. Keep the completed Linux/Windows same-chat continuation acceptance complete; do not repeat it merely to refill evidence.
- [ ] **P1:** obtain OpenCode real-provider, server restart/recovery and broader version evidence. Keep V2 as a separate future entry; consolidate shared descriptor metadata only where useful and test each actual loader.
- [ ] **P1, community help:** verify Claude's actual loader, skill session substitution and account-backed MCP workflow. Research public asyncRewake/channels before implementing continuation; keep the scaffold inbox-only until verified. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- [ ] **P1:** qualify host-driven bootstrap and authenticated artifact provenance. Offline hashes and loopback registry checks do not establish online publisher identity.
- [ ] **P2, later authorization:** prepare official-platform eligibility, publisher identity, listing assets, permissions and review materials. Recheck OpenAI local-MCP eligibility, Claude directory requirements and OpenCode's supported package mechanism; publication alone does not establish official approval.

## Runtime and maintenance

- [ ] **P1:** implement and natively verify macOS process identity, lifecycle and IPC. Plugin packaging and portable compilation do not establish runtime support.
- [ ] **P1:** strengthen Linux containment for escaped descendants/runner death and add longer soak plus controlled disk/power-loss tests. Current process-group cancellation assumes trusted non-daemonizing workloads.
- [ ] **P1:** strengthen per-call identity authentication for shared hosts. Current caller identity is trusted input; never infer it from a latest-session record.
- [x] Implement automatic age/count/log-budget archiving, transparent cold reads, consistent rolling backups/restore, recorded interruption recovery, migration fencing and live-session quota accounting. See [data maintenance](data-maintenance.md).
- [x] Natively qualify schema v9 data maintenance on Windows under SYSTEM and a non-elevated ordinary user.
- [ ] **P1:** qualify maintenance under longer disk-pressure/soak tests. Independent-storage backup automation and destructive history purge need separate designs; uninstall remains data-preserving.
- [ ] **P1:** extend interrupted-operation handling to setup/marketplace cross-file writes, uninstall and pre-journal/legacy remnants. Existing per-directory install/cache repair finishes recorded replacements forward and preserves backups/history.
- [ ] **P1:** independently verify the declared Rust 1.89 minimum or update it from build evidence.
- [ ] **P2:** negotiated standard MCP Tasks, richer progress and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats.

## Runtime inspection and monitoring

- [x] Implement Agent-independent inspect/watch, revision-fenced persistent output/silence/elapsed rules, bounded recent tails, automatic tree cancellation, optional same-chat alerts and preserved archive/backup/audit history.
- [x] Bundle a React + Vite task manager in the native executable, default-on authenticated loopback access, exact-profile management commands and CLI/MCP/plugin entry points. See [runtime monitoring](runtime-monitoring.md).
- [x] Natively qualify monitor/Web UI/schema v9 on Windows, including independent runner detection/tree stop, bounded tails, silence rearming, authenticated HTTP control and installed-package resources.
- [ ] **P1:** test real running-alert continuation in ChatGPT Desktop/OpenCode. Existing completion acceptance does not qualify this path.
- [ ] **P2:** qualify Windows browser interaction with the bundled panel; native HTTP/package checks and Linux browser acceptance are already complete.
- [ ] **P2:** evaluate structured progress, additional bounded matching conditions and native desktop notification channels from concrete usage. Rules currently fire once per revision; runtime alert records remain usable without any host or viewer.

## Cancelled work

Independent `.deb`/APT, `.rpm`/DNF, AppImage, Windows installers/WinGet, macOS `.dmg`/Homebrew, Flatpak/Flathub, PyPI/uv packaging and their repository/publication pipelines are cancelled. Reopen only on a new user request. Native payloads inside plugins do not revive independent OS packages.

## Reference links

Recheck current requirements before implementing a host or seeking publication:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://opencode.ai/v2/docs/plugins
