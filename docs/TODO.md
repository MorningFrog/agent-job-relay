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
- [ ] **P2:** negotiated standard MCP Tasks and optional public Codex writer integration for explicitly owned sessions. Tasks alone do not wake chats; progress work is tracked under [runtime inspection and monitoring](#runtime-inspection-and-monitoring).

## Parallel task submission

The runtime already executes independent jobs concurrently, with a maximum of eight active jobs per profile and separate IDs, workspaces, logs and notifications. Repeated `job_start` calls already use this capability; the missing convenience is submitting several commands in one request. See `StartSpec` in `src/core.rs`, admission in `src/storage.rs` and `concurrent_jobs_distinct_workdirs_quota_and_pagination` in `tests/runtime.rs`.

- [ ] **P2:** add bounded batch submission through shared CLI/MCP behavior so an Agent can submit multiple independent commands at once. Keep per-command argv/shell choice, cwd/env, caller binding, rules, idempotency keys and Job/Run IDs; return an input-to-result mapping including individual admission failures. Define quota/partial-admission behavior and safe retry after a lost response without duplicating accepted work. Preserve independent completion, notification and cancellation; verify overlapping execution, mixed failures, quota races and same-chat routing. This does not require a dependency graph or workflow scheduler.

## Remote execution and external-run observation

Remote execution is not implemented. Starting `ssh`, `gh` or a checker as a local job supervises that local process; it does not establish ownership, survival, completion or cancellation of remote work. The following entries add future scope requested by the user; they do not enable the remote-worker features deferred by the original specification.

Recommended approach: combine a small enforced integration contract with reference adapters and Agent-authored scripts. AgentJobRelay should enforce identity, lifecycle, bounded evidence and delivery semantics for integrations claiming durable remote support; the Agent can choose SSH, an existing scheduler or an optional remote AgentJobRelay runtime to suit the host. Avoid requiring a particular deep-learning/reinforcement-learning framework or a new distributed orchestration platform. A reference-only approach leaves recovery/stop semantics unverifiable by the runtime; a mandatory training framework would constrain existing environments unnecessarily.

- [ ] **P2, design first:** define the shared remote execution/observation contract and reference integration path. Record explicit target identity, remote execution ID/attempt, launch versus attach intent, capabilities, reconnect/cursor behavior and the original local Job/Run/session mapping. Keep credentials in existing SSH/authentication facilities rather than logs or event payloads; retain loopback-only local Web/IPC access. Advertise unsupported operations explicitly and distinguish transport loss, stale evidence and unknown outcomes from remote failure/completion.
- [ ] **P2:** support launching and reattaching to remote server commands, including deep-learning/reinforcement-learning training. Prefer an owned remote runner or a scheduler's durable job identity; a bare SSH client is only a limited transport. Specify remote cwd/env, stdout/stderr, checkpoint/milestone observation, disconnect survival and exact-run graceful/forced cancellation. Reconnect to the recorded run without replaying an uncertain launch or restarting training; stopping a local SSH/checker process must never claim that training stopped. Cover lost launch acknowledgements, duplicate requests, reconnect, remote runner/host loss and identity-fenced stopping before advertising durable support.
- [ ] **P2:** add observation of existing GitHub Actions runs through documented `gh`/GitHub APIs or an Agent-authored watcher. Bind an explicit repository, run ID and attempt so reruns cannot silently change the target. Observe queued/running/final status and preserve the final conclusion/URL; use bounded requests, retry/backoff and authentication/rate-limit diagnostics without Agent polling. A local watcher timeout/disconnect is separate from the workflow result. Default to observation; stopping observation must not cancel the workflow or dispatch another run. Qualify real external state and same-chat notification separately from simulated API tests.

## Runtime inspection and monitoring

- [x] Implement Agent-independent inspect/watch, revision-fenced persistent output/silence/elapsed rules, bounded recent tails, automatic tree cancellation, optional same-chat alerts and preserved archive/backup/audit history.
- [x] Bundle a React + Vite task manager in the native executable, default-on authenticated loopback access, exact-profile management commands and CLI/MCP/plugin entry points. See [runtime monitoring](runtime-monitoring.md).
- [x] Natively qualify monitor/Web UI/schema v9 on Windows, including independent runner detection/tree stop, bounded tails, silence rearming, authenticated HTTP control and installed-package resources.
- [ ] **P1:** test real running-alert continuation in ChatGPT Desktop/OpenCode. Existing completion acceptance does not qualify this path.
- [ ] **P2:** qualify Windows browser interaction with the bundled panel; native HTTP/package checks and Linux browser acceptance are already complete.

Current rules cover a known one-time milestone: an Agent-authored workload/checker can emit a distinctive, flushed stdout/stderr marker, and an initial `contains` rule with `notify_agent=true` can record it and enqueue notification to the explicitly bound chat while work continues. Runner-owned detection survives viewer/daemon loss, matches across output chunks and beyond the saved log prefix, and fences revisions/terminal states. The runner executes no arbitrary monitor callbacks; a checker must be an explicitly submitted workload or wrapper. A separate checker alerts for its own Job ID, so it must retain the observed workload/run identity. See [monitoring behavior and limits](runtime-monitoring.md) and `src/monitor.rs` / `tests/monitor.rs`.

This is sufficient for a small set of predefined output markers, but not a complete general progress interface: there are no native structured metrics or numeric thresholds, rules inspect future output only, each revision fires once, and each job retains at most 16 rule identities and 128 revisions per rule. Scripts can evaluate thresholds or file/API/checkpoint state and emit markers; buffered output can delay detection, and an unresolved alert can be superseded by terminal completion. Real mid-run Agent execution remains unqualified; a recorded alert or host acknowledgement does not prove continuation.

- [ ] **P2:** provide Agent/plugin guidance and a reference checker recipe that reuses existing rules for training checkpoints, milestones and external status. Install initial rules before emitting markers, flush output, bind the actual caller, retain observed-run identity and bound checks with timeouts/backoff. Keep external data untrusted and avoid requiring a live Agent polling loop. Verify the checker pattern through existing runner/fake-host infrastructure; do not duplicate the implemented monitoring engine.
- [ ] **P2:** evaluate structured progress and repeated milestone notifications where checker recipes are insufficient for concrete training/external-run use cases. Define versioned bounded stage/step/total/metric payloads, threshold or stage-change conditions, stable source-event deduplication, cooldown/rate limits and durable evidence. Preserve revision/session/terminal fences and define how important milestones remain visible in the final summary if terminal completion supersedes delivery. Reuse the Event/Outbox and CLI/MCP/Web paths; cover duplicate/out-of-order samples, recovery and terminal races before extending the current once-per-revision policy.
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
