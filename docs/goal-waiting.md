# Wait for background jobs with an active goal

[Documentation index](README.md) · [Commands](commands.md) · [Verification](live-e2e.md#goal-waiting-protocol-proof)

An active Desktop goal must pause through its native tool while Relay work runs. If pause permission is missing, ask once: **“允许本次后台任务暂停目标，完成后继续吗？”** A natural affirmative answer is enough for the remaining rounds of this same request and goal. No fixed authorization phrase or per-round question is required. The user has verified locally that automatic pause works after approval; the new direct MCP confirmation form and full multi-round restoration workflow have separate acceptance boundaries.

This is experimental source behavior, using schema v11 and 25 MCP tools. Published 0.1.11 packages do not contain it. Finish existing runs before migration, preserve the original profile and backups, and never downgrade a used database. `goal_wait_host_pause=true` advertises the supported handoff; `automatic_goal_pause_supported=false` means consent-free pause is unavailable.

## 1. Prepare this request

Use the selected plugin launcher's absolute path as `AJR_RELAY`. From the actual chat terminal, run:

```bash
"$AJR_RELAY" connect --cwd "<absolute-project>" --request-label "Three separate scans" --job-count 3
```

Without an active goal, use the returned ordinary binding and submit normally. With an active goal and missing permission, the command succeeds with `status=needs_confirmation`, `session_key` and a `consent` record containing `request_id`, `revision`, scope, expiry and remaining job count. There is no submission binding, wait group, workload or goal mutation yet. This is pending user input, not a tool failure or a blocked goal. The actual caller/runtime must come from the host terminal; shared MCP environment, guessed IDs and external terminals cannot identify this chat.

Keep the request ID. The job count is 1–16, default 1, and limits newly admitted jobs; retries do not consume another slot. If the initial user request already permits this pause, adding `--wait-goal` records that actual permission and skips the question. The flag is a cooperative assertion, not independent host attestation.

## 2. Ask once when needed

Call `job_goal_consent` with the returned `request_id`, `session_key` and `expected_revision`. Default `decision=ask` uses the negotiated MCP form capability. Legacy protocols use `elicitation/create`; protocol 2026-07-28 uses `input_required` with opaque request state and matching input responses. The form must return an accepted response containing `allow_goal_pause=true`. A default selection, empty response, cancellation or silence never approves work.

When the host does not advertise form support, the tool returns `ask_user_now=true`. The Agent asks the displayed short question through an available host question tool that permits permission requests, or ordinary chat. After the actual reply, record `decision=allow` or `decline` with the question's returned revision. Later calls while the question is pending return `question_pending`, without asking again. Plugin/tool-generated messages cannot provide user permission; a plain Agent assertion cannot override a pending native form.

While the answer is pending, end the turn without submitting work, polling, repeating the question or marking the goal blocked/complete. Declining leaves work unsubmitted. A prepared request expires after ten minutes; a claimed question has five minutes. A closed request is not automatically reopened or re-prompted.

## 3. Submit each requested round

After approval, reconnect from the same chat terminal and workspace:

```bash
"$AJR_RELAY" connect --cwd "<absolute-project>" --request-id "<returned-request-id>"
```

Expected: `status=ready`, approved consent and a binding with `session_key`, `cwd` and `goal_wait_id`. Pass the full binding unchanged to `job_start` or `job_start_batch`, using stable idempotency keys. Preparation itself neither submits nor pauses. Accepted jobs consume consent slots atomically with admission; identical accepted-key retries retain their IDs and consume no extra slot.

For three separate calls, submit one job, complete its handoff, inspect its result, then reconnect with the same request ID for the next round. Do not replace separate calls with a batch. Put a requested post-command delay inside that round's background workload, so the delay also occurs while the goal waits. Keep updates brief and give one final summary when all rounds finish.

Consent is pinned to the original registration/endpoint, goal/controller, workspace, request label and job limit. Approval lasts at most 24 hours for admission; it is not a workload timeout. A new request never silently adopts an earlier approval. Reconnection with an exhausted or closed request returns its state and original Job/Run/wait IDs for recovery, without replaying work. `decision=revoke` with the observed revision closes future admission/control/notification while admitted commands keep running; it does not resume an already paused goal.

## 4. Acknowledge the native pause

Admission returns `sealed`, `host_pause_required=true`, original Job/Run/wait IDs and a revision. Finish other Agent work, invoke this chat's native `update_goal(status="paused")` under the actual user permission, then confirm its successful acknowledgement:

```json
{"goal_wait_id":"<returned-wait-id>","confirm_paused":true,"expected_revision":2}
```

Replace `2` with the admission revision. Only `waiting` acknowledges the complete handoff; end the turn immediately. A separate CLI/app-server storage write cannot update Desktop's cached state. If native pause is unavailable, rejected or uncertain, cancel the wait with its observed revision and report the failure once; admitted jobs keep running. Do not ask for renewed permission or replay an uncertain host mutation.

Missing pause acknowledgement expires with `attention` after five minutes. Retry a lost Relay confirmation using its original revision, or an uncertain admission using the original binding/keys/specs. Never repeat native pause or execution to recover a lost acknowledgement. Direct `connect --wait-goal` without scoped options and `start --wake --wait-goal` retain the single-handoff compatibility path; scoped requests are preferred for multiple rounds.

## Completion and recovery

| Situation | Relay behavior |
|---|---|
| All original members finish | Verify the original paused goal/binding, restore status only and send one merged result notification |
| Goal deleted or observably replaced/changed | Invalidate consent and suppress continuation; never recreate a goal |
| Daemon lost before sealing | Recover committed members only, then await native acknowledgement; never replay execution |
| Daemon lost after acknowledged pause | Continue durable observation of the original runs |
| Restore acknowledgement lost or mutation interrupted | Close with `attention`; never replay the mutation |
| Original endpoint temporarily unavailable | Bounded retry with original delivery IDs and expiry |
| Binding/controller/profile/image replaced | Require manual goal handling; never move consent or waits |
| Backup restored | Invalidate consent and fence all restored controls/grouped notifications |

Wait cancellation and manual member stops close future continuation, without stopping other members. Batches freeze accepted membership; rejected/uncommitted items cannot join on retry. Local monitoring/stopping remains available, while `notify_agent` rules are rejected for goal-wait members. Running groups have no separate deadline beyond member timeouts; completion must resolve within 24 hours of the last member outcome. Consent and wait history each have a 10,000-record quota. Delivery expiry/attempts and membership/audits survive retries and archival. Host acceptance is not proof of Agent execution.

## Runtime updates and host limits

Cached skills and MCP initialization provide stable entries to the selected launcher's `instructions`. It reads the current embedded workflow/policy/digest without starting a daemon or job. Managed native MCP connections verify their recorded installation and forward later calls to updated bytes, including same-version source updates, while preserving the profile and negotiated input protocol. One initial migration/reconnection is required for historical full-skill/pre-forwarder entries; subsequent verified native updates need no cache-content rewrite. No cache scanning occurs. See [updates](updating.md#cache-inspection-and-refresh).

The controller uses documented [Codex goal RPCs](https://learn.chatgpt.com/docs/app-server#manage-a-thread-goal) through the actual Desktop-selected executable/profile for bounded reads and restoration. It does not load threads, invoke a model, change objectives/budgets or recreate deleted goals. Agent/terminal consent assertions and native-pause confirmation remain cooperative; they are not independent host attestation.

The host has no goal generation/CAS and uses second-resolution timestamps, leaving identical same-second replacement and final verification/write/send races. Existing turns/child Agents are not interrupted by a goal status change. Windows schema v11/controller behavior, direct MCP form UI, full real-model three-round restoration and long-duration savings remain unqualified. See [current evidence](live-e2e.md#goal-waiting-protocol-proof).
