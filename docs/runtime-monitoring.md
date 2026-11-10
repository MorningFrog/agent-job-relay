# Inspect and monitor tasks

[Documentation index](README.md) · [Command reference](commands.md) · [Daily usage](usage.md)

These interfaces require runtime 0.1.10 or newer, including the running daemon. Commands below use the published online ChatGPT/Codex entry, `agent-job-relay codex`; substitute `agent-job-relay opencode` for OpenCode or the installed `relay` / `relay.cmd` for a native offline profile. npm 0.1.11 and newer also provide `ajr`. Keep the same profile between plugin, terminal and browser; see [command entry points](commands.md#choose-the-entry-point-and-profile). `doctor` reports explicit argv for the selected profile.

## Open the bundled Web UI

```bash
agent-job-relay codex ui
agent-job-relay codex ui <job-id> --no-open
```

Expected: the first command opens the task manager and prints its access URL; `--no-open` prints it for manual opening. UI resources are built with React + Vite and embedded in the native executable, including offline/npm distributions. Recipients need no frontend installation. Each daemon enables its loopback HTTP listener by default on an available port. Opening `ui` starts/reuses the selected daemon without submitting a job or binding a chat. `ui --install-root <installation>` selects an offline installation's recorded profile. Browser access does not establish an Agent identity.

The current source workflow automatically shows the accepted background task in the originating chat’s embedded browser before yielding or handing off a goal. It keeps one panel per chat/profile: first find the existing embedded tab by the fresh `ui <job-id> --no-open` URL's exact origin/path, ignoring token/job fragments, then navigate and show that same tab ID. Only a successful inventory with no match permits a new tab; batches and repeated reopen requests reuse it. The panel accepts new task/credential fragments without a reload. Hosts without an embedded browser receive a local link; missing tab-discovery/reuse capability is reported rather than repeatedly creating panels. Opening failures do not alter accepted work. Published/sealed runtime workflows must be updated through their supported installation route to gain this behavior.

The task list supports search, state filters and older-history pages. Runtime 0.1.22+ task/alert page headers show running (including launching) and queued counts beside the title, using only the latest active-task snapshot. Loading history, searching or selecting tasks does not change those counts. Before the first sample they show “—”; a lost connection retains the last sample with an explicit disconnected/reconnecting label. Historical failures are task states, not unchecked reminders; use state filters for history and task/runtime details for notification diagnostics.

At widths up to 1250 CSS pixels, navigation collapses to an icon rail that expands on hover or its keyboard/touch toggle. Navigation buttons keep their positions during expansion and provide hover/focus feedback; task details expand immediately below the selected row. The selected row and its expanded details share a continuous left selection rail. Select the row again to collapse it; chevrons indicate its state. Wider windows retain adjacent list/detail columns. Current source task details open on terminal output below a compact title/status/stop header; stdout/stderr and follow controls stay in the terminal toolbar. Scrolling upward pauses following; “继续跟随” returns to the newest output. Progress, feedback/notifications, rules and events use separate tabs (arrow/Home/End keyboard navigation), with concise notification/Goal attention hints always visible. Full command, original IDs, directory and timing remain under “命令与执行信息” below the selected view. Rule changes display saved versus applied revisions; edits require the revision last observed. Runtime status identifies the profile and notification failures. The alert view lists recent hot alerts and opens task events; archived task details retain their original events. Browser updates pause while hidden, and reconnect with bounded backoff. Closing the browser affects only viewing.

Runtime 0.1.17+ supports [recognizable task names](usage.md#name-tasks) through CLI `start --title` or MCP/JSON `spec.title`; the runtime workflow asks Agents to name new tasks in the user’s language. Unnamed history shows bounded command/project hints, summarizing inline interpreter bodies as scripts. Search matches names, command previews, directories and IDs. Select a named task: expect the same name with wrapping and a reachable close button; expand execution information for the original IDs and full command.

For source GitHub Actions observations, task details show job state, current step and ended/known step counts for up to 20 jobs. The GitHub link opens the pinned run/attempt, including before a successful query or when step details are unavailable. Counts describe steps rather than elapsed-time percentage; partial matrices are labeled. See [workflow observation](remote-execution.md#observe-an-existing-github-actions-attempt).

Current-source viewers on the same listener share overview and same-task samples for up to one second, including serialization, with at most eight cached selections. Runtime health is sampled at most once per ten seconds while viewed and carries its own `sampled_at`; notification counts can lag by that interval. No viewer means no sampling queries. Explicit inspection/doctor and rule/stop requests bypass the viewer cache; storage errors after expiry are reported instead of serving an earlier successful task snapshot. Cold task details decode one validated archive snapshot per inspection, preserving event/delivery limits and cursors.

The access credential is generated per daemon, delivered through private IPC and placed in the URL fragment. The UI moves it into origin-scoped session storage and removes the fragment; API/SSE requests use a bearer header. Ordinary doctor/MCP health responses omit credentials. Reopen `ui` after daemon replacement; old pages cannot authenticate to the new listener. Treat access URLs as local credentials. HTTP binds only `127.0.0.1`, validates Host/Origin and exposes bounded inspection, rule and stop operations. `AJR_WEB_UI=off` is an explicit opt-out at daemon startup. A listener failure is reported by doctor; it does not silently stop workloads. The daemon and raw CLI submission do not open a browser themselves; the host Agent performs the embedded-panel step after accepted delegation.

## Inspect from a terminal

```bash
agent-job-relay codex inspect
agent-job-relay codex inspect <job-id>
agent-job-relay codex inspect <job-id> --watch
agent-job-relay codex inspect <job-id> --json
```

Expected: summaries identify the selected profile and runtime connection; a task view includes observation timestamps, output tails, rules, stop requests and recent events. Overview pages use descending `before`/`next_cursor`; active jobs are returned separately. JSON watch produces bounded newline-delimited snapshots; a human terminal redraws once per second. Ctrl-C ends the viewer only. Agent clients use `job_inspect` once when needed rather than running a watch loop.

Inspection never starts a daemon, migrates a database or creates an empty profile. When the runtime is absent, compatible current-schema history is read through a read-only connection and labelled `offline_snapshot`; process liveness is unverified. Unsupported older schemas require a matching runtime. Active observation ages exceeding five seconds require investigation; no output alone does not prove a hang.

Each stream retains a rolling 64 KiB tail and total-byte offset independently from its original 16 MiB log prefix. Detection continues after that prefix fills. Tail metadata identifies its retained range and truncation; binary/invalid text is displayed with replacements, while capped raw logs retain original bytes. Observations are sampled approximately once per second and flushed at completion; abrupt runner loss can leave the last snapshot stale.

## Notification status and compact results

Current source's “反馈” tab shows the “中途反馈” card with configured rules, runner application, observed progress sources, recorded notification targets and passive quiet-output advisories, followed by notification/recovery details. The concise hint above the tabs opens this view directly. Stage and elapsed-checkpoint shortcuts open an editable rule form; only Save changes configuration. Stage feedback needs matching structured output, elapsed checks do not estimate percent complete, and completion-only preferences remain valid. Goal feedback retains its original authorization and restoration budget.

Source 0.1.23 stage feedback shows the bounded pending-stage count and rule stage while waiting for cooldown/rate limits. Goal details correlate the current feedback or final delivery, its attempts, latest control audit and remaining cycles, and show how many earlier alerts were combined. Host acceptance remains separate from Agent execution; stopped final delivery retries require original-host handling. See [stage limits](progress.md#configure-repeated-feedback) and [Goal diagnostics](goal-waiting.md#completion-and-recovery).

Notification diagnostics include a read-only `recovery.plan`: a specific next action, original Job/Run/delivery/wait/request IDs, expiry and applicable goal revisions. Goal handoffs take precedence over ordinary binding recovery. Expired events require inspecting results/attempts; explicit retry changes expiry. Pending delivery needs no Agent polling. Binding recovery provides structured `desktop_preview`/`desktop_apply` argv for the exact profile, displayed as quoted commands in the panel for use only from the original chat terminal. A removed/replaced loaded executable returns `commands_available=false` instead of an unusable path; select the current trusted entry from the original chat while retaining the recorded profile. The panel does not execute recovery or infer chat identity; uncertain native controls and unexplained blockers require explicit host handling.

Runtime 0.1.18+ task details show notification state separately from execution: pending, retrying, stopped retries, inbox-only or accepted by host. Counts cover all retained deliveries, not only the displayed first page; terminal notification status summarizes completion deliveries separately from intermediate alerts. The newest unresolved error includes its delivery ID, failure class, expiry and retry time. Expand attempt history for a bounded page of recorded outcomes. Host acceptance leaves Agent execution `unconfirmed`.

Goal wait identity/status/revision and native-pause requirements remain visible. Recovery instructions return you to the original host caller: ChatGPT Desktop uses its recorded launcher/profile to preview `connect --recover`, then explicitly apply eligible recovery; OpenCode uses its caller-owned `relay_recover` tool. Inbox-only hosts inspect retained results. The browser cannot authenticate a chat or rebind a target. Recovery does not rerun commands or replay accepted/expired notifications; goal waits retain their original binding and need explicit host goal handling. An explicit delivery retry has different TTL semantics; inspect attempts before choosing it.

MCP `job_inspect` defaults to a compact result: original Job/Run and title, exit/timing, at most 2 KiB of text per stream, recent progress/milestones, notification and goal diagnostics, and evidence cursors. `recent_events_after` retrieves omitted retained event detail; `events_after` continues after the observed newest event, and `deliveries_after` continues the detailed delivery page. Output remains untrusted data. Use `full=true` for complete tails, rules and recent events. CLI `inspect <job-id> --json --compact` shares this summary; ordinary CLI JSON and HTTP details keep their complete contract. Overview pagination is unchanged. Read once on a relevant event and yield again while work continues.

## Read and export retained logs

Current source supports full-history search through the panel, CLI `ajr search` and MCP `job_search`. Text matches title, full local or submitted remote launch command (including shell interpreter), pinned remote target ID, local directory and Job/Run IDs using Unicode lowercase substring matching; `status` accepts an execution state or `active`, `cwd` is a directory substring, and inclusive `created_after`/`created_before` use Unix milliseconds. Each read-only page examines at most 128 hot/cold records with a 500-ms soft budget and validates each accessed cold snapshot. `has_more=true` requires continuing with `next_cursor`, including empty result pages. The panel continues bounded pages until 50 matches or the real end, shows scanned counts and cancels changed queries. This is cursor-based scanning without a full-text index or schema migration; a large archive can take time, and pagination is not a cross-request snapshot. CLI search also works against an existing stopped profile without starting a daemon.

```bash
ajr search --text "project tests" --status failed --cwd project --limit 20
ajr search --text "project tests" --before <next-cursor>
ajr search-logs <job-id> --stream stderr --text "error" --limit 20
ajr search-logs <job-id> --stream stderr --text "error" --cursor <next-cursor> --end <end>
```

MCP `job_search_logs` and the panel's log finder share the same case-sensitive literal byte search. Each request scans at most 512 KiB of candidate starts, plus bounded match/context lookahead, and returns at most 100 nonoverlapping matches with original `offset`/`end`, `context_from` and lossy UTF-8 text. Continue `next_cursor` with the returned fixed `end` until `complete=true`, including pages without matches. Adjacent retained blocks can match across their boundary; missing intervals cannot. `gaps` identify output that cannot be searched. Running recent segments may rotate between reads; retained evidence is sampled, not an atomic whole-log snapshot. Clicking a panel match reads context at its original byte position. Search never changes rules, deliveries or task execution.

Runtime 0.1.18+ native exec/shell runs keep the original 16-MiB prefix plus four rotating 1-MiB segments per stream after that prefix. Full segments publish while running; a final partial segment publishes at normal output completion, replacing the oldest slot. Up to one pending MiB remains in capture memory; use the independently sampled 64-KiB tail for the newest output. Abrupt runner loss can lose that pending segment and leave the tail stale. Missing middle output is not recoverable. Old runs and remote observer copies retain their existing prefix/tail limits.

```bash
ajr logs <job-id> --stream stderr --cursor 0 --limit 8192
ajr logs <job-id> --stream stderr --cursor <retained-offset> --limit 8192
ajr logs <job-id> --stream stderr --export <new-file>
```

Log responses include absolute `cursor`/`next_cursor`, `retained_ranges`, `gaps`, `retained_end` and an optional `skipped` range. A cursor inside an evicted interval advances to the next retained range with an explicit skip; reads never splice across a gap or renumber old bytes. `--follow` prints gap markers. In the panel, expand “查阅与导出保留日志”, load the ranges, select one and page through it; the export button reads a bounded snapshot without waiting for new output. Switching task/stream fences pending responses.

Exports use the existing UTF-8-lossy text representation and insert `[AgentJobRelay missing bytes A..B]` markers. They are diagnostic text, not byte-exact binary exports; byte positions refer to original output. CLI export creates a new file and refuses an existing destination; an interrupted export can leave a partial new file. `--export` is exclusive with `--follow`, `--cursor` and `--limit`. Native segment bytes/checksums survive verified compression, backup and new-profile restore. No schema migration, new MCP tool or remote execution replay is introduced. Windows behavior and actual host/browser adoption need separate qualification.

## Install persistent rules

```bash
agent-job-relay codex start --no-output 10m --action alert -- <program> <arguments...>
agent-job-relay codex monitor add <job-id> --contains 'CUDA out of memory' --stream stderr --action stop --grace 30s
agent-job-relay codex monitor add <job-id> --elapsed 2h --action alert
agent-job-relay codex monitor list <job-id>
agent-job-relay codex monitor update <job-id> <rule-id> --revision 1 --no-output 20m --action alert
agent-job-relay codex monitor disable <job-id> <rule-id> --revision 2
agent-job-relay codex monitor enable <job-id> <rule-id> --revision 3
agent-job-relay codex monitor remove <job-id> <rule-id> --revision 4
```

Expected: rules persist and their applied revision advances after the runner loads them. Commands accept seconds or `s`, `m`, `h`, `d` suffixes. Start accepts one initial CLI rule; MCP `spec.monitor_rules` accepts up to 16. All three interfaces share the same validation and revision fences. Rules cannot be changed on terminal jobs. A revision conflict requires inspecting again; it must not overwrite a concurrent edit.

Conditions are combined-stream silence, total elapsed runtime, or case-sensitive literal byte matches on stdout/stderr/both. Text matches span read boundaries but not separate streams; new/changed rules inspect future output only. A silence rule starts a fresh window when applied. Elapsed rules compare the whole run's elapsed time. Default action is a durable local alert. Stop acts on the owned workload tree with the rule's grace period. No model, live chat or viewer is required; daemon loss does not stop a runner's detection or local cancellation.

Each literal/time rule revision triggers once; enable it again to rearm. Progress and remote-alert subscriptions (0.1.20+) use their documented cooldown/budget semantics. At most 16 rule identities, including removed rules, and 128 revisions per rule are retained per job. Removal disables rather than erases audit history. Trigger records contain the original rule version and bounded evidence. Prefix truncation, long lines, invalid UTF-8 and split chunks do not disable literal matching. This release provides no semantic model judgement or arbitrary callback execution.

`--notify-agent` additionally enqueues an alert for the job's explicit bound session. It does not create or guess a binding; unsupported hosts remain inbox-only. Terminal results supersede unresolved alert deliveries and fence expired claims while retaining live per-chat exclusion, IDs, expiry and attempt history. Accepted or in-flight host effects cannot be recalled. Host acceptance remains separate from observed Agent execution; user-confirmed running feedback and its scope are recorded in [live verification](live-e2e.md#real-agent-mid-run-feedback).

## Mid-run feedback (0.1.12+)

Use existing flushed marker rules for a few known milestones, or `AJR_PROGRESS` frames (runtime 0.1.12+) and repeated stage/step/metric conditions. CLI `--progress`, MCP and Web share the same durable rules and snapshots; terminal events retain bounded milestone summaries. The [progress guide](progress.md) includes the versioned protocol, limits, runtime capability check and pinned-run reference checker. Runtime 0.1.20+ native remote observers can forward their runner alerts through a local `remote_alert` subscription, using the same CLI/MCP/Web rule editor and original caller binding; see [direct remote feedback](remote-execution.md#send-intermediate-feedback-to-the-origin-0120). Ordinary literal/time rules keep the once-per-revision policy. A cooldown limits event-triggered feedback; it does not schedule periodic summaries. Source 0.1.17 goal-wait notifying rules use [temporary restoration/report/native re-pause](goal-waiting.md#5-report-intermediate-feedback-and-pause-again), with a separate final notification.

## Stop and observe the result

```bash
agent-job-relay codex stop <job-id> --grace 30s --wait 10s
agent-job-relay codex stop <job-id> --grace 0 --wait 5s
```

Expected: `stop_outcome=requested` means the request was persisted; `terminal_observed` means a terminal state was read before the bounded wait ended. Inspect the actual status: a previously completed job stays completed. Defaults are five seconds of grace and no client wait; grace accepts 0–300 seconds and wait accepts 0–30. Repeated requests cannot extend an existing grace period. Reasons and graceful/forced outcomes are retained in task evidence. Windows Ctrl-Break is best effort; escalation remains Job Object scoped. Linux cancellation uses process groups; current source additionally cleans a live runner's adopted session-escaped descendants through parent-verified pidfds. Runner-death and adversarial-containment limits remain.

`daemon stop` stops dispatch and Web access; independent runners continue their rules, logs and final persistence. Restarting the same profile resumes notification dispatch and starts a new Web listener. Database write failures can prevent durable evidence/control, as described in [limitations](limitations.md); UI availability never substitutes for runner health.

Source builds generate and embed the frontend before Cargo checks. Use [source installation](source-install.md) for build/export commands and [testing](testing.md) for developer gates; recipients use the bundled assets. See [data maintenance](data-maintenance.md) before a schema upgrade.
