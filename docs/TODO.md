# TODO / Roadmap

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Live verification](live-e2e.md)

## Scope and baseline

Distribute one Rust/MCP runtime through ChatGPT Desktop/Codex, Claude Code and OpenCode plugins. Prepare and verify locally; pushing, publishing and official submissions require final confirmation. Release operations must be manually triggered. Both npm packages and a formal GitHub Release are public; OIDC publication completed and both packages expose provenance.

This is the sole project backlog. Current functionality, completed acceptance and version-specific evidence are maintained in [implementation status](IMPLEMENTATION_STATUS.md) and [live verification](live-e2e.md); changes by version are in [CHANGELOG](../CHANGELOG.md). Keep future work and explicit deferrals here.

## Host integration and distribution

- [x] Use standard npm global command registration for terminal/Web access; add `ajr` with the online Codex profile by default, explicit profile overrides and plugin task-manager guidance, without installation hooks or custom setup.
- [x] Qualify the 0.1.11 runtime/packages through Linux and Windows native CI, installed npm commands including Windows `ajr.cmd`, authenticated Web access and actual Codex/OpenCode loaders; OpenCode uses a simulated model.
- [x] Pass 0.1.11 release-revision CI, manual Prepare Release and actual OIDC publication dry-run; review sealed candidate 40715744368 and preserve its exact bytes.
- [ ] **P1, maintainer:** confirm publication and run the public checker for original 0.1.11 candidate 40715744368 from its reviewed source or an eligible documentation/tooling-only descendant; current Unreleased batch changes are outside that recovery exception. Never rebuild or replace the candidate. Revoke the unused bootstrap token. See [release handoff](release.md#current-release-handoff) and [implementation status](IMPLEMENTATION_STATUS.md).
- [ ] **P1:** verify additional ChatGPT Desktop versions, Windows single-command authorization under restricted permissions, results completed while ChatGPT Desktop is closed, real multi-day renewal and receiver deduplication. Keep the completed Linux/Windows same-chat continuation acceptance complete; do not repeat it merely to refill evidence.
- [x] Implement explicitly opted-in source goal waiting with atomic membership, frozen partial admission, all-run completion, status-only accounting-preserving control, merged notification, deletion/cancellation/uncertain-mutation fences, daemon/admission recovery, archival/restore and shared-MCP entry coverage. See [goal waiting](goal-waiting.md).
- [ ] **P1:** qualify actual Desktop goal UI and Windows controller/installed entry points, host restart/replacement manual fallback and long-duration savings before shipping goal waiting. Obtain a host generation/conditional-update API to close identical same-second replacement, final-usage and verification/mutation/send races; current snapshot checks cannot guarantee this. Current actual Codex checks use a simulated Desktop endpoint/model; keep this feature experimental and include it in a newly versioned candidate.
- [ ] **P1:** obtain OpenCode real-provider, server restart/recovery and broader version evidence. Keep V2 as a separate future entry; consolidate shared descriptor metadata only where useful and test each actual loader.
- [ ] **P1, community help:** verify Claude's actual loader, skill session substitution and account-backed MCP workflow. Research public asyncRewake/channels before implementing continuation; keep the scaffold inbox-only until verified. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- [ ] **P1:** qualify host-driven bootstrap and authenticated artifact provenance. Offline hashes and loopback registry checks do not establish online publisher identity.
- [ ] **P2, later authorization:** prepare official-platform eligibility, publisher identity, listing assets, permissions and review materials. Recheck OpenAI local-MCP eligibility, Claude directory requirements and OpenCode's supported package mechanism; publication alone does not establish official approval.

## Runtime and maintenance

- [x] Replace older daemons automatically on new runtime use and restart online daemons during applied owned-runtime updates; preserve profile/history/compatible runners, serialize concurrent starters and retain pinned-peer, downgrade, migration and active-job fences. See [upgrade decision](adr/0018-automatic-daemon-upgrade.md).
- [ ] **P1:** natively qualify automatic daemon handover and installed update on Windows, including host cleanup/CIM fallback and held MCP executable images. Linux regression evidence does not establish Windows or new real-host acceptance.
- [ ] **P1:** implement and natively verify macOS process identity, lifecycle and IPC. Plugin packaging and portable compilation do not establish runtime support.
- [ ] **P1:** strengthen Linux containment for escaped descendants/runner death and add longer soak plus controlled disk/power-loss tests. Current process-group cancellation assumes trusted non-daemonizing workloads.
- [ ] **P1:** strengthen per-call identity authentication for shared hosts. Current caller identity is trusted input; never infer it from a latest-session record.
- [x] Implement automatic age/count/log-budget archiving, transparent cold reads, consistent rolling backups/restore, recorded interruption recovery, migration fencing and live-session quota accounting. See [data maintenance](data-maintenance.md).
- [x] Natively qualify schema v9 data maintenance on Windows under SYSTEM and a non-elevated ordinary user.
- [ ] **P1:** qualify maintenance under longer disk-pressure/soak tests. Independent-storage backup automation and destructive history purge need separate designs; uninstall remains data-preserving.
- [ ] **P1:** extend interrupted-operation handling to setup/marketplace cross-file writes, uninstall and pre-journal/legacy remnants. Existing per-directory install/cache repair finishes recorded replacements forward and preserves backups/history.
- [ ] **P1:** independently verify the declared Rust 1.89 minimum or update it from build evidence.
- [ ] **P2:** negotiated standard MCP Tasks and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats; progress work is tracked under [runtime inspection and monitoring](#runtime-inspection-and-monitoring).

## Parallel task submission

The runtime executes independent jobs concurrently, with a maximum of eight active jobs per profile and separate IDs, workspaces, logs and notifications. Unreleased source now submits 1–16 specifications through CLI `start-batch` / MCP `job_start_batch`, with ordered partial admission, required per-item keys and safe retry of unchanged specifications. See [batch usage](usage.md#submit-independent-commands-together-unreleased-source), `src/batch.rs` and `tests/runtime.rs`.

- [x] Add bounded shared CLI/MCP batch submission, preserving per-command options/rules/identity, indexed admission failures, independent completion/notifications/stopping and unchanged-key retry after response loss/restart. Linux behavior covers overlap, transaction rollback on invalid sessions, mixed failures, quota races and fake-Desktop routing; actual OpenCode 1.19.4 batch loader/caller/session behavior passes with a simulated model. No dependency graph or workflow scheduler.
- [ ] **P2:** qualify batch submission through Windows native/installed entry points and real ChatGPT Desktop/provider-backed host calls. Include these unreleased changes in a new version/candidate after qualification; the original sealed 0.1.11 candidate remains unchanged.

## Remote execution and external-run observation

Unreleased source implements the [remote contract and reference path](remote-execution.md). Remote commands use an explicitly pinned native runtime profile (a copied executable is enough) or a separately qualified contract adapter. GitHub Actions uses local `gh` and an immutable repository/run/attempt. Ordinary local `ssh`/`gh` jobs retain local-process semantics.

- [x] Define bounded v1 target/request/run identity, launch/attach/lookup/sample/stop semantics, capabilities, origin Job/Run/session mapping and reconnect/cursors, while keeping credentials in existing authentication facilities and local Web/IPC loopback-only. See [ADR 0016](adr/0016-remote-execution-and-observation.md).
- [x] Implement native remote launch/attach, remote cwd/env/timeout/rules, raw byte output, bounded milestone/rule evidence, disconnect survival, unchanged-key launch recovery and exact-run graceful/forced cancellation. Read-only attachment stops observation only; transport loss/stale evidence remain distinct from terminal outcomes. Linux regressions and Windows native/cross-host verification are recorded in [implementation status](IMPLEMENTATION_STATUS.md).
- [x] Observe an existing GitHub Actions attempt through documented read-only `gh api`, retaining status/conclusion/URL with bounded calls and authentication/rate-limit diagnostics. Never dispatch/cancel/rerun on observation stop or timeout. Simulated transitions and a real completed workflow read are separate evidence.
- [ ] **P2:** qualify SSH-specific Linux/Windows shell/authentication deployments, additional scheduler adapters and prolonged disconnection/soak behavior. Native Windows cross-host verification uses the existing authenticated VM channel, not an SSH server. No framework coupling, checkpoint transfer or automatic training restart.
- [ ] **P2:** qualify external-task notifications through real ChatGPT Desktop/OpenCode continuation. Existing completion acceptance and source MCP/inbox routing do not independently qualify this new path.

## Runtime inspection and monitoring

- [x] Implement Agent-independent inspect/watch, revision-fenced persistent output/silence/elapsed rules, bounded recent tails, automatic tree cancellation, optional same-chat alerts and preserved archive/backup/audit history.
- [x] Bundle a React + Vite task manager in the native executable, default-on authenticated loopback access, exact-profile management commands and CLI/MCP/plugin entry points. See [runtime monitoring](runtime-monitoring.md).
- [x] Natively qualify monitor/Web UI/schema v9 on Windows, including independent runner detection/tree stop, bounded tails, silence rearming, authenticated HTTP control and installed-package resources.
- [ ] **P1:** test real running-alert continuation in ChatGPT Desktop/OpenCode. Existing completion acceptance does not qualify this path.
- [ ] **P2:** qualify Windows browser interaction with the bundled panel; native HTTP/package checks and Linux browser acceptance are already complete.

Unreleased source adds bounded structured progress and repeated stage/step/metric feedback alongside existing marker rules. Runner-owned detection, durable source sequences, revision/terminal fences, cooldowns, notification budgets and final milestone summaries share Event/Outbox and CLI/MCP/Web paths. See [mid-run feedback and checker recipe](progress.md). Real host acceptance remains distinct from recorded/acknowledged alerts.

- [x] Provide Agent/plugin guidance and a reference checker recipe reusing existing rules for checkpoints, milestones and external status. Install initial rules before flushed markers, bind the actual caller, pin observed-run identity and bound checks with deadline/backoff; verify through native runner and fake-host infrastructure without a live Agent polling loop.
- [x] Implement versioned bounded stage/step/total/metric samples, stage/step/threshold conditions, stable source-sequence deduplication, cooldown/rate limits and durable evidence. Preserve important retained milestones in terminal summaries when completion supersedes delivery; share Event/Outbox and CLI/MCP/Web, covering duplicate/out-of-order samples, recovery, revisions and terminal races.
- [ ] **P2:** qualify structured progress and checker workflows through Windows native/installed entry points and real ChatGPT Desktop/OpenCode mid-run continuation. Current Linux native/fake-host evidence does not qualify those paths; include these source changes in a newly versioned candidate. Prolonged high-rate training telemetry remains outside this bounded progress interface.
- [ ] **P2:** evaluate additional bounded matching conditions and native desktop notification channels from concrete usage. Runtime alert records remain usable without any host or viewer.

## Cancelled work

Independent `.deb`/APT, `.rpm`/DNF, AppImage, Windows installers/WinGet, macOS `.dmg`/Homebrew, Flatpak/Flathub, PyPI/uv packaging and their repository/publication pipelines are cancelled. Reopen only on a new user request. Native payloads inside plugins do not revive independent OS packages.

## Reference links

Recheck current requirements before implementing a host or seeking publication:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://opencode.ai/v2/docs/plugins
