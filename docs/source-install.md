# Install the local plugin from source

English | [简体中文](source-install.zh-CN.md)

This walkthrough installs the native Linux x86_64/glibc build into a stable user directory and registers the personal **AgentJobRelay Local** marketplace. With a newly created marketplace, select **`agent-job-relay@agent-job-relay-local`** in ChatGPT Desktop. Building alone does not install or enable the plugin.

The repository's `.agents/plugins/marketplace.json` exposes the separate `agent-job-relay@agent-job-relay` online candidate. Its MCP launcher downloads the published, pinned npm runtime and uses a separate profile. Adding the repository as a marketplace is not a step in this local installation. Neither Node.js nor npm is needed for the local plugin.

For Windows, use the build and bootstrap commands in the [Windows guide](windows.md). macOS and ARM runtime execution are unsupported.

## 1. Check prerequisites

Use Bash on Linux x86_64 with glibc, Git, Rust stable/Cargo, a C toolchain for bundled SQLite, and ChatGPT Desktop with a local executor. Check your build tools:

```bash
git --version
rustc --version
cargo --version
cc --version
```

Expected: each command reports an installed tool. The declared Rust 1.89 minimum has not been independently verified; current build evidence is in [implementation status](IMPLEMENTATION_STATUS.md). Obtain missing tools through their normal supported installation instructions before continuing.

## 2. Open the source checkout

For a new checkout:

```bash
git clone https://github.com/MorningFrog/agent-job-relay.git
cd agent-job-relay
```

If you already have the checkout, open a terminal in its root instead; do not clone over it. All following build/export commands run from the directory containing `Cargo.toml`.

## 3. Build and export a local bundle

```bash
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
"$AJR_BUNDLE/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

Expected: the preview retains the original state directory; apply retains history and reports any backup. Update does not stop the daemon automatically. Stop and diagnose active-job, ownership or downgrade errors; do not switch to a fresh profile to bypass them. This changes the managed runtime; the ChatGPT Desktop plugin/cache still needs step 6.

## 5. Check the installed entry point

```bash
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

Expected: the installed version matches the build, the owned marketplace entry matches, and `stdio_self_check.status` is `passed` with 20 tools. A stopped daemon before the first submission is normal. This check submits no job and establishes local stdio discovery, not ChatGPT Desktop tool connectivity or chat continuation.

Keep AJR_INSTALL_ROOT present: cached MCP and terminal launchers use its native executable. Rebuilding `target/release` alone does not update this installation. Task state/history stays separate; [maintenance](installation.md) covers profiles, updates and removal.

## 6. Install and select the correct plugin in ChatGPT Desktop

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

> Call AgentJobRelay's MCP job_doctor tool and report whether the connection works. Do not submit a task. If its MCP tools are missing, report that and stop; do not use CLI fallback or another plugin's tools.

Expected: an actual AgentJobRelay `job_doctor` call succeeds. For shared MCP, `binding_mode=explicit current-terminal binding` and `desktop_context.available=false` are normal; the current chat is bound later through its terminal.

Only after that succeeds, use your next real long task:

> Use the selected local AgentJobRelay plugin's MCP tools to run this project's full tests in the background. Bind this chat and end your turn after submission. When the task finishes, continue in this chat and summarize the result. If MCP is unavailable, report that and stop this acceptance; do not use CLI fallback or another plugin's tools.

Expected: the Agent obtains a fresh terminal binding, submits through AgentJobRelay MCP `job_start`, reports the job ID and ends its turn. Observed same-chat continuation is a separate acceptance result; host acceptance alone does not prove it. See [manual acceptance](manual-testing.md) for evidence boundaries.

## If the skill appears but job_start is missing

Check which marketplace entry you selected. `npm E404` for `agent-job-relay-runtime` requires checking the online entry's pinned version, registry and network. Select/install the local marketplace entry in step 6, then use a new chat. If step 5 passes but the local entry still has no tools, refresh/reinstall that entry and reconnect ChatGPT Desktop MCP; stop acceptance if the host cannot load it. Do not edit cached manifests, move state or submit through a different plugin to make the check pass.
