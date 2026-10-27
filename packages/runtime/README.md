# AgentJobRelay runtime

Shared native Rust/MCP executable for the AgentJobRelay OpenCode V1 and ChatGPT Desktop/Codex plugins. Linux x86_64/glibc or Windows x86_64 with Node.js 20+; no Rust compiler, install script, separate binary download, global PATH change or system service is required. The package includes the native executable and a platform-specific SHA-256 manifest. The manifest records the Linux build's minimum glibc version; incompatible platforms fail explicitly. Windows uses a statically linked MSVC runtime. Hashes check integrity, not publisher authenticity. Use the official npm registry and verify the publisher/provenance when available.

This is an experimental package. See the [repository](https://github.com/MorningFrog/agent-job-relay) for release availability, supported hosts and known limits. An npm version being downloadable is separate from official plugin-directory approval.

`agent-job-relay codex <arguments>` and `agent-job-relay opencode <arguments>` select persistent host profiles under `$XDG_STATE_HOME/agent-job-relay/<host>`, or `$HOME/.local/state/agent-job-relay/<host>`. An explicit absolute `AJR_STATE_DIR` overrides that choice. Ordinary arguments pass to the same native CLI. These profiles are outside npm/host caches and stable across version updates; they contain no guessed caller identity. New profiles do not migrate a prior offline installation. Review its recorded state path before deliberately selecting it.

Installation runs no code. Invoking the CLI or plugin validates the packaged executable, then runs it with your OS permissions. First submission/explicit session binding starts the per-profile daemon. Removal preserves jobs/logs/history and does not stop daemons. Finish jobs and explicitly stop the original profile before an incompatible upgrade; no automatic daemon stop, database rollback or workload replay is performed.

Development: this source directory intentionally has no native payload. Build and pack from the repository with `python3 scripts/prepare_release.py --binary target/release/agent-job-relay --output .tmp/release-candidate`.
