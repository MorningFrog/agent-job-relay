#!/usr/bin/env python3
"""Coordinate manual release workflows without changing their OIDC identity."""
import argparse
import os
import sys
import time
from urllib.parse import urlencode

sys.dont_write_bytecode = True
from release_actions import REPOSITORY, actions_context, candidate_run, github, output, require, summary

WORKFLOWS = {"prepare": ("release.yml", "Prepare Release"),
             "preview": ("release-publish.yml", "Publish Release"),
             "publish": ("release-publish.yml", "Publish Release"),
             "check": ("release-check.yml", "Check Release")}


def find_child(filename, title, revision):
    matches = []
    for page in range(1, 21):
        query = urlencode({"event": "workflow_dispatch", "branch": "main", "head_sha": revision, "per_page": 100, "page": page})
        runs = github(f"actions/workflows/{filename}/runs?{query}")["workflow_runs"]
        matches.extend(run for run in runs if run["display_title"] == title)
        require(len(matches) <= 1, "Multiple child runs claim the same pipeline step; inspect without dispatching again")
        if len(runs) < 100:
            return matches[0] if matches else None
    raise ValueError("Child-run search exceeded its bound; no workflow was dispatched")


def validate_child(run, filename, title, revision):
    require(run["event"] == "workflow_dispatch" and run["path"] == ".github/workflows/" + filename and run["head_branch"] == "main" and run["head_sha"] == revision and run["display_title"] == title and run["repository"]["full_name"] == REPOSITORY and run["head_repository"]["full_name"] == REPOSITORY, "Child workflow identity/source differs from this manual pipeline")


def dispatch_and_wait(step, inputs, revision):
    filename, name = WORKFLOWS[step]
    parent = os.environ.get("GITHUB_RUN_ID", "")
    require(parent.isdigit(), "Missing manual pipeline Run ID")
    key = f"release-{parent}-{step}"
    title = f"{name} [{key}]"
    require(github("git/ref/heads/main")["object"]["sha"] == revision, "main changed during the pipeline; start a new manual run with the original candidate and recovery enabled")
    run = find_child(filename, title, revision)
    minimum_attempt = 1
    if run is None:
        try:
            dispatched = github(f"actions/workflows/{filename}/dispatches", method="POST", payload={"ref": "main", "inputs": {**inputs, "pipeline_key": key, "expected_revision": revision}})
            require(isinstance(dispatched, dict) and isinstance(dispatched.get("workflow_run_id"), int), "Dispatch did not return an exact Run ID; inspect the pipeline key before retrying")
            run_id = dispatched["workflow_run_id"]
        except (RuntimeError, ValueError):
            # A timed-out response may have created the run. Never POST twice.
            for _ in range(6):
                run = find_child(filename, title, revision)
                if run:
                    break
                time.sleep(10)
            require(run is not None, f"Dispatch outcome is unknown for {key}; inspect Actions before retrying. No second dispatch was sent")
            run_id = run["id"]
    else:
        validate_child(run, filename, title, revision)
        run_id = run["id"]
        if run["status"] == "completed" and run["conclusion"] != "success":
            minimum_attempt = run["run_attempt"] + 1
            route = "rerun-failed-jobs" if run["conclusion"] == "failure" else "rerun"
            github(f"actions/runs/{run_id}/{route}", method="POST")
    summary(f"**{name}:** https://github.com/{REPOSITORY}/actions/runs/{run_id} (pipeline key `{key}`). Existing matching runs are reused; failed steps resume their original Run ID.")
    output("child_run_id", str(run_id))
    deadline = time.monotonic() + 5400
    while time.monotonic() < deadline:
        run = github(f"actions/runs/{run_id}")
        validate_child(run, filename, title, revision)
        if run["run_attempt"] >= minimum_attempt and run["status"] == "completed":
            require(run["conclusion"] == "success", f"{name} did not pass: https://github.com/{REPOSITORY}/actions/runs/{run_id}. Preserve candidate {inputs.get('candidate_run_id', run_id)}; retry only the failed step")
            return str(run_id)
        time.sleep(10)
    raise ValueError(f"Waiting for {name} exceeded 90 minutes; child run {run_id} was not cancelled or dispatched again")


def run_step(step, selected, channel, recovery):
    revision = actions_context()
    require(os.environ.get("GITHUB_WORKFLOW_REF") == f"{REPOSITORY}/.github/workflows/release-all.yml@refs/heads/main", "Pipeline coordination requires the manual Release All workflow")
    if step == "prepare" and selected:
        candidate_run(selected)
        output("candidate_run_id", selected)
        summary(f"Reusing original candidate **{selected}**; no preparation or rebuild was dispatched.")
        return selected
    if step == "prepare":
        selected = dispatch_and_wait(step, {"channel": channel}, revision)
        output("candidate_run_id", selected)
        return selected
    require(selected.isdigit(), "Select an original successful candidate Run ID")
    candidate_run(selected)
    inputs = {"candidate_run_id": selected, "recover_candidate": str(recovery).lower()}
    if step in ("preview", "publish"):
        inputs["dry_run"] = str(step == "preview").lower()
    return dispatch_and_wait(step, inputs, revision)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("step", choices=WORKFLOWS)
    args = parser.parse_args()
    try:
        run_step(args.step, os.environ.get("CANDIDATE_RUN_ID", ""), os.environ.get("RELEASE_CHANNEL", "latest"), os.environ.get("RECOVER_CANDIDATE") == "true")
    except (ValueError, RuntimeError, KeyError) as error:
        parser.exit(1, f"Release pipeline stopped: {error}\n")


if __name__ == "__main__":
    main()
