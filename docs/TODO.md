# TODO / Roadmap

## Current distribution decision (2026-10-26)

The user's latest direction supersedes the original system-package roadmap:
**distribute through Agent plugins**, with one shared Rust/MCP runtime and thin
host-specific integrations. Aim for the official supported plugin installation
surfaces of ChatGPT Desktop/Codex, Claude Code and OpenCode. Easy installation,
updates and data-preserving removal are product requirements.

The project is still in development. **Do not publish, submit, push or configure
online credentials now.** Local plugin development, validation and documentation
come first. An installable local plugin is not an official directory listing.

## Plugin delivery and integrations

- P0: finish qualifying the repaired Codex/Desktop installed MCP path. The user's
  release task loaded the plugin skill and resumed successfully through its CLI
  fallback, but initially hit an old daemon with a removed executable. It did
  not call MCP. Version 0.1.1 fixes loaded-image Runner spawning, rejects the
  affected legacy daemon before another job, and emits the explicit Codex
  compatibility manifest. Isolated Codex CLI 0.164.0 now discovers all 19 tools.
  The subsequent 0.1.2 upgrade repair gracefully stops pre-shutdown daemons via
  verified Linux pidfds and selects state with daemon stop --install-root. All
  50 tests passed; the user authorized upgrading the actual personal install and
  host plugin cache to 0.1.2. Installed/cache MCP self-check finds 19 tools.
  The user subsequently confirmed the updated Desktop MCP connectivity test
  passed following the host job_doctor procedure (user-attested, no new tool
  transcript reviewed). Remaining coverage is a naturally long real MCP task's
  submission and same-chat continuation; do not repeat connectivity or CLI tests.
- Implemented baseline: no-override local updates retain the configured state;
  regressions cover this and replaced-image Runner launch. Keep configuration,
  daemon readiness, local MCP self-check, real host discovery and same-chat
  continuation as separate checks when extending the integrations.
- P1: Claude Code plugin adapter: implement its manifest/MCP/skills layout,
  locally validate installation and tools, then qualify exact session identity
  and the host's supported completion/continuation mechanism. Current Desktop
  socket transport is not a Claude adapter. Do not promise automatic wake until
  it is observed. Reuse the Rust runtime instead of copying another runtime.
- P1: OpenCode plugin adapter: choose and record the target V1/V2 API, implement
  its official local/package plugin mechanism, MCP bridge and owned-session
  routing. Qualify notifications versus actual continuation separately. OpenCode
  V1/V2 configuration and plugin APIs differ; no placeholder adapter is shipped.
- P1: plugin-managed native runtime bootstrap/update/cache handling so eventual
  users need neither Rust nor manual executable paths. Native artifacts may be
  required per OS/architecture inside the plugin delivery mechanism; this is not
  a return to independent OS installers. Linux is the only implemented runtime.
- P1: generate host-specific plugin descriptors from shared metadata where useful;
  validate each host loader instead of assuming one portable manifest works
  everywhere. The current Codex loader ignores the portable MCP declaration in
  this layout, so the local install uses .codex-plugin/plugin.json + .mcp.json.
- P1: preserve user-modified files, other plugins and all histories on update/
  removal; distinguish marketplace source removal from host cache uninstallation.
  Review interrupted setup/update/removal recovery without unsafe takeover.
- P2, future authorization required: prepare official plugin platform submissions,
  publisher identity, listing assets, permissions, update provenance and required
  reviews. OpenAI local-MCP submission eligibility needs separate confirmation.
  Claude Code provides marketplaces and an Anthropic directory submission route.
  For OpenCode use its officially supported package/plugin mechanism; do not
  equate npm publication with approval by an official directory. No submissions
  or publications have occurred.

References checked for this direction:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://code.claude.com/docs/en/plugins
- https://code.claude.com/docs/en/plugin-marketplaces
- https://opencode.ai/v2/docs/plugins

## Runtime backlog

Prioritize demonstrated user problems and normal lifecycle behavior. The existing
CLI live passes do not need repetition with another workload or an artificial
manual crash matrix.

- P1: normal host restart/rebind and temporarily unavailable delivery recovery;
  concurrent exact routing, version-specific compatibility and event deduplication.
- P1: Linux containment for descendants escaping process groups and runner death;
  longer soak and controlled disk/power-loss tests. Existing process-group
  cancellation is for trusted, non-daemonizing workloads.
- P1: Windows SID/DACL, named pipes and Job Objects; macOS native process identity,
  lifecycle and IPC. OS runtime support still requires native verification even
  when distribution uses plugins.
- P1: leases beyond 24 hours and authenticated per-call identity for shared hosts;
  never infer the caller from a global latest-session record.
- P1: backup/export, migrations, retention previews and configurable budgets.
  Data purge remains a separate explicit design, not part of plugin uninstall.
- P2: negotiated standard MCP Tasks, richer progress, optional public Codex writer
  integration only for explicitly owned sessions. Tasks alone do not wake chats.

## Cancelled system-package plan

No longer planned: .deb/APT, .rpm/DNF, AppImage, Windows .exe installers/WinGet,
macOS .dmg/Homebrew, Flatpak/Flathub, PyPI wheels/uv tool and their package-repository
release pipelines. The original requirement to maintain those as future delivery
work has been withdrawn by the user. No installer, package repository, store
submission, signing/notarization or automatic publication workflow was implemented.
Do not reopen this plan without a new user request.
