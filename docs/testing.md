# Automated testing and evidence

For reader-facing installation and Desktop acceptance steps, use [manual testing](manual-testing.md). This guide covers developer verification. Tests keep state, fake endpoints and logs under repository-local `.tmp/`; they do not modify personal settings, services, Vanth state or real Desktop endpoints.

## Required checks

Run from the repository root:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

The current verified baseline is 71 passing tests; exact toolchain/version and limitations are in [implementation status](IMPLEMENTATION_STATUS.md). CI defines full Linux behavior tests plus macOS/Windows compilation and portable library checks. A configured matrix does not establish that remote CI ran or those runtimes work. The declared Rust 1.89 minimum is not independently verified.

## Behavior coverage

| Suite | Coverage | Boundary |
|---|---|---|
| Library | MCP schemas and legacy launch-readiness checks | Local checks |
| Runtime | Execution/output/cursors, idempotency, process trees, cancellation/timeout, daemon/runner independence, launch recovery, quotas, IPC/security | Real Linux children and injected failure windows |
| Delivery | Terminal/outbox transactions, migrations, exact routing, fenced attempts, retries/TTL/dead letters | Real SQLite and persistence simulations |
| Desktop | Framing, catalog/identity checks, endpoint replacement and fenced lease renewal | Fake hosts; no real restart/multi-day proof |
| Convenience | CLI/MCP binding, multi-caller routing, installed shared MCP, loaded-image spawn, recovery and long-job renewal | Real CLI/MCP processes plus fake hosts |
| Integration | Preview/apply, comments/foreign-entry preservation, ownership refusal and removal | Isolated files |
| Onboarding | Installed launcher/profile, manifests, marketplace setup, active-runtime refusal and data-preserving removal | Isolated installed processes/files |
| Plugin | Relocated/toolchain-free bootstrap, package validation, preserved DB/marketplace, custom-root defaults, profile/downgrade rules, cache refresh/refusal | Nine Linux tests; local payload/resource-change fixtures |
| Legacy upgrade | Verified same-profile stop, peer identity checks, lock release and immediate replacement | Std-only legacy fixture; no unrelated peer signals |
| Recovery | Scoped preview/apply, pagination, rollback/audit, preserved delivery IDs/history/TTL, accepted/expired exclusion and stale-claim fencing | Isolated storage/endpoint fixtures |
| Documentation | Bilingual introductions, language links and README local links | Static acceptance |

Focused commands are `cargo test --test <suite>`, using the lower-case suite filenames under `tests/`. Waits have deadlines; harnesses stop/reap only their own isolated processes. Output-open faults and injected expiry/boot/lock conditions are not physical ENOSPC, power-loss or multi-day tests.

## Actual Codex plugin-loader check

After the release build, with Codex CLI and Python available:

```bash
python3 scripts/check_codex_plugin.py
```

This exports and relocates an offline bundle, bootstraps in disposable repository-local HOME/CODEX_HOME, removes the delivery artifact, installs through the actual loader, checks its returned cache path/profile and no-op refresh, and requires 20 MCP tools including job_recover_session. It asserts that no daemon/workload state was created. Reports/logs remain in the printed ignored `.tmp/loader-...` directory. Python is a developer-check dependency only.

The recorded loader is Codex CLI 0.164.0. Portable root and PLUGIN_ROOT command/args/cwd behavior differed from documentation; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). The loader check starts no model turn and establishes neither Desktop browser discovery nor real MCP task/wake behavior.

## Evidence boundaries and live acceptance

| Evidence | What it establishes |
|---|---|
| Configuration preview/match | Owned files are configured as intended |
| Local stdio self-check | Executable can initialize and expose tools |
| Actual loader discovery | The tested loader finds the plugin's MCP declarations/tools |
| Host `job_doctor` | A real host tool call reached MCP |
| `accepted_by_host` | Host acknowledged that delivery |
| Observed same-chat continuation | Agent actually resumed for the matching result |

Past explicit-registration/CLI wake and installed CLI-fallback results, with provenance, remain in [live-e2e](live-e2e.md). The last personally installed v0.1.2 MCP connection is user-confirmed; source v0.1.4's real installed MCP submission/wake and plugin-browser installation remain separate open checks. Follow [manual acceptance](manual-testing.md) using the next naturally long real task. Do not repeat passed CLI workloads, add artificial delay or require a manual crash matrix to fill those gaps.

Resource-only updates and downgrade fixtures deliberately modify package metadata/hashes to test management behavior. They do not authenticate release provenance. No automatic adapter records confirmed_resumed; host acceptance and observed execution stay separate. Deferred native-platform, containment, disk/power-loss and artifact-provenance work is tracked in [TODO](TODO.md).
