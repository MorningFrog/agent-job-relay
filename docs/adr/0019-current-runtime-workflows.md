# ADR 0019: Current runtime workflows behind stable plugin entries

## Problem

A source installation can change native bytes and restart its daemon while the host retains a same-version skill snapshot and long-lived MCP server. Matching version strings do not establish matching workflow/code. Historical cached skills can therefore use an obsolete handoff workflow. Current instructions must distinguish supported protocol operations from missing host lifecycle authority; updated bytes alone cannot enable automatic goal pause.

## Decision

Keep native cached skills and MCP initialization as stable instruction entries. At each submission, result notification and maintenance request, load `instructions` through the selected profile-preserving launcher. Maintain the full workflow once in `plugins/workflows/background-jobs.md` and embed it in the executable. Return its policy and digest without creating daemon/job state. Native cached launchers already point to the managed installation; online npm entries retain their pinned release route.

A managed MCP connection captures its original installation/profile, platform, version and image digest. Before catalog/call handling it checks the current receipt/profile/image and takes a shared installation lock through execution, fencing cooperating replacements while permitting parallel readers. If bytes changed, use the verified current executable's internal stdio MCP worker with the connection's explicit state. This includes same-version source updates. Shared workers drop inherited caller IDs; the original arguments and stable workload identities remain untouched. Emit a catalog notification once per runtime image revision.

The worker uses the existing standard MCP transport. Preserve negotiated protocols/capabilities and native input responses/request state. Legacy input is forwarded once with bounded human-wait allowance; modern input-required returns immediately. Initialization/discovery, request/response correlation, bounded frames/deadlines and supervised child cleanup are mandatory. A transmitted request is never automatically retried, including acknowledgement loss. Job execution and delivery remain owned by the existing daemon/runner/storage lifecycles.

## Boundaries

No host-cache scan, deletion, implicit target/profile movement, unknown executable fallback or process-name killing is introduced. A missing, modified, pending-repair, downgraded or differently bound installation fails explicitly. Historical full-skill/pre-forwarder entries need one checked cache refresh and host reconnection; new code cannot replace already running old bytecode. Host notification support and Windows handoff need their own acceptance.

## Verification

A Linux retained standard MCP connection spans two same-version updates without refreshing its cache: a changed real native ELF executes one durable command through the actual worker, then a bounded fixture proves catalog/caller routing, current workflow selection, one notification per image revision, writer exclusion, acknowledgement-loss non-replay and modified-image refusal. Profile/history remain intact. Actual host loaders and real Desktop acceptance are separate evidence levels.
