# Wait for background jobs with an active goal

[Documentation index](README.md) · [Commands](commands.md) · [Verification](live-e2e.md#goal-waiting-protocol-proof)

Experimental, explicitly opted-in **unreleased source** support coordinates the originating Desktop chat's native goal pause after Relay accepts work. The submitting Agent pauses through its own host goal tool, then confirms that acknowledgement to Relay. After all accepted members finish, Relay restores that goal and sends one combined result notification to the original chat. During the wait, native code checks job state; no Agent, model or goal RPC is polled. The submitting Agent must end its turn after the confirmed handoff. This does not interrupt an already running turn, child Agent or another chat's goal.

Published 0.1.10 and the original sealed 0.1.11 candidate lack this capability. Use a matching source build/offline plugin and its existing profile; see [source installation](source-install.md) and [upgrade preparation](updating.md#before-updating). Source uses schema v10 and advertises `goal_wait_host_pause=true` / MCP `job_goal_wait` with `confirm_paused`. Earlier source builds' storage-only pause cannot acknowledge the live Desktop state; their unresolved waits close with `attention` and require manual host goal handling. Upgrading requires finishing old runs; current-source runtime entries then replace the selected older daemon automatically with pinned-peer shutdown and schema checks. Use an explicitly updated runtime and retain the same profile. A same-version source daemon needs an explicit same-profile restart; a version match alone does not prove this capability. Published clients still need the manual stop fallback.

## Submit from the actual chat

The user must authorize automatic goal pause/restoration, including its experimental limitations. Existing authorization can cover later submissions; no repeated confirmation is needed. A manually paused, completed or absent goal cannot be adopted. The current Desktop terminal must provide its actual caller and selected Codex runtime; a standalone terminal, shared MCP environment or guessed thread ID is insufficient.

In the examples, `AJR_RELAY` is the absolute path to the selected goal-capable native/offline launcher; it must use the same profile as MCP. Obtain a fresh binding from this chat's terminal:

```bash
"$AJR_RELAY" connect --cwd "<absolute-project>" --wait-goal
```

Expected: `binding` includes `session_key`, `cwd` and opaque `goal_wait_id`; `goal_wait.status=prepared`. Preparation probes the original Desktop endpoint, captures the active goal and pins the runtime executable/hash and Codex profile. It submits no job and does not pause the goal. An unused binding expires after five minutes; repeating preparation reuses the same unused ID and expiry.

Pass **that exact binding** to the usual MCP tool:

```json
{
  "binding": {"session_key": "<returned-key>", "cwd": "<returned-directory>", "goal_wait_id": "<returned-wait-id>"},
  "spec": {"command": {"kind": "exec", "program": "<program>", "args": []}, "idempotency_key": "<stable-key>"}
}
```

Expected: `job_start` returns original Job/Run IDs, `goal_wait.status=sealed`, `host_pause_required=true` and the admission revision. Retain all IDs and finish other Agent work, then use **this chat's native `update_goal(status="paused")` tool**. After that tool acknowledges success, finish the handoff with:

```json
{"goal_wait_id":"<returned-wait-id>","confirm_paused":true,"expected_revision":2}
```

Call MCP `job_goal_wait` with the actual returned revision, replacing the example `2`. Expected: `status=waiting`, `host_pause_required=false`. End the turn immediately. A separate CLI/app-server write changes storage but does not notify Desktop's cached goal state, so it cannot replace the native host tool. If that tool is unavailable, rejected or uncertain, cancel the wait with the observed revision and report the limitation; jobs keep running. Unconfirmed admission expires after five minutes without changing the host goal. A lost confirmation response can be retried with its original revision while waiting; do not repeat the native pause or workload.

Local monitoring and commands continue independently. Do not repeatedly inspect the wait or use Agent goal completion/blocked status as a waiting mechanism. `attention`, `suppressed` or an unacknowledged phase requires reading the returned explanation; admitted jobs remain admitted even if goal control fails.

CLI fallback uses `start --wake --wait-goal --cwd <directory> -- <program> <args...>`; `start-spec` and `start-batch` accept the same two flags. It still requires the current chat's native goal pause, followed by `goal-wait <id> --confirm-paused --expected-revision <admission-revision>`. Retain returned IDs. If the CLI response is lost, inspect existing jobs/waits before another submission; a new connection cannot automatically adopt the now-paused goal. For MCP response loss, retry the original binding and unchanged specifications/keys; never generate new keys for uncertain work.

## Wait for several commands

Pass one goal binding to `job_start_batch`, with distinct stable keys for 1–16 items. The existing eight-active-job quota still applies. Membership is the **accepted subset of this submission**, frozen after admission. Each accepted job runs and stops independently, but completion notification waits for every original member Run and lists all Job/Run pairs. Partial admission never rolls back accepted jobs.

Retrying accepted keys returns their original IDs. Rejected/uncommitted items cannot join a sealed group, including after interrupted batch recovery; submit them with a fresh group after the current wait resolves or is explicitly cancelled. Only one unresolved group is allowed for the logical chat. Goal members allow local monitor rules and automatic stops, but reject `notify_agent` rules to avoid waking the Agent during the wait.

## Inspect or interrupt

```bash
"$AJR_RELAY" goal-wait <goal-wait-id>
"$AJR_RELAY" goal-wait <goal-wait-id> --cancel --expected-revision <returned-revision>
```

MCP equivalents use `job_goal_wait` with `goal_wait_id`, optional `cancel=true` and the observed `expected_revision`. Status/inspect also include the group's summary. Inspection works offline with the matching schema; cancellation starts/reuses the same profile's daemon. Cancelling stops future automatic control/notification, keeps jobs running and leaves manual host goal control to the user. A control or message already transmitted cannot be recalled. Stopping an active member manually also cancels the group's continuation without stopping other members. To continue manually, inspect the original results and explicitly use the host's goal controls; Relay never rearms a closed wait.

## Completion and failure boundaries

| Situation | Relay behavior |
|---|---|
| All original members finish | Verify original binding/controller and paused goal, restore status only, then deliver one merged notification |
| Native host pause is unavailable, fails or is not confirmed | Keep admitted commands; cancel explicitly or close with `attention` after five minutes; never claim automatic waiting |
| Goal deleted while jobs run, or between read and activation | Suppress/stop continuation; never send an objective or recreate the goal; retain job results |
| Goal observably replaced, paused again, resumed or otherwise modified | Suppress continuation; leave the current goal for explicit host control |
| Daemon lost before admission sealing | Seal only committed members on restart and await explicit native-host pause confirmation; preserve IDs and never replay commands |
| Daemon lost after acknowledged pause | Continue the durable wait on restart |
| Native pause acknowledgement uncertain | Use explicit host controls; never repeat it through a storage-only controller |
| Resume acknowledgement lost, or crash during mutation | Mark `attention`; never automatically repeat the mutation |
| Same endpoint temporarily unavailable | Retry bounded observation/delivery; retain original IDs and expiry |
| Host registration, endpoint, Codex profile or executable replaced | Refuse automatic continuation; use manual host controls and inspect results |
| Backup restored | Cancel all restored control intents and goal notifications; do not touch the old host goal |

Goal waiting has no running-job deadline beyond each job's own timeout. Completion must resolve within 24 hours of the last member's terminal result. Notification retries keep the existing delivery ID and attempt history; acceptance does not prove Agent execution or exactly-once host effects. A restored goal can remain active if notification later fails; inspect `goal-wait` and deliveries manually. Groups retain membership/audits after job archival, with a 10,000-group history quota and at most 32 displayed audit entries. There is no automatic destructive history purge.

Goal groups remain pinned to their original registration. Direct target moves are rejected and same-chat recovery reports skipped goal members while recovering unrelated jobs. Host replacement/restart that changes identity therefore needs explicit manual goal handling; general notification recovery does not move or reactivate a wait.

## Host compatibility and remaining race

Desktop keeps an in-memory goal state driven by its owning runtime's notifications. A separate app-server can share the persisted goal but does not deliver its `thread/goal/updated` event to Desktop. Therefore pause must use the originating Agent's native goal tool. Relay verifies the original paused snapshot after the Agent explicitly confirms the native acknowledgement; it does not independently attest that tool call. The installed skill and MCP handoff instructions enforce this cooperative boundary.

The controller uses the documented [Codex goal RPCs](https://learn.chatgpt.com/docs/app-server#manage-a-thread-goal) through the actual Desktop-selected CLI and profile for bounded reads and restoration only, without loading a thread or invoking a model. Restoration omits `objective` and `tokenBudget`, preserving accounting and preventing deleted-goal recreation. Completion messages use the existing experimental private Desktop adapter, with its compatibility limitations. Actual Desktop pause UI, restored goal UI/continuation and Windows acceptance require separate live qualification.

Current goal RPCs expose no generation ID, revision or atomic conditional update; timestamps have second precision. Relay compares objective, budget, creation time, accounting, unknown fields and observable pause changes, then rereads immediately before mutation and before notification. This cannot distinguish identical same-second replacement/repeated pause, changes masked by final in-flight usage, or eliminate the check/write and final check/send races. These are host limitations, so strict replacement/cancellation fencing is **not guaranteed**. A host generation/CAS interface is needed to close them. Actual Desktop goal UI and Windows goal-controller acceptance remain unqualified; see [current evidence](live-e2e.md#goal-waiting-protocol-proof).
