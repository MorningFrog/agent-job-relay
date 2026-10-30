# Install the local plugin from source

[Documentation index](README.md) · [Published installation](installation.md) · [Updating](updating.md)

English | [简体中文](source-install.zh-CN.md)

This walkthrough installs the native Linux x86_64/glibc build into a stable user directory and registers the personal **AgentJobRelay Local** marketplace. With a newly created marketplace, select **`agent-job-relay@agent-job-relay-local`** in ChatGPT Desktop. Building alone does not install or enable the plugin.

Source builds require Node.js 24/npm for React + Vite assets; recipients of a built native bundle need neither. For online installation, use the [published release](installation.md) rather than adding the source checkout as the local marketplace. For Windows, use the [Windows guide](windows.md); macOS and ARM runtime execution are unsupported.

## 1. Check prerequisites

Use Bash on Linux x86_64 with glibc, Git, Node.js 24/npm, Rust stable/Cargo, a C toolchain for bundled SQLite, and ChatGPT Desktop with a local executor. Check your build tools:

```bash
git --version
rustc --version
cargo --version
node --version
npm --version
cc --version
```

Expected: each command reports an installed tool. Rust 1.89 all-target compilation is verified on Linux; current build evidence is in [implementation status](IMPLEMENTATION_STATUS.md). Obtain missing tools through their normal supported installation instructions before continuing.

## 2. Open the source checkout

For a new checkout:

```bash
git clone https://github.com/MorningFrog/agent-job-relay.git
cd agent-job-relay
```

If you already have the checkout, open a terminal in its root instead; do not clone over it. All following build/export commands run from the directory containing `Cargo.toml`.

## 3. Build and export a local bundle

```bash
npm ci --prefix web --ignore-scripts --no-audit --no-fund
npm run build --prefix web
cargo build --release --locked
./target/release/agent-job-relay --version
AJR_BUNDLE="$PWD/.tmp/agent-job-relay-offline-$(date +%Y%m%d-%H%M%S)"
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --dry-run
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --apply
```

Expected: the executable version matches `Cargo.toml`; pack reports the chosen directory, version and native platform. The directory contains `relay`, `bundle.json` and `payload/`, with no selected user state. Each export needs an unused directory; the timestamp supplies a new name for a later run. Keep this terminal open so AJR_BUNDLE remains defined.

## 4. Install or update the managed runtime

Choose your installation directory. The standard Linux default is:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
```

If you already installed AgentJobRelay elsewhere, set AJR_INSTALL_ROOT to that original absolute directory. A personal marketplace requires the installation to be inside HOME; if XDG_DATA_HOME is outside HOME, choose a directory such as `$HOME/.local/share/agent-job-relay` for a first installation. Keep the installation separate from the exported bundle and task state.

### First installation

Run this branch only when there is no existing installation at the selected root:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --apply
```

Expected: the preview shows the intended `install.root`, `install.state_dir` and `marketplace.config_path`; review them before apply. A fresh state selection uses AJR_STATE_DIR if set, otherwise the XDG/HOME default. Apply installs the native program and registers it in `$HOME/.agents/plugins/marketplace.json`, preserving other entries. A new catalog is named `agent-job-relay-local`, displayed as **AgentJobRelay Local**; an existing catalog retains its name.

### Existing installation

Use the same plugin install entry for an existing runtime, selecting its original installation root. Finish active jobs, then run:

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

Expected: the preview retains the original state directory; apply retains history and reports any backup. Apply automatically restarts an online daemon from the updated installation after active-job checks; an offline profile stays offline. Stop and diagnose active-job, ownership or downgrade errors; do not switch to a fresh profile to bypass them. See step 6 for subsequent MCP/cache handling.

## 5. Check the installed entry point

```bash
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Expected: the installed version matches the build, the owned marketplace entry matches, and `stdio_self_check.status` is `passed` with 25 tools for the current version. A stopped daemon before the first submission is normal. This check submits no job and establishes local stdio discovery, not ChatGPT Desktop tool connectivity or chat continuation.

Keep AJR_INSTALL_ROOT present: cached MCP and terminal launchers use its native executable. Rebuilding `target/release` alone does not update this installation. Task state/history stays separate; [installation profiles](installation.md#custom-directories-and-stored-data), [updates](updating.md) and [uninstallation](uninstalling.md) cover later maintenance.

## 6. Install and select the correct plugin in ChatGPT Desktop

For first installation, enable the plugin below. Existing managed MCP connections verify/follow later runtime updates, including same-version builds; historical full-skill/pre-forwarder entries need one [cache migration and reconnection](updating.md#cache-inspection-and-refresh).

1. Restart ChatGPT Desktop and open **Plugins**.
2. Choose the personal **AgentJobRelay Local** marketplace, then install/enable **AgentJobRelay**. If the personal catalog already had another name, use that name from `$HOME/.agents/plugins/marketplace.json`.
3. If the online candidate `agent-job-relay@agent-job-relay` is enabled, disable that candidate through ChatGPT Desktop before testing the local plugin. Keep unrelated plugins unchanged.
4. Open a new **local** chat. Type `@AgentJobRelay` and select the entry from the local marketplace. For a new catalog the reference is `plugin://agent-job-relay@agent-job-relay-local`.

If Codex CLI is already available, the explicit installation command for a newly created local catalog is:

```bash
codex plugin add agent-job-relay@agent-job-relay-local
```

Then restart/reconnect ChatGPT Desktop and open the new local chat as above. CLI installation does not by itself prove ChatGPT Desktop MCP availability. UI labels/availability depend on the host version; if the local marketplace is missing, check step 5 and the catalog path rather than adding this repository's online marketplace.

## 7. Verify MCP before a long task

With the local plugin selected, first ask:

> Call AgentJobRelay's MCP job_doctor tool and verify managed_mcp_updates_supported=true and goal_wait_policy=required_for_active_desktop_goal, scoped_goal_consent=true, plus job_goal_consent and confirm_paused in job_goal_wait. Do not submit a task. If its MCP tools are missing, report that and stop; do not use CLI fallback or another plugin's tools.

Expected: an actual AgentJobRelay `job_doctor` call succeeds with those capabilities. Compare its `runtime_workflow_sha256` with the selected launcher's `instructions` digest when investigating a same-version update; a matching version number alone is insufficient. For shared MCP, `binding_mode=explicit current-terminal binding` and `desktop_context.available=false` are normal; the current chat is bound later through its terminal.

Only after that succeeds, use your next real long task:

> Use the selected local AgentJobRelay plugin's MCP tools to run this project's full tests in the background. Bind this chat and end your turn after submission. When the task finishes, continue in this chat and summarize the result. If MCP is unavailable, report that and stop this acceptance; do not use CLI fallback or another plugin's tools.

Expected: the Agent obtains a fresh terminal binding, submits through AgentJobRelay MCP `job_start`, reports the job ID and ends its turn. Observed same-chat continuation is a separate acceptance result; host acceptance alone does not prove it. See [manual acceptance](manual-testing.md) for evidence boundaries.

Desktop goal pause asks once when permission is absent. Preflight returns `needs_confirmation`; `job_goal_consent` uses native input or one Agent question, then `connect --request-id` reuses the approved scope for later rounds. The user has confirmed automatic pause after approval locally. Direct MCP form UI/full multi-round restoration remain separate acceptance. See [goal waiting](goal-waiting.md).

## If the skill appears but job_start is missing

Check which marketplace entry you selected. `npm E404` for `agent-job-relay-runtime` requires checking the online entry's pinned version, registry and network. Select/install the local marketplace entry in step 6, then use a new chat. If step 5 passes but the local entry still has no tools, refresh/reinstall that entry and reconnect ChatGPT Desktop MCP; stop acceptance if the host cannot load it. Do not edit cached manifests, move state or submit through a different plugin to make the check pass.
