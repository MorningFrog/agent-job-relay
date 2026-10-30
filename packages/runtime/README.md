# AgentJobRelay runtime

Shared native Rust/MCP executable for the AgentJobRelay OpenCode V1 and ChatGPT Desktop/Codex plugins. Linux x86_64/glibc or Windows x86_64 with Node.js 20+; no Rust compiler, install script, separate binary download, global PATH change or system service is required. The package includes the native executable and a platform-specific SHA-256 manifest. The manifest records the Linux build's minimum glibc version; incompatible platforms fail explicitly. Windows uses a statically linked MSVC runtime. Hashes check integrity, not publisher authenticity. Use the official npm registry and verify the publisher/provenance when available.

To check the latest published runtime, run `npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@latest agent-job-relay --version`. This is an experimental package. See the [repository](https://github.com/MorningFrog/agent-job-relay) for release availability, supported hosts and known limits. An npm version being downloadable is separate from official plugin-directory approval.

## Terminal and Web UI

Install once with npm's standard command registration:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex list
agent-job-relay codex start --shell 'echo hello'
agent-job-relay codex ui
```

npm 0.1.11 and newer also register `ajr`, defaulting to the same persistent `codex` profile as the online ChatGPT/Codex plugin. `ajr opencode <arguments>` selects OpenCode. npm's global command directory must be on PATH; use a user-writable npm installation. Windows also provides `ajr.cmd` for PowerShell script-policy restrictions. New 0.1.14+ online ChatGPT/Codex plugin entries require this globally registered CLI. The host has no automatic global installation hook. Remove the CLI with `npm uninstall --global --ignore-scripts agent-job-relay-runtime`; history and existing processes remain.

## Unified online updates (0.1.14+)

After installing this version globally, run `ajr update` to install or update the CLI and its matching ChatGPT Desktop/Codex marketplace/plugin together. Use `ajr update --dry-run` to preview, `--to <x.y.z>` for an exact published version, or `--cli-only` to leave host plugins unchanged. Codex CLI and Git must be on PATH for plugin updates. Reconnect/restart the host afterward. OpenCode packages and native offline bundles keep their original maintenance routes.

The updater pins the selected global npm prefix and version, refuses downgrades/active jobs/foreign or modified catalogs, fences idle default/selected profiles and restarts their previously online daemons. A private copied native helper remains outside the package during replacement. Partial steps are recorded in the selected profile's `online-update.json` for forward retry; no rollback, cache scanning or automatic PATH editing occurs. Outdated disabled plugins need host management or `--cli-only`. Windows self-update and actual Desktop refresh require their own native evidence; see the [updating guide](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/updating.md).

The full `agent-job-relay` entry retains its original default profile; `agent-job-relay codex ui` is equivalent to `ajr ui`. Explicit `--state-dir` takes priority over `AJR_STATE_DIR` and host defaults. Offline native installations retain their separate recorded profile and installed launcher.

`agent-job-relay codex <arguments>` and `agent-job-relay opencode <arguments>` select persistent host profiles under `$XDG_STATE_HOME/agent-job-relay/<host>`, or `$HOME/.local/state/agent-job-relay/<host>`. Windows uses `%LOCALAPPDATA%\AgentJobRelay\profiles\<host>`. An explicit absolute `AJR_STATE_DIR` overrides that choice. Ordinary arguments pass to the same native CLI. These profiles are outside npm/host caches and stable across version updates; they contain no guessed caller identity. New profiles do not migrate a prior offline installation. Review its recorded state path before deliberately selecting it.

Installation runs no code. Invoking the CLI or plugin validates the packaged executable, then runs it with your OS permissions. First submission/explicit session binding starts the per-profile daemon. Removal preserves jobs/logs/history and does not stop daemons. Current source automatically replaces an older daemon on runtime-starting use, preserving compatible independent runners. Finish old runs before incompatible schema migration; published versions still require a manual same-profile stop. No database rollback or workload replay is performed.

Development: this source directory intentionally has no native payload. Build and pack from the repository with `python3 scripts/prepare_release.py --binary target/release/agent-job-relay --output .tmp/release-candidate`.

Windows starts its daemon on demand, including host cleanup jobs, through verified same-user local CIM when native startup is denied. No login task/service is installed. Built-in Windows PowerShell/CIM must be permitted; diagnostics provide a copyable same-profile outside-terminal fallback when policy blocks it. `AJR_WINDOWS_STARTUP=direct` disables CIM.

The runtime embeds its React + Vite task manager in the native executable. Use `agent-job-relay codex ui` (or `ajr ui` with npm 0.1.11 and newer) to open the matching profile; `--no-open` prints its local access URL. Daemons enable the authenticated loopback listener by default, without a separate frontend installation. `inspect`, persistent `monitor` and configurable `stop` work without a live Agent. Access URLs contain local credentials; ordinary diagnostics omit them. Current source automatically replaces an older running daemon on UI access. Published versions still report an upgrade instruction and require a manual same-profile stop after jobs finish. Source builds generate assets with Node.js 24 before Cargo, while recipients use the packaged bytes.

Use the [documentation index](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/README.md) for task guides, the [command reference](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/commands.md) for CLI/MCP lookup, and [updating](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/updating.md) for profile-preserving maintenance.
