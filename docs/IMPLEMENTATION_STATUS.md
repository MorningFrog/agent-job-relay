# Implementation status

[Documentation index](README.md) · [Live verification](live-e2e.md) · [Backlog](TODO.md) · [Changelog](../CHANGELOG.md)

## Current baseline

Current handoff: **2026-10-30 (Asia/Shanghai)**. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. This page keeps current applicable evidence; historical runs remain in Git.

| Baseline | Current state |
|---|---|
| Published | npm/GitHub **0.1.11**, confirmed through public release and both npm latest metadata; full public-byte/install checker evidence remains at 0.1.10 |
| Source | Cargo **0.1.12** with daemon upgrades, scoped goal consent/waiting, current-runtime forwarding, batch, external tasks, progress and subsequent Web UI/workflow changes |
| Storage/interfaces | One Rust package/executable, SQLite **v11**, IPC **v1**, **25 MCP tools**; published 0.1.11 baseline uses v9 and 22 tools |
| Release candidate | **0.1.12**, original Prepare Release **40908803482**, sealed on channel **latest** and unpublished; subsequent source UI/workflow changes are excluded |
| Native platforms | Linux and Windows verified for the applicable baselines below; macOS native execution/IPC unimplemented |
| Actual Desktop evidence | Completion continuation on Linux/Windows v0.1.8; user-confirmed current Linux goal pause after approval |

## Implemented behavior

| Area | Current behavior |
|---|---|
| Native execution | Explicit argv/shell, validated cwd/env/timeout, durable original Job/Run/attempt, unchanged-key idempotency, independent runner/process groups/Windows Job Objects, conservative orphan handling and no execution replay |
| Batch | 1–16 independent specs with distinct keys, indexed partial admission and safe retries; eight active jobs per profile, independent rules/stop/notifications |
| Scoped goal consent | Structured `needs_confirmation` before wait/job creation, one negotiated MCP or Agent question, natural user reply, original chat/goal/workspace scope and transactional job budget; reconnect by request ID for later rounds, expiry/revocation/restore fences |
| Goal handoff | Originating native pause plus revision-fenced confirmation before `waiting`; frozen exact-run membership, accounting-preserving restoration, merged notification, deletion/cancellation/uncertain-mutation fences. Consent-free pause remains unavailable |
| Remote tasks | Pinned native launch/attach over bounded authenticated stdio, origin IDs, durable launch keys, byte cursors/reconnect and exact-run control; read-only fixed GitHub Actions attempt observation |
| Storage/maintenance | SQLite WAL/FULL/foreign keys, bounded workers/reserved reads, short transactions, guarded forward migrations, capped logs, transparent cold archives, daily verified local backups and new-profile restore without execution replay |
| Monitoring/progress/Web | Independent runner rules and bounded tails/evidence; structured stages/steps/metrics with durable sequence/cooldown/terminal fences; default-on authenticated loopback React/Vite panel sharing CLI/MCP controls; narrow icon navigation and inline selected-task details; source workflow opens accepted tasks beside the originating chat |
| Delivery/recovery | Transactional Event/Outbox, original IDs/TTL/attempts, per-chat claim exclusion, bounded retry, same-chat recovery and verified same-endpoint opted-in renewal; host acceptance remains distinct from execution |
| Caller identity | Fresh actual terminal/tool context, explicit workspace/session bindings, shared MCP never infers a caller from its process environment; cooperative identity is not host attestation |
| Installation/update | Separate runtime/profile, owned preview/apply/removal, explicit cache snapshots, retained backups and forward-only repair; applied updates restart an idle online daemon from the new installation, with pinned-peer/downgrade/migration fences |
| Stable runtime instructions | Cached skills and MCP initialization point to current `instructions`; retained managed MCP verifies receipt/profile/image and forwards catalogs/calls/native input, including same-version changes, without uncertain-call replay or cache scanning |
| Host packages | Experimental private ChatGPT Desktop adapter; Codex CLI inbox-only; OpenCode V1 public session bridge; Claude scaffold inbox-only pending account-backed loader evidence |
| Release delivery | Shared Linux/Windows npm runtime, OpenCode package and self-hosted marketplace; manual immutable Prepare/Publish/Check workflows. No independent OS installers |

Submissions/register operations start the selected daemon on demand. Doctor/status do not. Windows daemon-only CIM bootstrap authenticates both endpoints and avoids services/elevation; runners still require native breakaway. Configuration/marketplace writes are individually atomic, not a cross-file transaction. The declared Rust 1.89 minimum remains independently unverified.

## Current verification

| Check | Evidence and limit |
|---|---|
| Linux delivery gates | React/Vite build, fmt, all-target Clippy, 158 Linux tests (0 failed/ignored) and release build pass |
| Scoped consent/handoff | Controller digest reuse requires stable file identity and modification/change timestamps outside the filesystem ambiguity window; equal-length/preserved-mtime rewrites are rejected. Isolated Linux storage/CLI/MCP fixtures cover one approval for three separate actual submissions, no duplicate-budget use, caller/goal/workspace/revision fences, native accept/cancel, one-question fallback, expiry/deletion/revocation/restore and unchanged native-pause/recovery behavior |
| Retained managed MCP | Actual native image replacement and fixture workers cover same-version updates, original history/caller/profile, writer exclusion, catalog notices, lost-ACK non-replay, a 91-second human wait and modern input-required forwarding; real host form UI/Windows remain separate |
| Local installed update | The earlier installed source snapshot (versioned 0.1.11) matches its verified build, with schema v11/25 tools and original profile/history/backups preserved. A runtime-only update leaves the cached skill unchanged; its launcher and the retained actual host MCP both report the new workflow digest. This host advertises no native form support, so uses the Agent-question fallback; newly added tool discovery still requires host catalog refresh/verification |
| Codex loader | Actual Codex plugin loader discovers 25 current-source tools without daemon/workload creation |
| Codex goal integration | Actual bundled Codex plus simulated submitting Agent/endpoint/model proves owner pause notification, zero model-request growth during confirmed pause and resume/deleted/replaced behavior; it does not qualify real-model dialogue/UI |
| User goal acceptance | User confirms local goal mode pauses automatically after approval. Direct MCP form UI and complete three-round real-host restoration remain separate acceptance |
| Core Linux regressions | Native execution/process loss, SQLite transactions, IPC permissions, delivery/recovery, archives/restore, daemon upgrade, monitor/Web, batch and external-task suites. Release orchestration has 61 service-fixture checks; no public writes |
| Windows native | Current 0.1.12 source passes all four MSVC gates on GitHub Windows, 69 tests plus frontend/static-CRT build, including schema v11 storage/progress checks. Windows goal/controller and native-form acceptance remain separate |
| Installed packages/loaders | Current 0.1.12 revision CI and candidate preparation pass installed Linux/Windows npm commands, Windows shim, Web resources, history/relocation/tamper and actual Codex/OpenCode loaders. OpenCode uses a simulated model; real-provider acceptance remains separate |
| Release verification | [Revision CI](https://github.com/MorningFrog/agent-job-relay/actions/runs/40906662635), [Release All dry-run](https://github.com/MorningFrog/agent-job-relay/actions/runs/40908776616), [Prepare Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/40908803482) and [actual publisher OIDC preview](https://github.com/MorningFrog/agent-job-relay/actions/runs/40911023219) pass; downloaded candidate checksums, inventory, source and notes verified. Publication/public checking were skipped |
| Real host continuation | Linux/Windows Desktop v0.1.8 completion continuation and Windows cold-profile actual Codex MCP/model acceptance; current approval→pause is separately user-confirmed. Details and remaining host scenarios are in [live verification](live-e2e.md) |
| Browser | Final Linux bundled panel passes embedded-browser layout checks at 360/480/761/1250/1251/1440 CSS pixels without horizontal overflow, single adjacent inline detail through 1250 pixels, wide columns, hover/keyboard/backdrop navigation, search/collapse, stdout/stderr/events, rule application and observed stop. The release executable exposes the exact updated panel-opening workflow; a newly updated real host Agent following it remains separate acceptance |
| External integration | Linux→Windows native execution through the authenticated VM channel with injected disconnect/observer loss, one launch and exact-run cancellation; one real Actions attempt read. SSH-specific deployment/provider-backed continuation remains unqualified |

Use [testing](testing.md) for reproduction/coverage. Fake-host, injected expiry and reconstructed interruption states do not prove physical power-loss/ENOSPC, actual multi-day renewal or prolonged reliability. Generated reports/logs stay in ignored `.tmp/` or `target/`.

## Release and support boundaries

Version 0.1.12 has dated release notes, synchronized pins and reviewed original candidate **40908803482**. npm latest remains 0.1.11; no 0.1.12 publication occurred. Preserve the original candidate. Current Web UI assets and runtime instructions differ from its source, so publishing these changes requires a new version/candidate; recover_candidate cannot include them. Candidate Linux bytes require glibc 2.34; local source binaries require the build machine's compatible libc. Hashes alone do not authenticate a publisher. See [release handoff](release.md#current-release-handoff).

Windows goal/controller, direct native confirmation UI, full real-model multi-round restoration, changed-host goal handling and long-duration savings need separate qualification. Host goal generation/CAS is unavailable. Actual results completed while Desktop is closed, receiver deduplication and multi-day renewal remain open. Claude account-backed loading, OpenCode real-provider/V2, stronger containment, disk-loss qualification and cross-file install recovery remain in [TODO](TODO.md). No adapter automatically records `confirmed_resumed`.
