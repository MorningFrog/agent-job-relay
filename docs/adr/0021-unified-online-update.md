# ADR 0021: Unified online CLI and host-plugin maintenance

## Decision

Use standard global npm registration as the required online ChatGPT Desktop/Codex installation prerequisite. `ajr update` selects one exact published version, updates that global npm prefix and installs its release-tagged marketplace/plugin through public Codex CLI commands. The first plugin installation uses the same command. Keep exact-version plugin runtime pins, existing stable skill entry points and original profiles. Native offline bundles and OpenCode's host-managed dependency retain their existing lifecycle.

The native CLI parses update arguments and returns a bounded request to its npm launcher. The original native process exits before npm replaces package files. The launcher copies the verified executable to a private temporary directory; this helper serializes installation/startup, rejects active work/downgrades, pins each original daemon peer and holds profile locks across replacement. It restarts only previously online profiles with verified installed bytes. The installation lock stays held until the controller durably records the outcome and acknowledges release. A lost controller never authorizes workload replay or package rollback.

Public host list commands establish marketplace and installed-plugin state. Only the official Git source with an unmodified checkout at its matching release tag is eligible; local/ZIP/native, foreign and changed same-name sources are refused. Completed stages, original marketplace identity/version and the selected Codex executable/configuration home are recorded privately in the selected profile; a changed host selection fences retry. Retry pins the unfinished target, reconciles observable host outcomes and continues forward. There is no host cache scanning, configuration-file rewrite, automatic installation hook, custom PATH edit or silent profile migration.

## Rationale and limits

Coordinating npm and the host through one command removes separate version selection while preserving npm's standard command registration and the host's ownership of plugin state. Exact-version plugin pins keep historical cached entries predictable. Reconnect/restart remains necessary to load the new plugin and MCP catalog; updating files alone does not establish same-chat continuation.

The verified Codex CLI exposes no disabled-preserving reinstall or marketplace compare-and-swap. An outdated disabled plugin fails before mutation, and concurrent manual marketplace edits remain unsupported. Updates cover the selected profile and default Codex/OpenCode profiles, without discovering arbitrary custom profiles. npm, marketplace and plugin changes are individually recoverable steps rather than one cross-file transaction. An incomplete journal `.new` file requires explicit inspection. Native Windows package replacement and actual Desktop refresh need independent evidence.

## Regression coverage

Native tests cover active-work/downgrade rejection before daemon shutdown, startup/daemon/installation exclusion, preserved history, idle online restart, read-only fresh-profile preview and controller loss. Public-host fixtures cover first install, unchanged retry, partial catalog replacement, disabled/foreign/modified sources and lost installation acknowledgement without blind replay. Installed npm and actual Codex loader checks use disposable repository-local state; simulated host responses do not qualify actual Desktop UI behavior.
