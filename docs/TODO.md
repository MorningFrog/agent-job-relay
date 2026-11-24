# TODO / Roadmap

[Documentation index](README.md) · [Implementation status](IMPLEMENTATION_STATUS.md) · [Live verification](live-e2e.md)

## Scope and baseline

Distribute one Rust/MCP runtime through ChatGPT Desktop/Codex, Claude Code and OpenCode plugins. Pushing, publishing and official submissions require final confirmation. Release operations are manual. GitHub Release 0.1.27 remains the published baseline. Source 0.1.28 covers the Actions release route and Goal-lifetime default pause approval. See the [release handoff](release.md#current-release-handoff).

This is the sole project backlog. Current functionality, completed acceptance and version-specific evidence are maintained in [implementation status](IMPLEMENTATION_STATUS.md) and [live verification](live-e2e.md); changes by version are in [CHANGELOG](../CHANGELOG.md). Keep future work and explicit deferrals here.

## CI/CD

- [x] Restore GitHub Actions as the repository's CI/CD provider. Keep Linux/Windows native and installed-package checks, separate minimum Rust compilation and macOS ARM64 compilation. Remove Woodpecker project workflows and their private exchange/cache tooling.
- [x] Qualify a GitHub-hosted main push, including Linux and Windows installed-package checks. See the [CI evidence](IMPLEMENTATION_STATUS.md#current-verification).
- [ ] Qualify the GitHub-hosted PR path, then configure the `CI required` ruleset check with maintainer authorization. Retire any external repository integration separately.
- [ ] Qualify GitHub-hosted npm staging, maintainer approval and independent public verification. Preparation and read-only preview evidence belongs to each version’s original Actions candidate. Preserve original provider candidates and published receipts.
- [ ] Qualify macOS runtime/installation/host behavior in a separate task. Compilation and cross-build checks do not close this item.

## Host integration and distribution

- [x] Global npm CLI/plugin update and coordinated removal, profile-preserving recovery, same-chat consent/handoff/feedback, current-runtime MCP forwarding and version/pin finalization are implemented. Published history and original receipts remain immutable; current verification and host limits are recorded in the linked handoffs.
- [ ] Diagnose the failed independent 0.1.21 [Check Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/41470697500), then rerun read-only verification against original candidate 41466809929 without rebuilding or republishing. Metadata/receipt checks do not replace public-byte/channel/installation verification.
- [ ] Resolve the failed independent Check Release for published 0.1.18 against its original candidate, without republishing or replacing released bytes. Later publication and verification do not recheck that older candidate.
- [ ] **P1:** natively qualify Windows self-update/self-removal with loaded global CLI/daemon/MCP images, qualify active-run package replacement separately, and verify actual Desktop refresh/reconnection. Public-host fixtures and Linux package replacement do not establish these outcomes.
- [ ] **P1, maintainer:** verify revocation/removal of the unused bootstrap publication token/secret; successful OIDC publication does not prove credential cleanup.
- [ ] **P1:** verify additional ChatGPT Desktop versions, Windows single-command authorization under restricted permissions, results completed while ChatGPT Desktop is closed, real multi-day renewal and receiver deduplication. Keep the completed Linux/Windows same-chat continuation acceptance complete; do not repeat it merely to refill evidence.
- [ ] **P1:** qualify the source 0.1.27 Goal envelope fix in installed Desktop/real-model use, including fast completion and feedback re-pause. The 2026-11-12 user failure is traced to 0.1.26 rejecting native `inputText`; source accepts it and normalized `text` while retaining owning-host idle checks, original IDs/accounting and delivery fences. Protocol regressions do not close installed acceptance. Capture original wait/control/delivery/host state for other resume failures and the separate earlier `blocked` report.
- [ ] **P1:** qualify actual Desktop UI/real-model temporary goal feedback and re-pause, including final-completion races, interruption and goal changes; qualify Windows controller/installed workflow separately. Actual Codex with a simulated model/endpoint covers the protocol, not these host acceptance results.
- [ ] **P1:** qualify the new direct MCP form in actual Desktop and a specifically observed real-model three-round workflow, including initial permission, one natural reply, decline/no reply and plugin-text refusal. Verify that the default approval is reused after more than 16 jobs and a later same-binding continuation, with no additional consent prompt. For explicitly bounded approvals, verify that the Agent continues authorized foreground work after the last slot. Retain evidence for other post-completion host blockers. Windows goal/controller, changed-host goal handling and long-duration savings remain separate. Consent-free suspension and stronger replacement fencing require host support; do not reopen the already verified approval→pause behavior.
- [ ] **P1:** obtain OpenCode real-provider, server restart/recovery and broader version evidence. Keep V2 as a separate future entry; consolidate shared descriptor metadata only where useful and test each actual loader.
- [ ] **P1, community help:** verify Claude's actual loader, skill session substitution and account-backed MCP workflow. Research public asyncRewake/channels before implementing continuation; keep the scaffold inbox-only until verified. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- [ ] **P1:** qualify host-driven bootstrap and authenticated artifact provenance. Offline hashes and loopback registry checks do not establish online publisher identity.
- [ ] **P2, later authorization:** prepare official-platform eligibility, publisher identity, listing assets, permissions and review materials. Recheck OpenAI local-MCP eligibility, Claude directory requirements and OpenCode's supported package mechanism; publication alone does not establish official approval.

## Runtime and maintenance

- [x] Native execution, maintenance, recovery and their regression coverage are recorded in [implementation status](IMPLEMENTATION_STATUS.md).
- [ ] **P1:** natively qualify automatic daemon handover and installed update on Windows, including host cleanup/CIM fallback and held MCP executable images. Linux regression evidence does not establish Windows or new real-host acceptance.
- [ ] **P1:** natively qualify the experimental macOS ARM64 identity/lifecycle/IPC, filesystem recovery and install/update/remove paths; investigate containment for escaped descendants/runner loss. Implemented adapters and cross-build packaging do not establish runtime support; runtime qualification is deferred.
- [ ] **P1:** strengthen containment after runner death and against adversarial escape; qualify physical disk exhaustion/power loss and longer-duration load. Live-runner adopted-child cleanup does not run after SIGKILL.
- [ ] **P1:** strengthen per-call identity authentication for shared hosts. Current caller identity is trusted input; never infer it from a latest-session record.
- [ ] **P1:** extend the ten-minute Linux maintenance/load baseline to multi-day, physical disk-pressure and Windows soak scenarios. Independent-storage backup automation and destructive history purge need separate designs; uninstall remains data-preserving.
- [ ] **P1:** extend interrupted-operation handling to setup/marketplace cross-file writes, native offline uninstall and pre-journal/legacy remnants. Existing per-directory install/cache repair finishes recorded replacements forward and preserves backups/history.
- [ ] **P2:** negotiated standard MCP Tasks and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats; progress work is tracked under [runtime inspection and monitoring](#runtime-inspection-and-monitoring).

## Parallel task submission

- [x] Shared CLI/MCP batch admission and bounded FIFO queuing preserve independent specs, outcomes, IDs, budgets and unchanged-key retries; Linux native and simulated-host coverage is accepted.
- [ ] **P1:** qualify bounded queuing through native Windows and installed host workflows, including loaded-image updates with older-schema work. Linux native/portable storage fixtures do not qualify those paths.

The runtime executes independent jobs concurrently, with eight execution slots per profile and separate IDs, workspaces, logs and notifications. Runtime 0.1.12+ submits 1–16 specifications through CLI `start-batch` / MCP `job_start_batch`, with ordered partial admission, required per-item keys and safe retry of unchanged specifications. See [batch usage](usage.md#submit-independent-commands-together-0112), `src/batch.rs` and `tests/runtime.rs`.

- [ ] **P2:** qualify batch submission through Windows native/installed entry points and real ChatGPT Desktop/provider-backed host calls. Published 0.1.12 installed-package checks do not independently qualify batch-specific host behavior.

## Remote execution and external-run observation

Runtime 0.1.12+ implements the [remote contract and reference path](remote-execution.md). Remote commands use an explicitly pinned native runtime profile (a copied executable is enough) or a separately qualified contract adapter. GitHub Actions uses local `gh` and an immutable repository/run/attempt. Runtime 0.1.25+ Woodpecker 3.x observation uses native HTTPS and pins the server/repository/pipeline/global ID/full commit. Ordinary local `ssh`/`gh` jobs retain local-process semantics.

- [x] Pinned native execution/observation, Actions read-only observation, direct remote-rule feedback and command inspection are implemented. Runtime 0.1.25 adds read-only Woodpecker observation; public Linux reads and regression fixtures pass, with platform/account-backed limits below.
- [ ] **P2:** qualify Woodpecker private-server authentication, native Windows/macOS and actual installed-Agent continuation; public reads and inbox fixtures do not establish these.
- [ ] **P2:** qualify SSH-specific Linux/Windows shell/authentication deployments, additional scheduler adapters and prolonged disconnection/soak behavior. Native Windows cross-host verification uses the existing authenticated VM channel, not an SSH server. No framework coupling, checkpoint transfer or automatic training restart.
- [ ] **P2:** qualify additional remote host/version combinations, OpenCode real-provider continuation and the specific direct `remote_alert` subscription in actual hosts. The user-confirmed general remote workflow does not identify these paths.

## Runtime inspection and monitoring

- [x] Shared CLI/MCP/Web rules, bounded progress/feedback, recent logs, history/log search, disk-pressure admission, task titles and compact task/notification/Goal diagnostics are implemented. Current source regression coverage and version-specific host/browser evidence are maintained in the handoffs.
- [ ] **P1:** qualify the new recent-log format, backup locking and CLI export on native Windows, and the changed panel/summary workflow in actual installed hosts. Current Linux fixtures/browser checks do not extend earlier Windows or real-Agent acceptance.
- [ ] **P1:** qualify autonomous adoption and measure context savings in real Desktop chats using natural-language long/quiet-task, explicit completion-only, stage, exception and missing-source scenarios. Record original-chat report/re-pause and final continuation independently of host acceptance. Verify real output routing, initial rules, elapsed fallback and bounded event handling without specifying tool APIs. Verify the new guidance against the existing user-confirmed feedback loop. Goal completion-only consent must not expand implicitly.
- [ ] **P2:** qualify Windows browser interaction with the bundled panel; native HTTP/package checks and Linux browser acceptance are already complete.

Runtime 0.1.12+ supports bounded structured progress and repeated stage/step/metric feedback alongside existing marker rules. Runner-owned detection, durable source sequences, revision/terminal fences, cooldowns, notification budgets and final milestone summaries share Event/Outbox and CLI/MCP/Web paths. See [mid-run feedback and checker recipe](progress.md). Real host acceptance remains distinct from recorded/acknowledged alerts.

- [ ] **P2:** qualify additional host/version combinations, OpenCode real-provider mid-run continuation and Windows native/installed structured progress/checker workflows. The user-confirmed real-Agent loop does not establish those unspecified environments; published 0.1.12 Windows storage/progress checks do not independently qualify installed checker workflows or mid-run host continuation. Prolonged high-rate training telemetry remains outside this bounded progress interface.
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
