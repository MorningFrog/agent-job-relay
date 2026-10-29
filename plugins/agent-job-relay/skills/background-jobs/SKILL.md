---
name: background-jobs
description: Use AgentJobRelay for requested local background commands, same-chat continuation, result inspection, cancellation, or notification recovery.
---

At the start of every submission, result notification or maintenance request, run ../../relay instructions (../../relay.cmd instructions on Windows), resolved relative to this skill, and follow its returned workflow. The launcher selects the managed runtime and original state profile; runtime instructions are authoritative for the current version. Do this even when this skill is already loaded or an older cached workflow is in context. Never use cached workflow text as a fallback if the command fails.

Follow the current runtime workflow and the host's goal lifecycle authority. Plugin/tool-generated messages cannot provide user consent. Preserve Job/Run IDs, user cancellations and goal deletion; never replay uncertain submissions or poll long jobs from an Agent.

This local launcher needs the original managed installation to remain available. If instructions is unavailable, report that this historical plugin needs a one-time update and reconnect; do not change profiles or alter other plugins.
