"""Behavioral release tests: no real accounts, registry writes or application state."""
from contextlib import contextmanager
import io
import json
import os
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
import release_actions as actions
import release_metadata as metadata


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
        self.network = patch.object(actions, "request", side_effect=AssertionError("Unexpected real network operation"))
        self.network.start()
        self.addCleanup(self.network.stop)

    def validate(self, run=None):
        return metadata.validate_promotion(self.directory, run or self.run, actions.REPOSITORY, self.revision, "123", self.root)

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

    def test_matching_stage_is_reused_only_after_byte_and_tag_check(self):
        entry = self.meta["runtime"]
        tarball = self.directory / entry["tarball"]
        stage = {"id": "11111111-1111-4111-8111-111111111111", "packageName": entry["name"], "version": self.version, "tag": "next"}
        with patch.object(actions, "command", return_value=json.dumps([stage])) as commands, patch.object(actions, "request", return_value=tarball.read_bytes()), patch.object(actions, "summary"):
            actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
            self.assertEqual(commands.call_count, 1)
        with patch.object(actions, "command", return_value=json.dumps([{**stage, "tag": "latest"}])):
            with self.assertRaisesRegex(ValueError, "another channel"):
                actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
        with patch.object(actions, "command", return_value=json.dumps([stage])), patch.object(actions, "request", return_value=b"foreign"):
            with self.assertRaisesRegex(ValueError, "differs"):
                actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})

    def test_new_stage_is_created_once_then_downloaded_for_verification(self):
        entry = self.meta["runtime"]
        tarball = self.directory / entry["tarball"]
        stage_id = "11111111-1111-4111-8111-111111111111"
        with patch.object(actions, "command", side_effect=["[]", json.dumps({entry["name"]: {"stageId": stage_id}})]) as commands, patch.object(actions, "request", return_value=tarball.read_bytes()) as download, patch.object(actions, "summary"):
            actions.stage_package(entry, tarball, "next", {"NODE_AUTH_TOKEN": "fake"})
        argv = commands.call_args_list[1].args[0]
        self.assertEqual(argv[:3], ["npm", "stage", "publish"])
        self.assertIn("--tag=next", argv)
        self.assertTrue(download.call_args.args[0].endswith(f"/{stage_id}/tarball"))

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
        with patch.object(actions, "check_tag"), patch.object(actions, "github", return_value={"body": "foreign", "target_commitish": self.revision, "prerelease": True}), patch.object(actions, "command") as mutation:
            with self.assertRaisesRegex(ValueError, "foreign/modified"):
                actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()
        body = (self.directory / "release-notes.md").read_text() + f"\n<!-- AgentJobRelay candidate 123 {self.revision} next -->"
        release = {"body": body, "target_commitish": self.revision, "prerelease": True, "draft": True, "id": 1}
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[release, [{"name": "foreign.txt"}]]), patch.object(actions, "command") as mutation:
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
        body = (self.directory / "release-notes.md").read_text() + f"\n<!-- AgentJobRelay candidate 123 {self.revision} next -->"
        release = {"body": body, "target_commitish": self.revision, "prerelease": True, "draft": True, "id": 1}
        assets = [{"name": p.name, "digest": "sha256:" + metadata.sha256(p)} for p in self.directory.iterdir()]
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[release, assets]), patch.object(actions, "command") as mutation:
            actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()
        assets[0]["digest"] = "sha256:" + "0" * 64
        with patch.object(actions, "check_tag"), patch.object(actions, "github", side_effect=[release, assets]), patch.object(actions, "command") as mutation:
            with self.assertRaisesRegex(ValueError, "differs"):
                actions.draft_release(self.directory, self.candidate)
            mutation.assert_not_called()

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


if __name__ == "__main__":
    unittest.main()
