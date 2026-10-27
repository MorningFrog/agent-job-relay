# Host plugins and community verification

One Rust/MCP runtime remains responsible for jobs, results, outbox attempts, recovery and leases. Host packages supply actual caller identity and translate a completion request to that host's public API. Shared runtime/OpenCode npm candidates and a self-hosted ChatGPT marketplace are prepared but unpublished; final npm names/account verification are pending. They are not official directory listings. Linux and Windows x86_64 have verified native runtimes; Windows Desktop acceptance remains separate.

## Current evidence

| Host | Entry point | Verified | Still needed |
|---|---|---|---|
| Codex/Desktop | Existing offline bundle and shared MCP | Actual Codex CLI loader; Desktop 26.1021.22027/plugin 0.1.5 restart/rebind/continuation | Broader versions, clean UI installation, real multi-day renewal |
| OpenCode V1 | `plugins/opencode-v1/index.mjs` plus core MCP | Actual Linux/Windows 1.19.4 loader, MCP submission, caller binding, completion admission and subsequent agent-loop response; local simulated model | Community real-provider/account sessions, server restart/recovery, broader versions and platforms |
| OpenCode V2 | Research/migration map below | Official interfaces researched; no V2 entry point claimed | Implement and test against an actual V2 loader |
| Claude Code | `.claude-plugin/plugin.json`, `.mcp.json`, skill and inbox binder | Synthetic caller separation, actual shared-MCP entry point/discovery/submission | Actual Claude loader, skill substitution and account-backed session testing; no maintainer account is available |

**Community testers and PRs are welcome, especially for Claude Code.** Please use the evidence checklist in [CONTRIBUTING.md](../CONTRIBUTING.md). Docker-based reproducible verification and version-matrix PRs are welcome; no tested Docker image is claimed. A container can validate a CLI/plugin interface but does not establish Desktop UI or a real account/provider session.

## OpenCode V1 development setup

The selected target is **OpenCode 1.19.4** and `@opencode-ai/plugin`/SDK **1.19.4**. V1 loads an exported plugin function and uses singular `plugin` configuration. Do not combine it with V2's plural `plugins`/`Plugin.define` interface. See the official [V1 plugin guide](https://opencode.ai/docs/plugins/), [custom-tool caller context](https://opencode.ai/docs/custom-tools/) and [SDK](https://opencode.ai/docs/sdk/).

For candidate package builds and the no-source-checkout installed-tarball check, follow [release preparation](release.md). To stage an installed development copy after preparing `.tmp/release-candidate`:

```bash
npm install --prefix .tmp/host-plugins --cache .tmp/npm-cache --ignore-scripts --no-audit --no-fund ./.tmp/release-candidate/agent-job-relay-runtime-0.1.8.tgz ./.tmp/release-candidate/agent-job-relay-opencode-v1-0.1.8.tgz
```

Use a new staging path for another copy rather than nesting copies. Configure the following once in your chosen OpenCode project, replacing the absolute paths with the built/staged paths and a short separate persistent state path:

```json
{
  "plugin": [["file:///absolute/repository/.tmp/host-plugins/node_modules/agent-job-relay-opencode-v1/index.mjs", {
    "executable": "/absolute/repository/target/release/agent-job-relay",
    "stateDir": "/absolute/short/path/relay-state"
  }]]
}
```

Published candidates default to their bundled runtime and persistent opencode profile; the tuple above demonstrates explicit development overrides. Normal npm-name installation is unavailable until publication. The plugin configures only its `agent_job_relay` MCP name and refuses an existing entry with that name. It requires native >=0.1.6 on Linux and >=0.1.7 on Windows; an older running daemon must be updated/stopped explicitly for the same profile, never silently replaced. A long socket path fails clearly; it does not switch profiles or use a shared temporary directory. Keep state outside plugin/cache directories.

On Windows, use the bundled `.exe` and a persistent Windows path (JSON requires escaped backslashes). The native helper pins the Node/Bun pipe's server process and secures its owner/DACL before registration. For Windows candidate checks and Windows automatic-startup/fallback behavior, see [Windows operation](windows.md).

The `tool.execute.before` hook obtains the real `sessionID`, verifies it through the public session API, registers/probes an owned bridge, then supplies `spec.session` to the actual MCP `job_start`. It overrides model-supplied caller keys. The plugin adds workflow guidance; everyday prompts need no internal path or session key. Other MCP tools stay available from the same core. Do not use Desktop `session connect`/`wake_me` in this host.

The plugin's private Unix socket / Windows named pipe accepts only sessions observed from actual tool contexts. The core pins its endpoint fingerprint; completion metadata contains stable IDs and status, never command output. The plugin uses the public `client.session.promptAsync` API. Acceptance means the host admitted a prompt, not that a model ran. In-process delivery-ID dedupe is bounded; persisted receiver dedupe across restart is not established. An abrupt host exit can leave its private instance directory for inspection; no cache/bridge-directory sweep or data purge runs.

After a plugin/server replacement, ask the Agent to use `relay_recover` with `apply=false` first. If restoration is requested, use `apply=true` and follow the cursor until an empty page. That tool binds the actual current caller; the core reuses the existing scoped recovery/audit/TTL rules and never reruns commands. This OpenCode restart workflow still needs real-host community verification; the shared core behavior has fake-host regression coverage.

### Account-free actual-loader check

Prepare the candidate tarballs as in [release preparation](release.md), then run from this repository with Node, npm and Python available:

```bash
npm install --prefix .tmp/opencode-toolchain --cache .tmp/npm-cache --ignore-scripts --no-audit --no-fund opencode-ai@1.19.4 @opencode-ai/plugin@1.19.4 @opencode-ai/sdk@1.19.4
python3 scripts/check_release.py --release .tmp/release-candidate --opencode .tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode
```

Expected: `result=passed`, OpenCode 1.19.4, a completed single-attempt command, one accepted delivery, a subsequent response in the correct session and no message in an unrelated session. The candidate checker installs the actual npm tarballs, then runs the real headless server and bundled native MCP with an isolated HOME/config/cache/state under `.tmp/`, uses a deterministic local model fixture, and stops only its own test processes. It needs no provider account, real API key or Docker installation. Reports stay ignored. The fixture proves interface routing and agent-loop invocation, not natural-language model judgment or a paid provider integration.

## Claude Code development framework

The framework follows the official [manifest layout](https://code.claude.com/docs/en/plugins-reference), [plugin MCP configuration](https://code.claude.com/docs/en/mcp#plugin-provided-mcp-servers) and [skill session substitutions](https://code.claude.com/docs/en/skills#available-string-substitutions). The root MCP file uses `mcpServers`; scripts and skills stay outside `.claude-plugin/`.

A contributor with an authorized Claude Code account can start a local development session with paths configured once:

```bash
AJR_EXECUTABLE="$PWD/target/release/agent-job-relay" AJR_STATE_DIR="$PWD/.tmp/claude-live" claude --plugin-dir "$PWD/plugins/claude-code"
```

This is an account-backed community acceptance command, not a locally verified Claude run. Use an explicit persistent state location for regular use, outside the plugin cache and `CLAUDE_PLUGIN_DATA`; host plugin removal can remove its own data directory. No personal Claude configuration is edited by the scaffold.

The skill passes the host-substituted `${CLAUDE_SESSION_ID}` to `scripts/bind.mjs`. Missing/literal placeholders are refused before state is created. The helper registers an exact **inbox-only** identity and returns its session key; the skill supplies it to MCP `job_start`. A shared MCP process never supplies the caller. This is durable job execution and explicit result inspection, **not automatic Claude wake**. No Desktop socket, guessed resume command or second Claude writer is used.

The official [hook reference](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background) describes `asyncRewake`, and [MCP channels](https://code.claude.com/docs/en/channels-reference) are another research direction. Their account/version behavior, session lifetime, delivery semantics and permissions need community evidence before a continuation adapter is implemented. This package installs neither hooks nor channels.

The local account-free scaffold check is:

```bash
python3 scripts/check_claude_scaffold.py --binary "$PWD/target/release/agent-job-relay"
```

Expected: synthetic callers produce distinct bindings, an unexpanded session placeholder fails without starting state, and the actual launcher exposes 20 MCP tools and accepts an explicitly bound command. The report explicitly says `claude_host_tested=false`. This is not `claude plugin validate`, actual plugin loading, skill expansion, model execution or wake verification.

## OpenCode V2 migration map

The official [V2 plugin API](https://opencode.ai/v2/docs/build/plugins/) uses `@opencode/plugin`, `Plugin.define({id, setup})`, plural `plugins`, domain registrations, actual tool/session contexts and `ctx.session.prompt`. Published `@opencode/cli`/plugin metadata was inspected at 2.1.4; no V2 loader or model turn was run here.

Keep `bridge.mjs`'s host-independent socket/native-request logic separate from the V1 entry. A V2 contribution should provide a new thin entry, obtain exact tool caller identity, register MCP through V2 transforms, submit through its public session API and dispose registrations/socket on unload. Test the actual loader, two caller sessions, MCP execution, completion admission, failure/retry and subsequent model invocation separately. Do not silently treat V1 configuration or a V1 pass as V2 support.
