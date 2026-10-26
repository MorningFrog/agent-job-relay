# ADR 0003: installed plugin and current-terminal binding for shared MCP

Accepted, 2026-10-26. **Manifest/launcher compatibility findings below are superseded by ADR 0004**: the installed bundle uses the tested Codex layout and an absolute local MCP command, not portable-root manifests. Users should configure the tool once and describe the real background task without copying executable/state paths or session IDs into prompts. The prior three-round CLI live pass remains useful; another CLI workload would not qualify the missing installed entry point.

## Local development installation

`install` previews a dedicated native/plugin directory, then writes only with `--apply`. It embeds the current native executable, portable root plugin/MCP manifests, a relative launcher, a workflow skill and the existing project icon. This is a local source-build install, not a system package or public publication.

An owner-only receipt records the installation root, separate absolute state profile and SHA-256 of each owned file. Explicit `--state-dir`, then AJR_STATE_DIR, override that installed profile; source binaries retain their XDG/HOME default. A cached copy of the plugin keeps the same state profile. Generated MCP arguments pin that state, and the relay launcher sets AJR_STATE_DIR to the installed profile, so an exported demo override cannot split the two entry points. Runtime launchers do not require management receipts to survive host-cache import. It never contains a host socket, thread ID, registered session key, token or job log. The native program and launcher resolve relative to the plugin, including when paths contain spaces. The source plugin is a template; `install --apply` supplies its binary.

Replacement stages a complete private tree, retains the previous tree as a backup and restores it if the final rename fails. The backup and final rename are separate operations; crash/power-loss atomicity across both is not claimed. Uninstall verifies all owned contents before removing them. Modified, unknown, missing or symlinked content causes refusal. State is never inside the install tree, and active daemon/startup locks or nonterminal jobs block replacement/removal. An interrupted removal can require inspection; this is not a transactional OS package manager. Host-managed plugin cache copies are removed through the host.

## Plugin and configuration

The portable plugin uses `plugin.json`, `mcp.json`, `${PLUGIN_ROOT}` and a `requires_local_executor` extension. `setup plugin` adds only the owned entry in a repo/personal `.agents/plugins/marketplace.json`, retaining other entries and metadata. The entry's source is relative to the marketplace root and stays inside it. The existing backup/receipt/preview/apply/removal mechanism is reused. No host configuration or global PATH is changed merely by installing the bundle.

The generated plugin and standalone MCP configurations launch `mcp --shared-host`. They do not presume a process is private to one chat. The skill gets a fresh binding using its bundled launcher's `session connect --brief` in the **calling chat's terminal tool**. That reads only its inherited context, registers/refreshes and probes the host. The skill supplies the returned key as `job_start.spec.session` over MCP. The user supplies none of these paths/IDs. A failed preflight prevents a wake-dependent workload from being submitted; no latest-session selection exists.

Shared mode rejects `wake_me=true` and MCP `job_connect_session`, even if its environment contains a valid but stale chat identity. It advertises the explicit terminal-binding workflow in instructions and tool schemas. Manually configured, provably per-session `mcp` retains the prior inherited-context shortcut. A cloud chat without a current local terminal cannot manufacture a safe binding. Ordinary non-wake jobs remain available. There is no guessed per-call metadata contract.

## Diagnostics and acceptance

CLI doctor reports daemon/configuration/context separately and works before a daemon exists. `doctor --check-mcp` initializes a private stdio subprocess and discovers its tools, without launching workloads, probing or messaging Desktop. This is a local self-check, not a claim that the user's host loaded the plugin. Only an actual `job_doctor` tool invocation reports a responding MCP host connection. No host acknowledgement is upgraded to confirmed Agent execution.

Automated evidence includes installed launcher/cache relocation, ownership/data preservation and actual shared stdio MCP with two terminal bindings and a fake Desktop host. A new real-host acceptance should install/configure once, invoke one real task through the plugin in a new chat without tool paths in its prompt, end the turn and observe same-chat continuation. The plugin browser/install UI and that real-host workflow remain unverified until separately observed. No repeated artificial CLI wake or manual crash matrix is required of the user.

Official references consulted:

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/desktop-only
- https://learn.chatgpt.com/docs/extend/mcp

Local marketplace development is separate from public-directory approval. Public local-MCP submissions currently require an approved/allowlisted partner; no publication or eligibility is claimed. The later user revision cancels independent OS installers and package repositories; Agent plugin platforms are the future goal, with no publication/submission during development (TODO.md).
