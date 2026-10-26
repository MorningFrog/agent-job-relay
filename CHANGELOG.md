# Changelog

## 0.1.0 — 2026-10-26 (unreleased development milestone)

- Independent Linux runner and process groups with durable launch claims,
  conservative recovery, capped logs, cancellation and timeout.
- SQLite jobs/runs/events, session leases, transactional delivery outbox,
  fenced attempts, retry/TTL/dead letters and exact target rebinding.
- Official rmcp stdio tools, Codex configuration and notification inbox.
- Opt-in experimental Desktop Unix socket transport with automated fake-host
  tests; user-confirmed basic live E2E on ChatGPT Desktop 26.1021.22027 /
  Ubuntu 26.04, with local artifacts cross-checked (docs/live-e2e.md).
- Dry-run setup, backed-up owned configuration edits, safe integration removal,
  CLI diagnostics and bilingual onboarding.
- Linux behavior tests and compile/test CI definition; future packaging roadmap.

No packages published or Windows/macOS runtime support claimed. Basic live wake
validation is limited to the recorded client/OS; no automated confirmed-resume
adapter exists. See docs/IMPLEMENTATION_STATUS.md for validation evidence.
