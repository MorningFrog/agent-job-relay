# AgentJobRelay for OpenCode V1

Experimental durable background jobs with completion routed to the initiating OpenCode session through the public session API. Supports Linux x86_64/glibc and Windows x86_64, targets OpenCode **1.19.4** and uses the shared `agent-job-relay-runtime` package. No Rust compiler, binary path or install script is needed for the published package. OpenCode V2, macOS and ARM are not supported.

Add the latest published package to your project's `opencode.json`:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Restart OpenCode. Ask it to run your project's tests in the background and tell you when they finish. The plugin provides MCP tools and binds the actual tool caller. It refuses a pre-existing `agent_job_relay` MCP entry; review that entry instead of enabling both. Commands run with your OS permissions; output is untrusted data. Completion admission is not proof that a model ran.

Unreleased source adds `job_start_batch` for 1–16 independent commands. Each item needs a distinct stable idempotency key and receives the actual caller binding. Admission is partial under the eight-active-job quota; retain individual results and retry unchanged keys/specs after a lost response. Published packages and the sealed 0.1.11 candidate do not include this capability; use the matching source runtime for development.

State defaults to `$XDG_STATE_HOME/agent-job-relay/opencode`, else `$HOME/.local/state/agent-job-relay/opencode`; an absolute `AJR_STATE_DIR` or plugin `stateDir` option overrides it. State must stay outside npm/host caches. Long Unix socket paths fail explicitly: select a shorter persistent state path once, rather than changing an existing profile to conceal an error. Development can supply absolute `executable` and `stateDir` options in the plugin tuple.

Refresh the host's package cache using its supported mechanism to resolve latest, then restart after finishing jobs. An already running daemon retains its loaded native image; incompatible daemons require an explicit same-profile stop/update. After host replacement, ask the Agent to preview `relay_recover`, then apply if you requested restoration; it never reruns commands. Remove the plugin config entry to disable it. History remains outside caches; no automatic daemon stop or data purge runs.

The real OpenCode 1.19.4 loader/MCP/session API/agent loop is checked with a **simulated model**, not a real provider account. Real-provider and broader-version community tests and PRs are welcome. For checks and limitations, see [host plugins](https://github.com/MorningFrog/agent-job-relay/blob/main/docs/host-plugins.md) and [contributing](https://github.com/MorningFrog/agent-job-relay/blob/main/CONTRIBUTING.md).
