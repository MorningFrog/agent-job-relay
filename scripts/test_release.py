"""Behavioral release tests: no real accounts, registry writes or application state."""
from contextlib import contextmanager
import io
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
from threading import Thread
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
import release_actions as actions
import release_metadata as metadata
import release_pipeline as pipeline


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        (metadata.REPO / ".tmp").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="release-test-", dir=metadata.REPO / ".tmp")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "source"
        self.root.mkdir()
        files = ["Cargo.toml", "Cargo.lock", "CHANGELOG.md", "plugins/chatgpt/plugin.json", "plugins/chatgpt/.codex-plugin/plugin.json", "plugins/agent-job-relay/plugin.json", "plugins/chatgpt/mcp.json", "plugins/chatgpt/.mcp.json", "plugins/chatgpt/relay", "plugins/chatgpt/relay.cmd", "plugins/chatgpt/skills/background-jobs/SKILL.md", "packages/runtime/package.json", "plugins/opencode-v1/package.json"]
        for name in files:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(metadata.REPO / name, target)
        self.directory = Path(self.temp.name) / "candidate"
        self.directory.mkdir()
        self.version = metadata.version(self.root)
        self.meta = {"version": self.version, "platforms": ["linux-x64", "win32-x64"], "minimumGlibc": "2.35", "marketplace": f"agent-job-relay-marketplace-{self.version}.zip"}
        for key, name in metadata.PACKAGES.items():
            tarball = self.directory / f"{name}-{self.version}.tgz"
            with tarfile.open(tarball, "w:gz") as archive:
                data = (self.root / metadata.PACKAGE_PATHS[key] / "package.json").read_bytes()
                member = tarfile.TarInfo("package/package.json")
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
            self.meta[key] = {"name": name, "version": self.version, "tarball": tarball.name, "integrity": metadata.integrity(tarball)}
        (self.directory / self.meta["marketplace"]).write_bytes(b"isolated fake marketplace")
        (self.directory / "release.json").write_text(json.dumps(self.meta))
        metadata.write_checksums(self.directory)
        self.revision = "a" * 40
        self.run = {"id": 123, "status": "completed", "conclusion": "success", "event": "workflow_dispatch", "path": ".github/workflows/release.yml", "head_branch": "main", "head_sha": self.revision, "repository": {"full_name": actions.REPOSITORY}, "head_repository": {"full_name": actions.REPOSITORY}}
        self.candidate = metadata.seal_candidate(self.directory, "next", actions.REPOSITORY, self.revision, "123", self.root)
        # A missing mock must fail before it can contact a real service.
        self.real_request = actions.request
        self.network = patch.object(actions, "request", side_effect=AssertionError("Unexpected real network operation"))
        self.network.start()
        self.addCleanup(self.network.stop)

    def validate(self, run=None):
        return metadata.validate_promotion(self.directory, run or self.run, actions.REPOSITORY, self.revision, "123", self.root)

    def draft(self, release_id=1):
        body = (self.directory / "release-notes.md").read_text() + f"\n<!-- AgentJobRelay candidate 123 {self.revision} next -->"
        return {"body": body, "tag_name": "v" + self.version, "target_commitish": self.revision, "prerelease": True, "draft": True, "id": release_id}

    def stage(self, status="staged"):
        entry = self.meta["runtime"]
        return {"id": "11111111-1111-4111-8111-111111111111", "packageName": entry["name"], "version": self.version, "tag": "next", "status": status, "shasum": actions.hashlib.sha1((self.directory / entry["tarball"]).read_bytes()).hexdigest()}

    @contextmanager
    def registry_clock(self):
        elapsed = 0

        def advance(seconds):
            nonlocal elapsed
            elapsed += seconds

        with patch.object(actions.time, "monotonic", side_effect=lambda: elapsed), patch.object(actions.time, "sleep", side_effect=advance) as sleep:
            yield sleep

    def test_version_and_changelog_are_enforced_before_build(self):
        self.assertEqual(metadata.source_metadata(self.root)["version"], self.version)
        path = self.root / "plugins/opencode-v1/package.json"
        doc = json.loads(path.read_text())
        doc["dependencies"][metadata.PACKAGES["runtime"]] = "0.0.1"
        path.write_text(json.dumps(doc))
        with self.assertRaisesRegex(ValueError, "exact Cargo"):
            metadata.source_metadata(self.root)
        for text in ["# Changes\n## [Unreleased]\n- Future\n", f"## [{self.version}]\n\n### Fixed\n", f"## [{self.version}]\n- One\n## [{self.version}]\n- Two\n"]:
            with self.assertRaises(ValueError):
                metadata.changelog_notes(text, self.version)
        text = f"## [Unreleased]\n- Future\n## [{self.version}] - 2026-10-27\n- Actual change\n## 0.0.1 — old\n- History\n"
        self.assertEqual(metadata.changelog_notes(text, self.version), "- Actual change\n")

    def test_promotion_fences_run_identity_and_source(self):
        self.assertEqual(self.validate()["channel"], "next")
        for field, value in [("head_sha", "b" * 40), ("event", "pull_request"), ("head_branch", "foreign"), ("conclusion", "failure"), ("path", ".github/workflows/ci.yml")]:
            run = {**self.run, field: value}
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(run)
        run = {**self.run, "head_repository": {"full_name": "foreign/repo"}}
        with self.assertRaisesRegex(ValueError, "repository"):
            self.validate(run)

    def test_changed_assets_and_rewritten_checksums_are_refused(self):
        notes = self.directory / "release-notes.md"
        notes.write_text("changed")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            self.validate()
        metadata.write_checksums(self.directory)
        with self.assertRaisesRegex(ValueError, "Sealed candidate"):
            self.validate()

    def test_checksum_inventory_rejects_symlink_extra_and_path_escape(self):
        checksums = self.directory / "SHA256SUMS"
        original = checksums.read_text()
        checksums.write_text(original + "0" * 64 + "  ../foreign\n")
        with self.assertRaisesRegex(ValueError, "checksum"):
            metadata.check_checksums(self.directory)
        checksums.write_text(original)
        extra = self.directory / "unexpected.txt"
        extra.write_text("extra")
        with self.assertRaisesRegex(ValueError, "inventory"):
            metadata.check_checksums(self.directory)
        extra.unlink()
        notes = self.directory / "release-notes.md"
        original_notes = notes.read_text()
        notes.unlink()
        outside = Path(self.temp.name) / "notes"
        outside.write_text(original_notes)
        notes.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            metadata.check_checksums(self.directory)

    def test_preview_only_uses_dry_run_and_never_creates_stage(self):
        before = {p.name: p.read_bytes() for p in self.directory.iterdir()}
        with patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "command", return_value="{}") as commands, patch.dict(os.environ, {"NPM_STAGE_TOKEN": "fake-stage-token"}), patch.object(sys, "argv", ["release_actions.py", "preview", "--release", str(self.directory)]), patch.object(actions, "summary"):
            actions.main()
        self.assertEqual(len(commands.call_args_list), 2)
        for call in commands.call_args_list:
            self.assertIn("--dry-run", call.args[0])
            self.assertEqual(call.args[0][:3], ["npm", "stage", "publish"])
            self.assertNotIn("ACTIONS_ID_TOKEN_REQUEST_TOKEN", call.kwargs["env"])
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.directory.iterdir()})

    def test_oidc_environment_cannot_fall_back_to_stage_token(self):
        with patch.dict(os.environ, {"NPM_STAGE_TOKEN": "fake-stage-token", "NPM_TOKEN": "fake-other-token", "NODE_AUTH_TOKEN": "fake-inherited", "ACTIONS_ID_TOKEN_REQUEST_TOKEN": "oidc-context"}):
            with actions.npm_environment("oidc") as env:
                for key in ("NPM_STAGE_TOKEN", "NPM_TOKEN", "NODE_AUTH_TOKEN"):
                    self.assertNotIn(key, env)
                self.assertEqual(env["ACTIONS_ID_TOKEN_REQUEST_TOKEN"], "oidc-context")
                self.assertNotIn("fake", Path(env["npm_config_userconfig"]).read_text())
            with actions.npm_environment("stage") as env:
                self.assertNotIn("ACTIONS_ID_TOKEN_REQUEST_TOKEN", env)
                self.assertEqual(env["NODE_AUTH_TOKEN"], "fake-stage-token")
                self.assertNotIn("fake", Path(env["npm_config_userconfig"]).read_text())

    def test_prepare_checks_source_without_publishing_credentials(self):
        with patch.object(actions, "actions_context", return_value=self.revision), patch.object(actions, "github", return_value={"full_name": actions.REPOSITORY}), patch.object(actions, "check_tag") as tag, patch.object(actions, "github_list", return_value=[]), patch.object(actions, "registry_version", return_value=None), patch.object(actions, "command") as npm, patch.object(actions, "request") as exchange, patch.object(actions, "output"), patch.object(actions, "summary"):
            actions.prepare()
            tag.assert_called_once_with("v" + self.version, self.revision)
            npm.assert_not_called()
            exchange.assert_not_called()

    def test_oidc_preflight_rejects_the_prepare_workflow_identity(self):
        with patch.dict(os.environ, {"CANDIDATE_RUN_ID": "", "GITHUB_WORKFLOW_REF": f"{actions.REPOSITORY}/.github/workflows/release.yml@refs/heads/main"}), patch.object(actions, "actions_context", return_value=self.revision), patch.object(actions, "github", return_value={"full_name": actions.REPOSITORY}), patch.object(actions, "check_tag"), patch.object(actions, "request") as exchange:
            with self.assertRaisesRegex(ValueError, "Publish Release"):
                actions.preflight("oidc")
            exchange.assert_not_called()

    def test_oidc_preflight_exchanges_for_both_packages_from_publish_workflow(self):
        env = {"CANDIDATE_RUN_ID": "", "GITHUB_WORKFLOW_REF": f"{actions.REPOSITORY}/.github/workflows/release-publish.yml@refs/heads/main", "ACTIONS_ID_TOKEN_REQUEST_URL": "https://identity.example.invalid/token", "ACTIONS_ID_TOKEN_REQUEST_TOKEN": "fake-request-token"}
        token = {"token_type": "oidc", "token": "fake-short-lived-token", "expires": "2026-10-28T01:00:00Z"}
        with patch.dict(os.environ, env), patch.object(actions, "actions_context", return_value=self.revision), patch.object(actions, "github", return_value={"full_name": actions.REPOSITORY}), patch.object(actions, "check_tag"), patch.object(actions, "request", side_effect=[{"value": "fake-jwt"}, token, token]) as exchange, patch.object(actions, "summary"), patch.object(actions, "output"):
            actions.preflight("oidc")
        self.assertEqual(exchange.call_count, 3)
        for call, name in zip(exchange.call_args_list[1:], metadata.PACKAGES.values()):
            self.assertTrue(call.args[0].endswith("/package/" + name))
            self.assertEqual(call.kwargs["method"], "POST")

    def test_oidc_preview_uses_the_sealed_channel_and_never_uploads(self):
        candidate = {**self.candidate, "channel": "latest"}
        before = {p.name: p.read_bytes() for p in self.directory.iterdir()}
        with patch.object(actions, "validate", return_value=candidate) as validate, patch.object(actions, "find_release", return_value=None), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "command", return_value="{}") as commands, patch.object(actions, "summary"), patch.dict(os.environ, {"NPM_STAGE_TOKEN": "fake", "NODE_AUTH_TOKEN": "fake"}):
            actions.preview(self.directory, "oidc", "next", "123")
        validate.assert_called_once_with(self.directory, "123")
        self.assertEqual(commands.call_count, 2)
        for call in commands.call_args_list:
            self.assertEqual(call.args[0][:2], ["npm", "publish"])
            self.assertIn("--dry-run", call.args[0])
            self.assertIn("--tag=latest", call.args[0])
            self.assertNotIn("NODE_AUTH_TOKEN", call.kwargs["env"])
            self.assertNotIn("NPM_STAGE_TOKEN", call.kwargs["env"])
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.directory.iterdir()})

    def test_publication_preview_reuses_a_matching_partially_published_version(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", side_effect=[{}, None]), patch.object(actions, "find_release", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "command", return_value="{}") as commands, patch.object(actions, "github") as github, patch.object(actions, "summary"):
            actions.preview(self.directory, "oidc", "latest", "123")
        self.assertEqual(commands.call_count, 1)
        self.assertIn(self.directory / self.meta["opencode"]["tarball"], commands.call_args.args[0])
        github.assert_not_called()

    def test_oidc_publication_never_polls_and_reuses_matching_public_uploads(self):
        for runtime_exists in (False, True):
            with self.subTest(runtime_exists=runtime_exists):
                uploaded = []

                def publish(argv, **_kwargs):
                    self.assertEqual(argv[:2], ["npm", "publish"])
                    uploaded.append(Path(argv[2]).name)
                    return "{}"

                def lookup(entry):
                    self.assertFalse(uploaded, "Publication must not read registry visibility after a write")
                    if runtime_exists and entry["name"] == metadata.PACKAGES["runtime"]:
                        return {"name": entry["name"], "version": self.version, "dist": {"integrity": entry["integrity"]}}
                    return None

                with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "registry_version", side_effect=lookup), patch.object(actions, "check_channel", return_value={"next": self.version}), patch.object(actions, "draft_release", return_value=self.draft()), patch.object(actions, "command", side_effect=publish), patch.object(actions, "record_publish_acceptance") as receipt, patch.object(actions, "github") as github, patch.object(actions.time, "sleep") as sleep, patch.object(actions, "output") as output, patch.object(actions, "summary"):
                    actions.publish(self.directory, "123", "oidc")
                keys = ["opencode"] if runtime_exists else list(metadata.PACKAGES)
                self.assertEqual(uploaded, [self.meta[key]["tarball"] for key in keys])
                self.assertEqual([call.args[2] for call in receipt.call_args_list], keys)
                sleep.assert_not_called()
                output.assert_called_once_with("submitted", "true")
                github.assert_called_once_with("releases/1", method="PATCH", payload={"draft": False, "make_latest": "false"})

    def test_acceptance_receipt_skips_upload_while_public_metadata_is_missing(self):
        release = {**self.draft(), "_accepted_npm_packages": {"runtime"}}
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "draft_release", return_value=release), patch.object(actions, "command", return_value="{}") as command, patch.object(actions, "record_publish_acceptance") as receipt, patch.object(actions, "github"), patch.object(actions.time, "sleep") as sleep, patch.object(actions, "output") as output, patch.object(actions, "summary"):
            actions.publish(self.directory, "123", "oidc")
        command.assert_called_once()
        self.assertEqual(command.call_args.args[0][2], self.directory / self.meta["opencode"]["tarball"])
        receipt.assert_called_once_with(self.candidate, release, "opencode")
        sleep.assert_not_called()
        output.assert_called_once_with("submitted", "true")

    def test_check_rejects_conflicting_public_bytes_without_mutation(self):
        entry = self.meta["runtime"]
        foreign = {"name": entry["name"], "version": self.version, "dist": {"integrity": "foreign"}}
        with self.registry_clock() as sleep, patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "registry_version", return_value=foreign), patch.object(actions, "verify_public") as install, patch.object(actions, "github") as github, patch.object(actions, "command") as command:
            with self.assertRaisesRegex(ValueError, "conflict"):
                actions.check_publication(self.directory, "123")
        sleep.assert_not_called()
        install.assert_not_called()
        github.assert_not_called()
        command.assert_not_called()

    def test_matching_stage_is_reused_only_after_byte_and_tag_check(self):
        entry = self.meta["runtime"]
        tarball = self.directory / entry["tarball"]
        stage = self.stage()
        with patch.object(actions, "command", side_effect=[json.dumps([stage]), json.dumps(stage)]) as commands, patch.object(actions, "request", return_value=tarball.read_bytes()), patch.object(actions, "summary"):
            actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
            self.assertEqual(commands.call_count, 2)
        with patch.object(actions, "command", return_value=json.dumps([{**stage, "tag": "latest"}])):
            with self.assertRaisesRegex(ValueError, "another channel"):
                actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
        with patch.object(actions, "command", side_effect=[json.dumps([stage]), json.dumps(stage)]), patch.object(actions, "request", return_value=b"foreign"):
            with self.assertRaisesRegex(ValueError, "differs"):
                actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})

    def test_new_stage_is_created_once_then_downloaded_for_verification(self):
        entry = self.meta["runtime"]
        tarball = self.directory / entry["tarball"]
        stage_id = "11111111-1111-4111-8111-111111111111"
        with patch.object(actions, "command", side_effect=["[]", json.dumps({entry["name"]: {"stageId": stage_id}}), json.dumps(self.stage())]) as commands, patch.object(actions, "request", return_value=tarball.read_bytes()) as download, patch.object(actions, "summary"):
            actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
        argv = commands.call_args_list[1].args[0]
        self.assertEqual(argv[:3], ["npm", "stage", "publish"])
        self.assertIn("--tag=next", argv)
        self.assertTrue(download.call_args.args[0].endswith(f"/{stage_id}/tarball"))

    def test_validating_stage_is_retained_without_premature_download(self):
        entry = self.meta["runtime"]
        stage = self.stage("validating")
        with patch.object(actions, "command", side_effect=["[]", json.dumps({"stageId": stage["id"]}), json.dumps(stage)]) as commands, patch.object(actions, "request") as download, patch.object(actions, "summary") as summary:
            self.assertFalse(actions.stage_package(entry, self.directory / entry["tarball"], "next", {"NODE_AUTH_TOKEN": "fake"}))
            self.assertEqual(commands.call_count, 3)
            download.assert_not_called()
            self.assertIn("Do not approve yet", summary.call_args.args[0])

    def test_pending_validation_does_not_prevent_considering_the_second_package(self):
        @contextmanager
        def env(_auth):
            yield {"NODE_AUTH_TOKEN": "fake"}

        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "draft_release", return_value=self.draft()), patch.object(actions, "npm_environment", env), patch.object(actions, "stage_package", side_effect=[False, True]) as stage, patch.object(actions, "output") as output, patch.object(actions, "summary") as summary:
            actions.publish(self.directory, "123", "stage")
        self.assertEqual([c.args[0]["name"] for c in stage.call_args_list], list(metadata.PACKAGES.values()))
        output.assert_called_once_with("ready", "false")
        self.assertIn("Waiting for npm automated validation", summary.call_args.args[0])

    def test_stage_metadata_conflicts_and_unknown_status_stop_before_download(self):
        entry = self.meta["runtime"]
        for field, value in [("status", "future-state"), ("shasum", "0" * 40), ("packageName", "foreign"), ("tag", "latest")]:
            with self.subTest(field=field), patch.object(actions, "command", side_effect=[json.dumps([self.stage()]), json.dumps({**self.stage(), field: value})]), patch.object(actions, "request") as download:
                with self.assertRaises(ValueError):
                    actions.stage_package(entry, self.directory / entry["tarball"], "next", {"NODE_AUTH_TOKEN": "fake"})
                download.assert_not_called()

    def test_conflicting_public_version_stops_before_draft_or_upload(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "registry_version", return_value={"name": metadata.PACKAGES["runtime"], "version": self.version, "dist": {"integrity": "foreign"}}), patch.object(actions, "draft_release") as draft:
            with self.assertRaisesRegex(ValueError, "conflict"):
                actions.publish(self.directory, "123", "stage")
            draft.assert_not_called()

    def test_staging_waits_and_approval_resume_skips_all_uploads(self):
        @contextmanager
        def env(_auth):
            yield {"NODE_AUTH_TOKEN": "fake"}

        for approved in (False, True):
            with self.subTest(approved=approved), patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value={} if approved else None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "draft_release", return_value={"id": 1}), patch.object(actions, "npm_environment", env), patch.object(actions, "stage_package") as stage, patch.object(actions, "output") as outputs, patch.object(actions, "summary"):
                actions.publish(self.directory, "123", "stage")
                self.assertEqual(stage.call_count, 0 if approved else 2)
                outputs.assert_called_once_with("ready", str(approved).lower())

    def test_foreign_release_and_assets_are_preserved(self):
        with patch.object(actions, "check_tag"), patch.object(actions, "github", return_value=[{**self.draft(), "body": "foreign"}]), patch.object(actions, "upload_release_asset") as mutation:
            with self.assertRaisesRegex(ValueError, "foreign/modified"):
                actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()
        release = self.draft()
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[release], [{"name": "foreign.txt"}]]), patch.object(actions, "upload_release_asset") as mutation:
            with self.assertRaisesRegex(ValueError, "foreign/duplicate"):
                actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()

    def test_local_process_cannot_publish(self):
        with patch.dict(os.environ, {"GITHUB_ACTIONS": "false"}):
            with self.assertRaisesRegex(ValueError, "manual GitHub"):
                actions.publish(self.directory, "123", "stage")

    def test_repository_api_uses_the_canonical_path(self):
        with patch.dict(os.environ, {"GH_TOKEN": "fake"}), patch.object(actions, "request", return_value={}) as api:
            actions.github("")
            self.assertEqual(api.call_args.args[0], f"https://api.github.com/repos/{actions.REPOSITORY}")

    def test_newer_npm_channel_stops_before_draft_or_upload(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "request", return_value={"next": "99.0.0-beta.1"}), patch.object(actions, "draft_release") as draft, patch.object(actions, "command") as mutation:
            with self.assertRaisesRegex(ValueError, "backwards"):
                actions.publish(self.directory, "123", "stage")
            draft.assert_not_called()
            mutation.assert_not_called()

    def test_existing_tag_is_revalidated_by_commit_including_annotated_tags(self):
        with patch.object(actions, "github", side_effect=[{"object": {"type": "tag", "sha": "c" * 40}}, {"object": {"type": "commit", "sha": self.revision}}]):
            actions.check_tag("v" + self.version, self.revision)
        with patch.object(actions, "github", return_value={"object": {"type": "commit", "sha": "b" * 40}}):
            with self.assertRaisesRegex(ValueError, "not be moved"):
                actions.check_tag("v" + self.version, self.revision)

    def test_matching_draft_assets_are_not_uploaded_again(self):
        release = self.draft()
        assets = [{"name": p.name, "digest": "sha256:" + metadata.sha256(p)} for p in self.directory.iterdir()]
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[release], assets]) as api, patch.object(actions, "upload_release_asset") as mutation:
            actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()
            self.assertEqual(api.call_args_list[0].args[0], "releases?per_page=100&page=1")
        assets[0]["digest"] = "sha256:" + "0" * 64
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[release], assets]), patch.object(actions, "upload_release_asset") as mutation:
            with self.assertRaisesRegex(ValueError, "differs"):
                actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()

    def test_empty_retry_draft_is_preserved_and_populated_original_is_reused(self):
        original, duplicate = self.draft(1), self.draft(2)
        assets = [{"name": p.name, "digest": "sha256:" + metadata.sha256(p)} for p in self.directory.iterdir()]
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[duplicate, original], [], assets]) as api, patch.object(actions, "upload_release_asset") as upload, patch.object(actions, "summary"):
            self.assertEqual(actions.draft_release(self.directory, self.candidate)["id"], 1)
            upload.assert_not_called()
            self.assertTrue(all(not c.kwargs.get("method") for c in api.call_args_list))

    def test_partial_draft_uploads_missing_assets_only_to_its_exact_id(self):
        release = self.draft(17)
        present = self.directory / "release.json"
        assets = [{"name": present.name, "digest": "sha256:" + metadata.sha256(present)}]
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[release], assets]), patch.object(actions, "upload_release_asset") as upload:
            actions.draft_release(self.directory, self.candidate)
        self.assertEqual({c.args[1].name for c in upload.call_args_list}, {p.name for p in self.directory.iterdir()} - {present.name})
        self.assertTrue(all(c.args[0] == 17 for c in upload.call_args_list))

    def test_multiple_populated_drafts_are_preserved_without_any_write(self):
        asset = {"name": "release.json", "digest": "sha256:" + metadata.sha256(self.directory / "release.json")}
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[[self.draft(1), self.draft(2)], [asset], [asset]]) as api, patch.object(actions, "upload_release_asset") as upload:
            with self.assertRaisesRegex(ValueError, "Multiple populated"):
                actions.draft_release(self.directory, self.candidate)
            upload.assert_not_called()
            self.assertTrue(all(not c.kwargs.get("method") for c in api.call_args_list))

    def test_release_listing_paginates_before_creating_any_draft(self):
        others = [{"tag_name": f"other-{i}"} for i in range(100)]
        release = self.draft()
        assets = [{"name": p.name, "digest": "sha256:" + metadata.sha256(p)} for p in self.directory.iterdir()]
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[others, [release], assets]) as api, patch.object(actions, "upload_release_asset") as upload:
            actions.draft_release(self.directory, self.candidate)
            upload.assert_not_called()
            self.assertEqual(api.call_args_list[1].args[0], "releases?per_page=100&page=2")

    def test_asset_upload_uses_binary_bytes_and_verifies_returned_digest(self):
        path = self.directory / "release.json"
        asset = {"name": path.name, "digest": "sha256:" + metadata.sha256(path)}
        with patch.dict(os.environ, {"GH_TOKEN": "fake"}), patch.object(actions, "request", return_value=asset) as api:
            actions.upload_release_asset(17, path)
        self.assertEqual(api.call_args.args[0], f"https://uploads.github.com/repos/{actions.REPOSITORY}/releases/17/assets?name=release.json")
        self.assertEqual(api.call_args.kwargs["payload"], path.read_bytes())

    def test_recovery_requires_opt_in_before_accepting_an_older_source(self):
        newer = "b" * 40
        artifact = {"artifacts": [{"name": "release-candidate", "expired": False}]}
        with patch.dict(os.environ, {"GITHUB_SHA": newer, "RECOVER_CANDIDATE": "false"}), patch.object(actions, "github", return_value=self.run), patch.object(actions, "check_recovery_source") as check:
            with self.assertRaisesRegex(ValueError, "explicitly select recovery"):
                actions.candidate_run("123")
            check.assert_not_called()
        with patch.dict(os.environ, {"GITHUB_SHA": newer, "RECOVER_CANDIDATE": "true"}), patch.object(actions, "github", side_effect=[self.run, artifact]), patch.object(actions, "check_recovery_source") as check:
            self.assertEqual(actions.candidate_run("123")["head_sha"], self.revision)
            check.assert_called_once_with(self.revision, newer)

    def test_recovery_preflight_checks_the_original_candidate_tag(self):
        @contextmanager
        def env(_auth):
            yield {}

        with patch.dict(os.environ, {"CANDIDATE_RUN_ID": "123"}), patch.object(actions, "actions_context", return_value="b" * 40), patch.object(actions, "candidate_run", return_value=self.run), patch.object(actions, "github", return_value={"full_name": actions.REPOSITORY}), patch.object(actions, "check_tag") as tag, patch.object(actions, "npm_environment", env), patch.object(actions, "command", return_value='"morningfrog"'), patch.object(actions, "summary"), patch.object(actions, "output"):
            actions.preflight("stage")
        tag.assert_called_once_with("v" + self.version, self.revision)

    def test_recovery_allows_only_unchanged_payloads_in_a_descendant_checkout(self):
        def git(*args):
            return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE, text=True).strip()

        git("init", "-q")
        git("config", "user.name", "Release fixture")
        git("config", "user.email", "fixture@example.invalid")
        git("add", ".")
        git("commit", "-qm", "fixture: candidate")
        original = git("rev-parse", "HEAD")
        path = self.root / "docs/release.md"
        path.parent.mkdir(exist_ok=True)
        path.write_text("release repair\n")
        fixture = self.root / "tests/integration.rs"
        fixture.parent.mkdir(exist_ok=True)
        fixture.write_text("// isolated integration fixture\n")
        git("add", ".")
        git("commit", "-qm", "fixture: tooling repair")
        tooling = git("rev-parse", "HEAD")
        with patch.object(actions, "REPO", self.root), patch.object(actions, "command", wraps=lambda argv, **kw: subprocess.check_output(argv, cwd=self.root, text=not kw.get("binary", False), stderr=subprocess.PIPE)), patch.object(actions, "summary"):
            actions.check_recovery_source(original, tooling)
            for relative in ["Cargo.toml", "packages/runtime/README.md", "plugins/chatgpt/mcp.json", "CHANGELOG.md", "src/main.rs", "tests/runtime.rs"]:
                with self.subTest(path=relative):
                    target = self.root / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    original_bytes = target.read_bytes() if target.exists() else None
                    target.write_bytes((original_bytes or b"") + b"\nchanged\n")
                    git("add", ".")
                    git("commit", "-qm", "fixture: changed payload")
                    with self.assertRaisesRegex(ValueError, "allowlisted"):
                        actions.check_recovery_source(original, git("rev-parse", "HEAD"))
                    if original_bytes is None:
                        target.unlink()
                    else:
                        target.write_bytes(original_bytes)
                    git("add", ".")
                    git("commit", "-qm", "fixture: restore payload")
            with self.assertRaisesRegex(ValueError, "checkout differs"):
                actions.check_recovery_source(original, tooling)

    def test_http_binary_upload_and_sanitized_failure_context(self):
        received = {}

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_POST(self):
                if self.path.endswith("/empty"):
                    self.send_response(204)
                    self.end_headers()
                    return
                received["content_type"] = self.headers["Content-Type"]
                received["body"] = self.rfile.read(int(self.headers["Content-Length"]))
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{"uploaded":true}')

            def do_GET(self):
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b'fake-private-server-response')

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            url = f"http://127.0.0.1:{server.server_port}/fixture"
            payload = b"\x00binary\xff\x00"
            self.assertEqual(self.real_request(url, token="fake-fixture-token", method="POST", payload=payload), {"uploaded": True})
            self.assertEqual(received, {"content_type": "application/octet-stream", "body": payload})
            self.assertIsNone(self.real_request(url + "/empty", token="fake-fixture-token", method="POST"))
            with self.assertRaisesRegex(RuntimeError, "npm staged tarball download failed with HTTP 404") as error:
                self.real_request(url, token="fake-fixture-token", context="npm staged tarball download")
            self.assertNotIn("fake-private", str(error.exception))
            self.assertNotIn("fake-fixture-token", str(error.exception))
            with patch.object(actions, "urlopen", side_effect=actions.URLError("fake-private-signed-url")):
                with self.assertRaisesRegex(RuntimeError, "transport failed") as error:
                    self.real_request(url)
            self.assertNotIn("fake-private", str(error.exception))
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_finalization_requires_public_validation_before_any_write(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "verify_public", side_effect=ValueError("not approved")), patch.object(actions, "github") as mutation, patch.object(actions, "command") as commands:
            with self.assertRaisesRegex(ValueError, "not approved"):
                actions.finalize(self.directory, "123", "stage")
            mutation.assert_not_called()
            commands.assert_not_called()
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "verify_public"), patch.object(actions, "request", return_value={"next": self.version}), patch.object(actions, "draft_release", return_value={"id": 1, "draft": True}), patch.object(actions, "github") as mutation, patch.object(actions, "command") as commands, patch.object(actions, "summary"), patch.dict(os.environ, {"NPM_STAGE_TOKEN": "fake"}):
            actions.finalize(self.directory, "123", "stage")
            commands.assert_not_called()
            mutation.assert_called_once_with("releases/1", method="PATCH", payload={"draft": False, "make_latest": "false"})

    def test_authenticated_redirect_cannot_forward_a_token(self):
        with self.assertRaisesRegex(RuntimeError, "redirects"):
            actions.NoCredentialRedirect().redirect_request(None, None, 302, "redirect", {}, "https://foreign.invalid")

    def test_channel_update_must_be_public_before_github_completion(self):
        with self.registry_clock() as sleep, patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "verify_public"), patch.object(actions, "request", return_value={}), patch.object(actions, "command", return_value="") as command, patch.object(actions, "draft_release") as draft, patch.object(actions, "github") as github, patch.object(actions, "summary"), patch.dict(os.environ, {"NPM_STAGE_TOKEN": "fake"}):
            with self.assertRaisesRegex(ValueError, "not yet publicly visible"):
                actions.finalize(self.directory, "123", "stage")
        self.assertEqual(command.call_args.args[0][:3], ["npm", "dist-tag", "add"])
        command.assert_called_once()
        self.assertEqual(sum(call.args[0] for call in sleep.call_args_list), 120)
        draft.assert_not_called()
        github.assert_not_called()

    def test_channel_propagation_waits_before_github_completion_without_rewriting_tags(self):
        updated = set()
        visible = set()
        pending = {name: 2 for name in metadata.PACKAGES.values()}

        def update(argv, **_kwargs):
            self.assertEqual(argv[:3], ["npm", "dist-tag", "add"])
            name = argv[3].split("@")[0]
            self.assertNotIn(name, updated)
            updated.add(name)
            return ""

        def tags(url, **_kwargs):
            name = next(name for name in metadata.PACKAGES.values() if name in url)
            if name not in updated:
                return {"next": "0.0.1"}
            if pending[name]:
                pending[name] -= 1
                return {"next": "0.0.1"}
            visible.add(name)
            return {"next": self.version}

        def complete(*_args, **_kwargs):
            self.assertEqual(visible, set(metadata.PACKAGES.values()))

        with self.registry_clock() as sleep, patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "verify_public"), patch.object(actions, "request", side_effect=tags), patch.object(actions, "command", side_effect=update) as command, patch.object(actions, "draft_release", return_value={"id": 1, "draft": True}), patch.object(actions, "github", side_effect=complete) as github, patch.object(actions, "summary"):
            actions.finalize(self.directory, "123", "oidc")
        self.assertEqual(command.call_count, 2)
        self.assertEqual(sleep.call_count, 4)
        github.assert_called_once_with("releases/1", method="PATCH", payload={"draft": False, "make_latest": "false"})

    def test_check_waits_read_only_and_verifies_the_published_release(self):
        pending = {name: 2 for name in metadata.PACKAGES.values()}

        def lookup(entry):
            if pending[entry["name"]]:
                pending[entry["name"]] -= 1
                return None
            return {"dist": {"integrity": entry["integrity"]}}

        release = {**self.draft(), "draft": False, "_assets": [{"name": p.name} for p in self.directory.iterdir()]}
        with self.registry_clock() as sleep, patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", side_effect=lookup), patch.object(actions, "check_channel", return_value={"next": self.version}), patch.object(actions, "verify_public") as install, patch.object(actions, "find_release", return_value=release), patch.object(actions, "check_tag", return_value=True), patch.object(actions, "github") as mutation, patch.object(actions, "command") as command, patch.object(actions, "summary") as summary:
            actions.check_publication(self.directory, "123")
        self.assertEqual(sleep.call_count, 2)
        install.assert_called_once_with(self.directory)
        mutation.assert_not_called()
        command.assert_not_called()
        self.assertIn("Public verification passed", summary.call_args.args[0])

    def test_check_timeout_never_republishes_or_completes_github(self):
        with self.registry_clock() as sleep, patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "verify_public") as install, patch.object(actions, "find_release") as release, patch.object(actions, "github") as mutation, patch.object(actions, "command") as command, patch.object(actions, "summary"):
            with self.assertRaisesRegex(ValueError, "rerun Check Release only"):
                actions.check_publication(self.directory, "123")
        self.assertEqual(sum(call.args[0] for call in sleep.call_args_list), 120)
        install.assert_not_called()
        release.assert_not_called()
        mutation.assert_not_called()
        command.assert_not_called()

    def public_download(self, url, **_kwargs):
        for entry in self.meta.values():
            if not isinstance(entry, dict):
                continue
            tarball = f"{actions.REGISTRY}/{entry['name']}/-/{entry['tarball']}"
            if url == f"{actions.REGISTRY}/{entry['name']}/{self.version}":
                return {**entry, "dist": {"integrity": entry["integrity"], "tarball": tarball}}
            if url == tarball:
                return (self.directory / entry["tarball"]).read_bytes()
        raise AssertionError("Unexpected public request")

    def test_public_install_retries_etarget_with_fresh_state_and_no_credentials(self):
        installs, commands = [], []
        before = {p.name: p.read_bytes() for p in self.directory.iterdir()}
        credentials = {key: "fake-private-value" for key in ("GH_TOKEN", "GITHUB_TOKEN", "NODE_AUTH_TOKEN", "NPM_TOKEN", "NPM_STAGE_TOKEN", "NPM_ID_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL")}

        def run(argv, **kwargs):
            commands.append(argv)
            env, root = kwargs["env"], Path(kwargs["cwd"])
            self.assertTrue(all(key not in env for key in credentials))
            self.assertEqual(env["HOME"], str(root))
            for key in ("AJR_STATE_DIR", "CODEX_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"):
                self.assertTrue(Path(env[key]).is_relative_to(root))
            if argv[:2] == ["npm", "install"]:
                cache = Path(env["npm_config_cache"])
                for old_root, old_cache in installs:
                    self.assertNotEqual((root, cache), (old_root, old_cache))
                    self.assertFalse(old_root.exists())
                    self.assertFalse(old_cache.exists())
                self.assertIn("--prefer-online", argv)
                self.assertIn("--ignore-scripts", argv)
                self.assertEqual(argv[-2:], [f"{name}@{self.version}" for name in metadata.PACKAGES.values()])
                installs.append((root, cache))
                (root / "partial-install").touch()
                cache.mkdir(parents=True)
                (cache / "stale-metadata").touch()
                if len(installs) == 1:
                    return subprocess.CompletedProcess(argv, 1, "", "npm error code ETARGET\nfake-private-value")
            return subprocess.CompletedProcess(argv, 0, f"agent-job-relay {self.version}\n" if "--version" in argv else "", "")

        with self.registry_clock() as sleep, patch.dict(os.environ, credentials), patch.object(actions, "request", side_effect=self.public_download), patch.object(actions.subprocess, "run", side_effect=run), patch.object(actions, "github") as github, patch.object(actions, "summary"):
            actions.verify_public(self.directory)
        self.assertEqual(len(installs), 2)
        self.assertEqual(len(commands), 4)
        self.assertEqual(sleep.call_count, 1)
        self.assertTrue(all(not path.exists() for pair in installs for path in pair))
        github.assert_not_called()
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.directory.iterdir()})

    def test_public_install_etarget_timeout_never_mutates_publication(self):
        def run(argv, **_kwargs):
            self.assertEqual(argv[:2], ["npm", "install"])
            return subprocess.CompletedProcess(argv, 1, "", "npm error code ETARGET")

        with self.registry_clock() as sleep, patch.object(actions, "request", side_effect=self.public_download), patch.object(actions.subprocess, "run", side_effect=run), patch.object(actions, "github") as github, patch.object(actions, "summary"):
            with self.assertRaisesRegex(ValueError, "clean npm installation.*ETARGET.*rerun Check Release only"):
                actions.verify_public(self.directory)
        self.assertEqual(sum(call.args[0] for call in sleep.call_args_list), 120)
        github.assert_not_called()

    def test_public_install_permanent_errors_stop_immediately_without_logging_output(self):
        for code in ("E401", "E403", "EINTEGRITY", None):
            error = f"npm error code {code}\n" if code else ""
            result = subprocess.CompletedProcess([], 1, "fake-private-stdout", error + "fake-private-stderr")
            with self.subTest(code=code), self.registry_clock() as sleep, patch.object(actions, "request", side_effect=self.public_download), patch.object(actions.subprocess, "run", return_value=result) as run, patch.object(actions, "summary"):
                with self.assertRaises(actions.CommandError) as failure:
                    actions.verify_public(self.directory)
                self.assertEqual(failure.exception.npm_code, code)
                self.assertNotIn("fake-private", str(failure.exception))
                run.assert_called_once()
                sleep.assert_not_called()

    def test_public_native_version_and_import_failures_are_not_retried(self):
        for failed_step in ("version", "import"):
            def run(argv, **_kwargs):
                if "--version" in argv:
                    return subprocess.CompletedProcess(argv, 0, "wrong version" if failed_step == "version" else f"agent-job-relay {self.version}", "")
                if "--input-type=module" in argv:
                    return subprocess.CompletedProcess(argv, 1, "", "npm error code ETARGET")
                return subprocess.CompletedProcess(argv, 0, "", "")

            with self.subTest(step=failed_step), self.registry_clock() as sleep, patch.object(actions, "request", side_effect=self.public_download), patch.object(actions.subprocess, "run", side_effect=run) as commands, patch.object(actions, "summary"):
                with self.assertRaises((ValueError, actions.CommandError)):
                    actions.verify_public(self.directory)
                self.assertEqual(sum(call.args[0][:2] == ["npm", "install"] for call in commands.call_args_list), 1)
                sleep.assert_not_called()

    def test_public_tarball_conflict_stops_before_install_without_retry(self):
        def download(url, **kwargs):
            return b"foreign bytes" if kwargs.get("binary") else self.public_download(url)

        with self.registry_clock() as sleep, patch.object(actions, "request", side_effect=download), patch.object(actions, "command") as command:
            with self.assertRaisesRegex(ValueError, "Public tarball bytes differ"):
                actions.verify_public(self.directory)
            sleep.assert_not_called()
            command.assert_not_called()

    def test_receipt_is_bound_to_candidate_bytes_and_actual_publisher_run(self):
        receipt = {**actions.receipt_fields(self.candidate, "runtime"), "publisher_run_id": "456", "publisher_run_attempt": "1", "publisher_revision": self.revision}
        run = {**self.run, "id": 456, "path": ".github/workflows/release-publish.yml", "run_attempt": 1, "conclusion": "failure"}
        name = "npm-publish-receipt-456-1-runtime.json"
        match = actions.RECEIPT_NAME.fullmatch(name)

        def asset(data):
            return {"name": name, "id": 7, "size": len(data), "digest": "sha256:" + actions.hashlib.sha256(data).hexdigest()}

        data = json.dumps(receipt).encode()
        with patch.object(actions, "command", return_value=data), patch.object(actions, "github", return_value=run):
            self.assertEqual(actions.verify_release_assets(self.directory, [asset(data)], self.candidate), {"runtime"})
        for field, value in [("candidate_run_id", "999"), ("tarball_sha256", "0" * 64), ("channel", "latest"), ("package_name", "foreign")]:
            foreign = json.dumps({**receipt, field: value}).encode()
            with self.subTest(field=field), patch.object(actions, "command", return_value=foreign), patch.object(actions, "github") as github:
                with self.assertRaisesRegex(ValueError, "sealed candidate"):
                    actions.read_receipt(asset(foreign), self.candidate, match)
                github.assert_not_called()
        with patch.object(actions, "command", return_value=data), patch.object(actions, "github", return_value={**run, "path": ".github/workflows/release-all.yml"}):
            with self.assertRaisesRegex(ValueError, "publishing workflow"):
                actions.read_receipt(asset(data), self.candidate, match)

    def test_receipt_writer_keeps_candidate_inventory_unchanged(self):
        before = {p.name: p.read_bytes() for p in self.directory.iterdir()}
        recorded = []

        def upload(release_id, path):
            self.assertEqual(release_id, 1)
            self.assertEqual(path.name, "npm-publish-receipt-456-2-opencode.json")
            recorded.append(json.loads(path.read_text()))

        with patch.dict(os.environ, {"GITHUB_RUN_ID": "456", "GITHUB_RUN_ATTEMPT": "2"}), patch.object(actions, "actions_context", return_value=self.revision), patch.object(actions, "upload_release_asset", side_effect=upload):
            actions.record_publish_acceptance(self.candidate, self.draft(), "opencode")
        self.assertEqual(recorded, [{**actions.receipt_fields(self.candidate, "opencode"), "publisher_run_id": "456", "publisher_run_attempt": "2", "publisher_revision": self.revision}])
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.directory.iterdir()})

    def test_receipt_failure_stops_before_next_upload_and_public_metadata_allows_recovery(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value=None), patch.object(actions, "check_channel", return_value={}), patch.object(actions, "draft_release", return_value=self.draft()), patch.object(actions, "command", return_value="{}") as command, patch.object(actions, "record_publish_acceptance", side_effect=RuntimeError("receipt response lost")), patch.object(actions, "github") as github:
            with self.assertRaisesRegex(RuntimeError, "receipt response lost"):
                actions.publish(self.directory, "123", "oidc")
        command.assert_called_once()
        github.assert_not_called()
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", side_effect=[{}, None]), patch.object(actions, "check_channel", return_value={"next": self.version}), patch.object(actions, "draft_release", return_value=self.draft()), patch.object(actions, "command", return_value="{}") as command, patch.object(actions, "record_publish_acceptance") as receipt, patch.object(actions, "github"), patch.object(actions, "output"), patch.object(actions, "summary"):
            actions.publish(self.directory, "123", "oidc")
        command.assert_called_once()
        self.assertEqual(command.call_args.args[0][2], self.directory / self.meta["opencode"]["tarball"])
        self.assertEqual(receipt.call_args.args[2], "opencode")

    def test_completed_publication_is_idempotent_without_any_remote_write(self):
        with patch.object(actions, "validate", return_value=self.candidate), patch.object(actions, "check_candidate", return_value=self.meta), patch.object(actions, "public_package", return_value={}), patch.object(actions, "check_channel", return_value={"next": self.version}), patch.object(actions, "draft_release", return_value={**self.draft(), "draft": False}), patch.object(actions, "command") as command, patch.object(actions, "record_publish_acceptance") as receipt, patch.object(actions, "github") as github, patch.object(actions, "output"), patch.object(actions, "summary"):
            actions.publish(self.directory, "123", "oidc")
        command.assert_not_called()
        receipt.assert_not_called()
        github.assert_not_called()

    def child(self, step="publish", **overrides):
        filename, name = pipeline.WORKFLOWS[step]
        return {**self.run, "id": 456, "path": ".github/workflows/" + filename, "display_title": f"{name} [release-789-{step}]", "run_attempt": 1, **overrides}

    def test_pipeline_lost_dispatch_response_recovers_exact_child_without_second_post(self):
        posts = []
        created = False
        child = self.child()

        def api(path, **kwargs):
            nonlocal created
            if path == "git/ref/heads/main":
                return {"object": {"sha": self.revision}}
            if "/dispatches" in path:
                posts.append((path, kwargs["payload"]))
                created = True
                raise RuntimeError("dispatch response lost")
            if "/runs?" in path:
                return {"workflow_runs": [child] if created else []}
            self.assertEqual(path, "actions/runs/456")
            return child

        with patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api), patch.object(pipeline, "output"), patch.object(pipeline, "summary"):
            self.assertEqual(pipeline.dispatch_and_wait("publish", {"candidate_run_id": "123", "dry_run": "false"}, self.revision), "456")
        self.assertEqual(len(posts), 1)
        self.assertEqual(posts[0][0], "actions/workflows/release-publish.yml/dispatches")
        self.assertEqual(posts[0][1]["inputs"]["expected_revision"], self.revision)
        self.assertEqual(posts[0][1]["inputs"]["pipeline_key"], "release-789-publish")

    def test_pipeline_initial_metadata_settles_without_dispatching_twice(self):
        posts = []
        snapshots = iter([
            self.child("preview", display_title="Publish Release", status="queued", conclusion=None),
            self.child("preview", head_repository=None, status="queued", conclusion=None),
            self.child("preview", status="in_progress", conclusion=None),
            self.child("preview"),
        ])

        def api(path, **kwargs):
            if path == "git/ref/heads/main":
                return {"object": {"sha": self.revision}}
            if "/runs?" in path:
                return {"workflow_runs": []}
            if kwargs.get("method") == "POST":
                posts.append(path)
                return {"workflow_run_id": 456}
            self.assertEqual(path, "actions/runs/456")
            return next(snapshots)

        with self.registry_clock() as sleep, patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api), patch.object(pipeline, "output"), patch.object(pipeline, "summary"):
            self.assertEqual(pipeline.dispatch_and_wait("preview", {"candidate_run_id": "123", "dry_run": "true"}, self.revision), "456")
        self.assertEqual(posts, ["actions/workflows/release-publish.yml/dispatches"])
        self.assertGreater(sum(call.args[0] for call in sleep.call_args_list), 0)

    def test_pipeline_initial_metadata_wait_is_bounded_without_redispatch(self):
        posts = []

        def api(path, **kwargs):
            if path == "git/ref/heads/main":
                return {"object": {"sha": self.revision}}
            if "/runs?" in path:
                return {"workflow_runs": []}
            if kwargs.get("method") == "POST":
                posts.append(path)
                return {"workflow_run_id": 456}
            return self.child("preview", display_title="Publish Release", status="queued", conclusion=None)

        with self.registry_clock() as sleep, patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api), patch.object(pipeline, "output"), patch.object(pipeline, "summary"):
            with self.assertRaisesRegex(ValueError, "did not initialize"):
                pipeline.dispatch_and_wait("preview", {"candidate_run_id": "123", "dry_run": "true"}, self.revision)
        self.assertEqual(posts, ["actions/workflows/release-publish.yml/dispatches"])
        self.assertEqual(sum(call.args[0] for call in sleep.call_args_list), 60)

    def test_pipeline_populated_child_identity_mismatch_is_never_treated_as_initialization(self):
        for field, value in [("head_sha", "b" * 40), ("head_repository", {"full_name": "foreign/repo"}), ("display_title", "Publish Release [foreign-key]"), ("id", 457)]:
            posts = []

            def api(path, **kwargs):
                if path == "git/ref/heads/main":
                    return {"object": {"sha": self.revision}}
                if "/runs?" in path:
                    return {"workflow_runs": []}
                if kwargs.get("method") == "POST":
                    posts.append(path)
                    return {"workflow_run_id": 456}
                return self.child("preview", status="queued", conclusion=None, **{field: value})

            with self.subTest(field=field), self.registry_clock() as sleep, patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api), patch.object(pipeline, "output"), patch.object(pipeline, "summary"):
                with self.assertRaisesRegex(ValueError, field):
                    pipeline.dispatch_and_wait("preview", {"candidate_run_id": "123", "dry_run": "true"}, self.revision)
            sleep.assert_not_called()
            self.assertEqual(posts, ["actions/workflows/release-publish.yml/dispatches"])

    def test_pipeline_retries_failed_check_only_using_its_original_run_id(self):
        failed = self.child("check", conclusion="failure")
        runs = iter([failed, self.child("check", status="in_progress", conclusion=None, run_attempt=2), self.child("check", run_attempt=2)])
        posts = []

        def api(path, **kwargs):
            if path == "git/ref/heads/main":
                return {"object": {"sha": self.revision}}
            if "/runs?" in path:
                return {"workflow_runs": [failed]}
            if kwargs.get("method") == "POST":
                posts.append(path)
                return None
            self.assertEqual(path, "actions/runs/456")
            return next(runs)

        with self.registry_clock(), patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api), patch.object(pipeline, "output"), patch.object(pipeline, "summary"):
            self.assertEqual(pipeline.dispatch_and_wait("check", {"candidate_run_id": "123"}, self.revision), "456")
        self.assertEqual(posts, ["actions/runs/456/rerun-failed-jobs"])

    def test_pipeline_reuses_selected_candidate_without_preparing_again(self):
        with patch.dict(os.environ, {"GITHUB_WORKFLOW_REF": f"{actions.REPOSITORY}/.github/workflows/release-all.yml@refs/heads/main"}), patch.object(pipeline, "actions_context", return_value=self.revision), patch.object(pipeline, "candidate_run") as candidate, patch.object(pipeline, "dispatch_and_wait") as dispatch, patch.object(pipeline, "output") as output, patch.object(pipeline, "summary"):
            self.assertEqual(pipeline.run_step("prepare", "123", "latest", True), "123")
        candidate.assert_called_once_with("123")
        dispatch.assert_not_called()
        output.assert_called_once_with("candidate_run_id", "123")

    def test_pipeline_source_change_and_ambiguous_children_stop_without_dispatch(self):
        for changed in (False, True):
            writes = []

            def api(path, **kwargs):
                if kwargs.get("method") == "POST":
                    writes.append(path)
                if path == "git/ref/heads/main":
                    return {"object": {"sha": "b" * 40 if changed else self.revision}}
                return {"workflow_runs": [self.child(), self.child(id=457)]}

            with self.subTest(main_changed=changed), patch.dict(os.environ, {"GITHUB_RUN_ID": "789"}), patch.object(pipeline, "github", side_effect=api):
                with self.assertRaises(ValueError):
                    pipeline.dispatch_and_wait("publish", {"candidate_run_id": "123"}, self.revision)
            self.assertEqual(writes, [])

    def test_pipeline_child_ref_race_is_rejected_by_the_child_context(self):
        env = {"GITHUB_ACTIONS": "true", "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REPOSITORY": actions.REPOSITORY, "GITHUB_REF": "refs/heads/main", "GITHUB_SHA": "b" * 40, "EXPECTED_RELEASE_REVISION": self.revision}
        with patch.dict(os.environ, env), patch.object(actions, "request") as request:
            with self.assertRaisesRegex(ValueError, "main changed"):
                actions.actions_context()
        request.assert_not_called()

    def test_pipeline_preview_dispatches_only_the_actual_publisher_dry_run(self):
        env = {"GITHUB_WORKFLOW_REF": f"{actions.REPOSITORY}/.github/workflows/release-all.yml@refs/heads/main"}
        with patch.dict(os.environ, env), patch.object(pipeline, "actions_context", return_value=self.revision), patch.object(pipeline, "candidate_run") as candidate, patch.object(pipeline, "dispatch_and_wait", return_value="456") as dispatch:
            self.assertEqual(pipeline.run_step("preview", "123", "next", True), "456")
        candidate.assert_called_once_with("123")
        dispatch.assert_called_once_with("preview", {"candidate_run_id": "123", "recover_candidate": "true", "dry_run": "true"}, self.revision)

    def test_prepare_refuses_a_partially_published_version_before_any_build(self):
        with patch.object(actions, "actions_context", return_value=self.revision), patch.object(actions, "github", return_value={"full_name": actions.REPOSITORY}), patch.object(actions, "check_tag"), patch.object(actions, "github_list", return_value=[]), patch.object(actions, "registry_version", return_value={}), patch.object(actions, "command") as command:
            with self.assertRaisesRegex(ValueError, "original candidate_run_id"):
                actions.prepare()
        command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
