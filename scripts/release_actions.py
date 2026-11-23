#!/usr/bin/env python3
"""Manual Actions preparation, staged publication and independent verification."""
import argparse
from contextlib import contextmanager
from concurrent.futures import ThreadPoolExecutor
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
from urllib.parse import quote
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
    ".github/actions/release-candidate/action.yml", "scripts/release_pipeline.py",
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
            env["npm_config_fetch_retries"] = "0"
            content += "//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n"
        for key in ("ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL",
                    "GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN", "GH_CONFIG_DIR"):
            env.pop(key, None)
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
    require(not check_tag(source["tag"], revision), "This version already has a Git tag; reuse its original candidate")
    require(not any(release["tag_name"] == source["tag"] for release in github_list("releases")), "This version already has a GitHub release/draft; reuse its original candidate_run_id instead of rebuilding")
    require(all(registry_version({"name": name, "version": source["version"]}) is None for name in PACKAGES.values()), "This version is already partially/publicly published; reuse the original candidate_run_id instead of rebuilding")
    output("version", source["version"])
    summary(f"Preparing **{source['version']}** from Cargo.toml with matching pins and CHANGELOG. Build jobs receive no publishing credentials. Review the sealed candidate before selecting any publication operation.")


def preview(directory, run_id):
    candidate = validate(directory, run_id)
    meta = check_candidate(directory)
    find_release(directory, candidate)
    existing = check_registry_selection(meta, candidate["channel"])
    selected = ", ".join(meta[key]["name"] for key, public in existing.items() if public is not None) or "none"
    summary(f"**Preview verified:** version `{meta['version']}`, sealed channel `{candidate['channel']}`, candidate run `{run_id}`. Matching public npm versions: {selected}. Stage and finalize are separately selected operations.")


def check_registry_selection(meta, channel):
    existing = {key: public_package(meta[key]) for key in PACKAGES}
    for entry in PACKAGES.values():
        check_channel(entry, channel, meta["version"])
    return existing


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
    require(re.fullmatch(r"[1-9][0-9]{0,19}", str(run_id)), "candidate_run_id must be the original numeric Prepare Release Run ID")
    run = github(f"actions/runs/{run_id}")
    # Validate before asking download-artifact to read any files.
    require(str(run.get("id")) == str(run_id), "Candidate API returned a different run")
    require(run["status"] == "completed" and run["conclusion"] == "success" and run["event"] == "workflow_dispatch" and run["path"] == ".github/workflows/release.yml", "Candidate must be a successful manual Release workflow run")
    require(run["head_branch"] == "main" and run["repository"]["full_name"] == REPOSITORY and run["head_repository"]["full_name"] == REPOSITORY, "Candidate must belong to this repository's main branch")
    require(re.fullmatch(r"[0-9a-f]{40}", run["head_sha"]), "Candidate has an invalid commit")
    if run["head_sha"] != os.environ["GITHUB_SHA"]:
        require(os.environ.get("RECOVER_CANDIDATE") == "true", "Candidate source differs; generate a fresh dry-run or explicitly select recovery after release-tooling-only fixes")
        check_recovery_source(run["head_sha"], os.environ["GITHUB_SHA"])
    inventory = github(f"actions/runs/{run_id}/artifacts?per_page=100")
    artifacts = inventory["artifacts"]
    require(len(artifacts) < 100 and inventory.get("total_count", len(artifacts)) == len(artifacts),
            "Candidate artifact inventory exceeds the complete listing")
    matches = [a for a in artifacts if a["name"] in ("release-candidate-bundle", "release-candidate") and not a["expired"]]
    require(len(matches) == 1, "Missing/expired verified release-candidate artifact; never rebuild during promotion")
    return {**run, "candidate_artifact": matches[0]}


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
    require(isinstance(stage_id, str) and re.fullmatch(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", stage_id), "npm returned an invalid stage ID")
    stage = json.loads(command(["npm", "stage", "view", stage_id, "--json", f"--registry={REGISTRY}"], env=env))
    require(stage.get("id") == stage_id and stage.get("packageName") == entry["name"] and stage.get("version") == entry["version"] and stage.get("tag") == channel, "Staged package identity/channel mismatch")
    require(stage.get("shasum") == hashlib.sha1(tarball.read_bytes()).hexdigest(), "Staged package checksum differs from candidate")
    status = stage.get("status")
    require(status in ("validating", "staged"), "Unknown npm stage status; inspect on npm before retrying")
    if status == "validating":
        summary(f"`{entry['name']}@{entry['version']}` uploaded; npm automated validation is pending. Stage ID: `{stage_id}`. Do not approve yet. Rerun Stage with the same candidate after validation to verify the staged bytes.")
        return False
    # Same download endpoint as npm stage download, with a byte bound and no tar parsing.
    data = request(f"{REGISTRY}/-/stage/{stage_id}/tarball", token=env["NODE_AUTH_TOKEN"],
                   binary=True, context="npm staged tarball download")
    require(len(data) == tarball.stat().st_size and hashlib.sha256(data).hexdigest() == sha256(tarball),
            "Staged tarball differs from reviewed candidate; do not approve it")
    summary(f"`{entry['name']}@{entry['version']}` staged and bytes verified. Stage ID: `{stage_id}`. Approve on npm with 2FA (runtime first), then select Finalize with the same original candidate Run ID.")
    return True


def stage_release(directory, run_id):
    candidate = validate(directory, run_id)
    meta = check_candidate(directory)
    existing = check_registry_selection(meta, candidate["channel"])
    release = find_release(directory, candidate)
    accepted = release.get("_accepted_npm_packages", set()) if release else set()
    pending = [key for key in PACKAGES if existing[key] is None and key not in accepted]
    if not pending:
        summary("Both npm packages are public or have legacy upload receipts. Select Finalize after both public bytes and channels are available.")
        return

    with npm_environment("stage") as env:
        verified = [stage_package(meta[key], directory / meta[key]["tarball"], candidate["channel"], env)
                    for key in pending]
    if not all(verified):
        summary("Waiting for npm automated validation. Rerun Stage for the same candidate to verify staged bytes before approval.")
    else:
        summary("Waiting for maintainer npm approval with 2FA. After approval, select Finalize for the same candidate.")


def verify_public_bytes(directory, meta, *, channel=None):
    """Verify both complete tarballs with bounded, independent read-only requests."""
    def verify(key):
        entry = meta[key]
        published = public_package(entry)
        require(published is not None, "Both npm versions must be public before verification; finish maintainer approval or retry read-only verification of the same original candidate")
        if channel is not None:
            require(check_channel(entry['name'], channel, entry['version']).get(channel) == entry['version'],
                    'Public channel has not selected the sealed version; do not move tags automatically')
        tarball_url = published["dist"]["tarball"]
        require(tarball_url.startswith(REGISTRY + "/"), "Unexpected registry tarball origin")
        data = request(tarball_url, binary=True)
        require(hashlib.sha256(data).hexdigest() == sha256(directory / entry["tarball"]), "Public tarball bytes differ from candidate")

    with ThreadPoolExecutor(max_workers=2) as pool:
        # Consume every result before a caller can publish or report success.
        list(pool.map(verify, PACKAGES))


def verify_public(directory, *, source_root=None, channel=None):
    meta = check_candidate(directory) if source_root is None else check_candidate(directory, source_root)
    verify_public_bytes(directory, meta, channel=channel)

    def install():
        # npm's install metadata can lag the version/dist-tag endpoints. Retry
        # only missing versions, with no cached metadata or partial installation.
        with npm_environment("public") as env, tempfile.TemporaryDirectory(prefix="release-public-", dir=REPO / ".tmp") as temp:
            root = Path(temp)
            # No publishing credentials or real application state reach the install.
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


def finalize(directory, run_id):
    candidate = validate(directory, run_id)
    verify_public_bytes(directory, check_candidate(directory), channel=candidate["channel"])
    release = draft_release(directory, candidate)
    if release["draft"]:
        github(f"releases/{release['id']}", method="PATCH", payload={"draft": False, "make_latest": "true" if candidate["channel"] == "latest" else "false"})
    require(check_tag("v" + candidate["version"], candidate["revision"]), "Finalized Release tag is missing")
    summary(f"**Release complete:** https://github.com/{REPOSITORY}/releases/tag/v{candidate['version']} — npm channel `{candidate['channel']}`, candidate run `{run_id}`. Existing matching uploads were retained.")


def check_publication(directory, run_id):
    candidate = validate(directory, run_id)
    meta = check_candidate(directory)

    def visible():
        packages = {key: public_package(meta[key]) for key in PACKAGES}
        channels = {key: check_channel(name, candidate["channel"], candidate["version"]) for key, name in PACKAGES.items()}
        return all(packages[key] is not None and channels[key].get(candidate["channel"]) == candidate["version"] for key in PACKAGES)

    wait_for_registry(visible, bool, "both public package versions and their sealed channel; rerun Check Release only")
    verify_public(directory, channel=candidate["channel"])
    release = find_release(directory, candidate)
    require(release is not None and not release["draft"], "GitHub Release is not public; select Finalize in Publish Release with the same candidate")
    require({p.name for p in directory.iterdir()} <= {a["name"] for a in release["_assets"]}, "Public GitHub Release lacks candidate assets; preserve it and inspect")
    require(check_tag("v" + candidate["version"], candidate["revision"]), "Public GitHub Release tag is missing")
    summary(f"**Public verification passed:** npm bytes, channel, clean installation and GitHub Release match candidate `{run_id}`. This read-only check changed no package, tag, receipt or Release.")


def verify_workflow(operation, run_id):
    allowed = {
        "prepare": ("release.yml",), "seal": ("release.yml",),
        "run": ("release-publish.yml", "release-check.yml"),
        "preview": ("release-publish.yml",), "stage": ("release-publish.yml",),
        "finalize": ("release-publish.yml",), "check": ("release-check.yml",),
    }
    revision = actions_context()
    require(os.environ.get("GITHUB_WORKFLOW_REF") in {
        f"{REPOSITORY}/.github/workflows/{file}@refs/heads/main" for file in allowed[operation]
    }, "Release operation is outside its selected workflow")
    require(command(["git", "rev-parse", "HEAD"]).strip() == revision, "Checkout differs from the manual release revision")
    if operation in ("stage", "finalize", "preview"):
        event_path = os.environ.get("GITHUB_EVENT_PATH")
        require(event_path, "Missing GitHub dispatch event")
        inputs = json.loads(Path(event_path).read_text()).get("inputs", {})
        require(inputs.get("operation", "preview") == operation, "Release operation differs from the manual dispatch selection")
        require(inputs.get("candidate_run_id") == run_id, "The candidate differs from the manual dispatch selection")
    return revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["source", "prepare", "run", "seal", "preview", "stage", "finalize", "check"])
    parser.add_argument("--release", type=Path, default=REPO / ".tmp/release-candidate")
    parser.add_argument("--candidate-run-id", default="")
    parser.add_argument("--channel", choices=["latest", "next"], default="latest")
    parser.add_argument("--allow-unreleased", action="store_true", help="Allow development notes for source validation only")
    args = parser.parse_args()
    try:
        require(not args.allow_unreleased or args.operation == "source", "--allow-unreleased is only supported for source validation")
        if args.operation == "source":
            meta = source_metadata(allow_unreleased=args.allow_unreleased)
            print(json.dumps(meta))
            return
        revision = verify_workflow(args.operation, args.candidate_run_id)
        if args.operation == "prepare":
            prepare()
        elif args.operation == "seal":
            prepare()  # Recheck unused-version evidence after the builds and installed-package checks.
            candidate = seal_candidate(args.release, args.channel, REPOSITORY, revision, os.environ["GITHUB_RUN_ID"])
            summary(f"**Verified candidate:** version `{candidate['version']}`, channel `{candidate['channel']}`, original Run ID **{candidate['run_id']}**. Review the notes and select Preview in Publish Release.")
            summary((args.release / "release-notes.md").read_text())
        elif args.operation == "run":
            run = candidate_run(args.candidate_run_id)
            artifact = run["candidate_artifact"]
            require(type(artifact.get("id")) is int and artifact["id"] > 0, "Invalid candidate artifact ID")
            output("artifact_id", artifact["id"])
            output("bundle", str(artifact["name"] == "release-candidate-bundle").lower())
            output("revision", run["head_sha"])
        else:
            operations = {"preview": preview, "stage": stage_release, "finalize": finalize, "check": check_publication}
            operations[args.operation](args.release.resolve(), args.candidate_run_id)
    except (ValueError, RuntimeError, KeyError, OSError) as error:
        parser.exit(1, f"Release stopped: {error}\n")


if __name__ == "__main__":
    main()
