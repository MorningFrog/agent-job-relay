# Manual acceptance

[Documentation index](README.md) · [Install](installation.md) · [Troubleshooting](troubleshooting.md) · [Automated testing](testing.md)

This procedure checks the reader's actual installation and ChatGPT Desktop workflow. Use a trusted compatible offline bundle; no source rebuild is required. Test configuration, local stdio, host discovery, actual MCP calls and same-chat continuation as separate outcomes. The isolated automated baseline is in [implementation status](IMPLEMENTATION_STATUS.md); previously passed CLI wake tests do not need repeating.

AgentJobRelay v0.1.8 is verified with ChatGPT Desktop on Linux, including same-chat continuation after task completion; see [live verification](live-e2e.md#linux-chatgpt-desktop). Use this procedure for a new installation or relevant change. Do not repeat an unchanged passed workflow merely to refill evidence.

## 1. Install or update the runtime

Set AJR_BUNDLE to the new bundle's absolute directory and AJR_INSTALL_ROOT to the existing installation. For a default installation:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
```

Keep the original root/profile if you previously chose a custom one. For an existing installation, finish active jobs and use the new bundle's launcher:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

Expected: preview reports the intended original installation/state; apply preserves that state and history, reports any backup, and installs the new bundle's version. Use the new launcher; apply automatically restarts a previously online daemon from the updated installation, while an offline profile stays offline. An active-job/ownership error is a stop-and-diagnose result, not a reason to choose a fresh state directory.

For a first installation only, replace the update sequence with:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --apply
```

Then check:

```bash
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Expected: version matches the bundle, owned marketplace entry matches, and `stdio_self_check.status=passed` with 25 tools for the current version. `daemon=stopped` is normal before a submission; the check must not start a workload. This result establishes local installation/configuration/stdio only. For a source build, follow the [local installation walkthrough](source-install.md) ([简体中文](source-install.zh-CN.md)) first.

## 2. Refresh the plugin in ChatGPT Desktop

Restart/reconnect ChatGPT Desktop and use its Plugins interface to refresh/reinstall AgentJobRelay from the existing local marketplace. A newly created marketplace is `agent-job-relay-local`, labeled AgentJobRelay Local; an older one may have a custom name/label. Select `agent-job-relay@agent-job-relay-local` for a new catalog. For this native local acceptance, disable the separate npm-backed `agent-job-relay@agent-job-relay` entry if enabled; it uses a different profile and cannot qualify the local installation. Keep the native installation present. Legacy caches without the new runtime profile require host refresh first; do not try to repair them by changing their receipts.

Open a new local chat and select the plugin. Expected: the plugin and its MCP tools are available, without changing other plugins. Record UI discovery separately from tool connectivity. If the host offers no refresh or cannot load the plugin, record that host/version limitation and stop this acceptance path.

## 3. Check one actual MCP call

After upgrading/reconnecting, ask in the selected chat:

> Call AgentJobRelay's job_doctor tool and report its MCP connection and binding mode. Do not submit a task.

Expected: an actual tool call returns `mcp_host_connection` saying it reached the MCP server, with `binding_mode=explicit current-terminal binding`. `desktop_context.available=false` is expected for a shared server: current-chat binding happens in the terminal, not the MCP process environment. If the daemon is stopped, its version field may be absent; use the executable version from step 1. This call does not prove wake capability.

If the current installation/host connection was already checked and has not changed, skip repeating this check. If tools are missing, diagnose host loading; a skill or CLI fallback does not qualify this MCP step.

## 4. Use the next real long task

Choose a naturally long build, test, training or processing task you actually need. Do not add sleep merely to repeat a passed CLI wake test. In the same local chat, ask:

> Use AgentJobRelay's MCP tools to run this project's real long task in the background. Bind this chat, retain the job ID, and end your turn after submission. When it finishes, continue here and summarize the result. If MCP is unavailable, report that and stop this test rather than using CLI fallback.

Expected sequence:

1. The Agent obtains a fresh binding from the current chat's terminal and calls MCP `job_start` with that exact key.
2. It reports the job ID and ends its turn. There is no repeated waiting, log-follow loop or auxiliary timer.
3. You send no continuation message. After completion, the same chat resumes and reads the matching job's bounded results.
4. The result/exit status matches the real command. Another chat receives no wrong notification.

A job that finishes before the submitting turn ends qualifies execution only; use a later natural task to check continuation. `accepted_by_host` alone qualifies host acceptance, not an observed Agent turn. Record a CLI fallback as that route, not a passed MCP acceptance. On failure preserve the job and diagnostic evidence; never replay the command or switch state to make the test pass.

## 5. Check history and an unchanged update

After a submission has started the daemon, inspect a known pre-upgrade job if one exists. Its status/logs should still be available through the same installation/profile. No pre-upgrade history is required for a first installation.

```bash
"$AJR_INSTALL_ROOT/relay" plugin status
"$AJR_INSTALL_ROOT/relay" plugin install --from "$AJR_BUNDLE" --dry-run
```

Expected: the original state path is retained; the same unmodified bundle reports `install.changed=false`. This preview does not stop the runtime, alter configuration or resubmit work.

## Record the outcome

| Item | Passing evidence |
|---|---|
| Runtime installation/update | Bundle version and original state/profile retained |
| Local MCP | Owned configuration match and 25-tool stdio self-check |
| Host UI | Plugin visibly discovered and installed/refreshed |
| Host MCP connection | Actual `job_doctor` tool response |
| Submission | Actual MCP `job_start` with fresh current-chat binding |
| Continuation | Matching result consumed in a new same-chat turn without another human message |
| Cache handling | Host refresh observed, or an explicitly tested snapshot result |

Use pass/fail/not tested for each item. Record host/app version, OS, actual submission route and job/delivery IDs privately; do not publish endpoints, credentials, databases or raw private logs. Keep the runtime delivery state separate from your observation of continuation. Use the restart/rebind procedure below only when explicitly testing that scenario. Real multi-day renewal remains a separate check.

## ChatGPT Desktop restart and same-chat rebinding

This separate acceptance checks a changed real host endpoint and an existing job target. It does not repeat the passed ordinary MCP continuation test. Native ChatGPT Desktop UI/restart needs a human operator when computer tools expose browsers only; record that limit instead of claiming an observed UI action.

1. Record the actual app version and whether AgentJobRelay is visibly installed/enabled in the Plugins interface. Preserve existing configuration and other plugins. Keep the installed runtime/profile unchanged unless an update is separately needed.
2. From this chat's terminal, obtain a fresh binding with the installed plugin launcher. Choose a fresh directory beneath the repository’s `.tmp/` and resolve its full non-symlink path from the current terminal; substitute that value for `<probe-directory>` in both commands below. Privately retain the session/instance fingerprint and submit `python3 scripts/check_desktop_restart.py wait --directory <probe-directory> --timeout 1800` through MCP `job_start`, with the repository cwd, that exact binding and a bounded job timeout. Use a fresh path; the script creates it. Retain the returned job/target IDs. The script's ready marker establishes only a waiting test command, not a ChatGPT Desktop result. End the submitting turn.
3. Quit/reopen ChatGPT Desktop normally, reopen this same chat and confirm that the plugin is still available. Report the app version and actual restart; no assistant process kill or configuration rewrite is needed.
4. In the reopened chat, use the installed launcher's `session recover` preview. Compare the new host instance with the private baseline and match the pending target to the retained job. Apply only the same-chat recovery requested for this test, preserving job/target/delivery history. An unchanged host instance establishes reconnect only, not a restarted bridge. An in-flight target is skipped; inspect its recorded outcome rather than polling or replaying the command.
5. After successful recovery, release the existing command with `python3 scripts/check_desktop_restart.py release --directory <probe-directory>` from the repository. End the turn. The same chat should continue from its completion notification, read the matching result and correlate the recovered target/event/delivery. The command is never resubmitted. The FIFO uses an OS wait with a deadline, not Agent polling.

Record UI/restart as observed or user-attested, old/new bridge identity comparison, recovered job/target correlation, host acceptance and actual continuation separately. Do not expose private endpoints or session keys. A timeout, missing tools or failed rebind is a failed/incomplete acceptance with preserved state. The script's `ui_restart_observed=false` deliberately cannot attest to the UI. This active-job scenario does not establish recovery of an event that completed while ChatGPT Desktop was closed, receiver deduplication or multi-day renewal.

## Optional: isolated installation and cache protection

This checks the new local file-management workflow without writing personal settings or contacting ChatGPT Desktop. Run from the repository root with AJR_BUNDLE set. It uses a new repository-local directory and a repo-local marketplace:

```bash
mkdir -p .tmp
AJR_CHECK="$(mktemp -d "$PWD/.tmp/manual-plugin.XXXXXX")"
"$AJR_BUNDLE/relay" --state-dir "$AJR_CHECK/state" plugin install --root "$AJR_CHECK/plugin" --config "$AJR_CHECK/.agents/plugins/marketplace.json" --dry-run
"$AJR_BUNDLE/relay" --state-dir "$AJR_CHECK/state" plugin install --root "$AJR_CHECK/plugin" --config "$AJR_CHECK/.agents/plugins/marketplace.json" --apply
"$AJR_CHECK/plugin/relay" doctor --client plugin --config "$AJR_CHECK/.agents/plugins/marketplace.json" --check-mcp
cp -a "$AJR_CHECK/plugin" "$AJR_CHECK/cache"
"$AJR_CHECK/plugin/relay" plugin status --cache "$AJR_CHECK/cache"
"$AJR_CHECK/plugin/relay" plugin cache refresh --path "$AJR_CHECK/cache" --dry-run
```

Expected: installation/self-check pass, status points to `$AJR_CHECK/state`, and the fresh cache matches the installation with `changed=false`. No daemon/workload, personal marketplace or real host is involved. To check modified-file protection, change only this disposable cache copy:

```bash
printf '\nmanual cache edit\n' >> "$AJR_CHECK/cache/skills/background-jobs/SKILL.md"
"$AJR_CHECK/plugin/relay" plugin cache refresh --path "$AJR_CHECK/cache" --apply
```

Expected: the last command fails with a modified-file error, leaving your edit and original installation intact. This expected refusal is the passing result. Retain the isolated directory for inspection; no manual crash matrix or real cache edits are needed. Automated cross-version/resource-change coverage remains in [testing](testing.md).

## Published self-hosted marketplace

The two npm packages and GitHub Release are public. Candidate and clean public-registry installation checks pass; a loopback-registry pass alone is not online ChatGPT Desktop UI acceptance. To qualify the latest published runtime and its matching marketplace, use a clean supported Linux ChatGPT Desktop profile:

1. Confirm Node.js 20+/npm, Codex CLI and Git are on PATH. Follow [latest published installation](installation.md#chatgpt-desktop--codex): install the global CLI, then run `ajr update`. Restart ChatGPT Desktop and choose the **AgentJobRelay** marketplace. Expected: one AgentJobRelay entry matching the resolved published version, with an explicit experimental Linux/Windows support description.
2. Install/enable it in a new local chat and invoke `job_doctor`. Expected: actual MCP connectivity and the reviewed packaged version; no developer executable path is requested. CLI loader discovery alone is insufficient.
3. Ask for a harmless short background command and completion in that chat. Expected: a fresh current-terminal binding, one job/run, correct result and correlated completion/continuation. Distinguish admission from observed execution. An unrelated chat must receive nothing.
4. Inspect the selected persistent profile before and after host refresh/removal. Expected: state stays outside caches and history is retained. `ajr uninstall` stops idle daemons by default and refuses active work unless explicitly stopped with `--force`; manual host removal does not stop daemons. If migrating an offline installation, explicitly review its recorded state rather than accepting a new profile as a repair.

Clean public-registry installation is verified separately from online ChatGPT Desktop UI acceptance. Version-specific Linux/Windows live evidence is in [live verification](live-e2e.md); Windows uses Full access. OpenCode real-provider/restart and Claude account-backed checks remain in [host plugins](host-plugins.md).

## Windows automatic startup

1. Use a fresh disposable profile and the latest installed entry point, without running `daemon start`. Submit a single bounded job from the actual host. Expected: one Job/Run completes; doctor reports `windows_cim` if host cleanup denies direct creation, otherwise `windows_native`. Record the runtime version and host/model evidence separately.
2. Close the host while an owned gated task runs. Expected: the original runner/task remains alive and persists its final result. Native automated coverage additionally destroys an enclosing cleanup Job Object.
3. In an isolated profile only, simulate daemon failure during a gated task, then make a new submission/registration. Expected: automatic startup reuses that profile, the original task remains single-attempt and history is retained. Recovery occurs on a request, not a resident restart watchdog.
4. Set `AJR_WINDOWS_STARTUP=direct` before launching a restrictive host with a cold profile. Expected: explicit refusal before job submission. Diagnostics/errors provide the exact literal-quoted external PowerShell fallback for that profile. Restore `auto` afterward. Restricted-token and invalid-parent cases have native automated coverage; never change policy, elevate or switch accounts to turn a refusal into a pass.
