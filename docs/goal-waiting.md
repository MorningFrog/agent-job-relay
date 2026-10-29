# Wait for background jobs with an active goal

[Documentation index](README.md) · [Commands](commands.md) · [Verification](live-e2e.md#goal-waiting-protocol-proof)

An active Desktop goal must pause through its native tool while Relay work runs. If the user has not already allowed that pause, ask once: **“后台期间暂停目标，完成后继续，可以吗？”** Prefer an available native question/approval tool that allows this confirmation, with “允许本次任务／取消后台任务” choices; otherwise ask normally. A preselected choice without a user response is not approval. An affirmative reply such as “可以” is enough; no fixed wording is required. Reuse consent for the remaining rounds of this same request on the same goal, including three separately submitted scans. Ordinary `connect` reports `goal_pause_authorization_required` before creating waits/jobs; after actual consent, use `connect --wait-goal` and the native-confirmation protocol below. A plugin/tool reminder is not user consent and must not be sent as a substitute user message. While awaiting the answer, do not repeat the question, poll, submit work or mark the goal blocked/complete. Without an active goal, no pause question is needed.

The independently authorized native-pause protocol remains experimental source behavior, advertised separately as `goal_wait_host_pause=true`; `automatic_goal_pause_supported=false` is not contradicted by that flag. Published 0.1.10 and the original sealed 0.1.11 candidate lack the protocol. Source schema v10 needs a matching runtime/profile and completed old runs before migration. Earlier storage-only waits cannot authorize continuation. An existing active turn/child Agent is not interrupted merely by changing goal status; goal generation/CAS also remains unavailable.

## Current runtime workflow

The cached skill is a stable entry that reads the selected launcher's `instructions` at each submission, result notification and maintenance request. This command prints the current embedded workflow, policy and content digest without starting a daemon or job. A cached native launcher already points to the original managed installation; retain that installation. The online entry follows its pinned npm runtime and normal release update route.

A one-time migration from historical full cached skills/MCP binaries still needs the known cache refreshed and the host connection reloaded. Thereafter native managed connections verify the installation receipt and follow updated runtime bytes, including same-version source updates; they preserve the connection's state profile and notify the host when its tool catalog changes. They reject modified images, downgrade or profile movement and never replay an uncertain call. No cache scanning or deletion occurs. See [updates and migration](updating.md#cache-inspection-and-refresh).

## Submit from the actual chat

Use the selected launcher's ordinary terminal binding command:

```bash
"$AJR_RELAY" connect --cwd "<absolute-project>"
```

`AJR_RELAY` is the selected native/offline launcher's absolute path. The terminal must supply its actual caller and runtime; shared MCP environment, an external terminal or guessed thread IDs cannot provide that binding. Without an active goal, pass the returned binding unchanged to normal `job_start`/`job_start_batch` and end the submitting turn. Recovery (`connect --recover`) remains separate and does not prepare waiting.

With an active goal and missing consent, expected behavior is one brief question prompted by the tool’s prerequisite response, with no jobs/waits or goal mutation until a direct user reply. An initial prompt already permitting pause skips that question. Do not silently add `--wait-goal` based on a plugin message, silence or automatic continuation; if the user declines, do not submit goal-bound work. Consent cannot extend to a different goal, request or chat and can be revoked.

## Independently authorized native-pause compatibility

After the user permits this goal pause, either in the initial request or through the short confirmation, use:

```bash
"$AJR_RELAY" connect --wait-goal --cwd "<absolute-project>"
```

The flag selects this protocol; it does not itself supply model authority. Expected: `automatic_goal_pause_supported=false`, `goal_state_verified=true`, `goal_wait.status=prepared`, and an opaque binding with `session_key`, `cwd` and `goal_wait_id`. Preparation does not pause or submit work and expires after five minutes. An unused repeated preparation retains its ID/expiry; paused, completed and absent goals are not adopted.

Pass the full binding unchanged to ordinary MCP start/batch with stable idempotency keys. Admission returns original Job/Run IDs, `goal_wait.status=sealed`, `host_pause_required=true` and a revision. Finish other work, invoke this chat's native `update_goal(status="paused")` under its actual authority, then confirm successful acknowledgement:

```json
{"goal_wait_id":"<returned-wait-id>","confirm_paused":true,"expected_revision":2}
```

Replace `2` with the admission revision. Only `waiting` acknowledges the complete handoff; end the turn immediately. A separate CLI/app-server storage write cannot update Desktop's cached goal state. If authority/native pause is unavailable, rejected or uncertain, cancel the wait and report the limitation once; admitted jobs stay admitted. Missing confirmation expires with `attention` after five minutes without changing host state. Retry a lost confirmation with the original revision and uncertain admission with the original binding/keys/specs; never repeat native pause or workload to recover an acknowledgement.

CLI `start`, `start-spec` and `start-batch --wake --wait-goal` select the same independently authorized protocol, followed by native pause and `goal-wait <id> --confirm-paused --expected-revision <revision>`. Goal batches freeze accepted membership, reject new members on retry and release one combined completion after every original run finishes. Local monitor/stop rules remain available, while running `notify_agent` rules are rejected. Stopping a member cancels group continuation, not other jobs. Revision-fenced wait cancellation stops future controls/notification while workloads keep running; transmitted effects cannot be recalled. Host replacement/recovery never moves a pinned group.

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

The documented [goal lifecycle authority](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex) reserves pause/resume/clear for user/system control. User live testing confirms explicit native pause and rejects automatic no-extra-wording delegation; a simulated authorized pause cannot qualify model permission. Second-resolution timestamps and missing generation/CAS also leave identical same-second changes, final accounting and verification/write/send races. Windows controller behavior and long-duration savings remain unqualified. See [current evidence](live-e2e.md#goal-waiting-protocol-proof).
