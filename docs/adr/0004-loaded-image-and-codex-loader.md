# ADR 0004: loaded-image spawning and verified Codex plugin loading

Accepted, 2026-10-26; v0.1.1 development repair. Supersedes ADR 0003's portable manifest/cache independence assumptions for the installed local MCP entry.

## Observed failures

The user's real release-test task loaded the installed workflow skill but had no AgentJobRelay MCP tools, so it used the installed CLI. Its first job failed before starting a Runner: `runner spawn failed: No such file or directory (os error 2)`. Read-only peer-credential/proc inspection identified an old daemon whose mapped source-build executable had been unlinked by a rebuild. `current_exe()` returned a pathname ending in ` (deleted)`, unusable as a new child executable. The Agent then selected another state directory and succeeded. That was a workaround, not a repair of the first daemon. The workload failure motivated the loaded-image regression below.

A real Codex CLI 0.164.0 app-server in a fresh repository-local HOME loaded the portable-root skill but discovered zero MCP declarations. Adding compatibility files alongside that root still gave zero. A Codex-only manifest discovered MCP, but neither `${PLUGIN_ROOT}` nor `${CLAUDE_PLUGIN_ROOT}` expanded in the tested stdio command. The MCP child had none of the tested plugin-root environment variables. These are observed version/layout results, not a universal claim about all future Codex or ChatGPT Desktop versions.

## Decisions

- Linux internal daemon/Runner launch executes `/proc/self/exe`, so it uses the mapped binary even after pathname replacement. This stays in platform code; unsupported runtimes still fail explicitly. No workload replay or daemon replacement is added. `/proc` access remains a runtime requirement.
- Doctor advertises `runner_launch_from_loaded_image`. The client derives `daemon_executable_path_available` from the connected peer PID, not a guessed process list. A legacy daemon with a removed path and no new capability is rejected before submission. Doctor explains the required explicit idle restart. Existing processes do not acquire the repair merely because source was rebuilt.
- Local install emits `.codex-plugin/plugin.json` declaring skills and `.mcp.json`, with a stable absolute installed native command and explicit `--state-dir ... mcp --shared-host` arguments. It omits portable root manifests to avoid their observed precedence. Portable source templates are not shipped in the installed bundle. This is a local development format, not an official listing or a qualified relocatable public distribution bootstrap.
- The terminal wrapper remains relative and pins the same state profile across cache copies. Cached MCP depends on the original installation remaining at its configured path. Receipt ownership accepts previous and current layouts; modified/foreign content is still refused. Updates without a state override retain the old profile; explicit state overrides remain intentional.
- The workflow reports the actual route, preserves failed jobs and does not claim that loading a skill proves MCP connectivity. Changing state or silently retrying/restarting is not a repair.

## Evidence and limits

A new real-process regression failed with the same ENOENT before the runtime fix, then passed: start a copied daemon image, replace its pathname, submit a new workload, observe exactly one execution and the same daemon PID. A client test checks legacy refusal/capability acceptance. Installed onboarding checks cover manifest, absolute command, retained profile, ownership and shared caller binding.

`scripts/check_codex_plugin.py` exercises actual loader/tool discovery in isolated HOME/CODEX_HOME without a model turn, daemon, workload or real Desktop endpoint. Python is a developer-check dependency only. Current gate/live results are maintained in [implementation status](../IMPLEMENTATION_STATUS.md) and [live verification](../live-e2e.md); subsequent launcher/cache decisions are in [ADR 0007](0007-offline-plugin-runtime.md).

Distribution now targets host-specific Agent plugins around one Rust core. Claude Code and OpenCode remain planned, not implemented. Independent OS packages are cancelled; official platform submission is future work, not authorized now.
