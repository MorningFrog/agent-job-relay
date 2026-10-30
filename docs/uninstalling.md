# Uninstalling

[Documentation index](README.md) · [Updating](updating.md) · [Troubleshooting](troubleshooting.md)

All removal routes retain task history, logs and backups. Choose the route matching your installation.

| Installation | Start here |
|---|---|
| Online ChatGPT Desktop/Codex and global CLI | [`ajr uninstall`](#one-command-online-uninstall-0114) |
| Global CLI only | [CLI-only removal](#remove-the-global-npm-cli) |
| OpenCode or manually registered marketplace | [Host-managed removal](#remove-an-online-host-plugin) |
| Native local/source/offline plugin | [Native removal](#remove-a-native-local-installation) |
| Standalone MCP configuration | [Configuration-only removal](#remove-standalone-mcp-configuration-only) |

## One-command online uninstall (0.1.14+)

```bash
ajr uninstall --dry-run
ajr uninstall
```

Expected: preview lists affected profiles and owned components; apply stops idle selected/default daemons, removes `agent-job-relay@agent-job-relay` through Codex, removes its official marketplace when no other installed plugin needs it, then removes the global npm CLI and both command registrations. History, logs, backups and download caches remain. Restart/reconnect Desktop or Codex afterward to discard loaded plugin connections. Existing MCP processes are not killed by name.

Requires the global npm CLI 0.1.14+ and Codex CLI/Git for host removal. PowerShell can use `ajr.cmd`. Older CLIs can first [update through npm](updating.md#update-the-terminal-command), or use [manual host removal](#remove-an-online-host-plugin). Local/ZIP/native, foreign, modified or untagged marketplaces keep their original removal route and are refused by the online command.

### If active tasks block removal

Finish those tasks and retry. To explicitly stop them and then uninstall:

```bash
ajr uninstall --dry-run --force
ajr uninstall --force
```

Expected: preview lists affected profiles/task count without cancelling work. Apply uses normal durable task stops, preserves original Job/Run IDs and manual-stop history, and waits for exact-run terminal state and the same local runner's exit/lock release before removal. An unconfirmed stop retains the runtime for diagnosis. Read-only external observation stops only that observer; it does not cancel the external target or prove its outcome. `--force` does not bypass identity/schema/ownership checks or purge data.

The command checks the selected profile and default Codex/OpenCode profiles. Other custom profiles are not discovered: select a custom one with `--state-dir <absolute-profile>` and finish other custom runs before removing their shared global runtime. Shared marketplaces and other plugins remain intact; host commands remove only this plugin's exact cache, without scanning or deleting enclosing cache directories.

## Remove the global npm CLI

With 0.1.14+, preview/apply coordinated daemon shutdown and CLI removal while retaining host registrations:

```bash
ajr uninstall --cli-only --dry-run
ajr uninstall --cli-only
```

Expected: stop idle selected/default daemons and remove only the global CLI. No Codex/Git is needed. Active work still blocks removal unless explicitly stopped with `--force`. Retained online plugin entries require the CLI on their next use.

For older versions, or raw npm removal:

```bash
npm uninstall --global --ignore-scripts agent-job-relay-runtime
```

Expected: npm removes the package and command registrations. Raw npm removal does not stop jobs/daemons or remove host plugins; finish jobs and run same-profile `daemon stop` before it if shutting down the runtime. In PowerShell use `npm.cmd` if needed. Native offline plugins retain their own launcher; OpenCode retains its host-managed dependency.

## Remove an online host plugin

For OpenCode, remove only the AgentJobRelay entry from `opencode.json` and restart the host. Finish tasks first and, if shutting down its runtime, use the matching profile's `daemon stop` before host removal. Persistent history stays outside the host's cache.

For ChatGPT Desktop/Codex managed manually or from a Release ZIP, remove AgentJobRelay through the host, or use its public CLI:

```bash
codex plugin remove agent-job-relay@agent-job-relay
```

If no remaining plugin needs this marketplace, remove its registration:

```bash
codex plugin marketplace remove agent-job-relay
```

Expected: the host removes the exact plugin/cache and optional catalog registration, while external task data remains. Restart/reconnect the host. This manual route does not remove the global CLI or stop daemons; finish jobs and stop the same profile first if retiring that runtime. Use the actual catalog name for a local marketplace. Preserve unrelated entries and shared catalogs.

## Remove a native local installation

Set AJR_INSTALL_ROOT to the original installation's absolute directory. Remove/disable AgentJobRelay through the host, finish active jobs, then preview/apply owned configuration and file removal:

```bash
"$AJR_INSTALL_ROOT/relay" integration remove plugin --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove plugin --apply
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

Expected: preview lists only unchanged owned destinations; apply removes those installation/configuration files while retaining the state directory. Native removal previews by default and requires `--apply`; it does not implicitly stop daemons and has no online `--force` behavior. Reuse any original marketplace `--config` when removing that entry. Windows uses `relay.cmd`; release held MCP executable files before replacement/removal as described in the [Windows guide](windows.md#update-recover-and-remove).

If ownership checks refuse a change, inspect reported files/backups instead of deleting an enclosing directory. Repair an [interrupted directory replacement](updating.md#compatibility-and-interrupted-operations) before another lifecycle operation.

## Remove standalone MCP configuration only

Through the original launcher, preview then apply the appropriate route:

```bash
"$AJR_INSTALL_ROOT/relay" integration remove codex --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove codex --apply
```

For generic JSON, use `integration remove json --config <same-file> --dry-run` / `--apply` instead. Expected: only the unchanged owned AgentJobRelay entry is removed; unrelated entries/comments remain. Restart/reconnect the client afterward. Native files, job data and running processes remain.

## Recover a partial online uninstall

Private `online-uninstall.json` pins the original npm prefix, version, Codex executable/configuration home and options. Repeat `ajr uninstall` with the original profile/options to finish forward. Public host reads reconcile uncertain acknowledgements before repeated changes; the CLI is removed last so host-removal failures remain recoverable.

If npm already removed the CLI but final recording failed, reinstall the recorded version in the original prefix, then repeat the uninstall. Stopped daemons/tasks remain stopped on failure. Finish any recorded update before uninstalling; the operations cannot overlap. Preserve unfamiliar journals and partial `.new` files for inspection. Cross-service operations and native removal are not one atomic transaction. Windows self-removal and actual Desktop refresh remain separately unqualified; see [implementation status](IMPLEMENTATION_STATUS.md).

There is no destructive history-purge command. Use [data maintenance](data-maintenance.md) for archival, backup and restore.
