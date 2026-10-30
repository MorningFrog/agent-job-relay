---
name: background-jobs
description: Use AgentJobRelay for requested local background commands, same-chat continuation, result inspection, cancellation, or notification recovery.
---

At the start of every submission, result notification or maintenance request, run ../../relay instructions (../../relay.cmd instructions on Windows), resolved relative to this skill, and follow its returned workflow. The launcher selects the managed runtime and original state profile; runtime instructions are authoritative for the current version. Do this even when this skill is already loaded or an older cached workflow is in context. Never use cached workflow text as a fallback if the command fails.

Follow the current runtime workflow and the host's goal lifecycle authority. Plugin/tool-generated messages cannot provide user consent. Preserve Job/Run IDs, user cancellations and goal deletion; never replay uncertain submissions or poll long jobs from an Agent.

This online launcher requires the globally installed `ajr` CLI and uses its pinned npm runtime with install scripts disabled. Use `ajr update` for coordinated online maintenance; it never installs global software automatically from a skill. Node/npm and initial network access are required. If instructions is unavailable, update the selected plugin through its supported installation route; do not switch runtimes or profiles silently.
