# Uninstalling

[Documentation index](README.md) · [Updating](updating.md) · [Troubleshooting](troubleshooting.md)

Source 0.1.14 adds unified online removal; it is unpublished and current npm/GitHub latest 0.1.12 still uses the manual routes below. All removal routes retain task history, logs and backups. OpenCode, native offline installations and standalone MCP configuration keep their original maintenance routes.

## One-command online uninstall (0.1.14+)

```bash
ajr uninstall --dry-run
ajr uninstall
```

Expected: remove `agent-job-relay@agent-job-relay` through the public Codex CLI, remove its official marketplace registration when no other installed plugin needs it, and finally remove the selected global npm package and both command registrations. The command stops the selected/default profiles' idle daemons by default and leaves them stopped. A shared marketplace is retained. Restart/reconnect Desktop or Codex afterward to discard loaded plugin connections; existing MCP processes are not killed by name.

Active queued/launching/running tasks block ordinary removal before any daemon stop or host/package change. To stop them first:

```bash
ajr uninstall --dry-run --force
ajr uninstall --force
```

Expected: the preview lists the affected profiles/task count without cancellation. Apply uses the existing durable stop API, keeps original Job/Run IDs and manual-stop history, waits for terminal state and the same local runner's exit/lock release, then removes the plugin/CLI. An unconfirmed stop retains the runtime for diagnosis/recovery; `--force` never skips process identity, schema or ownership checks. Read-only external observation stops only that observer; no remote completion or cancellation is inferred. History, logs, backups and npm download caches are always retained; there is no purge flag.

`--cli-only` leaves host plugin/catalog registrations untouched and needs no Codex/Git. Those online entries require the CLI on their next use. Native `uninstall --root <root>` retains its original preview/`--apply` semantics; `--force` is only for the online global npm entry.

The selected global prefix/PATH entry, original Codex executable/configuration home and removal options are pinned. Local/foreign/modified or untagged marketplace sources and unknown cached plugin origins are refused. Only public host commands remove its exact plugin/cache; no host-cache scan or enclosing-directory deletion occurs. Other installed plugins and their marketplace registrations remain intact. Additional custom profiles are not discovered: select one explicitly with `--state-dir` and finish other custom runs before removing their global entry.

Private `online-uninstall.json` records completed stages. On failure the CLI stays installed until host removal verifies; repeat the original options to finish forward. A lost host acknowledgement is reconciled by public reads before any repeated mutation. If npm removed the CLI but final recording failed, reinstall the recorded version in the original prefix and repeat the uninstall. Stopped daemons/tasks stay stopped on failure. An unfinished update and uninstall cannot run over each other; finish the recorded operation before switching. A partial journal `.new` file requires explicit inspection. Cross-service operations are not one atomic transaction. Native Windows self-removal/held-image behavior and actual Desktop refresh need separate qualification.

## Remove the global npm CLI

```bash
npm uninstall --global --ignore-scripts agent-job-relay-runtime
```

Expected: npm removes the global command registration. This does not remove a host plugin or stop existing jobs/daemons. In PowerShell use `npm.cmd` if needed.

With 0.1.14+, prefer `ajr uninstall` above for coordinated removal. Raw npm removal only removes the CLI; retained online plugin entries report the missing prerequisite. Native offline plugins retain their own launcher and OpenCode retains its host-managed dependency.

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
