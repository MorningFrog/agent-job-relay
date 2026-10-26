# Current limitations (0.1.0 development milestone)

- Linux x86_64 is the only locally compiled and process-tested runtime. Windows
  named-pipe ACLs/Job Objects and macOS instance identity/lifecycle are not
  implemented. Other OS runtime requests explicitly fail. CI is configured but
  has not been run remotely; it is not evidence of platform support.
- Codex CLI integration provides MCP tools plus a durable notification inbox.
  It does not start/resume an Agent automatically. No public app-server writer
  is owned by this program. No authentication or model invocation is performed.
- Desktop transport is opt-in, private and experimental. Automated transport
  tests use fake hosts. A basic live same-session test passed according to the
  user on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04, with local artifacts
  cross-checked (see live-e2e.md). This does not qualify other versions, host
  restart/rebind or recovery scenarios. An ack remains accepted_by_host;
  can_resume is request capability, not an automatic confirmation mechanism.
  There is no confirmed-resume adapter.
- Explicit host/session identities are supplied by the trusted caller. The
  private API cannot independently attest them. Socket fingerprint/lease changes
  require refresh or rebind; no magical historical conversation discovery.
- Job attempt=1. Idempotent submission returns the original job; it does not
  rerun failure. Use a new key/job for a deliberate rerun. Ambiguous launches
  become orphaned after a 5-second grace period and are never auto-replayed.
- Runner writes terminal state directly to SQLite while daemon is absent.
  Lost runner lock means orphaned, not success. SIGKILL/power loss cannot run
  Rust cleanup; Linux parent-death kills the direct workload, but descendants
  can escape process groups or remain after runner death. Audit external side
  effects before deliberately submitting again. Graceful cancellation handles
  non-daemonizing process groups, not adversarial containment.
- A machine reboot stops processes; persisted jobs are classified on restart,
  never promised to keep executing across shutdown. Fault injection simulates
  vanished locks/stale boot identity; actual power-loss/ENOSPC VM tests pending.
- Disk write failure is a failed job with diagnostics where DB remains writable.
  If SQLite itself cannot commit due to disk/full/corruption, recovery may only
  classify unknown/orphaned. No offline result journal or backup/export yet.
- Default maximum eight active jobs; no waiting pool beyond this quota, tuning
  config, structured progress, interactive stdin/PTY, schedules, SSH or artifacts.
- Per-stream logs cap at 16 MiB, then drain/discard. Text queries use lossy UTF-8;
  byte cursors can split multibyte characters. No automatic retention or purge.
- Every durable record is bounded by quotas but disk is not a reservation system.
  Historical jobs/targets/inboxes and manually retried attempts need future
  retention tooling. Dispatcher is serial; a failing host may delay other wakes
  by up to its 3-second bounded attempt.
- At-least-once delivery: a host may execute a request even if ack is lost.
  Stable IDs are provided; host deduplication is not proven. Rebind cancels
  future claims, not effects already transmitted externally.
- Setup supports Codex TOML and generic mcpServers JSON, not a guessed ChatGPT
  settings format. Interrupted receipt/config updates fail closed. No installer,
  OS service manager, system removal or destructive data purge is shipped.
- Source build/development installation only. All binary packages, stores,
  package repositories and automatic release work are deferred in TODO.md.
