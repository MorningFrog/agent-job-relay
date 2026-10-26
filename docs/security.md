# Security and data ownership

Trusted single local user; this executes arbitrary commands with that user's
permissions. It is not a sandbox, permission broker or multi-user service.
No telemetry, TCP execution API, remote worker or automatic host discovery.

Linux state/root directories require same owner and mode 0700; socket 0600 and
peer UID check. Files are created under umask 077. Locks and log files use
O_NOFOLLOW/O_CLOEXEC. Existing symlink database/configuration/receipt files are
rejected. Keep the state directory on a local filesystem supporting SQLite WAL
and OS file locks. Do not put it in a shared or attacker-controlled tree.
Same-UID malicious processes can read state and invoke commands: out of scope.
Ancestor directory replacement by the same user is not a filesystem sandbox.

Commands default to argv execution. Shell requires explicit choice, and cwd is
canonicalized and validated. Limits: 8 active jobs, 10000 retained jobs, 1000
session registrations, 16 targets/job, 256 args, 64 environment overrides,
64 KiB spec, 8 KiB event payload, 16 MiB per stdout/stderr file, bounded frames
and result pages. Extra output is drained/discarded and truncation recorded.
There is no automatic cleanup. Monitor disk space; large retained histories can
still consume substantial storage. CPU/memory quotas and escaped-daemon process
containment are not provided.

Full inherited environment is not stored; explicit override values are stored
in the private database and omitted from status output. Command arguments,
working directories, logs and environment overrides can contain secrets.
Do not pass secrets in argv; logs are not automatically secret-masked. Do not
publish state, logs, setup backups or receipts. Output/event payloads are
untrusted data and are never executed as internal instructions. Desktop prompts
contain only stable job/event metadata, not command output.

Setup changes only the owned agent-job-relay MCP entry, with private adjacent
receipt/lock/backup files. It preserves other settings and refuses modified or
unowned entries. Default dry-run has no writes; --apply is explicit consent.
JSON formatting may be normalized; TOML foreign comments are preserved.
Backups may contain unrelated secrets and remain private. Config and receipt
are individually atomic, not a cross-file transaction: interrupted setup may
require reviewing the backup/receipt; it fails closed rather than taking over.
No service, global PATH, sudo, real config modification or network publishing
is performed by development tests.

Desktop endpoint is explicitly supplied, owned and peer-checked; private API
compatibility and receiver dedupe remain uncertain. Registration stores a socket
fingerprint, not cryptographic host attestation. It never scans other sessions,
reads credentials, injects binaries or creates a second Codex writer.

Current-context convenience accepts only inherited CODEX_THREAD_ID and
CODEX_APP_TOOLS_PIPE_PATH for opted-in --wake/wake_me=true. These remain trusted
caller declarations, not host attestation. Shared MCP processes cannot choose
per-chat destinations and must use explicit bindings or the current Agent's
terminal-tool CLI. Host capability is probed before executing a wake-enabled job.
A missing context/capability is an error, not a weaker notification fallback.
On-demand launch uses a private startup lock and a detached daemon boundary;
only absent/refused sockets trigger launch. No system service is installed.
Shutdown remains owner-only IPC and is excluded from MCP discovery.
