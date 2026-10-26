# Current limitations (0.1.2 development milestone)

- Linux x86_64 is the only locally compiled and process-tested runtime. Windows
  named-pipe ACLs/Job Objects and macOS instance identity/lifecycle are not
  implemented. Other OS runtime requests explicitly fail. CI is configured but
  has not been run remotely; it is not evidence of platform support.
- Codex CLI integration provides MCP tools plus a durable notification inbox.
  It does not start/resume an Agent automatically. No public app-server writer
  is owned by this program. No authentication or model invocation is performed.
- Desktop transport is opt-in, private and experimental. Automated transport
  tests use fake hosts. The user confirmed explicit-registration basic and
  three-round CLI shortcut live passes on ChatGPT Desktop 26.1021.22027 /
  Ubuntu 26.04; local artifacts were cross-checked (see live-e2e.md). CLI rounds
  cover first start, reuse and idle-daemon stop/auto-start, without human
  continuation or wrong control-chat notification. They do not qualify other
  versions, in-flight daemon interruption, Desktop restart/rebind or delivery
  failure recovery. An ack remains accepted_by_host;
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
- Setup supports Codex TOML, generic mcpServers JSON and an owned local plugin
  marketplace entry, not a guessed ChatGPT settings format. Local install/uninstall
  manages a source-built native/plugin bundle with receipts; native system packages,
  OS service manager, system-package removal and destructive purge are not shipped.
  Interrupted config/install/removal can require inspection. Host cache copies
  and enablement are handled in the host UI; changing source files does not update
  them. The user's installed skill + CLI fallback real release task passed wake,
  after an initial failed launch and separate-state workaround. It did not call
  MCP. The repaired Codex CLI 0.164.0 loader discovers 19 tools in isolation;
  the user confirmed updated v0.1.2 Desktop MCP connectivity following the
  job_doctor procedure, without new independently reviewed tool evidence. MCP
  task submission/wake and browser discovery/install remain separately unverified. Source templates alone contain no native executable.
- The current local Codex bundle uses `.codex-plugin/plugin.json` and `.mcp.json`
  with an absolute installed native command. Cached MCP needs that original
  installation to remain; the terminal wrapper is relative. Portable root MCP
  was not discovered and plugin-root command variables were not expanded by
  the tested CLI loader. This is a version-specific development workaround,
  not a qualified public plugin bootstrap (ADR 0004).
- Current Linux daemons spawn from `/proc/self/exe`, surviving image pathname
  replacement, not gaining new code until restarted. An already-running legacy
  daemon with an unlinked executable needs an explicit idle restart. New client
  preflight detects this condition and refuses before submitting another job;
  no silent restart, state switch or failed-workload replay occurs.
- Source build/local Plugin development only. Claude Code and OpenCode plugin
  integrations are planned, not implemented. Independent OS installers and package
  repositories are cancelled. Future official plugin platforms remain in TODO.md;
  no publication/submission occurs during development.

Convenience support: `start --wake` / MCP wake_me=true remove manual registration
when current host context is inherited. They cannot infer identity in an ordinary
terminal or shared MCP process. Use the current Desktop Agent terminal tool when
MCP lacks those variables. Automatic sessions lease for 24 hours and need refresh
for longer work. CLI shortcut live evidence is user-attested and version-specific;
the reviewer correlated private artifacts without independently observing the UI.
MCP wake_me=true has fake-host automation and real process coverage, but no
separate real-host shortcut validation.

Installed/generated MCP uses shared-host mode, rejecting inherited-context wake.
The plugin skill obtains a fresh current-terminal session key and supplies it to
MCP. That route is tested with two callers, real installed CLI/MCP processes and
a fake Desktop host; it needs a local terminal and is not available in a cloud
chat without one. Updated Desktop MCP exposure/doctor is user-confirmed. The
next naturally long real MCP task can qualify submission and same-chat wake,
without internal paths in its prompt. Prior CLI passes do not qualify this path, and
another manual crash matrix is not a prerequisite.

Early 0.1.0 daemons without IPC shutdown can be stopped by the 0.1.2 client's
verified Linux pidfd/SIGINT path. It requires proc access, pidfd support and a
recognized `agent-job-relay` executable in daemon mode. Renamed binaries or failed
identity/health checks are refused, with no force-kill fallback. Use the updated
source executable with `daemon stop --install-root` to avoid an old installed
launcher or wrong shell state; the real install still needs an explicit update.
