# Wait for background jobs with an active goal

[Documentation index](README.md) · [Commands](commands.md) · [Verification](live-e2e.md#goal-waiting-protocol-proof)

In current **unreleased source**, handing Desktop goal work to Relay includes pausing that goal while accepted jobs run and restoring it on completion. This is required default scheduling: users do not need to add “allow goal pause” or approve each handoff. The originating Agent still uses its native host goal tool; Relay verifies the acknowledgement before entering `waiting`. No Agent/model/goal-RPC polling runs during the wait. An already executing turn or child Agent is not interrupted by a goal status change.

Published 0.1.10 and the original sealed 0.1.11 candidate lack this behavior. Use a matching source build and profile with `goal_wait_host_pause=true`. Current source uses schema v10; finish old runs before migration and preserve the same profile. Earlier storage-only waits require manual host goal handling and cannot authorize automatic continuation. Current host APIs lack goal generation/CAS, so replacement and final verification/write/send races remain experimental limitations.

## Current runtime workflow

The cached skill is a stable entry that reads the selected launcher's `instructions` at each submission, result notification and maintenance request. This command prints the current embedded workflow, policy and content digest without starting a daemon or job. A cached native launcher already points to the original managed installation; retain that installation. The online entry follows its pinned npm runtime and normal release update route.

A one-time migration from historical full cached skills/MCP binaries still needs the known cache refreshed and the host connection reloaded. Thereafter native managed connections verify the installation receipt and follow updated runtime bytes, including same-version source updates; they preserve the connection's state profile and notify the host when its tool catalog changes. They reject modified images, downgrade or profile movement and never replay an uncertain call. No cache scanning or deletion occurs. See [updates and migration](updating.md#cache-inspection-and-refresh).

## Submit from the actual chat

Use the selected launcher's ordinary terminal binding command:

```bash
"$AJR_RELAY" connect --cwd "<absolute-project>"
```

`AJR_RELAY` is the selected native/offline launcher's absolute path. The current Desktop terminal must supply its actual caller and selected Codex runtime. Shared MCP environment, a standalone terminal and guessed thread IDs cannot provide this binding. Recovery (`connect --recover`) remains a separate operation and does not prepare goal waiting.

Expected for an active goal: `goal_wait_policy=required_for_active_desktop_goal`, `goal_state_verified=true`, `goal_wait.status=prepared` and a binding containing `session_key`, `cwd` and opaque `goal_wait_id`. Preparation does not pause or submit work and expires after five minutes. Repeating an unused preparation retains its ID and expiry. A user-paused, completed or absent goal is not adopted. Without the selected runtime's goal API, an Agent in goal mode must stop before submission and report unsupported goal detection.

Pass the **entire binding unchanged** to MCP `job_start` or `job_start_batch`:

```json
{
  "binding": {"session_key":"<returned-key>","cwd":"<returned-directory>","goal_wait_id":"<returned-wait-id>"},
  "spec": {"command":{"kind":"exec","program":"<program>","args":[]},"idempotency_key":"<stable-key>"}
}
```

Expected: original Job/Run IDs, `goal_wait.status=sealed`, `host_pause_required=true` and an admission revision. Ordinary submission cannot bypass an unresolved wait by removing `goal_wait_id`. Retain the IDs, finish other Agent work, then call **this chat's native `update_goal(status="paused")`** as part of the requested Relay delegation. Do not ask the user for additional pause wording or approval. Finish handoff bookkeeping with MCP `job_goal_wait`:

```json
{"goal_wait_id":"<returned-wait-id>","confirm_paused":true,"expected_revision":2}
```

Replace the example revision `2` with the actual admission revision. Only `status=waiting` acknowledges the full handoff; end the turn immediately. A separate CLI/app-server write changes storage without notifying Desktop's cached goal and cannot substitute for the native tool. If native pause is unavailable, rejected or uncertain, cancel this wait and report the capability failure; admitted jobs remain admitted. Never wait by marking the goal complete/blocked, looping on status or starting a checking Agent.

Missing confirmation closes with `attention` after five minutes without changing the host goal. Retry a lost confirmation response with its original revision while waiting; do not repeat the native pause or workload. After a lost admission response, inspect existing original IDs/waits before retrying. Reuse the original binding, keys and specifications; never assign new keys to uncertain work.

CLI `start`, `start-spec` and `start-batch` with `--wake` also detect active goals automatically. The compatibility `--wait-goal` flag asserts that an active goal must exist. CLI confirmation uses `goal-wait <id> --confirm-paused --expected-revision <admission-revision>` after the same native host pause.

## Several commands and interruption

A batch has 1–16 items with distinct stable keys; the eight-active-job quota still applies. Membership freezes to the accepted subset, with each original Job/Run retaining independent execution and stopping. One completion notification waits for every accepted member. Accepted-key retries retain IDs; rejected/uncommitted items cannot join a sealed group and need a fresh group after resolution/cancellation. Local monitoring and automatic stops remain available; `notify_agent` rules are rejected for goal members.

Inspect and cancel future automatic continuation with:

```bash
"$AJR_RELAY" goal-wait <goal-wait-id>
"$AJR_RELAY" goal-wait <goal-wait-id> --cancel --expected-revision <returned-revision>
```

MCP uses `job_goal_wait` with optional `cancel=true` and the observed revision. Cancellation keeps workloads running and leaves explicit manual goal control to the user; an already transmitted host effect cannot be recalled. Stopping an active member also closes group continuation without stopping other members. Closed waits are never rearmed. Direct target moves and implicit host/profile replacement are refused; ordinary notification recovery skips pinned goal members.

## Completion and failure boundaries

| Situation | Relay behavior |
|---|---|
| All original members finish | Verify original paused goal and binding, restore status only, then send one merged result notification |
| Goal deleted or observably replaced/resumed/paused again | Suppress continuation; never recreate a goal or modify a detected replacement |
| Daemon lost before sealing | Recover committed members only, then await native pause confirmation; never replay execution |
| Daemon lost after acknowledged pause | Resume durable observation of the original member runs |
| Restore acknowledgement lost or crash during mutation | Close with `attention`; never replay the mutation |
| Original endpoint temporarily unavailable | Bounded retry with original IDs and expiry |
| Binding/controller/profile/image replaced | Require manual goal handling; never move the wait implicitly |
| Backup restored | Fence all restored controls and grouped notifications; do not touch the old host goal |

Running groups have no separate deadline beyond member timeouts. Completion must resolve within 24 hours of the last member outcome. Delivery IDs, expiry and attempt history survive retries; host acceptance does not prove Agent execution or exactly-once effects. A restored goal can remain active after notification failure, requiring manual inspection. Membership/audits survive archival, with a 10,000-group quota and at most 32 displayed audits; there is no destructive history purge.

## Host compatibility

The controller uses the documented [Codex goal RPCs](https://learn.chatgpt.com/docs/app-server#manage-a-thread-goal) through the actual Desktop-selected executable/profile for bounded reads and restoration, without loading a thread or invoking a model. Restoration omits objectives/budgets, preserving accounting and preventing deleted-goal recreation. Pause acknowledgement is a cooperative assertion from the originating Agent, checked against the paused snapshot; it is not independent host attestation.

Second-resolution timestamps and missing generation/CAS cannot distinguish identical same-second changes, final in-flight accounting that masks a pause change, or every final verification/write/send race. Completion uses the existing experimental Desktop adapter. The user has confirmed Desktop native pause when explicitly requested; the new no-extra-wording default, Windows controller behavior and long-duration savings require separate live acceptance. See [current evidence](live-e2e.md#goal-waiting-protocol-proof).
