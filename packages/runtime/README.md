# AgentJobRelay runtime

Shared native Rust/MCP executable for AgentJobRelay's ChatGPT Desktop/Codex and OpenCode V1 plugins. Supports Linux x86_64/glibc and Windows x86_64 with Node.js 20+/npm. The package includes its native executable, bundled React/Vite task manager and a platform/SHA-256/minimum-glibc manifest. No Rust compiler, install script, secondary binary download or system service is required. Windows uses a static MSVC runtime; incompatible platforms fail explicitly. Hashes check integrity, not publisher authenticity; use the official registry and reviewed publisher/provenance.

## Install

For terminal and Web UI use:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr --version
ajr ui
```

For the matching ChatGPT Desktop/Codex plugin, also run:

```bash
ajr update
```

Requires Codex CLI and Git on PATH for plugin installation. Restart/reconnect the host and select AgentJobRelay afterward. Use a user-writable npm installation with its global command directory on PATH; PowerShell can use `npm.cmd` and `ajr.cmd` when script policy blocks `.ps1` wrappers. Online plugin entries require this global CLI; the host does not install it automatically. For OpenCode, install `agent-job-relay-opencode-v1@latest` through that host instead; it resolves its own matching runtime dependency.

For a version/integrity check without global registration:

```bash
npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@latest agent-job-relay --version
```

## Update and uninstall

These commands require a global CLI 0.1.14 or newer. For older CLIs, run the npm installation command above first.

| Task | Preview | Apply |
|---|---|---|
| Update CLI and matching online plugin | `ajr update --dry-run` | `ajr update` |
| Update only the CLI | `ajr update --cli-only --dry-run` | `ajr update --cli-only` |
| Remove CLI and online plugin | `ajr uninstall --dry-run` | `ajr uninstall` |
| Remove only the CLI | `ajr uninstall --cli-only --dry-run` | `ajr uninstall --cli-only` |

`update --to <x.y.z>` selects an exact published version. Update verifies target bytes/schema compatibility and restarts previously online daemons. Compatible Linux runners retain original Job/Run IDs without replay; Windows package replacement and incompatible migration require completed jobs. Uninstall stops idle selected/default daemons and refuses active work; explicitly adding `--force` stops exact runs and confirms local runner exit before removal. All routes retain history/logs/backups. CLI-only maintenance leaves online plugin entries requiring a CLI on their next use.

Only the official unmodified Git marketplace is managed automatically. Local/ZIP/foreign/modified sources, OpenCode and native offline installations keep their own routes. Outdated disabled plugins need host management or `--cli-only`. Recorded `online-update.json` / `online-uninstall.json` stages support forward retry with the same profile/options; no rollback, cache scan or automatic PATH editing occurs. Native Windows self-update/self-removal and actual Desktop refresh remain separate qualifications. Follow the [update guide](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/updating.md) or [uninstall guide](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/uninstalling.md) for recovery and ownership limits.

Raw npm removal is also available:

```bash
npm uninstall --global --ignore-scripts agent-job-relay-runtime
```

It removes only the package/commands and does not stop daemons/jobs or remove host registrations. Use coordinated removal above when retiring the online installation. No route purges task data.

## Profiles and everyday use

```bash
ajr start --shell 'echo hello'
ajr list
ajr ui
```

`ajr` defaults to the online `codex` profile; `ajr opencode <command>` selects OpenCode. The full entry retains its original native default: use `agent-job-relay codex <command>` for the online plugin profile. Explicit `--state-dir` takes priority over AJR_STATE_DIR and host defaults. Native offline installations use their separate recorded profile/launcher.

Online profiles live under `$XDG_STATE_HOME/agent-job-relay/<host>`, falling back to `$HOME/.local/state/agent-job-relay/<host>`; Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. They stay outside npm/host caches across updates. No prior offline history is migrated or caller identity guessed. Other custom profiles are not discovered automatically; select them deliberately before shared-runtime maintenance.

Invoking the CLI/plugin validates the executable, then runs it with your OS permissions. First submission/session binding starts its daemon on demand. Runtime 0.1.12+ replaces an older same-profile daemon during runtime-starting use, preserving compatible runners. Read-only diagnosis does not start it; no command is replayed after uncertainty. Windows can use verified same-user local CIM for daemon startup when host cleanup blocks native creation; it installs no task/service. `AJR_WINDOWS_STARTUP=direct` disables that fallback.

The bundled UI uses authenticated loopback access without a separate frontend install. `ui --no-open` prints its credentialed access URL; keep that URL private. `inspect`, runner-owned `monitor` rules and bounded `stop` work without a live Agent. Job completion, host acceptance and observed Agent continuation remain separate results.

For workflows and support limits, use the [documentation index](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/README.md), [command reference](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/commands.md) and [current evidence](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/IMPLEMENTATION_STATUS.md). Self-hosted marketplace publication is separate from official-directory approval.

## Development

This source directory has no native payload. Build frontend assets before Cargo, then prepare a repository-local candidate with `python3 scripts/prepare_release.py --binary target/release/agent-job-relay --output .tmp/release-candidate`. See the repository's [source walkthrough](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/source-install.md) and [release guide](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/release.md).
