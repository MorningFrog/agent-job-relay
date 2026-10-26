# TODO / Roadmap

## Scope

Distribute through Agent plugins around one Rust/MCP runtime, targeting ChatGPT Desktop/Codex, Claude Code and OpenCode. Local development, easy installation/update and data-preserving removal come first. **Prepare and verify now; publish/push only after final confirmation.** The npm account is awaiting verification and package names are provisional. Release operations must be manually triggered. Official directory approval remains separate.

## Current baseline

- Source **0.1.6**, Linux only: offline native plugin export/bootstrap/update, explicit checked cache refresh, pinned profiles, owned configuration, shared-MCP caller binding, public host-plugin bridge, TTL-bounded retries, same-chat recovery and verified delivery-time renewal are implemented and tested.
- The isolated actual Codex CLI 0.164.0 loader bootstraps a relocated offline bundle, checks its actual cache and discovers **20 tools**. Current read-only checks confirm installed native **0.1.5**, actual MCP connectivity and matching state; Desktop **26.1021.22027**/plugin enablement is user-attested.
- CLI live wake and installed MCP submission/same-chat continuation passed by user confirmation, with persisted task/log/delivery correlation. The 0.1.5 active-job restart/rebind/continuation check passed with measured endpoint change and matching audit/event/delivery; UI/version/restart are user-attested. Clean install and broader compatibility remain open. Detailed evidence belongs in [implementation status](IMPLEMENTATION_STATUS.md) and [live E2E](live-e2e.md).

## Host integration and plugin delivery

- **Completed P0 acceptance:** real installed MCP submission, successful quality-gate workload and completion-triggered same-chat continuation, with no CLI fallback, are user-confirmed and correlated with persisted records. Do not repeat this test merely to refill evidence; see [live E2E](live-e2e.md).
- **Completed live acceptance (2026-10-26):** Desktop 26.1021.22027/plugin 0.1.5 active-job restart/rebind and same-chat continuation passed. UI/version/restart are user-attested; bridge identity change, unchanged Job/Run, recovery audit, exit 0 and one accepted delivery to the recovered target are correlated with observed Agent execution. Clean UI installation and other host versions remain separate.
- **Completed local increment:** offline `plugin pack/bootstrap/update`, version/profile preservation, managed cache launchers and explicit snapshot inspection/refresh. Recipients need no Rust. Installation/cache upgrade provenance is separate from live wake acceptance; see [ADR 0007](adr/0007-offline-plugin-runtime.md).
- **Candidate implementation completed:** shared Linux native-runtime/OpenCode npm packages and `.agents/plugins/marketplace.json` for ChatGPT, with exact version pins, no install scripts, integrity/glibc checks and persistent profiles outside caches. Actual Bun name/version installation and Codex marketplace/MCP discovery pass through a loopback registry; npm publication, final package names and clean public-registry/Desktop UI acceptance remain pending. Offline installs retain their separate stable-root/cache lifecycle. See [release preparation](release.md).
- **Manual release workflow completed locally:** all gates, packaged host checks, downloadable artifacts and optional prerelease draft; no push/tag release or npm publishing. Windows/macOS remote checks pass. Current Linux retry includes retained SIGINT handling during blocked reconciliation and its deterministic regression. Workflows pin Node.js 24 actions and omit bulky test debug symbols without removing behavior cases.
- **P1:** finalize npm ownership/verification and protected publisher provenance before publication; review the same tested bytes, then verify public-registry installation. Native plugin artifacts do not revive independent OS packages.
- **Framework completed; community help needed:** Claude Code has documented manifest/MCP/skill layout, explicit session-substitution binder and inbox-only capability. Synthetic helper/actual MCP-entry checks pass, but no maintainer account or actual Claude loader/model test is available. Welcome account-backed tests and PRs; research public asyncRewake/channels before implementing continuation. See [host plugins](host-plugins.md) and [contributing](../CONTRIBUTING.md).
- **OpenCode V1 local increment completed:** target CLI/SDK/plugin 1.19.4, actual caller-context MCP binding and public session callback. Real loader/MCP/agent-loop continuation passed with a local simulated model and an untouched control session. Real-provider accounts, server restart/recovery and more versions need community evidence. V2 was researched separately and has no runnable entry point yet; Docker/version-matrix PRs are welcome.
- **P1:** consolidate shared descriptor metadata where useful, but test each real loader; V2 remains a separate entry. The offline Codex bundle keeps its stable absolute native command; the self-hosted candidate uses a pinned npm command without root-variable expansion.
- **Completed local increment:** durable install/update/cache replacement journals and explicit checked forward repair with retained backups/history. Setup/marketplace cross-file recovery, interrupted uninstall and pre-journal/legacy remnants remain follow-up work; distinguish source removal, host-cache uninstall and user-data deletion.
- **P2, later authorization required:** prepare official platform eligibility, publisher identity, listing assets, permissions, update provenance and review materials. Recheck OpenAI local-MCP eligibility, Claude marketplace/directory requirements and OpenCode's supported package mechanism; package publication does not itself prove official approval.

## Testing and maintenance

- **Completed:** testing/artifact rules now prioritize behavioral regression coverage; remove README wording/structure assertions, retain required fixtures and keep only concise current verification in documentation. Generated outputs/reports/logs remain ignored.

## Runtime backlog

- **P1, deferred user request (2026-10-26): task output monitoring and interruption.** Log reads/`logs --follow` and manual `job_stop` do not implement it. Define monitored conditions, Agent/runtime responsibility, who decides to interrupt, bounded output handling, duplicate-trigger and false-positive protection, and acceptance scenarios in a later increment. Record only until implementation is requested.
- **P1:** qualify events completed while Desktop is closed and real long-job renewal when an appropriate use case occurs. Active-job restart/rebind/continuation passed on Desktop 26.1021.22027/plugin 0.1.5; multi-day expiry is still injected. Broader host compatibility and receiver deduplication remain open.
- **P1:** strengthen Linux containment for escaped descendants/runner death; add longer soak and controlled disk/power-loss tests. Current process-group cancellation assumes trusted non-daemonizing workloads.
- **Windows compile verification completed:** Windows 11 VM/Rust MSVC 1.99.1 passes fmt/all-target Clippy/2 portable library tests/release build/help and explicit Linux-only refusal. This does not implement Windows background execution.
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
