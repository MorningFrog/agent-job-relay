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

Installed plugins and generated MCP entries enforce shared-host mode: no inherited
chat environment can authorize an implicit wake. A fresh exact session key comes
from the current terminal's inherited context and is passed explicitly over MCP.
This remains the trusted local-user model, not cryptographic caller attestation.
The CLI binding's --brief form omits private endpoint/registration details.

Local installation stores a state-only profile and SHA-256 ownership receipt,
not live host identity or credentials. The wrapper and MCP pin the same profile
despite unrelated AJR_STATE_DIR exports. Replacements stage new files and retain
backups; modified or foreign contents and symlink paths are refused. Data cannot
be nested in the install tree. Runtime locks and active jobs block binary changes.
Two-rename upgrades and multi-file removals are not crash-atomic; after interruption
inspect the private backup/receipt. Host plugin caches and user enablement are
managed by the host, never scanned or purged by this CLI. Marketplace setup edits
only the owned entry and preserves the existing catalogue and other plugins.

Linux internal child launch executes the mapped `/proc/self/exe`, retaining the
same program image after a rebuild unlinks its pathname. It does not adopt an
arbitrary replacement binary or replay work. Peer-derived legacy readiness checks
use the connected socket PID and do not scan user processes. A legacy runtime is
never silently stopped: finish active work before an explicit same-state restart.

The Codex local development MCP command is an absolute path into the owned stable
installation, not an unexpanded plugin-root variable. Cached MCP depends on this
path remaining present. Local real-loader checks run in isolated HOME/CODEX_HOME,
without model requests, credentials or real Desktop endpoints. Independent OS
packages are cancelled; no online plugin submission/upload is performed now.

Legacy shutdown is only reached after the original peer rejects the exact IPC
shutdown operation. Same-UID/socket/root ownership, the original boot/start
identity, native daemon executable/mode and a healthy known legacy response are
rechecked before SIGINT. A pidfd pins the process against PID reuse; the signal
never targets a process group. No broad kill, force kill or process-name scan is
used. The trusted local-user model remains; this is not cryptographic product
attestation. Explicit stop authorization is separate from install --apply.

Notification recovery uses trusted explicit caller identity, the same endpoint
owner/fingerprint/capability checks, and exact logical/caller/host matching.
Delivery-time renewal is opted in and rechecks invalidation and the current claim
after its network probe. Audits commit with the state change. Neither mechanism
adds host attestation, endpoint discovery or confirmed Agent-resume evidence.
