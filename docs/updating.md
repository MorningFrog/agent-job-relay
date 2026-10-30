# Updating and repair

[Documentation index](README.md) · [Install](installation.md) · [Troubleshooting](troubleshooting.md)

Use the route matching your original installation. Current source replaces an older daemon when the new runtime first starts/binds work or opens the UI. Applied local runtime updates restart an online daemon from the new installation. Historical full-skill/MCP entries need one migration refresh/reconnection. New native cached entries read current runtime instructions, and retained managed MCP connections verify/follow the updated installation; history stays in its original profile. Versions before 0.1.12 predate automatic handover and retain the manual stop fallback below.

| Installation | Update procedure |
|---|---|
| Global npm CLI | [One-command update](#one-command-online-update-0114), with an [older-CLI bootstrap](#update-the-terminal-command) |
| npm-backed ChatGPT Desktop/Codex marketplace | [One-command update](#one-command-online-update-0114), with an [older-plugin procedure](#update-the-online-marketplace) |
| OpenCode V1 npm plugin | [Update OpenCode V1](#update-opencode-v1) |
| Native local/offline plugin | [Unified install and update](#unified-install-and-update); [compatible older entry](#updating-an-existing-installation) |
| Known native cache or interrupted replacement | [Cache refresh](#cache-inspection-and-refresh); [recorded repair](#compatibility-and-interrupted-operations) |

## Before updating

Retain the original profile. Finish active jobs before native offline replacement, Windows package replacement or an incompatible schema migration. Record the installed version and inspect that profile with `doctor`, `list` or `inspect` using its [correct command prefix](commands.md#choose-the-entry-point-and-profile). Current source serializes the handover, pins the original daemon process before shutdown and verifies readiness/version afterward. Equal/newer daemons are reused. Compatible independent runners survive the handover; native offline file replacement, Windows package replacement and incompatible schema migration still require completed jobs. `daemon stop` leaves runners independent, so it is not a substitute for finishing jobs.

External-task command variants are unreadable by older binaries; 0.1.12 scoped goal waiting additionally migrates the profile to schema v11. Before first use, finish active jobs and use the updated runtime for that same profile. Automatic handover refuses migration while old runs remain active. Preserve the matching runtime on both hosts and the remote target profile; do not downgrade a profile containing external-task records. See [remote task requirements](remote-execution.md).

Published 0.1.12 and source 0.1.14 use schema v11; 0.1.11 uses v9 and 0.1.9 uses v6. No schema migration is needed from 0.1.12 to 0.1.14. Migration refuses an old daemon lock or launching/running jobs, and older executables refuse newer schemas. Preserve backups and never downgrade a used database. See [data maintenance](data-maintenance.md) for backup and restore.

## One-command online update (0.1.14+)

Source 0.1.14 adds the unified online route. It is not yet published; npm/GitHub latest remains 0.1.12. After installing a released runtime containing this command, the initial plugin installation and later updates use the same entry:

```bash
ajr update --dry-run
ajr update
```

Expected: the preview resolves one exact published version; apply updates the selected global npm CLI, registers its matching release-tagged marketplace and installs the matching ChatGPT Desktop/Codex plugin through the public Codex CLI. Restart/reconnect Desktop or Codex afterward to load the plugin and MCP catalog. Node.js 20+/npm, Codex CLI, Git and npm's global command directory must be on PATH. PowerShell can use `ajr.cmd`. Use `--to <x.y.z>` for an exact published version or `--cli-only` to leave host plugins untouched. Downgrades are refused. When CLI/plugin files are current, the updater still checks for an older online daemon and replaces it automatically. A fully current installation returns `changed=false` without stopping a daemon.

The updater validates that it is running from the selected global npm package and pins that npm prefix. The plugin retains its exact-version npm runtime; this command coordinates both versions. New online plugin entries require the installed global CLI and fail with a copyable installation command when it is missing. Host installation hooks are not used, and existing older cached entries require their initial update/reconnection. Native offline bundles and OpenCode packages keep their own routes below.

The updater stages and verifies the exact target tarball/native image before replacing anything, then installs those same tarball bytes globally. Its target helper checks the selected plus default Codex/OpenCode profiles without migration. Linux active jobs may continue only when the target can read the existing schema/job contract; independent runners retain original Job/Run/attempts and are never relaunched. An active incompatible schema or Windows package replacement is refused before shutdown; finish those jobs first. Preview may download temporary verification bytes but never starts/stops daemons or creates profiles. Apply holds installation/startup locks, revalidates original daemon peers, stops their old daemons, replaces package files after the original CLI exits and restarts only previously online profiles from verified installed bytes. Other custom profiles are not discovered or moved; select one with `--state-dir`. No schema rollback or workload replay occurs.

Only the official Git marketplace at its matching release tag is managed automatically. Development/untagged checkouts are preserved and refused. Local/ZIP/native, foreign and modified same-name marketplaces are refused; use their original route or `--cli-only`. Other marketplace registrations remain host-managed. An outdated disabled plugin is refused before changes because the verified Codex CLI cannot reinstall it while preserving its disabled state; manage it in the host or use `--cli-only`. Host configuration changes have no public compare-and-swap operation: do not edit marketplace registrations concurrently.

Applied steps are recorded in the selected profile's private `online-update.json`, including the original marketplace identity/version, Codex executable/configuration home and target. Retry refuses a changed host selection. An unfinished uninstall must finish before updating. On a partial failure, completed changes stay installed; rerun the original options to finish the pinned version forward. Public host reads reconcile a removed/re-added catalog or a successfully installed plugin after an uncertain reply; no package rollback or blind install replay is performed. A lost controller leaves stopped profiles stopped, with explicit same-profile recovery instructions. An interrupted journal write may leave its exact `.new` file for inspection; it is never scanned or deleted automatically. If the installed CLI itself is unavailable after npm failure, reinstall the recorded target with npm first. Do not overwrite or remove an unfamiliar journal. Native Windows self-update/held-image behavior and real Desktop refresh remain separate qualification until recorded in [implementation status](IMPLEMENTATION_STATUS.md).

## Update the terminal command

With 0.1.14+, use `ajr update` above; `ajr update --cli-only` updates just the global CLI. For older published CLIs, bootstrap once with standard npm registration:

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex --version
```

Expected: the CLI reports npm's latest published version. Before 0.1.14 is published this remains 0.1.12 and the manual marketplace procedure below still applies. npm's global command directory must be on PATH; use a user-writable npm installation. PowerShell can use `npm.cmd` and `agent-job-relay.cmd`. Current runtimes replace an older same-profile daemon on runtime-starting use; incompatible migration still requires completed jobs.

## Update the online marketplace

With 0.1.14+, `ajr update` installs/updates both the CLI and plugin. For published versions without that command, finish jobs, install/update the global CLI above, then use the older manual route:

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION" &&
codex plugin add agent-job-relay@agent-job-relay
```

Expected: the matching release-tagged catalog and plugin are installed. For a first installation omit marketplace removal. Restart/reconnect Desktop or Codex afterward. Older CLI versions still need the manual same-profile daemon stop before an incompatible upgrade; retain the original profile/history.

## Update OpenCode V1

Keep `agent-job-relay-opencode-v1@latest` in `opencode.json`. Finish jobs, refresh its package cache using the host-supported mechanism and restart OpenCode. The plugin's native runtime dependency is pinned to its matching version. Current source replaces an older daemon during the new plugin's explicit session binding/submission. Published versions still need a manual same-profile stop before an incompatible upgrade. Use the plugin's actual caller-context recovery when needed; see [host integration](host-plugins.md).

## Unified install and update

Set AJR_INSTALL_ROOT to the existing installation's absolute directory and AJR_BUNDLE to the trusted new native bundle's absolute directory. Use the new bundle's launcher to preview and apply the update:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Expected: the preview keeps the recorded state profile, apply preserves history and reports its backup, and the installed version/local stdio check match the new bundle. Review the preview and finish active jobs. Apply automatically stops an online daemon, replaces the runtime and starts/verifies the installed new daemon; its report includes `daemon_restarted`. An offline profile stays offline. Repeating an unchanged update is a no-op; downgrades and active jobs are refused before shutdown. The installed launcher's `plugin install --from "$AJR_BUNDLE"` is also available when it supports this entry.

The command validates bundle ownership/version and each explicitly selected cache (`--cache` may repeat), then maintains the runtime and local marketplace entry. `--config` selects a custom marketplace file; reuse that selection on later maintenance. Default output includes changed destinations, preserved profile, backups and next action; `--full` includes complete reports. Directory replacements and marketplace writes are separate operations. No cache scanning or automatic profile migration occurs.

For historical entries, refresh the known cache and reconnect MCP once to install the stable instruction entry and managed forwarding support. Thereafter cached skills read `instructions` from the installed runtime, while retained managed MCP connections route calls/catalogs to verified current bytes, preserving their original profile. Same-version source updates are included; modified or downgraded runtimes fail without fallback/replay. A tool-catalog change emits one notification per runtime revision. The updated daemon is already running after a successful applied local update. Rebuilding source does not update installed binaries or caches. If replacement was interrupted, use the [recorded repair procedure](#compatibility-and-interrupted-operations), then retry the update.

## Updating an existing installation

For bundles without `plugin install`, use the compatible `plugin update` entry from the new bundle, even when the installed version is too old to recognize it. Keep AJR_BUNDLE and AJR_INSTALL_ROOT selected as above. Older update executables require `"$AJR_BUNDLE/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"` after preview and before apply; current source handles stop/start automatically:

```bash
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

Expected: the same profile-preservation, backup, no-op and downgrade/active-job checks apply. Review the preview and finish jobs; current source automatically restarts an online daemon during apply. Older update executables still require `daemon stop --install-root` first. Then verify the installed version/local stdio and refresh the host as in the unified procedure. Do not switch state directories to work around a failure. Early legacy shutdown handling is described in [ADR 0005](adr/0005-legacy-shutdown-upgrade.md).

## Cache inspection and refresh

Prefer the host's refresh/reinstall for normal plugin updates: the host owns its version registry, cache selection and enabled state. Legacy caches without `.agent-job-relay-runtime.json` need this host refresh before using managed cache repair.

For a known cache directory belonging to this plugin, set AJR_CACHE to that exact absolute directory:

```bash
"$AJR_INSTALL_ROOT/relay" plugin status --cache "$AJR_CACHE"
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --dry-run
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --apply
```

Commands run through a custom installed launcher default to its own root. Refresh validates the original installation/state/platform, complete file set and hashes. It supports host copies with or without the management receipt, refuses altered/unknown/missing/symlinked/foreign files, retains the previous snapshot as a backup and is idempotent. It does not scan/delete caches or change the host's registry/enabled state. Do not pass a parent containing other plugins. Reconnect MCP for the initial historical-entry migration; current managed connections follow subsequent verified native updates. If the host has not refreshed newly added tools after a catalog notification, reconnect once to load that catalog. This is separate from rewriting cached workflow content. This cannot retrofit an already running pre-forwarder binary without that first reconnection.

For a stable instruction entry, `matches_installed` describes snapshot bytes, including an unused historical native copy; it can be false after a later runtime update without stale behavior. Compare the selected launcher's `instructions.workflow_sha256` with MCP `runtime_workflow_sha256`, and require `managed_mcp_updates_supported=true` for the connection. Do not treat a matching version string as proof, or delete snapshots to make the byte comparison pass.

To preflight and refresh several known snapshots during runtime update, repeat `--cache <absolute-directory>` on `plugin update`. All selected caches are checked before runtime replacement. Cache refresh alone does not stop the daemon. Each directory replacement records a durable recovery operation; multiple directories and marketplace writes remain separate. On interruption use `plugin repair` for the exact affected directory, then retry the original update. No runtime/schema rollback occurs.

## Compatibility and interrupted operations

The tested Codex CLI 0.164.0 loader uses `.codex-plugin/plugin.json` and `.mcp.json` with a stable absolute native command. Its portable root and PLUGIN_ROOT behavior differed from the documented format; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). Packaging does not establish other host/OS support.

Installation and cache replacements durably stage files and record a private sibling operation journal before renaming the original directory. A pending operation blocks another lifecycle change. If the installation path is missing, use the new offline bundle or source executable, not the missing installed launcher:

```bash
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --apply
```

Use an explicitly selected cache directory for a cache operation. Preview reports `prepared`, `destination_missing` or `installed`; apply finishes the recorded replacement and retains the original backup. Repeating a completed repair reports `changed=false`. Repair verifies the complete staged/original/backup snapshots, profiles and hashes; modified, unknown, missing or symlinked contents are refused. Installation repair requires idle old/new profiles and never stops a daemon. Cache repair leaves the managed runtime running.

Retry the original bootstrap/update afterward to finish marketplace registration or other selected caches. Repair neither changes the database nor rolls back an installed runtime. Cross-directory/configuration operations and uninstall remain separate and are not crash-atomic. Before journal publication, interruption leaves the original untouched and may leave unreferenced staging files; these and old pre-0.1.5 interruptions require inspection, not automatic scanning/deletion. Physical power-loss/ENOSPC testing remains pending. Version 0.1.9 uses schema v6, published 0.1.11 uses v9, and current source uses v11. Migration refuses until the old daemon stops and old launching/running jobs finish. Older executables refuse newer schemas. See [data maintenance](data-maintenance.md) for schema/restore behavior, [implementation status](IMPLEMENTATION_STATUS.md) for verification limits and [ADR 0008](adr/0008-interrupted-plugin-replacement.md) for repair rationale.
