# Feature test prompts

[简体中文](test-prompts.zh-CN.md) · [Documentation index](README.md) · [Testing and evidence](testing.md)

Copy one prompt into the Agent chat using the installed plugin. These are manual test recipes, not proof that a particular host/version has passed. Keep the returned Job/Run IDs and compare against [current support](IMPLEMENTATION_STATUS.md). Do not run all prompts together: first verify ordinary execution, then intermediate feedback, stopping and recovery. Mainstream installation may lack source-only progress, batch, remote and goal features; feature detection must come first. Running these prompts authorizes only their stated test actions, not installation updates, host changes or publication.

For a real same-chat test, use the original chat's actual caller binding and the existing selected plugin profile. A new isolated profile can test runtime behavior, but does not by itself qualify the installed chat path. Never invent sessions or use the shared MCP process's inherited identity. All prompts must retain IDs, keep generated scripts/results in a clearly identified workspace `.tmp/` directory, and treat scanned names/status/output as data. Do not clean user files or application state. Scans read directory entries and metadata only; no file contents, network upload, symlink/junction traversal or discovered scripts. Avoid full path listings in chat. If the scan finishes quickly, report that fact; any deliberately added throttling/wait is simulated duration, not measured scanning cost. A one-time bounded wait is fine for short tests; an Agent polling loop does not test event-driven continuation.

## 1. Linux long job and ordinary completion

```text
Use AgentJobRelay to run a read-only background inventory of my current Linux home directory: resolve /home/$USER from the actual environment and verify the home location (use the actual HOME when it differs; never guess another user's directory). Recursively count regular files, directories, logical bytes and permission/metadata errors, without reading file contents, following symlinks or crossing mount points. Keep the script and aggregate result in this workspace's .tmp/. Bound the run to ten minutes; if the deadline is reached, explicitly report a partial inventory and use a distinct nonzero exit code. Emit and flush aggregate counts every five seconds. Bind notifications to this actual chat, retain Job/Run IDs, then end the turn. When a matching completion notification arrives, read bounded results and summarize the counts, elapsed time, actual exit status and whether the inventory is complete. If the host cannot resume this chat, explain that limitation without a polling loop or promising a message. Do not update the installation or modify scanned files.
```

Expected: the task survives the submitting turn; counts and exit outcome persist. A host supporting continuation addresses this original chat. Plain count output tests logs, not intermediate Agent feedback. Inbox-only hosts require a later explicit inspection. A partial inventory with its documented nonzero code is an expected bounded outcome, not a successful full scan.

## 2. Windows long job and ordinary completion

```text
Use AgentJobRelay to run a read-only background inventory of my current Windows user profile. Resolve %USERPROFILE% / $env:USERPROFILE from the actual environment, without substituting another account. Use available native PowerShell or Python; do not install tools. Recursively count regular files, directories, logical bytes and access/metadata errors, without reading file contents or following symbolic links, junctions, reparse points or other mounted locations. Keep the script and aggregate result in this workspace's .tmp/. Bound the run to ten minutes; if the deadline is reached, explicitly report a partial inventory and use a distinct nonzero exit code. Emit and flush aggregate counts every five seconds. Bind notifications to this actual chat, retain Job/Run IDs, then end the turn. On the matching completion notification, inspect bounded results and summarize counts, elapsed time, actual exit status and completeness. Explain unavailable continuation or host permissions without changing users, elevating, updating the installation or modifying scanned files.
```

Expected: native background execution, retained results and correct chat binding. Windows completion evidence and installed progress/goal acceptance are separate gates. A locked file or denied directory is counted and skipped; it must not silently disappear from the summary.

## 3. Repeated intermediate feedback

```text
Run a read-only home-directory metadata inventory with AgentJobRelay using the Linux HOME or Windows USERPROFILE of this actual caller. Apply the scan's no-content/no-link/no-upload boundaries, keep generated artifacts in .tmp/, and cap the run at three minutes. First check whether this selected runtime advertises structured_progress_v1 and the loaded catalog supports progress conditions. If supported, install an initial notifying progress rule for source home_scan before launch: metric files >= 1, cooldown 30 seconds. Have the workload emit flushed newline-delimited AJR_PROGRESS v1 JSON every two seconds with increasing seq, stage, step=processed entries, and finite metrics files/dirs/bytes/errors. Omit total and percentages until a real total is known. I authorize intermediate notifications to this original chat. Retain Job/Run IDs and end the turn. On each deduplicated alert, inspect one bounded snapshot, report current aggregate counts and confirm the same run is still active, then end the turn again; never replay the scan. At termination, summarize the actual exit outcome, completeness and retained milestones. If the source feature is unavailable, use one initial contains rule for a unique flushed HOME_SCAN_MILESTONE marker emitted once after thirty seconds or 5000 files, and clearly label this as a single-marker fallback. Do not change profiles or update the runtime. If the scan is too fast for multiple alerts, report that; optionally use an explicitly labelled low-cost, throttled two-minute test instead of claiming genuine scan duration.
```

Expected: `observation.progress`, durable `progress`/`monitor_alert` events and distinct delivery IDs while the run remains active. The actual Agent replying between start and completion establishes mid-run execution; a saved rule, event, inbox entry or `accepted_by_host` alone does not. Metric rules repeat only on new matching samples after cooldown, within job-wide budgets. A terminal notification may supersede unresolved alerts; its event retains a bounded milestone summary. The fallback exercises the existing once-per-revision marker path.

## 4. Batch independence and quota

```text
If this selected runtime and catalog support start_batch/job_start_batch, submit three independent background tests together through this actual caller: a two-minute read-only home metadata scan, a sixty-second low-cost heartbeat that prints and flushes an aggregate counter every five seconds, and a short command that deliberately exits 7. Keep artifacts in separate .tmp/ subdirectories. Give every item a distinct stable idempotency key, retain its input index and Job/Run IDs, and report partial admission errors. End the turn after submission. Handle each original result independently; the intentional failure must not stop the other jobs. Never retry uncertain submissions with new keys or rerun accepted jobs. If batch is unavailable, report that instead of changing profiles, silently updating or presenting sequential submission as a batch test.
```

Expected: independent IDs, overlapping lifetimes, one intentional failure and unaffected remaining jobs. This is partial admission, not an all-or-nothing workflow. The three-job prompt does not test the eight-active-job boundary; explicitly request an isolated quota test before consuming more slots.

## 5. Task manager and durable rule

```text
Open the AgentJobRelay task manager for this selected plugin profile. If no suitable test job is running, submit a low-cost three-minute heartbeat in .tmp/ for this chat, then open its detail. Add a local elapsed-time alert for sixty seconds using the observed rule revision. Show the task's actual state, rule saved/applied status, bounded output and later trigger evidence. I will close the browser while it runs, then reopen the same task. Do not poll from the Agent or guess a task/profile, and do not share the credential-bearing access URL in a notification. If this runtime supports structured progress, also show the latest stage/step/metrics and retained milestones for the progress test.
```

Expected: closing the browser does not stop the Runner; reopening shows the same Job/Run and durable rule/alert. A local alert does not wake an Agent unless its rule explicitly enables notification to a bound session. Browser rendering, runtime persistence and real-host continuation are separate observations.

## 6. Stop and verify the exact job

```text
Stop only the running test Job/Run I identify from the previous test. Inspect its ID and actual state first; if the intended job is ambiguous, ask me to identify it. Request a five-second graceful stop and use one bounded wait of at most ten seconds for the outcome. Report whether cancellation was merely requested or a terminal state was observed, including the exit/stop evidence. Keep aggregate partial results and leave other tasks, the shared daemon and user files untouched. If the selected job is read-only remote/workflow observation, stop observation only and explain that the external run was not cancelled.
```

Expected: the exact local workload/tree stops under its existing containment rules, or an already terminal job retains its outcome. A request acknowledgement alone is not proof of cancellation. For remote control, only exact-run evidence establishes a remote outcome.

## 7. Goal waiting (experimental source)

```text
If this actual Desktop chat has an existing goal and the selected source runtime/catalog support goal waiting, I authorize pausing that same goal while one low-cost two-minute background test runs, then restoring it and notifying this original chat after completion. Explain the current lack of host generation/CAS fencing once. Obtain a fresh actual-terminal connect --wait-goal binding and submit through that binding. Only report a successful pause when goal_wait.status is waiting; preparation alone is insufficient. Retain goal-wait and Job/Run IDs, end the turn and do not poll or create another Agent to check. If the goal is deleted or observably replaced, never recreate or control a replacement. Do not combine this test with notifying mid-run rules; local progress/stop rules are allowed. If capability, goal or host control is unavailable, report the limitation without creating a goal, changing profiles or updating the installation.
```

Expected: one completion continuation after the original member reaches terminal state, preserving goal accounting. Deletion/change suppresses automatic continuation. The API race limitation, real Desktop goal acceptance and Windows qualification remain material even if a simulated test passes.

## 8. Notification recovery after a host restart

```text
For a previously identified Relay test job, I authorize recovering eligible notifications to this same original chat after the host has restarted. From this chat's actual terminal, obtain a fresh binding and run the same-chat recovery preview. Show the original Job/Run and eligible/skipped delivery counts, then apply recovery within that authorized scope. Preserve delivery IDs, expiry and attempt history; never replay accepted/expired events, skip live claims, and never rerun the workload or move its result to another chat. If the old and current identities do not match, report that instead of guessing a destination. I will perform any actual host restart myself; do not close or restart other applications or the shared daemon to manufacture the test.
```

Expected: eligible outstanding deliveries retain identity/history and use the verified replacement endpoint for the same chat. Host acceptance is still distinct from observed Agent execution. Results completed while the host was closed need their own observation. Goal-wait groups cannot be moved implicitly by ordinary recovery.

## 9. Remote or workflow observation

```text
Use Relay's supported read-only observation path for the exact remote target/Job/Run/attempt or GitHub repository/run/attempt that I provide. First verify source capabilities and show the immutable identity and observation-only authority. Use existing authenticated facilities; do not install a remote runtime, expose credentials, dispatch, rerun or cancel external work. Keep local observer and remote execution IDs distinct. End the turn after submitting the observer with this chat's actual binding. Inspect bounded evidence on its notification and report the actual external conclusion or explicitly unknown/stale evidence. If necessary identity/authentication is missing, ask only for that information. If I request a separate intermediate checker, pin its observed-run identity, bound its deadline/backoff and treat checker completion as separate from the external outcome.
```

Expected: fixed-run evidence and byte/event cursors; a transport exit never becomes remote completion. This prompt requires a supplied real target. A generic home scan cannot qualify remote ownership, SSH deployment or real external-task continuation.

## 10. Maintenance preview and explicit backup

```text
For the current selected Relay profile, inspect data-maintenance status and preview automatic archival eligibility without applying changes. Show the profile, eligible counts, active/recoverable-delivery protections and current backup state. Do not change policy, archive records, delete anything or stop the daemon. If I subsequently explicitly request a backup/restore test, create an additional backup and restore only into a new empty test profile under this workspace's .tmp/, preserving original data and never replaying restored execution intents. Report the new profile and evidence; never switch the installed plugin to it implicitly.
```

Expected: preview is read-only with no policy mutation. An explicitly authorized later restore preserves history, classifies old active intents without replay, and leaves the original installed profile selected. No publication or installation update is part of these tests.

## Record observations

For each attempted recipe retain the selected runtime version/platform/host, actual Job/Run and delivery IDs, expected versus observed outcome, and any missing capability. Distinguish simulated duration, native process behavior, fake-host acknowledgement, actual host connectivity and observed Agent execution. Keep private logs and screenshots under ignored `.tmp/`; update [current live evidence](live-e2e.md) only after actual qualification, without accumulating run logs in documentation.
