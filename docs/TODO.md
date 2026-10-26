# TODO / Roadmap

Status labels are **Deferred / Not implemented** unless marked otherwise in
IMPLEMENTATION_STATUS.md. No public release, system installer or package repository exists.
Core correctness and usable local source builds have priority over distribution.

## Runtime and integrations

- P0: qualify the installed Desktop Plugin/MCP entry point with one real task
  whose prompt needs no tool/state/session paths. Local install, marketplace
  setup/removal and two callers through shared MCP are implemented and tested
  with fake Desktop; plugin browser discovery/install and installed real-host
  workflow remain unverified. Existing three-round CLI live results need no
  repetition. Follow installation.md and testing.md; fix demonstrated user-path
  failures before expanding manual test matrices.
- P1: extend the user-confirmed Desktop live E2E passes (ChatGPT Desktop
  26.1021.22027 / Ubuntu 26.04; see live-e2e.md) to in-flight daemon interruption
  and completion while offline, Desktop restart, explicit rebind, temporary
  delivery failure and concurrent exact routing. CLI shortcut first start,
  registration reuse and idle-daemon stop/auto-start passed; the user confirmed
  all three continued without another message and the control chat stayed quiet.
  Keep private evidence local, revalidate API compatibility for each client
  version and never treat host acceptance as automated resume confirmation.
- P1: Linux cgroups or equivalent optional containment for descendants that escape
  process groups; stronger containment after SIGKILL of the runner. Existing
  process-group cancellation is for non-daemonizing trusted workloads.
- P1: real ENOSPC/power-loss VM tests and longer soak tests. Existing regression
  injects output-open/transaction failures, stale identities and vanished runners.
- P1: Windows native named-pipe SID/DACL checks, endpoint identity and Job Objects,
  documented survival policy and native process-tree tests. Do not emulate setsid.
- P1: macOS process instance identity, native lifecycle/restart tests and IPC audit.
- P1: opt-in public Codex app-server integration only for an explicitly owned
  writer/session; handle active turns, approvals, auth and reconnect safely.
- P1: lease renewal for jobs longer than 24 hours, authenticated per-call identity
  handoff for shared MCP hosts, and real-host MCP wake_me=true shortcut validation
  when per-session inherited context is available. CLI --wake live verification
  passed; never infer a current chat from a global latest-session record.
- P1: inspect all active runner/host health from doctor; versioned config file and
  tunable resource budgets; bounded structured progress event protocol.
- P1: atomic schema migration fault tests across every supported version; online
  backup/export and explicit retention preview/cleanup. `data purge` requires a
  separate confirmation design; no destructive cleanup command exists today.
- P1: setup receipt/config crash recovery and Windows atomic replacement tests;
  broader official client formats only when verified. No automatic ChatGPT
  configuration path is guessed. Preserve comments and foreign entries.
- P2: optional standard MCP Tasks after negotiated client support. Tasks are not
  automatic Agent conversation wake. OpenCode adapter only with verified APIs.

## Distribution & Packaging — Deferred / Not implemented

Desktop-only public Plugin Directory submission is also deferred. Local Plugin
authoring/installation is implemented, but public local-MCP submissions currently
require approved-partner eligibility. Confirm that separately before any later
submission; do not replace local execution with a public command-execution service
merely to obtain a listing. No upload, publication or submission was performed.

Every channel must preserve task history, logs and user configuration on normal
uninstall. Integration removal, service removal and data purge are separate.
Do not implement distribution in the current core milestone.

| Priority | Platform/channel and deliverable | Install / update / uninstall design | Limits and acceptance tests | Future GitHub Actions plan |
|---|---|---|---|---|
| P1 | Linux/Windows/macOS GitHub Releases: native binaries + archives | Download/extract; replace after checking active jobs; remove binary while keeping state | Architecture detection, SHA-256, provenance, clean startup, upgrade/downgrade schema safety | Authorized tag, matrix builds, checksums, signed metadata, protected Release upload |
| P1 | Debian/Ubuntu `.deb` and signed APT repository | Package install/upgrade/remove through APT; no default state purge | Dependencies, user-service ownership, maintainer-script rollback; clean install/upgrade/remove with running-job checks | Evaluate cargo-deb; build package, test containers/VMs, sign index, authorized APT publish |
| P1 | Fedora/RHEL `.rpm` and DNF/YUM repository | DNF install/upgrade/remove; keep user history/config | Distro differences, signed repository metadata, service boundaries, migration rollback smoke tests | Evaluate cargo-generate-rpm; isolated RPM jobs, signing and authorized repository sync |
| P1 | Windows `.exe` installer (evaluate NSIS/WiX) and WinGet manifest | User-level install/update/uninstall; reversible PATH and owned integration entries | x64/ARM64, signatures, locked executables, background jobs, no forced job termination/data loss | Windows build/sign/VM smoke; Release assets; separate reviewed WinGet manifest submission |
| P1 | macOS `.dmg`, evaluate `.pkg`; Homebrew formula/tap | Document CLI location, upgrade and uninstall; separate opt-in LaunchAgent | Intel/Apple silicon, signing/notarization, quarantine, service/data boundaries | macOS package/notarize; install/upgrade/remove tests; authorized Homebrew manifest publication |
| P1 optional | PyPI binary Wheels + `uv tool` installation | Evaluate maturin binary-wheel mode; versioned install/upgrade/uninstall via uv | Verify package name, platform tags, Python/uv prerequisites and native binary behavior. A Rust crate alone is not a uv tool package | Build/test wheels, clean uv environments, trusted publishing/OIDC only after authorization |
| P2 | Linux `.AppImage` | Portable executable; replace/remove file; explicit PATH/integration setup | Portability is not package-manager support. Prove CLI invocation, FUSE behavior, daemon separation and state retention | Linux image build, clean desktop/CLI smoke, checksums and Release asset upload |
| P2 feasibility | Flatpak and Flathub manifest | Sandbox-aware install/update/uninstall; preserve state | Host command execution, MCP config, Unix sockets and daemon service access may conflict with sandbox. Document justified permissions; no silent weakening | First feasibility VM tests; then manifest/build/smoke and separate Flathub review/submission |

### Work items required for each channel

1. Feasibility research and ADR: select actual supported formats/tools at that time
   (`cargo-dist`, `cargo-deb`, `cargo-generate-rpm`, `maturin`, OS-native/Flatpak
   tools). One tool need not generate every format.
2. Packaging scripts/manifests with explicit install location and ownership
   receipts; upgrade, rollback, schema compatibility and uninstall behavior.
3. Separate build/release workflows with minimum permissions, protected
   environments, authorized version tags, traceable source and reproducibility
   metadata. No credentials, release tags or publishing permission configured now.
4. Platform install/run/upgrade/uninstall smoke tests in isolated machines,
   including active jobs, services, foreign MCP entries and retained data.
5. SHA-256, signing/certificate management, macOS notarization, attestations;
   protected Secrets or OIDC with explicit user authorization.
6. Authorized GitHub Release publication, followed by separately maintained
   APT/DNF indices, PyPI publishing or reviewed WinGet/Homebrew/Flathub manifests.
   Building an asset does not make it listed in a package manager.
7. Synchronized English/Chinese install, update, troubleshooting, rollback,
   uninstall and optional explicit data-cleanup documentation.

Planned flow: test matrix → authorized tag/Release → native builds → packages →
isolated lifecycle smoke tests → checksum/sign/notarize/metadata → GitHub Release
→ separately authorized repository/store submissions. Current CI only tests and
builds. Never push tags, publish, submit manifests or provision online credentials
without explicit authorization.

Candidate future uv command: `uv tool install agent-job-relay` — **Deferred /
Not implemented / not published**; do not run it as a current install instruction.
