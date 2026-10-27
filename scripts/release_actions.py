#!/usr/bin/env python3
"""Manual Actions release orchestration; never infer publication from a dry-run."""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import HTTPRedirectHandler, Request, build_opener, urlopen

sys.dont_write_bytecode = True
from release_metadata import (REPO, PACKAGES, check_candidate, require, seal_candidate,
                              sha256, source_metadata, validate_promotion)

REGISTRY = "https://registry.npmjs.org"
REPOSITORY = "MorningFrog/agent-job-relay"
# These files do not contribute bytes to the runtime, npm packages or marketplace.
# Recovery is opt-in and still verifies the original candidate and every asset.
RECOVERY_PATHS = frozenset({
    ".github/workflows/release.yml", ".github/workflows/release-publish.yml",
    ".github/workflows/release-check.yml", ".github/workflows/release-all.yml",
    "scripts/release_pipeline.py",
    "tests/integration.rs",  # Fixture isolation only; not embedded in payloads.
    "scripts/release_actions.py", "scripts/test_release.py", "CONTRIBUTING.md",
    "README.md", "README.zh-CN.md", "docs/release.md", "docs/testing.md",
    "docs/architecture.md", "docs/IMPLEMENTATION_STATUS.md", "docs/TODO.md",
    "docs/adr/0013-unified-manual-release.md",
})


class NoCredentialRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise RuntimeError("Authenticated release API redirects are refused")


def request(url, token=None, method="GET", payload=None, binary=False, missing=False, context="Release API request"):
    headers = {"Accept": "application/octet-stream" if binary else "application/json", "User-Agent": "AgentJobRelay-release"}
    if url.startswith("https://api.github.com/"):
        headers["X-GitHub-Api-Version"] = "2026-03-10"
    if token:
        headers["Authorization"] = "Bearer " + token
    data = None
    if payload is not None:
        data = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
        headers["Content-Type"] = "application/octet-stream" if isinstance(payload, bytes) else "application/json"
    try:
        opener = build_opener(NoCredentialRedirect).open if token else urlopen
        with opener(Request(url, data=data, headers=headers, method=method), timeout=30) as response:
            # Do not persist/log credential responses or unbounded registry output.
            content = response.read(128 * 1024 * 1024 + 1 if binary else 8 * 1024 * 1024 + 1)
            require(len(content) <= (128 if binary else 8) * 1024 * 1024, "Remote response exceeded release limit")
            return content if binary else (json.loads(content) if content else None)
    except HTTPError as error:
        error.close()
        if missing and error.code == 404:
            return None
        raise RuntimeError(f"{context} failed with HTTP {error.code} ({method}); no credential response was logged") from None
    except (URLError, TimeoutError):
        raise RuntimeError(f"{context} transport failed; no credential response was logged") from None


def github(path, **kwargs):
    token = os.environ.get("GH_TOKEN")
    require(token, "Missing automatic GitHub token")
    suffix = "/" + path if path else ""
    return request(f"https://api.github.com/repos/{REPOSITORY}{suffix}", token=token, **kwargs)


class CommandError(RuntimeError):
    def __init__(self, argv, returncode, npm_code=None):
        self.npm_code = npm_code
        code = f", {npm_code}" if npm_code else ""
        super().__init__(f"{Path(str(argv[0])).name} {argv[1]} failed (exit {returncode}{code}); remote output withheld")


def command(argv, env=None, cwd=REPO, binary=False):
    result = subprocess.run(list(map(str, argv)), cwd=cwd, env=env, capture_output=True, text=not binary, timeout=180)
    if result.returncode:
        # npm/HTTP errors may contain tokens, signed URLs or private stage metadata.
        error_text = result.stderr.decode(errors="replace") if binary else result.stderr
        codes = re.findall(r"npm error code ([A-Z][A-Z0-9_]+)", error_text)
        raise CommandError(argv, result.returncode, codes[0] if codes else None)
    return result.stdout


def summary(message):
    print(message, flush=True)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as stream:
            stream.write(message + "\n\n")


def output(key, value):
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as stream:
            stream.write(f"{key}={value}\n")


def actions_context():
    require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch", "Release network operations require a manual GitHub Actions run")
    require(os.environ.get("GITHUB_REPOSITORY") == REPOSITORY and os.environ.get("GITHUB_REF") == "refs/heads/main", "Release runs must select this repository's main branch")
    revision = os.environ.get("GITHUB_SHA", "")
    require(re.fullmatch(r"[0-9a-f]{40}", revision), "Invalid source revision")
    expected = os.environ.get("EXPECTED_RELEASE_REVISION")
    require(not expected or expected == revision, "main changed after the release pipeline started; no release operation was authorized for this revision")
    return revision


@contextmanager
def npm_environment(auth):
    (REPO / ".tmp").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="release-auth-", dir=REPO / ".tmp") as temp:
        root = Path(temp)
        config = root / "npmrc"
        env = os.environ.copy()
        for key in list(env):
            if key.lower().startswith("npm_config_") or key in ("NODE_AUTH_TOKEN", "NPM_TOKEN", "NPM_STAGE_TOKEN", "NPM_ID_TOKEN"):
                env.pop(key)
        content = f"registry={REGISTRY}\nignore-scripts=true\n"
        if auth == "stage":
            token = os.environ.get("NPM_STAGE_TOKEN")
            require(token, "NPM_STAGE_TOKEN is missing from the release environment")
            env["NODE_AUTH_TOKEN"] = token
            content += "//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n"
        if auth != "oidc":
            env.pop("ACTIONS_ID_TOKEN_REQUEST_TOKEN", None)
            env.pop("ACTIONS_ID_TOKEN_REQUEST_URL", None)
        config.write_text(content)
        config.chmod(0o600)
        env.update(npm_config_userconfig=str(config), npm_config_globalconfig=str(root / "global-npmrc"), npm_config_cache=str(root / "cache"))
        yield env


def registry_version(entry):
    return request(f"{REGISTRY}/{quote(entry['name'], safe='')}/{quote(entry['version'], safe='')}", missing=True)


def public_package(entry):
    published = registry_version(entry)
    if published is not None:
        require(published.get("name") == entry["name"] and published.get("version") == entry["version"] and published.get("dist", {}).get("integrity") == entry["integrity"], f"Published bytes conflict with candidate: {entry['name']}@{entry['version']}")
    return published


def wait_for_registry(read, ready, description):
    deadline = time.monotonic() + 120
    announced = False
    while True:
        value = read()
        if ready(value):
            return value
        remaining = deadline - time.monotonic()
        require(remaining > 0, f"{description} is not yet publicly visible after waiting for npm propagation; retry the same original candidate without rebuilding")
        if not announced:
            summary(f"Waiting up to two minutes for npm registry propagation: {description}. No upload is retried.")
            announced = True
        time.sleep(min(5, remaining))


def check_channel(name, channel, selected):
    tags = request(f"{REGISTRY}/-/package/{quote(name, safe='')}/dist-tags", missing=True) or {}
    current = tags.get(channel)
    if current:
        match = re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", current)
        require(match and tuple(map(int, match.groups())) <= tuple(map(int, selected.split("."))), "Refusing to move a newer/unknown npm channel backwards")
    return tags


def prepare():
    revision = actions_context()
    source = source_metadata()
    repo = github("")
    require(repo["full_name"] == REPOSITORY, "Unexpected GitHub repository")
    check_tag(source["tag"], revision)
    require(not any(release["tag_name"] == source["tag"] for release in github_list("releases")), "This version already has a GitHub release/draft; reuse its original candidate_run_id instead of rebuilding")
    require(all(registry_version({"name": name, "version": source["version"]}) is None for name in PACKAGES.values()), "This version is already partially/publicly published; reuse the original candidate_run_id instead of rebuilding")
    output("version", source["version"])
    summary(f"Preparing **{source['version']}** from Cargo.toml with matching pins and CHANGELOG. Build jobs receive no publishing credentials. After reviewing the candidate, use **Publish Release** with dry-run enabled to check the actual publishing identity.")


def preflight(auth):
    revision = actions_context()
    if os.environ.get("CANDIDATE_RUN_ID"):
        revision = candidate_run(os.environ["CANDIDATE_RUN_ID"])["head_sha"]
    source = source_metadata()
    repo = github("")
    require(repo["full_name"] == REPOSITORY, "Unexpected GitHub repository")
    check_tag(source["tag"], revision)
    if auth == "stage":
        with npm_environment(auth) as env:
            identity = command(["npm", "whoami", "--json", f"--registry={REGISTRY}"], env=env)
            username = json.loads(identity)
            require(isinstance(username, str) and re.fullmatch(r"[a-zA-Z0-9_-]+", username), "Unexpected npm identity response")
            expected = os.environ.get("NPM_EXPECTED_USER")
            require(not expected or expected == username, "npm token belongs to a different expected user")
        summary(f"npm stage token authenticates as `{username}`. Staging permission and new-name eligibility still require an actual stage operation; dry-run does not upload.")
    else:
        require(os.environ.get("GITHUB_WORKFLOW_REF") == f"{REPOSITORY}/.github/workflows/release-publish.yml@refs/heads/main", "OIDC checks must run from Publish Release on main; configure npm's Trusted Publisher for release-publish.yml")
        url = os.environ.get("ACTIONS_ID_TOKEN_REQUEST_URL")
        token = os.environ.get("ACTIONS_ID_TOKEN_REQUEST_TOKEN")
        require(url and token and url.startswith("https://"), "Missing GitHub OIDC context / id-token: write permission")
        oidc = request(url + ("&" if "?" in url else "?") + urlencode({"audience": "npm:registry.npmjs.org"}), token=token)["value"]
        for name in PACKAGES.values():
            exchange = request(f"{REGISTRY}/-/npm/v1/oidc/token/exchange/package/{quote(name, safe='')}", token=oidc, method="POST")
            require(exchange.get("token_type") == "oidc" and exchange.get("token") and exchange.get("expires"), f"Invalid OIDC exchange for {name}")
        summary("npm OIDC token exchange succeeded for both packages. Actual upload, direct-publish policy and provenance acceptance are not exercised by dry-run.")
    summary(f"Source version: **{source['version']}**, tag `{source['tag']}`. GitHub repository access verified; write permission/rules are not proven without a write. No registry version, tag or Release was created.")
    output("version", source["version"])


def preview(directory, auth, channel, run_id=""):
    accepted = set()
    if run_id:
        candidate = validate(directory, run_id)
        channel = candidate["channel"]
        if auth == "oidc":
            release = find_release(directory, candidate)
            accepted = release.get("_accepted_npm_packages", set()) if release else set()
    meta = check_candidate(directory)
    with npm_environment(auth) as env:
        for key in PACKAGES:
            entry = meta[key]
            published = public_package(entry)
            check_channel(entry["name"], channel, entry["version"])
            if published is not None or key in accepted:
                summary(f"`{entry['name']}@{entry['version']}` has matching public integrity or a verified acceptance receipt; preview upload skipped.")
                continue
            argv = ["npm", "stage", "publish"] if auth == "stage" else ["npm", "publish"]
            command([*argv, directory / entry["tarball"], "--dry-run", "--ignore-scripts", "--access=public", f"--tag={channel}", f"--registry={REGISTRY}"], env=env)
    summary(f"**Publication dry-run passed:** version `{meta['version']}`, sealed channel `{channel}`. No npm stage/version/dist-tag or GitHub Release was changed. Actual write/provenance acceptance is checked during publication.")


def check_tag(tag, revision):
    ref = github(f"git/ref/tags/{quote(tag, safe='')}", missing=True)
    if ref:
        obj = ref["object"]
        for _ in range(5):
            if obj["type"] != "tag":
                break
            obj = github(f"git/tags/{obj['sha']}")["object"]
        require(obj["type"] == "commit" and obj["sha"] == revision, "Existing tag targets another commit; it will not be moved")
    return ref is not None


def check_recovery_source(original, current):
    require(all(re.fullmatch(r"[0-9a-f]{40}", value) for value in (original, current)), "Invalid recovery source revision")
    require(command(["git", "rev-parse", "HEAD"]).strip() == current, "Recovery checkout differs from the manual workflow revision")
    command(["git", "merge-base", "--is-ancestor", original, current])
    changed = command(["git", "diff", "--name-only", "-z", original, current, "--"], binary=True).decode().split("\0")
    changed = {name for name in changed if name}
    require(changed and changed <= RECOVERY_PATHS, "Recovery permits only allowlisted release-tooling/documentation changes; runtime, package, build, version and CHANGELOG changes require a new candidate")
    summary(f"**Candidate recovery:** original source `{original}`, workflow tools `{current}`. Only allowlisted release-tooling/documentation files changed; the original candidate bytes and source identity are retained.")


def candidate_run(run_id):
    require(str(run_id).isdigit(), "candidate_run_id must be the numeric successful dry-run ID")
    run = github(f"actions/runs/{run_id}")
    # Validate before asking download-artifact to read any files.
    require(run["status"] == "completed" and run["conclusion"] == "success" and run["event"] == "workflow_dispatch" and run["path"] == ".github/workflows/release.yml", "Candidate must be a successful manual Release workflow run")
    require(run["head_branch"] == "main" and run["repository"]["full_name"] == REPOSITORY and run["head_repository"]["full_name"] == REPOSITORY, "Candidate must belong to this repository's main branch")
    if run["head_sha"] != os.environ["GITHUB_SHA"]:
        require(os.environ.get("RECOVER_CANDIDATE") == "true", "Candidate source differs; generate a fresh dry-run or explicitly select recovery after release-tooling-only fixes")
        check_recovery_source(run["head_sha"], os.environ["GITHUB_SHA"])
    artifacts = github(f"actions/runs/{run_id}/artifacts?per_page=100")["artifacts"]
    matches = [a for a in artifacts if a["name"] == "release-candidate" and not a["expired"]]
    require(len(matches) == 1, "Missing/expired verified release-candidate artifact; never rebuild during promotion")
    return run


def validate(directory, run_id):
    actions_context()
    run = candidate_run(run_id)
    return validate_promotion(directory, run, REPOSITORY, run["head_sha"], run_id)


def github_list(path):
    items = []
    for page in range(1, 21):
        batch = github(f"{path}?per_page=100&page={page}")
        require(isinstance(batch, list), "Unexpected GitHub list response")
        items.extend(batch)
        if len(batch) < 100:
            return items
    raise ValueError("GitHub inventory exceeds the bounded release listing; nothing was changed")


RECEIPT_NAME = re.compile(r"npm-publish-receipt-([1-9]\d*)-([1-9]\d*)-(runtime|opencode)\.json")


def receipt_fields(candidate, key):
    entry = PACKAGES[key]
    return {"format": 1, "kind": "npm_publish_accepted", "repository": REPOSITORY,
            "candidate_run_id": candidate["run_id"], "revision": candidate["revision"],
            "version": candidate["version"], "channel": candidate["channel"],
            "package_name": entry, "tarball_sha256": candidate["assets"][f"{entry}-{candidate['version']}.tgz"]}


def read_receipt(asset, candidate, match):
    require(asset.get("size", 0) <= 16384, "Publication receipt is too large")
    data = command(["gh", "api", "-H", "Accept: application/octet-stream", f"repos/{REPOSITORY}/releases/assets/{asset['id']}"], binary=True)
    require(len(data) <= 16384 and (not asset.get("digest") or asset["digest"] == "sha256:" + hashlib.sha256(data).hexdigest()), "Publication receipt digest mismatch")
    receipt = json.loads(data)
    revision = receipt.get("publisher_revision") if isinstance(receipt, dict) else None
    require(isinstance(revision, str) and re.fullmatch(r"[0-9a-f]{40}", revision), "Invalid publication receipt")
    expected = {**receipt_fields(candidate, match[3]), "publisher_run_id": match[1], "publisher_run_attempt": match[2], "publisher_revision": receipt.get("publisher_revision")}
    require(receipt == expected, "Publication receipt does not match this sealed candidate")
    run = github(f"actions/runs/{match[1]}")
    require(str(run["id"]) == match[1] and run["event"] == "workflow_dispatch" and run["path"] == ".github/workflows/release-publish.yml" and run["head_branch"] == "main" and run["head_sha"] == receipt["publisher_revision"] and run["repository"]["full_name"] == REPOSITORY and run["head_repository"]["full_name"] == REPOSITORY and int(match[2]) <= run["run_attempt"], "Publication receipt does not belong to the publishing workflow")
    return match[3]


def verify_release_assets(directory, assets, candidate=None):
    local = {p.name: p for p in directory.iterdir()}
    require(len(assets) == len({a['name'] for a in assets}), "Existing Release has foreign/duplicate assets; nothing will be overwritten")
    accepted = set()
    for asset in assets:
        if asset["name"] not in local:
            match = RECEIPT_NAME.fullmatch(asset["name"])
            require(candidate is not None and match is not None, "Existing Release has foreign/duplicate assets; nothing will be overwritten")
            accepted.add(read_receipt(asset, candidate, match))
            continue
        expected = sha256(local[asset["name"]])
        if asset.get("digest"):
            require(asset["digest"] == "sha256:" + expected, "Existing Release asset differs from reviewed candidate")
        else:
            data = command(["gh", "api", "-H", "Accept: application/octet-stream", f"repos/{REPOSITORY}/releases/assets/{asset['id']}"], binary=True)
            require(hashlib.sha256(data).hexdigest() == expected, "Existing Release asset differs from reviewed candidate")
    return accepted


def upload_release_asset(release_id, path):
    token = os.environ.get("GH_TOKEN")
    require(token, "Missing automatic GitHub token")
    require(isinstance(release_id, int) and release_id > 0, "Invalid GitHub Release ID")
    asset = request(f"https://uploads.github.com/repos/{REPOSITORY}/releases/{release_id}/assets?name={quote(path.name, safe='')}", token=token, method="POST", payload=path.read_bytes(), context="GitHub release asset upload")
    require(asset.get("name") == path.name, "Uploaded asset identity mismatch")
    verify_release_assets(path.parent, [asset])


def find_release(directory, candidate):
    tag = "v" + candidate["version"]
    check_tag(tag, candidate["revision"])
    marker = f"<!-- AgentJobRelay candidate {candidate['run_id']} {candidate['revision']} {candidate['channel']} -->"
    body = (directory / "release-notes.md").read_text() + "\n" + marker
    # The by-tag endpoint omits drafts. Enumerate authenticated drafts as well,
    # and upload to an exact ID rather than gh's potentially ambiguous tag lookup.
    matches = [r for r in github_list("releases") if r["tag_name"] == tag]
    inventories = {}
    accepted = {}
    for release in matches:
        require(release["body"] == body and release["target_commitish"] == candidate["revision"] and release["prerelease"] == (candidate["channel"] == "next"), "Existing Release is foreign/modified; preserve it and stop")
        inventories[release["id"]] = github_list(f"releases/{release['id']}/assets")
        accepted[release["id"]] = verify_release_assets(directory, inventories[release["id"]], candidate)
    published = [r for r in matches if not r["draft"]]
    populated = [r for r in matches if inventories[r["id"]]]
    require(len(published) <= 1 and len(populated) <= 1, "Multiple populated/published Releases claim this candidate; preserve them and stop")
    if matches:
        release = (published or populated or sorted(matches, key=lambda r: r["id"]))[0]
        for duplicate in matches:
            if duplicate["id"] != release["id"]:
                require(duplicate["draft"] and not inventories[duplicate["id"]], "Conflicting duplicate Release; preserve it and stop")
                summary(f"Preserving matching empty duplicate draft `{duplicate['id']}`; using Release `{release['id']}` without deleting or replacing anything.")
        return {**release, "_assets": inventories[release["id"]], "_accepted_npm_packages": accepted[release["id"]]}
    return None


def draft_release(directory, candidate):
    release = find_release(directory, candidate)
    if release is None:
        marker = f"<!-- AgentJobRelay candidate {candidate['run_id']} {candidate['revision']} {candidate['channel']} -->"
        body = (directory / "release-notes.md").read_text() + "\n" + marker
        release = github("releases", method="POST", payload={"tag_name": "v" + candidate["version"], "target_commitish": candidate["revision"], "name": f"AgentJobRelay {candidate['version']}", "body": body, "draft": True, "prerelease": candidate["channel"] == "next"})
        require(release["body"] == body and release["target_commitish"] == candidate["revision"] and release["prerelease"] == (candidate["channel"] == "next"), "Existing Release is foreign/modified; preserve it and stop")
    assets = release.get("_assets", [])
    local = {p.name: p for p in directory.iterdir()}
    missing = sorted(set(local) - {a["name"] for a in assets})
    require(release["draft"] or not missing, "Published Release has missing assets; preserve it and stop")
    for name in missing:
        upload_release_asset(release["id"], local[name])
    return release


def record_publish_acceptance(candidate, release, key):
    run_id, attempt = os.environ.get("GITHUB_RUN_ID", ""), os.environ.get("GITHUB_RUN_ATTEMPT", "")
    require(run_id.isdigit() and attempt.isdigit(), "Missing publication run identity for acceptance receipt")
    receipt = {**receipt_fields(candidate, key), "publisher_run_id": run_id, "publisher_run_attempt": attempt, "publisher_revision": actions_context()}
    with tempfile.TemporaryDirectory(prefix="release-receipt-", dir=REPO / ".tmp") as temp:
        path = Path(temp) / f"npm-publish-receipt-{run_id}-{attempt}-{key}.json"
        path.write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
        upload_release_asset(release["id"], path)


def stage_package(entry, tarball, channel, env):
    stages = json.loads(command(["npm", "stage", "list", entry["name"], "--json", f"--registry={REGISTRY}"], env=env))
    require(isinstance(stages, list), "Unexpected npm stage inventory")
    matching = [s for s in stages if s.get("packageName") == entry["name"] and s.get("version") == entry["version"]]
    require(len(matching) <= 1, "Multiple stages claim this package/version")
    if matching:
        stage = matching[0]
        require(stage.get("tag") == channel, "Existing stage uses another channel; review it on npm before continuing")
    else:
        result = json.loads(command(["npm", "stage", "publish", tarball, "--ignore-scripts", "--access=public", f"--tag={channel}", "--json", f"--registry={REGISTRY}"], env=env))
        require(isinstance(result, dict), "Unexpected npm stage publish response")
        staged = result.get(entry["name"], result)
        stage = {"id": staged.get("stageId")}
    stage_id = stage.get("id", "")
    require(isinstance(stage_id, str) and re.fullmatch(r"[0-9a-fA-F-]{36}", stage_id), "npm returned an invalid stage ID")
    stage = json.loads(command(["npm", "stage", "view", stage_id, "--json", f"--registry={REGISTRY}"], env=env))
    require(stage.get("id") == stage_id and stage.get("packageName") == entry["name"] and stage.get("version") == entry["version"] and stage.get("tag") == channel, "Staged package identity/channel mismatch")
    require(stage.get("shasum") == hashlib.sha1(tarball.read_bytes()).hexdigest(), "Staged package checksum differs from candidate")
    status = stage.get("status")
    require(status in ("validating", "staged"), "Unknown npm stage status; inspect on npm before retrying")
    if status == "validating":
        summary(f"`{entry['name']}@{entry['version']}` uploaded; npm automated validation is pending. Stage ID: `{stage_id}`. Do not approve yet. Re-run this candidate after validation so staged bytes can be verified.")
        return False
    data = request(f"{REGISTRY}/-/stage/{stage_id}/tarball", token=env["NODE_AUTH_TOKEN"], binary=True, context="npm staged tarball download")
    require(hashlib.sha256(data).hexdigest() == sha256(tarball), "Staged tarball differs from reviewed candidate; do not approve it")
    summary(f"`{entry['name']}@{entry['version']}` staged and bytes verified. Stage ID: `{stage_id}`. Approve on npm with 2FA (runtime first), then run Release again with the **same original candidate_run_id** and `npm_auth=stage`.")
    return True


def publish(directory, run_id, auth):
    candidate = validate(directory, run_id)
    meta = check_candidate(directory)
    # Check every existing package before the first external mutation.
    existing = {key: public_package(meta[key]) for key in PACKAGES}
    channels = {key: check_channel(name, candidate["channel"], candidate["version"]) for key, name in PACKAGES.items()}
    release = draft_release(directory, candidate)
    accepted = release.get("_accepted_npm_packages", set())
    verified_stages = True
    with npm_environment(auth) as env:
        for key in PACKAGES:
            entry = meta[key]
            if existing[key] is not None:
                summary(f"`{entry['name']}@{entry['version']}` already has the reviewed public bytes; upload skipped.")
                if auth == "oidc" and channels[key].get(candidate["channel"]) != candidate["version"]:
                    command(["npm", "dist-tag", "add", f"{entry['name']}@{candidate['version']}", candidate["channel"], f"--registry={REGISTRY}"], env=env)
                continue
            if auth == "oidc" and key in accepted:
                summary(f"`{entry['name']}@{entry['version']}` has a verified prior acceptance receipt; upload skipped even while public metadata is propagating.")
                continue
            if auth == "stage":
                verified_stages = stage_package(entry, directory / entry["tarball"], candidate["channel"], env) and verified_stages
            else:
                command(["npm", "publish", directory / entry["tarball"], "--ignore-scripts", "--access=public", f"--tag={candidate['channel']}", "--json", f"--registry={REGISTRY}"], env=env)
                record_publish_acceptance(candidate, release, key)
                summary(f"`{entry['name']}@{entry['version']}` upload accepted and receipt retained. Public synchronization is checked separately.")
    if auth == "oidc":
        if release["draft"]:
            github(f"releases/{release['id']}", method="PATCH", payload={"draft": False, "make_latest": "true" if candidate["channel"] == "latest" else "false"})
        output("submitted", "true")
        summary(f"**Publication submitted:** https://github.com/{REPOSITORY}/releases/tag/v{candidate['version']} — channel `{candidate['channel']}`, original candidate `{run_id}`. npm accepted the uploads and GitHub publication completed; public registry synchronization/installability has not been asserted. Run **Check Release** separately.")
        return release
    ready = all(public_package(meta[key]) is not None for key in PACKAGES)
    output("ready", str(ready).lower())
    if not ready and not verified_stages:
        summary("**Waiting for npm automated validation.** Both package stages have been considered; GitHub remains a draft. After npm validation completes, run Release again with the same original candidate ID to verify staged bytes before approval. No automatic approval or polling runs.")
    elif not ready:
        summary(f"**Waiting for npm maintainer approval.** GitHub Release remains a draft: https://github.com/{REPOSITORY}/releases/tag/v{candidate['version']}. No automatic approval or polling runs.")
    else:
        summary("Both npm versions match the candidate. Public installation and GitHub completion run next.")
    return release


def verify_public(directory):
    meta = check_candidate(directory)
    for key in PACKAGES:
        entry = meta[key]
        published = public_package(entry)
        require(published is not None, "Both npm versions must be public before verification; rerun Check Release only")
        tarball_url = published["dist"]["tarball"]
        require(tarball_url.startswith(REGISTRY + "/"), "Unexpected registry tarball origin")
        data = request(tarball_url, binary=True)
        require(hashlib.sha256(data).hexdigest() == sha256(directory / entry["tarball"]), "Public tarball bytes differ from candidate")

    def install():
        # npm's install metadata can lag the version/dist-tag endpoints. Retry
        # only missing versions, with no cached metadata or partial installation.
        with npm_environment("public") as env, tempfile.TemporaryDirectory(prefix="release-public-", dir=REPO / ".tmp") as temp:
            root = Path(temp)
            # No publishing credentials or real application state reach the install.
            for key in ("GH_TOKEN", "GITHUB_TOKEN"):
                env.pop(key, None)
            env.update(HOME=temp, XDG_STATE_HOME=str(root / "state"), XDG_CONFIG_HOME=str(root / "config"), XDG_CACHE_HOME=str(root / "cache"), AJR_STATE_DIR=str(root / "state"), CODEX_HOME=str(root / "codex"))
            try:
                command(["npm", "install", "--prefix", root, "--prefer-online", "--ignore-scripts", "--no-audit", "--no-fund", f"--registry={REGISTRY}", *[f"{name}@{meta['version']}" for name in PACKAGES.values()]], env=env, cwd=root)
            except CommandError as error:
                if error.npm_code != "ETARGET":
                    raise
                return False
            modules = root / "node_modules"
            require(command(["node", modules / PACKAGES["runtime"] / "cli.mjs", "--version"], env=env, cwd=root).strip() == f"agent-job-relay {meta['version']}", "Public native runtime version mismatch")
            command(["node", "--input-type=module", "-e", "const p=await import(process.argv[1]);if(typeof p.AgentJobRelay!=='function')process.exit(1)", (modules / PACKAGES["opencode"] / "index.mjs").as_uri()], env=env, cwd=root)
            return True

    wait_for_registry(install, bool, "clean npm installation (ETARGET); rerun Check Release only")
    summary("Public npm tarballs match the candidate; clean registry installation, native --version and OpenCode module import passed. Candidate host-loader checks remain distinct from real-provider/ChatGPT UI acceptance.")


def finalize(directory, run_id, auth):
    candidate = validate(directory, run_id)
    verify_public(directory)
    with npm_environment(auth) as env:
        for name in PACKAGES.values():
            tags = check_channel(name, candidate["channel"], candidate["version"])
            if tags.get(candidate["channel"]) != candidate["version"]:
                command(["npm", "dist-tag", "add", f"{name}@{candidate['version']}", candidate["channel"], f"--registry={REGISTRY}"], env=env)
                wait_for_registry(lambda: check_channel(name, candidate["channel"], candidate["version"]), lambda value: value.get(candidate["channel"]) == candidate["version"], f"{name} channel {candidate['channel']} at {candidate['version']}")
    release = draft_release(directory, candidate)
    if release["draft"]:
        github(f"releases/{release['id']}", method="PATCH", payload={"draft": False, "make_latest": "true" if candidate["channel"] == "latest" else "false"})
    summary(f"**Release complete:** https://github.com/{REPOSITORY}/releases/tag/v{candidate['version']} — npm channel `{candidate['channel']}`, candidate run `{run_id}`. Existing matching uploads were retained.")


def check_publication(directory, run_id):
    candidate = validate(directory, run_id)
    meta = check_candidate(directory)

    def visible():
        packages = {key: public_package(meta[key]) for key in PACKAGES}
        channels = {key: check_channel(name, candidate["channel"], candidate["version"]) for key, name in PACKAGES.items()}
        return all(packages[key] is not None and channels[key].get(candidate["channel"]) == candidate["version"] for key in PACKAGES)

    wait_for_registry(visible, bool, "both public package versions and their sealed channel; rerun Check Release only")
    verify_public(directory)
    release = find_release(directory, candidate)
    require(release is not None and not release["draft"], "GitHub Release is not public; resume Publish Release with the same candidate")
    require({p.name for p in directory.iterdir()} <= {a["name"] for a in release["_assets"]}, "Public GitHub Release lacks candidate assets; preserve it and inspect")
    require(check_tag("v" + candidate["version"], candidate["revision"]), "Public GitHub Release tag is missing")
    summary(f"**Public verification passed:** npm bytes, channel, clean installation and GitHub Release match candidate `{run_id}`. This read-only check changed no package, tag, receipt or Release.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["source", "prepare", "preflight", "run", "seal", "validate", "preview", "publish", "finalize", "check"])
    parser.add_argument("--release", type=Path, default=REPO / ".tmp/release-candidate")
    parser.add_argument("--candidate-run-id", default="")
    parser.add_argument("--channel", choices=["latest", "next"], default="latest")
    parser.add_argument("--auth", choices=["stage", "oidc"], default="stage")
    args = parser.parse_args()
    try:
        if args.operation == "source":
            meta = source_metadata()
            print(json.dumps(meta))
            output("version", meta["version"])
        elif args.operation == "prepare":
            prepare()
        elif args.operation == "preflight":
            preflight(args.auth)
        elif args.operation == "run":
            actions_context()
            candidate_run(args.candidate_run_id)
        elif args.operation == "seal":
            revision = actions_context()
            candidate = seal_candidate(args.release, args.channel, REPOSITORY, revision, os.environ["GITHUB_RUN_ID"])
            summary(f"**Verified candidate:** version `{candidate['version']}`, channel `{candidate['channel']}`, original Run ID **{candidate['run_id']}**. Linux/Windows native and package/loader jobs passed. Prepare Release created no npm version, tag or GitHub Release; check OIDC credentials with a Publish Release dry-run before publishing.")
            summary((args.release / "release-notes.md").read_text())
        elif args.operation == "validate":
            candidate = validate(args.release, args.candidate_run_id)
            summary(f"Reviewed candidate verified: `{candidate['version']}`, channel `{candidate['channel']}`, source `{candidate['revision']}`. Promotion will not rebuild.")
        elif args.operation == "preview":
            preview(args.release, args.auth, args.channel, args.candidate_run_id)
        elif args.operation == "publish":
            publish(args.release.resolve(), args.candidate_run_id, args.auth)
        elif args.operation == "check":
            check_publication(args.release.resolve(), args.candidate_run_id)
        else:
            finalize(args.release.resolve(), args.candidate_run_id, args.auth)
    except (ValueError, RuntimeError, KeyError) as error:
        parser.exit(1, f"Release stopped: {error}\n")


if __name__ == "__main__":
    main()
