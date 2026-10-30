"""Installed Linux updater fixture, called by check_release.py; no public writes."""
import hashlib
import json
import os
import time
from pathlib import Path


def check_online_update(root, prefix, env, meta, registry, command, job_id):
    package = prefix / "lib/node_modules/agent-job-relay-runtime"
    updater = package / "update.mjs"
    # Only the copied package's registry transport changes. The fixture serves
    # the original candidate tarball and integrity; production remains pinned.
    updater.write_text(updater.read_text().replace("https://registry.npmjs.org", registry))
    env["npm_config_prefix"] = str(prefix)
    fixture = root / "update-host.json"
    roots = {}
    for version in ("0.0.1", meta["version"]):
        checkout = root / ("market-" + version)
        descriptor = checkout / "plugins/chatgpt/.codex-plugin/plugin.json"
        descriptor.parent.mkdir(parents=True)
        descriptor.write_text(json.dumps({"name": "agent-job-relay", "version": version}))
        command(["git", "init", checkout], "update-git-init-" + version)
        command(["git", "-C", checkout, "add", "."], "update-git-add-" + version)
        command(["git", "-C", checkout, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false", "commit", "-m", "fixture"], "update-git-commit-" + version)
        command(["git", "-C", checkout, "-c", "tag.gpgsign=false", "tag", "v" + version], "update-git-tag-" + version)
        roots[version] = str(checkout)
    fixture.write_text(json.dumps({"market": "0.0.1", "plugin": "0.0.1", "failAdd": True, "roots": roots, "calls": []}))
    fake = prefix / "bin/codex"
    # A public-CLI model, not an actual host loader or Desktop session.
    fake.write_text('''#!/usr/bin/env node
const fs = require('node:fs');
const path = process.env.AJR_TEST_CODEX_STATE;
const s = JSON.parse(fs.readFileSync(path));
const a = process.argv.slice(2); s.calls.push(a);
const save = () => fs.writeFileSync(path, JSON.stringify(s));
const out = x => { save(); console.log(JSON.stringify(x)); };
if (a[1] === 'marketplace') {
  if (a[2] === 'list') out({marketplaces:s.market ? [{name:'agent-job-relay',root:s.roots[s.market],marketplaceSource:{sourceType:'git',source:'https://github.com/MorningFrog/agent-job-relay.git'}}] : []});
  else if (a[2] === 'remove') {
    if (s.failRemove) { s.failRemove=false;save();console.error('injected removal failure');process.exit(1); }
    s.market=null; out({});
  }
  else if (a[2] === 'add') {
    if (s.failAdd) { s.failAdd=false;save();console.error('injected fetch failure');process.exit(1); }
    s.market=a[a.indexOf('--ref')+1].slice(1);out({});
  } else process.exit(2);
} else if (a[1] === 'list') out({installed:s.plugin ? [{pluginId:'agent-job-relay@agent-job-relay',marketplaceName:'agent-job-relay',version:s.plugin,enabled:true,marketplaceSource:{sourceType:'git',source:'https://github.com/MorningFrog/agent-job-relay.git'}}] : []});
else if (a[1] === 'add') { s.plugin=s.market;out({pluginId:'agent-job-relay@agent-job-relay',version:s.plugin}); }
else if (a[1] === 'remove') { s.plugin=null;out({}); }
else process.exit(2);
''')
    fake.chmod(0o755)
    env["AJR_TEST_CODEX_STATE"] = str(fixture)
    # Exercise a real native package replacement with the same-schema older
    # image fixture used by native daemon-upgrade tests, not a historical build.
    command(["ajr", "daemon", "stop"], "update-initial-stop")
    metadata = json.loads((package / "package.json").read_text())
    original = meta["version"]
    older = "0.0." + "1" * (len(original) - 4)
    image = package / "native/linux-x64/agent-job-relay"
    data = image.read_bytes().replace(original.encode(), older.encode())
    if data == image.read_bytes():
        raise RuntimeError("older-version native fixture was not changed")
    image.write_bytes(data)
    metadata["version"] = older
    (package / "package.json").write_text(json.dumps(metadata))
    manifest = json.loads((package / "native/manifest.json").read_text())
    manifest["version"] = older
    manifest["targets"]["linux-x64"]["sha256"] = hashlib.sha256(data).hexdigest()
    (package / "native/manifest.json").write_text(json.dumps(manifest))
    old_image = root / "old-image"
    old_image.write_bytes(data)
    old_image.chmod(0o700)
    command(["ajr", "daemon", "start"], "update-older-daemon")
    work = root / "continued-work"
    work.mkdir()
    running = json.loads(command(["ajr", "start", "--cwd", work, "--shell", "echo once >> launches; echo ready; while [ ! -f finish ]; do sleep 0.05; done"], "update-running-job"))["job"]
    preview = json.loads(command(["ajr", "update", "--dry-run"], "update-preview"))
    if preview["version"] != original or not preview["changed"]:
        raise RuntimeError("unified update preview did not pin the candidate")
    if json.loads(fixture.read_text())["market"] != "0.0.1":
        raise RuntimeError("preview changed the host")
    other_bin = root / "other-bin"
    other_bin.mkdir()
    (other_bin / "ajr").write_text("#!/bin/sh\nexit 1\n")
    (other_bin / "ajr").chmod(0o755)
    original_path = env["PATH"]
    env["PATH"] = str(other_bin) + os.pathsep + original_path
    calls_before = json.loads(fixture.read_text())["calls"]
    command([prefix / "bin/ajr", "update"], "update-other-path-refused", expected=1)
    if json.loads(fixture.read_text())["calls"] != calls_before:
        raise RuntimeError("a different PATH installation was allowed to mutate the host")
    env["PATH"] = original_path
    command(["ajr", "update"], "update-partial", expected=1)
    journal = root / "agent-job-relay/codex/online-update.json"
    if json.loads(journal.read_text())["stage"] != "adding_marketplace":
        raise RuntimeError("partial update stage was not retained")
    if command(["ajr", "--version"], "update-runtime-version").strip() != f"agent-job-relay {original}":
        raise RuntimeError("actual npm replacement did not install the candidate")
    if json.loads(command(["ajr", "doctor"], "update-restarted"))["version"] != original:
        raise RuntimeError("previously online profile did not restart from updated bytes")
    continued = json.loads(command(["ajr", "status", running["job_id"]], "update-continued-status"))
    if continued["run_id"] != running["run_id"] or continued["status"] != "running":
        raise RuntimeError("package replacement did not preserve the original running attempt")
    # npm replaced the test transport along with the rest of the actual package.
    updater.write_text(updater.read_text().replace("https://registry.npmjs.org", registry))
    env["CODEX_HOME"] = str(root / "different-codex")
    calls_before = json.loads(fixture.read_text())["calls"]
    command(["ajr", "update"], "update-host-selection-refused", expected=1)
    if json.loads(fixture.read_text())["calls"] != calls_before:
        raise RuntimeError("retry changed a different host selection")
    del env["CODEX_HOME"]
    result = json.loads(command(["ajr", "update"], "update-resume"))
    if not result["resumed_update"] or json.loads(journal.read_text())["stage"] != "complete":
        raise RuntimeError("partial update did not finish forward")
    calls = json.loads(fixture.read_text())["calls"]
    if sum(a[:3] == ["plugin", "marketplace", "remove"] for a in calls) != 1:
        raise RuntimeError("retry replayed marketplace removal")
    if json.loads(command(["ajr", "update"], "update-unchanged"))["changed"]:
        raise RuntimeError("unchanged update was not a no-op")
    if json.loads(command(["ajr", "status", job_id], "update-history"))["status"] != "completed":
        raise RuntimeError("npm replacement changed the original history/profile")
    # A current CLI/plugin must still replace an older daemon, without requiring
    # another package update or restarting the independent runner.
    command(["ajr", "daemon", "stop"], "update-daemon-only-stop")
    state = root / "agent-job-relay/codex"
    command([old_image, "--state-dir", state, "daemon", "start"], "update-daemon-only-older")
    if not json.loads(command(["ajr", "update"], "update-daemon-only"))["changed"] or json.loads(command(["ajr", "doctor"], "update-daemon-only-health"))["version"] != original:
        raise RuntimeError("current package did not automatically upgrade the older daemon")
    (work / "finish").write_text("")
    deadline = time.monotonic() + 5
    while json.loads(command(["ajr", "status", running["job_id"]], "update-completed-status"))["status"] != "completed":
        if time.monotonic() > deadline:
            raise RuntimeError("continued runner did not complete")
        time.sleep(0.05)
    if (work / "launches").read_text() != "once\n":
        raise RuntimeError("update replayed the original workload")
    removal_work = root / "removal-work"
    removal_work.mkdir()
    active = json.loads(command(["ajr", "start", "--cwd", removal_work, "--shell", "echo ready; while [ ! -f finish ]; do sleep 0.05; done"], "uninstall-active"))["job"]
    command(["ajr", "uninstall"], "uninstall-active-refused", expected=1)
    preview = json.loads(command(["ajr", "uninstall", "--dry-run", "--force"], "uninstall-preview"))
    if preview["active_jobs"] != 1 or not package.exists():
        raise RuntimeError("force removal preview changed the installation")
    pending = json.loads(fixture.read_text())
    pending["failRemove"] = True
    fixture.write_text(json.dumps(pending))
    command(["ajr", "uninstall", "--force"], "uninstall-partial", expected=1)
    removal_journal = state / "online-uninstall.json"
    if json.loads(removal_journal.read_text())["stage"] != "removing_marketplace" or not package.exists():
        raise RuntimeError("partial uninstall did not retain its CLI/recovery stage")
    command(["ajr", "update"], "update-during-uninstall-refused", expected=1)
    command(["ajr", "uninstall", "--force"], "uninstall-resume")
    if package.exists() or (prefix / "bin/ajr").exists() or json.loads(removal_journal.read_text())["stage"] != "complete":
        raise RuntimeError("uninstall did not remove its exact global package/commands")
    # Inspect through the separately installed native package, without starting
    # the removed global runtime or changing its preserved profile.
    fallback = root / "installed/node_modules/agent-job-relay-runtime/cli.mjs"
    if json.loads(command(["node", fallback, "--state-dir", state, "doctor"], "uninstall-offline"))["daemon"] != "stopped":
        raise RuntimeError("uninstall restarted the daemon")
    # CLI status requires an online daemon; direct read-only inspection returns
    # the stored exact-run result without creating one.
    inspected = json.loads(command(["node", fallback, "--state-dir", state, "inspect", active["job_id"], "--json"], "uninstall-retained-status"))
    if inspected["job"]["status"] != "cancelled" or not (state / "relay.sqlite3").exists():
        raise RuntimeError("forced uninstall did not preserve the cancelled task history")
    fake.unlink()
    del env["AJR_TEST_CODEX_STATE"]
    return {"native_package_replacement": True, "active_runner_continuation": True, "daemon_only_upgrade": True, "partial_public_host_fixture": True, "forward_retry": True, "history_preserved": True, "force_uninstall": True, "actual_desktop": False}
