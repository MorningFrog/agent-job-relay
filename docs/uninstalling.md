# Uninstalling

[Documentation index](README.md) · [Updating](updating.md) · [Troubleshooting](troubleshooting.md)

Choose the route matching your installation. Removing the CLI, a host plugin, a marketplace entry and native installation files are separate operations. Task history, logs and backups remain in the external profile. Normal removal does not purge data or stop running daemons automatically.

## Remove the global npm CLI

```bash
npm uninstall --global --ignore-scripts agent-job-relay-runtime
```

Expected: npm removes the global command registration. This does not remove a host plugin or stop existing jobs/daemons. In PowerShell use `npm.cmd` if needed.

## Remove an online host plugin

For ChatGPT Desktop/Codex, disable/remove AgentJobRelay through the host first. If the online marketplace is no longer needed, remove its registration:

```bash
codex plugin marketplace remove agent-job-relay
```

Expected: the catalog registration is removed; external state/history remains. The host handles its own plugin cache. For OpenCode, remove the AgentJobRelay entry from `opencode.json` and restart the host. Do not delete other plugin entries or the persistent state directory.

## Remove a native local installation

Set AJR_INSTALL_ROOT to the original installation's absolute directory. Remove/disable AgentJobRelay through the host, finish active jobs, then preview/apply owned configuration and file removal:

```bash
"$AJR_INSTALL_ROOT/relay" integration remove plugin --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove plugin --apply
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

Expected: preview lists only unchanged owned destinations; apply removes those installation/configuration files while retaining the state directory. If ownership checks refuse a change, inspect the reported files and backups rather than deleting an enclosing directory. Reuse any custom `--config` selected during installation when removing that entry.

## Remove standalone MCP configuration only

Use `integration remove codex --dry-run` / `--apply`, or `integration remove json --config <same-file> --dry-run` / `--apply`, through the original launcher. Expected: only the owned AgentJobRelay entry is removed and unrelated entries/comments remain. Restart/reconnect the client afterward. Configuration removal leaves native installation files, job data and running processes intact.

An interrupted directory replacement must be [repaired](updating.md#compatibility-and-interrupted-operations) before another lifecycle operation. Uninstall and cross-file configuration changes are not crash-atomic. There is no destructive history-purge command; [data maintenance](data-maintenance.md) covers archival, backup and restore.
