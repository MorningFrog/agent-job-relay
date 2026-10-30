# TODO / Roadmap

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Live verification](live-e2e.md)

## Scope and baseline

Distribute one Rust/MCP runtime through ChatGPT Desktop/Codex, Claude Code and OpenCode plugins. Prepare and verify locally; pushing, publishing and official submissions require final confirmation. Release operations must be manually triggered. Both npm packages and a formal GitHub Release are public; OIDC publication completed and both packages expose provenance.

This is the sole project backlog. Current functionality, completed acceptance and version-specific evidence are maintained in [implementation status](IMPLEMENTATION_STATUS.md) and [live verification](live-e2e.md); changes by version are in [CHANGELOG](../CHANGELOG.md). Keep future work and explicit deferrals here.

## Host integration and distribution

- [x] Implement source 0.1.14 `ajr update` for global CLI and matching online ChatGPT Desktop/Codex plugin installation/update, mandatory CLI prerequisite, exact-version/preview/CLI-only options, profile/schema fencing, original-peer daemon restart and recorded forward retry through public host commands. Add coordinated `ajr uninstall`, default idle daemon shutdown, forced shared task stops, retained data and recorded removal recovery; preserve compatible active Linux runners during updates and replace older daemons even when package files are current. Keep offline/OpenCode routes separate.
- [ ] **P1:** natively qualify Windows self-update/self-removal with loaded global CLI/daemon/MCP images, qualify active-run package replacement separately, and verify actual Desktop refresh/reconnection. Public-host fixtures and Linux package replacement do not establish these outcomes.
- [ ] **P1, maintainer:** prepare a new 0.1.14 candidate, pass revision CI/Linux/Windows/package/actual-loader gates and publisher preview, then review before requesting publication approval. The sealed 0.1.13 candidate cannot carry these runtime/package changes; do not rebuild or replace its bytes. Run the independent public checker after authorized publication and revoke the unused bootstrap token.

- [x] Use standard npm global command registration for terminal/Web access; add `ajr` with the online Codex profile by default, explicit profile overrides and plugin task-manager guidance, without installation hooks or custom setup.
- [x] Verify published 0.1.12 against original candidate 40908803482 with a local read-only public-byte/channel/install/GitHub audit after the original Actions checker timed out on registry propagation.
- [ ] **P1:** verify additional ChatGPT Desktop versions, Windows single-command authorization under restricted permissions, results completed while ChatGPT Desktop is closed, real multi-day renewal and receiver deduplication. Keep the completed Linux/Windows same-chat continuation acceptance complete; do not repeat it merely to refill evidence.
- [x] Implement native source goal waiting with atomic membership, frozen partial admission, all-run completion, status-only accounting-preserving control, merged notification, deletion/cancellation/uncertain-mutation fences, daemon/admission recovery, archival/restore and shared-MCP entry coverage. See [goal waiting](goal-waiting.md).
- [x] Fix Desktop's continued goal checks after handoff: require the originating chat's native goal pause and revision-fenced Relay acknowledgement, retain accepted jobs while confirmation is pending, expire missing acknowledgement and fence earlier storage-only waits. Verify the owning runtime's pause event separately from persisted state.
- [x] Use stable cached skill/MCP initialization entries and verified current-runtime forwarding, preserving profiles, negotiated input and uncertain-call fences; historical entries need one migration/reconnection. Add structured pending consent, durable original-request scope/budget, native input and one Agent-question fallback before admission.
- [x] User confirms local goal-mode automatic pause works after approval.
- [x] Fence goal-controller digest reuse against same-tick filesystem writes and preserved modification times; retain stable-image caching and fail on changed bytes.
- [x] Separate blocked/inactive host goals and controller read failures from consent invalidation; preserve original request budgets/waits, gate CLI/MCP admission/questions, reuse approval after host Resume and fence stale observations against concurrent revocation.
- [ ] **P1:** qualify the new direct MCP form in actual Desktop and complete real-model three-round restoration, including initial permission, one natural reply, decline/no reply, plugin-text refusal and the reported post-completion `blocked` state. The source of that host transition still needs real-host evidence; retained-consent recovery does not establish automatic blocker repair. Windows goal/controller, changed-host goal handling and long-duration savings remain separate. Consent-free suspension and stronger replacement fencing require host support; do not reopen the already verified approval→pause behavior.
- [ ] **P1:** obtain OpenCode real-provider, server restart/recovery and broader version evidence. Keep V2 as a separate future entry; consolidate shared descriptor metadata only where useful and test each actual loader.
- [ ] **P1, community help:** verify Claude's actual loader, skill session substitution and account-backed MCP workflow. Research public asyncRewake/channels before implementing continuation; keep the scaffold inbox-only until verified. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- [ ] **P1:** qualify host-driven bootstrap and authenticated artifact provenance. Offline hashes and loopback registry checks do not establish online publisher identity.
- [ ] **P2, later authorization:** prepare official-platform eligibility, publisher identity, listing assets, permissions and review materials. Recheck OpenAI local-MCP eligibility, Claude directory requirements and OpenCode's supported package mechanism; publication alone does not establish official approval.

## Runtime and maintenance

- [x] Replace older daemons automatically on new runtime use and restart online daemons during applied owned-runtime updates; preserve profile/history/compatible runners, serialize concurrent starters and retain pinned-peer, downgrade, migration and active-job fences. See [upgrade decision](adr/0018-automatic-daemon-upgrade.md).
- [ ] **P1:** natively qualify automatic daemon handover and installed update on Windows, including host cleanup/CIM fallback and held MCP executable images. Linux regression evidence does not establish Windows or new real-host acceptance.
- [ ] **P1:** implement and natively verify macOS process identity, lifecycle and IPC. Plugin packaging and portable compilation do not establish runtime support.
- [x] Flush buffered log writes before sync and cover profile-specific log-write/log-sync/WAL-write ENOSPC, failed-versus-unknown evidence, stable IDs/non-replay and live-runner cleanup of adopted session escapes without touching unrelated processes.
- [x] Run a ten-minute Linux eight-job soak through nine daemon restarts, capped output/structured progress, bounded RSS/descriptors, unchanged-key retries and active backup/new-profile restore without replay.
- [ ] **P1:** strengthen containment after runner death and against adversarial escape; qualify physical disk exhaustion/power loss and longer-duration load. Live-runner adopted-child cleanup does not run after SIGKILL.
- [ ] **P1:** strengthen per-call identity authentication for shared hosts. Current caller identity is trusted input; never infer it from a latest-session record.
- [x] Implement automatic age/count/log-budget archiving, transparent cold reads, consistent rolling backups/restore, recorded interruption recovery, migration fencing and live-session quota accounting. See [data maintenance](data-maintenance.md).
- [x] Natively qualify schema v9 data maintenance on Windows under SYSTEM and a non-elevated ordinary user.
- [ ] **P1:** extend the ten-minute Linux maintenance/load baseline to multi-day, physical disk-pressure and Windows soak scenarios. Independent-storage backup automation and destructive history purge need separate designs; uninstall remains data-preserving.
- [ ] **P1:** extend interrupted-operation handling to setup/marketplace cross-file writes, native offline uninstall and pre-journal/legacy remnants. Existing per-directory install/cache repair finishes recorded replacements forward and preserves backups/history.
- [x] Verify the declared Rust 1.89 minimum with Linux all-target compilation and add its CI check; stable native behavior gates remain separate.
- [ ] **P2:** negotiated standard MCP Tasks and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats; progress work is tracked under [runtime inspection and monitoring](#runtime-inspection-and-monitoring).

## Parallel task submission

The runtime executes independent jobs concurrently, with a maximum of eight active jobs per profile and separate IDs, workspaces, logs and notifications. Runtime 0.1.12+ submits 1–16 specifications through CLI `start-batch` / MCP `job_start_batch`, with ordered partial admission, required per-item keys and safe retry of unchanged specifications. See [batch usage](usage.md#submit-independent-commands-together-0112), `src/batch.rs` and `tests/runtime.rs`.

- [x] Add bounded shared CLI/MCP batch submission, preserving per-command options/rules/identity, indexed admission failures, independent completion/notifications/stopping and unchanged-key retry after response loss/restart. Linux behavior covers overlap, transaction rollback on invalid sessions, mixed failures, quota races and fake-Desktop routing; actual OpenCode 1.19.4 batch loader/caller/session behavior passes with a simulated model. No dependency graph or workflow scheduler.
- [ ] **P2:** qualify batch submission through Windows native/installed entry points and real ChatGPT Desktop/provider-backed host calls. Published 0.1.12 installed-package checks do not independently qualify batch-specific host behavior.

## Remote execution and external-run observation

Runtime 0.1.12+ implements the [remote contract and reference path](remote-execution.md). Remote commands use an explicitly pinned native runtime profile (a copied executable is enough) or a separately qualified contract adapter. GitHub Actions uses local `gh` and an immutable repository/run/attempt. Ordinary local `ssh`/`gh` jobs retain local-process semantics.

- [x] Define bounded v1 target/request/run identity, launch/attach/lookup/sample/stop semantics, capabilities, origin Job/Run/session mapping and reconnect/cursors, while keeping credentials in existing authentication facilities and local Web/IPC loopback-only. See [ADR 0016](adr/0016-remote-execution-and-observation.md).
- [x] Implement native remote launch/attach, remote cwd/env/timeout/rules, raw byte output, bounded milestone/rule evidence, disconnect survival, unchanged-key launch recovery and exact-run graceful/forced cancellation. Read-only attachment stops observation only; transport loss/stale evidence remain distinct from terminal outcomes. Linux regressions and Windows native/cross-host verification are recorded in [implementation status](IMPLEMENTATION_STATUS.md).
- [x] Observe an existing GitHub Actions attempt through documented read-only `gh api`, retaining status/conclusion, bounded job/current-step summaries and an attempt-specific Web link. Detail-query failure leaves the verified run result intact; authentication/rate-limit diagnostics remain bounded. Never dispatch/cancel/rerun on observation stop or timeout. Simulated transitions and a real completed workflow read are separate evidence.
- [ ] **P2:** qualify SSH-specific Linux/Windows shell/authentication deployments, additional scheduler adapters and prolonged disconnection/soak behavior. Native Windows cross-host verification uses the existing authenticated VM channel, not an SSH server. No framework coupling, checkpoint transfer or automatic training restart.
- [ ] **P2:** qualify external-task notifications through real ChatGPT Desktop/OpenCode continuation. Existing completion acceptance and source MCP/inbox routing do not independently qualify this new path.

## Runtime inspection and monitoring

- [x] Implement Agent-independent inspect/watch, revision-fenced persistent output/silence/elapsed rules, bounded recent tails, automatic tree cancellation, optional same-chat alerts and preserved archive/backup/audit history.
- [x] Adapt the embedded panel to narrow chat sidebars with expandable icon navigation, adjacent inline task details and stateful chevrons; automatically open the accepted task through the host Agent workflow before yielding or goal handoff.
- [x] Reuse the originating chat's same-profile embedded tab before opening another; handle repeated task/credential fragment navigation in place, with explicit limits when host tab discovery is unavailable.
- [x] Reuse one validated archive snapshot per task inspection and share bounded per-listener Web samples/serialization with one-second task and ten-second health expiry; keep explicit reads and controls direct, errors visible and profiles isolated.
- [x] Split runner capture/supervision and React components/connection handling, type execution states without changing wire/storage strings, fence stale rule responses by selected task and cover reconnect/visibility/authentication/revision conflicts and component rendering.
- [x] Bundle a React + Vite task manager in the native executable, default-on authenticated loopback access, exact-profile management commands and CLI/MCP/plugin entry points. See [runtime monitoring](runtime-monitoring.md).
- [x] Natively qualify monitor/Web UI/schema v9 on Windows, including independent runner detection/tree stop, bounded tails, silence rearming, authenticated HTTP control and installed-package resources.
- [ ] **P1:** test real running-alert continuation in ChatGPT Desktop/OpenCode. Existing completion acceptance does not qualify this path.
- [ ] **P2:** qualify Windows browser interaction with the bundled panel; native HTTP/package checks and Linux browser acceptance are already complete.

Runtime 0.1.12+ supports bounded structured progress and repeated stage/step/metric feedback alongside existing marker rules. Runner-owned detection, durable source sequences, revision/terminal fences, cooldowns, notification budgets and final milestone summaries share Event/Outbox and CLI/MCP/Web paths. See [mid-run feedback and checker recipe](progress.md). Real host acceptance remains distinct from recorded/acknowledged alerts.

- [x] Provide Agent/plugin guidance and a reference checker recipe reusing existing rules for checkpoints, milestones and external status. Install initial rules before flushed markers, bind the actual caller, pin observed-run identity and bound checks with deadline/backoff; verify through native runner and fake-host infrastructure without a live Agent polling loop.
- [x] Implement versioned bounded stage/step/total/metric samples, stage/step/threshold conditions, stable source-sequence deduplication, cooldown/rate limits and durable evidence. Preserve important retained milestones in terminal summaries when completion supersedes delivery; share Event/Outbox and CLI/MCP/Web, covering duplicate/out-of-order samples, recovery, revisions and terminal races.
- [ ] **P2:** qualify structured progress and checker workflows through Windows native/installed entry points and real ChatGPT Desktop/OpenCode mid-run continuation. Published 0.1.12 Windows storage/progress checks do not independently qualify installed checker workflows or mid-run host continuation. Prolonged high-rate training telemetry remains outside this bounded progress interface.
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
