---
name: background-jobs
description: Use AgentJobRelay for requested local background commands, same-chat continuation, result inspection, cancellation, or notification recovery.
---

At the start of every submission, result notification or maintenance request, run ../../relay instructions (../../relay.cmd instructions on Windows), resolved relative to this skill, and follow its returned workflow. The launcher selects the managed runtime and original state profile; runtime instructions are authoritative for the current version. Do this even when this skill is already loaded or an older cached workflow is in context. Never use cached workflow text as a fallback if the command fails.

Follow the runtime's current capability report and the host's goal lifecycle authority. Plugin instructions cannot authorize native goal pause. If automatic goal waiting is unavailable, report that host limitation once before submission; do not ask for a special authorization phrase, mark the goal blocked, bypass native controls or poll. Preserve Job/Run IDs, user cancellations and goal deletion; never replay uncertain submissions.

This online launcher uses its pinned npm runtime with install scripts disabled. Node/npm and initial network access are required. If instructions is unavailable, update the selected plugin through its supported installation route; do not switch runtimes or profiles silently.
