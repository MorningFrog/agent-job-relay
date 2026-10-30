# Updating and repair

[Documentation index](README.md) · [Install](installation.md) · [Uninstall](uninstalling.md) · [Troubleshooting](troubleshooting.md)

Use the route matching your original installation. All routes keep the original task profile/history.

| Installation | Start here |
|---|---|
| Online ChatGPT Desktop/Codex CLI and plugin | [`ajr update`](#one-command-online-update-0114) |
| CLI older than 0.1.14 | [Update the terminal command first](#update-the-terminal-command) |
| OpenCode V1 npm plugin | [Host-managed update](#update-opencode-v1) |
| Native local/source/offline plugin | [Update from a new bundle](#update-a-native-local-or-offline-plugin) |
| Downloaded Release ZIP marketplace | [Replace the local catalog](#update-a-release-zip-marketplace) |
| Known native cache or interrupted operation | [Cache refresh](#cache-inspection-and-refresh) · [repair and retry](#compatibility-and-interrupted-operations) |

## One-command online update (0.1.14+)

```bash
ajr update --dry-run
ajr update
ajr --version
```

Expected: preview resolves one exact published version; apply updates the global npm CLI and installs its matching released ChatGPT Desktop/Codex marketplace/plugin. Restart or reconnect the host, then call `job_doctor` from the selected plugin to verify MCP connectivity. A fully current installation returns `changed=false`. An older online daemon is replaced even when package files are already current.

Requires a globally installed CLI 0.1.14+, Node.js 20+/npm, Codex CLI and Git on PATH. PowerShell can use `ajr.cmd`. For first installation or an older CLI, run the [npm installation command](#update-the-terminal-command) first. OpenCode, native local plugins and ZIP marketplaces use their own routes below.

| Option | Effect |
|---|---|
| `--to <x.y.z>` | Select an exact published version; downgrades are refused |
| `--cli-only` | Update only the global CLI, leaving host registrations unchanged; no Codex/Git required |
| `--state-dir <absolute-profile>` | Include a custom profile alongside the default Codex/OpenCode profiles; other custom profiles are not discovered |

Apply verifies the exact target tarball and schema/job compatibility, then installs those same bytes in the selected global npm prefix. Compatible Linux runners continue under their original Job/Run without command replay. Windows package replacement and incompatible schema migration require completed jobs. Only previously online profiles are restarted from the verified installation. Preview may download verification bytes, but does not start/stop daemons or create profiles.

Only the official Git marketplace at an unmodified matching release tag is managed automatically. Local/ZIP/native, foreign, modified and untagged sources are preserved and refused; use their original route or `--cli-only`. An outdated disabled plugin needs host management because the public Codex CLI cannot reinstall it while preserving disabled state. Avoid concurrent manual changes to host registrations. Windows self-update and actual Desktop refresh remain separately unqualified; see [implementation status](IMPLEMENTATION_STATUS.md).

### Marketplace reports local changes

Use `codex plugin marketplace list --json` to find the registered marketplace's exact `root`, then inspect `git -C <marketplace-root> status --short --untracked-files=all`. Preserve and review actual edits before retrying. Published 0.1.14 also rejects Codex's own untracked `.codex-marketplace-install.json`; do not delete that provenance record to bypass the check. Until a fixed release is available, `ajr update --cli-only` updates the terminal runtime while leaving the plugin unchanged.

The source fix validates that record against the official source, matching release tag and HEAD revision, accepts only a full checkout and continues refusing all other changes. Refusals identify the checkout and up to ten changed paths with staged/working-tree/untracked/conflict status and rename origins; larger inventories show the remaining count. Record failures identify the file and invalid fields with expected values, malformed JSON, unsupported fields, file size or symlink reason without exposing record contents. See [current verification](IMPLEMENTATION_STATUS.md).

## Before updating

Use the same launcher and profile to check `--version`, `doctor` and `list`. Finish active jobs before native offline replacement, Windows package replacement or an incompatible schema migration. `daemon stop` stops dispatch while runners continue, so it cannot substitute for finishing jobs.

Runtime 0.1.12+ replaces older same-profile daemons on runtime-starting operations, including session binding/submission and `ui`; applied native runtime updates also restart an online daemon. Equal/newer peers are reused. Doctor and ordinary read-only queries do not start or replace daemons. Versions before 0.1.12 need a manual same-profile `daemon stop` after their jobs finish; an updated native bundle can select that profile with `daemon stop --install-root <original-installation>`.

Current storage is schema v11, unchanged since 0.1.12. Older profiles migrate only after old daemons stop and old runs finish. Older binaries refuse newer schemas and may not understand external-task records. Preserve backups, never downgrade a used database and keep matching runtimes/target identity for remote tasks. See [data maintenance](data-maintenance.md) and [remote task requirements](remote-execution.md).

## Update the terminal command

For a CLI predating `update`, or a CLI-only installation using npm directly:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr --version
```

Expected: the CLI reports the latest published version. In PowerShell, use `npm.cmd` and `ajr.cmd` if script policy blocks `.ps1` wrappers. npm's global command directory must be on PATH and user-writable. After bootstrapping an older CLI, run `ajr update` to update its online plugin too. Raw npm installation alone does not refresh a loaded plugin/MCP connection; reconnect the host afterward. Follow [active-job requirements](#before-updating) before direct package replacement.

## Update OpenCode V1

Keep this entry in `opencode.json`, preserving other plugins:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

Finish active jobs, refresh the package through OpenCode's supported package-management mechanism and restart OpenCode. Expected: the host loads the new plugin and its exact matching native runtime while retaining the original `opencode` profile. `ajr update` does not update this dependency. Runtime 0.1.12+ replaces an older daemon during the new plugin's binding/submission, subject to schema checks. If notifications need rebinding after host replacement, preview the caller-owned `relay_recover` operation; see [host integration](host-plugins.md).

## Update a native local or offline plugin

For a native local/source/offline installation, first obtain a trusted new bundle using the [source walkthrough](source-install.md) ([中文](source-install.zh-CN.md)) or your original bundle provider. Set AJR_BUNDLE to its absolute directory and AJR_INSTALL_ROOT to the original installation's absolute directory. Finish jobs, then use the new bundle's launcher:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Expected: preview retains the recorded state directory; apply preserves history and reports its backup. An online daemon is stopped and restarted from the installed new image (`daemon_restarted`); an offline profile stays offline. The installed version/local stdio check must match the new bundle. Repeating an unchanged update is a no-op. Downgrades, active jobs and changed/unowned destinations are refused before replacement.

Reuse any custom marketplace `--config` from the original installation. Repeated `--cache <exact-directory>` options preflight and refresh known native snapshots with the runtime. Directory/configuration changes are separate operations; no cache scan or profile migration occurs. The compatible `plugin update --from <bundle>` entry remains available for older bundles; the current `plugin install` entry handles both installation and updates. Windows uses `relay.cmd`; follow the [held-image guidance](windows.md#update-recover-and-remove) before replacement.

Cached skills read `instructions` from the managed runtime, and retained managed MCP connections verify/follow its current executable, including same-version updates. Historical entries need the one-time [cache migration and reconnection](#cache-inspection-and-refresh). Rebuilding source alone does not update installed binaries. If a newly added tool remains absent after a catalog notification, reconnect MCP once.

## Update a Release ZIP marketplace

Finish jobs, download and verify the new marketplace ZIP from [GitHub Releases](https://github.com/MorningFrog/agent-job-relay/releases/latest), and extract it into a new stable directory. Update the global CLI using the [npm command](#update-the-terminal-command), then replace only this marketplace registration:

```bash
codex plugin marketplace remove agent-job-relay
codex plugin marketplace add <absolute-new-extracted-marketplace-directory>
codex plugin add agent-job-relay@agent-job-relay
```

Expected: the host installs the plugin from the new extracted catalog. Stop if a command fails; restart/reconnect after success and verify `job_doctor`. Keep the new directory. Other plugins using that catalog require host review before replacing its registration. This route remains local/ZIP-managed and is not eligible for `ajr update` host changes.

## Cache inspection and refresh

Use host refresh/reinstall for normal plugin-cache management. For a historical native plugin entry, use the exact cache directory reported by the host; set AJR_CACHE to that absolute directory and AJR_INSTALL_ROOT to its original installation:

```bash
"$AJR_INSTALL_ROOT/relay" plugin status --cache "$AJR_CACHE"
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --dry-run
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --apply
```

Expected: refresh validates the original installation/profile/platform and complete snapshot hashes, retains a backup and reports the new matching snapshot. Legacy caches without `.agent-job-relay-runtime.json` need host refresh first. Modified, unknown, missing, symlinked or foreign content is refused. Use only the exact plugin directory, never a parent containing other plugins. Refresh neither scans/deletes caches nor changes the host's registry/enabled state or stops the daemon.

Reconnect MCP once after migrating historical full-skill/pre-forwarder entries. Current managed connections follow later verified native updates. After those later updates, `matches_installed=false` can reflect an unused historical binary in the snapshot rather than stale behavior: compare the launcher's `instructions.workflow_sha256` with MCP `runtime_workflow_sha256` and require `managed_mcp_updates_supported=true`. A matching version string alone is insufficient; deleting snapshots is not a repair.

## Compatibility and interrupted operations

### Online update retry

Completed online stages remain in the selected profile's private `online-update.json`, pinning the target, npm prefix and original Codex executable/configuration home. Rerun `ajr update` with the original options to finish that recorded version forward. Public host reads reconcile uncertain replies before mutations are repeated; no package rollback or workload replay occurs. Finish an incomplete update before uninstalling, and an incomplete uninstall before updating. Do not delete an unfamiliar journal or partial `.new` file.

If npm failure leaves the CLI unavailable, reinstall the journal's recorded target version in the original prefix, then retry with the same profile/options. A lost controller leaves stopped profiles stopped; follow the reported same-profile daemon recovery command. Windows held-image/self-update and real Desktop refresh need their own qualification.

### Native installation or cache repair

Use the new trusted bundle if the installed launcher is missing. Select the exact destination named by the interrupted operation:

```bash
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --apply
```

Expected: preview reports `prepared`, `destination_missing` or `installed`; apply validates recorded original/staged/backup snapshots and finishes replacement forward, retaining the backup. Repeating a completed repair reports `changed=false`. For cache repair, substitute the exact cache path. Altered, unknown, missing or symlinked snapshots are refused. Installation repair requires idle profiles and never stops a daemon; cache repair leaves the managed runtime running.

Retry the original install/update afterward to finish marketplace registration or other selected caches. Repair does not roll back a runtime/schema. Cross-directory/configuration operations and native uninstall are not crash-atomic. Pre-journal or legacy remnants need explicit inspection, without scanning/deleting host caches. See [repair rationale](adr/0008-interrupted-plugin-replacement.md) and [current verification limits](IMPLEMENTATION_STATUS.md).
