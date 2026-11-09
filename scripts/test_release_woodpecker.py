"""Preparation fences and immutable seals; no real publication or credentials."""
import copy
import hashlib
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch
import release_metadata as metadata
import release_woodpecker as release
import test_release as legacy


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = legacy.ReleaseTests(); self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root; self.directory = self.fixture.directory
        for name in ['candidate.json', 'release-notes.md']: (self.directory / name).unlink()
        self.meta = self.fixture.meta; self.meta['minimumGlibc'] = '2.34'
        self.save_meta()
        self.context = {'format': 1, 'run': {'repository': release.REPOSITORY, 'repo_id': '1', 'pipeline': '42',
                         'created': '1700000000', 'commit': 'a' * 40, 'server': 'https://ci.example.test'},
                        'branch': 'main', 'task': 'prepare', 'channel': 'next'}
        self.env = {'CI': 'woodpecker', 'CI_PIPELINE_EVENT': 'manual', 'CI_REPO': release.REPOSITORY,
                    'CI_REPO_ID': '1', 'CI_PIPELINE_NUMBER': '42', 'CI_PIPELINE_CREATED': '1700000000',
                    'CI_COMMIT_SHA': 'a' * 40, 'EXPECTED_RELEASE_REVISION': 'a' * 40,
                    'CI_COMMIT_REF': 'refs/heads/main', 'CI_COMMIT_BRANCH': 'main',
                    'CI_PIPELINE_URL': 'https://ci.example.test/repos/1/pipeline/42',
                    'AJR_RELEASE_TASK': 'prepare', 'AJR_RELEASE_CHANNEL': 'next'}
        self.record = self.preflight()
        self.native = {'linux': {'minimumGlibc': '2.34'}, 'windows': {'revision': 'a' * 40}}
        self.report = json.dumps({'result': 'passed', 'platform': 'native Windows', 'npm_global_short_cli': True,
                       'installed_runtime': {'version': self.meta['version'], 'history_preserved': True,
                                             'corruption_refused': True, 'bundled_ui': True},
                       'codex': {'actual_catalog': True, 'tools': 27}, 'opencode': {'model': 'simulated'}}).encode()
        self.evidence = {'format': 1, 'execution': self.context, 'packages_sha256': 'b' * 64,
                         'report_sha256': hashlib.sha256(self.report).hexdigest()}
        self.destination = self.directory.parent / 'woodpecker-candidate'
        self.pipeline = {'status': 'success', 'event': 'manual', 'number': 42, 'created': 1700000000,
                         'repo_id': 1, 'commit': 'a' * 40, 'branch': 'main'}
        self.workflows = [{'name': name, 'state': 'success'} for name in release.WORKFLOWS]

    def save_meta(self):
        (self.directory / 'release.json').write_text(json.dumps(self.meta)); metadata.write_checksums(self.directory)

    def preflight(self):
        with patch.object(release, 'unused_version'):
            return release.preflight(self.context, root=self.root)

    def seal(self, **override):
        args = dict(directory=self.directory, destination=self.destination, record=self.record, context=self.context,
                    native=self.native, report=self.report, evidence=self.evidence, package_digest='b' * 64, root=self.root)
        return release.seal(**{**args, **override})

    def test_manual_context_rejects_automatic_events_wrong_source_and_implicit_policy(self):
        with patch.object(release, 'command', side_effect=['a' * 40, '']):
            self.assertEqual(release.execution(self.env, self.root), self.context)
        for change in [{'CI_PIPELINE_EVENT': e} for e in ['push', 'tag', 'pull_request', 'cron']] + [
                {'CI': 'true'}, {'EXPECTED_RELEASE_REVISION': ''}, {'EXPECTED_RELEASE_REVISION': 'b' * 40},
                {'AJR_RELEASE_TASK': ''}, {'AJR_RELEASE_CHANNEL': ''}, {'CI_REPO_ID': '0'},
                {'CI_REPO': 'another/repo'}, {'CI_COMMIT_BRANCH': 'feature', 'CI_COMMIT_REF': 'refs/heads/feature'},
                {'CI_PIPELINE_URL': 'https://secret@ci.example.test/repos/1/pipeline/42'},
                {'CI_PIPELINE_URL': 'https://ci.example.test/repos/1/pipeline/43'}]:
            with self.subTest(change=change), self.assertRaises((ValueError, RuntimeError)), patch.object(release, 'command') as commands:
                release.execution({**self.env, **change}, self.root)
                commands.assert_not_called()
        for replies in [['b' * 40], ['a' * 40, ' M Cargo.toml']]:
            with patch.object(release, 'command', side_effect=replies), self.assertRaises((ValueError, RuntimeError)):
                release.execution(self.env, self.root)

    def test_source_preflight_refuses_main_drift_tags_drafts_and_partial_publication(self):
        source = metadata.source_metadata(self.root)
        def responses(url, **kwargs):
            if url.endswith('/heads/main'): return {'object': {'sha': 'a' * 40}}
            if '/tags/' in url: return None
            if '/releases?' in url: return []
            return None
        with patch.object(release, 'request', side_effect=responses):
            release.unused_version(source, self.context, 'fixture-read-only')
        for suffix, value in [('/heads/main', {'object': {'sha': 'c' * 40}}),
                              ('/tags/', {'object': {'sha': 'a' * 40}}),
                              ('/releases?', [{'tag_name': source['tag'], 'draft': True}]),
                              ('registry.npmjs.org', {'version': source['version']})]:
            def altered(url, **kwargs): return value if suffix in url else responses(url, **kwargs)
            with self.subTest(suffix=suffix), patch.object(release, 'request', side_effect=altered), self.assertRaises((ValueError, RuntimeError)):
                release.unused_version(source, self.context, 'fixture-read-only')
        with self.assertRaises((ValueError, RuntimeError)), patch.object(release, 'request') as network:
            release.unused_version(source, self.context, '')
        network.assert_not_called()

    def test_seal_acceptance_requires_the_exact_successful_manual_graph(self):
        candidate = self.seal()
        self.assertEqual(candidate['format'], 2)
        self.assertEqual(release.verify_prepared(self.destination, self.pipeline, self.workflows, self.root, server='https://ci.example.test'), candidate)
        for change in [{'status': 'running'}, {'status': 'failure'}, {'event': 'push'}, {'number': 43},
                       {'repo_id': 2}, {'created': 1700000001}, {'commit': 'b' * 40}, {'branch': 'feature'}]:
            with self.subTest(change=change), self.assertRaises((ValueError, RuntimeError)):
                release.verify_prepared(self.destination, {**self.pipeline, **change}, self.workflows, self.root, server='https://ci.example.test')
        for workflows in [self.workflows[:-1], [{**w, 'state': 'pending'} for w in self.workflows]]:
            with self.assertRaises((ValueError, RuntimeError)): release.verify_prepared(self.destination, self.pipeline, workflows, self.root, server='https://ci.example.test')
        with self.assertRaises((ValueError, RuntimeError)):
            metadata.validate_promotion(self.destination, self.fixture.run, release.REPOSITORY, 'a' * 40, '123', self.root)

    def test_report_fences_the_consumed_package_bytes_and_never_partially_seals(self):
        before = {p.name: p.read_bytes() for p in self.directory.iterdir()}
        for override in [{'package_digest': 'c' * 64}, {'report': b'{"result":"failed"}'},
                         {'record': {**self.record, 'version': '99.0.0'}},
                         {'context': {**self.context, 'channel': 'latest'}}]:
            with self.subTest(override=override), self.assertRaises((ValueError, RuntimeError)): self.seal(**override)
            self.assertFalse(self.destination.exists())
            self.assertEqual({p.name: p.read_bytes() for p in self.directory.iterdir()}, before)
        with patch.object(release.os, 'rename', side_effect=OSError('injected interrupted publication')), self.assertRaises(OSError): self.seal()
        self.assertFalse(self.destination.exists())
        self.assertFalse(list(self.directory.parent.glob('release-seal-*')))
        self.seal()
        with self.assertRaises((ValueError, RuntimeError)): self.seal()

    def test_development_rehearsal_stays_development_and_cannot_be_promoted(self):
        self.meta['development'] = True; self.save_meta()
        with self.assertRaises((ValueError, RuntimeError)): self.seal()
        self.context['task'] = 'rehearse'; self.context['branch'] = 'ci/prepare'
        self.record = self.preflight(); self.evidence['execution'] = self.context
        candidate = self.seal()
        self.assertTrue(candidate['development'])
        with self.assertRaises((ValueError, RuntimeError)): metadata.check_candidate(self.destination, self.root)
        release.verify_prepared(self.destination, {**self.pipeline, 'branch': 'ci/prepare'}, self.workflows, self.root, server='https://ci.example.test')
        candidate['development'] = False
        (self.destination / 'candidate.json').write_text(json.dumps(candidate)); metadata.write_checksums(self.destination)
        with self.assertRaises((ValueError, RuntimeError)): release.verify_prepared(self.destination, {**self.pipeline, 'branch': 'ci/prepare'}, self.workflows, self.root, server='https://ci.example.test')

    def test_rehashing_tampered_assets_does_not_rewrite_the_original_seal(self):
        self.seal(); (self.destination / 'release-notes.md').write_text('changed notes')
        metadata.write_checksums(self.destination)
        with self.assertRaises((ValueError, RuntimeError)): release.verify_prepared(self.destination, self.pipeline, self.workflows, self.root, server='https://ci.example.test')

    def test_linux_boundary_requires_real_matching_compatibility_evidence(self):
        binary = self.directory.parent / 'linux'
        image = bytearray(80); image[:6] = b'\x7fELF\x02\x01'; image[18:20] = (62).to_bytes(2, 'little'); binary.write_bytes(image)
        compatibility = {'glibc': '2.34', 'sha256': hashlib.sha256(image).hexdigest(), 'result': 'passed'}
        version = self.meta['version']
        with patch.object(release, 'command', side_effect=['agent-job-relay ' + version, 'GLIBC_2.17 GLIBC_2.34']):
            self.assertEqual(release.linux_descriptor(binary, compatibility, self.context, version)['minimumGlibc'], '2.34')
        for symbols, proof in [('GLIBC_2.38', compatibility), ('GLIBC_2.34', {**compatibility, 'sha256': '0' * 64}),
                               ('GLIBC_2.34', {**compatibility, 'result': 'failed'})]:
            with patch.object(release, 'command', side_effect=['agent-job-relay ' + version, symbols]), self.assertRaises((ValueError, RuntimeError)):
                release.linux_descriptor(binary, proof, self.context, version)


if __name__ == '__main__': unittest.main()
