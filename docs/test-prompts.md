# Example task prompts

[简体中文](test-prompts.zh-CN.md) · [Documentation index](README.md)

Copy a prompt to your Agent and replace directories, filenames or links as needed.

## Scan a Linux home directory

```text
Scan /home/$USER and summarize the file count, file types and total size. Use AgentJobRelay to manage the background task and tell me the results when it finishes.
```

## Scan a Windows user directory

```text
Scan %USERPROFILE% and summarize the file count, file types and total size. Use AgentJobRelay to manage the background task and tell me the results when it finishes.
```

## Find a file

```text
Search my home directory for a file named report.pdf. Use AgentJobRelay to manage the background search and tell me its full path when found.
```

## Find large files

```text
Find the 20 largest files in my home directory and list their paths and sizes. Use AgentJobRelay to manage the background task.
```

## Report progress during a scan

```text
Scan my home directory and count the files and total size. Use AgentJobRelay to manage the background task, update me every 30 seconds and summarize the results when it finishes.
```

## Verify real-Agent mid-run feedback

Use a new ordinary chat without an active goal. Goal-wait members reject intermediate Agent notifications. Run this for a relevant feedback change or a new environment; user-confirmed acceptance is in [live verification](live-e2e.md#real-agent-mid-run-feedback).

```text
Use AgentJobRelay to create and run a controlled background test under this project's .tmp/ directory. It should take about three minutes, with three stages of 60 seconds each, recording its start, stage completions and exit times and exposing detectable stage progress.

Automatically tell me when stage one finishes and again when stage two finishes, while the original task is still running. After stage three and actual process exit, give a final summary. Configure the progress source and notification rules yourself.

All delays belong in the background task. End your turn after submission and after each intermediate report; do not repeatedly query status, loop on waits or create an auxiliary Agent timer. No additional message from me should be needed. On each notification, verify the original task's state before reporting.

In the final summary, say whether both intermediate reports occurred before task exit, whether the same Job/Run was retained throughout, and what its final exit result was. Mark anything you cannot verify. Reports reconstructed after exit, panel updates and host acknowledgement alone do not count as observed Agent feedback. Do not replay the workload, modify project source/user configuration or install software. If the current host or mode cannot support this, explain the limitation.
```

Expected: two automatic same-chat reports before the original task exits, yielding between reports, followed by a final report after exit, without human continuation or Agent polling. See [manual acceptance](manual-testing.md#optional-real-agent-mid-run-feedback) for evidence boundaries.

## Scan multiple directories together

```text
Calculate the size of each subdirectory in my home directory concurrently. Use AgentJobRelay to manage these background tasks and summarize the results in size order.
```

## Build and test a project

```text
Run the current project's full build and test suite. Use AgentJobRelay to manage the background task and report the results and any failure reasons when it finishes.
```

## Pause a goal while a scan runs

```text
Use AgentJobRelay to scan my home directory in the background three times, with one separate call per round. Wait 15 seconds after each scan finishes.
```

Expected in an active goal: ask once if pause permission is missing; a natural affirmative answer covers all three rounds. Each round pauses through the native host tool, includes its delay in background work and restores before the next submission.


## Observe a workflow

```text
Use AgentJobRelay to observe this GitHub Actions run in the background: <run URL>. Tell me the result when it finishes.
```

## View task progress

```text
Open the AgentJobRelay task manager and show the progress of the directory scan we just started.
```

## Stop a task

```text
Use AgentJobRelay to stop the directory scan we just started and tell me how far it got.
```

## Recover task notifications

```text
Recover notifications for the background task we just started in AgentJobRelay and tell me its current result without running it again.
```
