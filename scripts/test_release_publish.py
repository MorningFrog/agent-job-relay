"""Publication boundary coverage; no real account mutations or extra native builds."""
import copy
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch
from urllib.request import Request
import release_publish as publisher
import release_actions as shared
import test_release_woodpecker as preparation


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.prep=preparation.PreparationTests();self.prep.setUp();self.addCleanup(self.prep.doCleanups)
        self.candidate=self.prep.seal();self.directory=self.prep.destination;self.source=self.prep.root
        self.context={'provider':'woodpecker','run':{**self.candidate['execution']['run'],'pipeline':'43','created':'1700000001'},'branch':'main','task':'preview'}
        self.pub=publisher.Publication(self.directory,self.candidate,self.source,self.context)

    def test_publication_refuses_rehearsals_automatic_context_and_changed_runtime_source(self):
        dev=copy.deepcopy(self.candidate);dev['development']=True
        with self.assertRaises(ValueError),patch.object(publisher,'identity') as identity:
            publisher.eligible(dev,self.context)
        identity.assert_not_called()
        env={**self.prep.env,'AJR_RELEASE_TASK':'preview','CI_PIPELINE_NUMBER':'43',
             'CI_PIPELINE_CREATED':'1700000001','CI_PIPELINE_URL':'https://ci.example.test/repos/1/pipeline/43'}
        with patch.object(publisher,'git',side_effect=['a'*40,'']):self.assertEqual(publisher.context(env),self.context)
        for change in [{'CI_PIPELINE_EVENT':'push'},{'CI_COMMIT_REF':'refs/pull/1/head'},{'EXPECTED_RELEASE_REVISION':''}]:
            with self.subTest(change=change),self.assertRaises(ValueError):publisher.context({**env,**change})
        altered={**self.context,'run':{**self.context['run'],'commit':'b'*40}}
        with patch.object(publisher,'git',side_effect=['',b'src/runner.rs\0']),self.assertRaises(ValueError):publisher.eligible(self.candidate,altered)

    def test_preview_is_read_only_and_stage_never_creates_a_release_or_approves_npm(self):
        with patch.object(publisher,'identity',return_value={'npm_user':'fixture'}) as identity,\
             patch.object(shared,'public_package',return_value=None),patch.object(shared,'check_channel',return_value={}),\
             patch.object(self.pub,'find_release',return_value=None),patch.object(shared,'command',return_value='{}') as commands,\
             patch.dict(os.environ,{'NPM_STAGE_TOKEN':'fixture-only'}):
            result=self.pub.preview('fixture');self.assertFalse(result['registry_or_github_writes'])
            self.assertEqual(len(commands.call_args_list),2)
            for call in commands.call_args_list:
                self.assertEqual(call.args[0][:3],['npm','stage','publish']);self.assertIn('--dry-run',call.args[0])
                self.assertIn('--ignore-scripts',call.args[0]);self.assertIn(str(self.directory),str(call.args[0][3]))
        self.context['task']='stage'
        with patch.object(publisher,'identity'),patch.object(shared,'public_package',return_value=None),\
             patch.object(shared,'check_channel',return_value={}),patch.object(self.pub,'find_release',return_value=None),\
             patch.object(shared,'stage_package',side_effect=[False,True]) as stages,patch.object(shared,'github') as github,\
             patch.dict(os.environ,{'NPM_STAGE_TOKEN':'fixture-only'}):
            result=self.pub.stage('fixture');self.assertEqual(result['status'],'awaiting_npm_validation')
            self.assertEqual(stages.call_count,2);github.assert_not_called()
        # Check every package before a first write, including partial/conflicting publication.
        with patch.object(publisher,'identity'),patch.object(shared,'public_package',side_effect=[{},ValueError('conflicting bytes')]),\
             patch.object(shared,'stage_package') as stages,self.assertRaises(ValueError):self.pub.stage('fixture')
        stages.assert_not_called()

    def test_finalization_requires_public_original_bytes_and_never_replaces_existing_assets(self):
        self.context['task']='finalize'
        with patch.object(publisher,'public_bytes',side_effect=ValueError('approval pending')),patch.object(shared,'github') as github,self.assertRaises(ValueError):self.pub.finalize()
        github.assert_not_called()
        assets=[{'name':name,'digest':'sha256:'+digest} for name,digest in self.pub.assets.items()]
        release={'id':77,'draft':False,'tag_name':'v'+self.pub.meta['version'],'target_commitish':self.pub.run['commit'],
                 'body':self.pub.body,'prerelease':True,'_assets':assets}
        with patch.object(publisher,'public_bytes'),patch.object(self.pub,'find_release',return_value=release),\
             patch.object(self.pub,'upload_asset') as upload,patch.object(shared,'github') as github:
            self.pub.finalize();upload.assert_not_called();github.assert_not_called()
        foreign={**release,'body':'another candidate'}
        with patch.object(shared,'check_tag',return_value=True),patch.object(shared,'github_list',return_value=[foreign]),self.assertRaises(ValueError):self.pub.find_release()

    def test_independent_check_downloads_public_bytes_without_stage_credentials_or_mutations(self):
        self.context['task']='check'
        assets=[{'name':name,'digest':'sha256:'+digest} for name,digest in self.pub.assets.items()]
        with patch.object(publisher,'public_bytes'),patch.object(self.pub,'find_release',return_value={'id':77,'draft':False,'_assets':assets}),\
             patch.object(shared,'check_tag',return_value=True),\
             patch.object(publisher,'asset_bytes',side_effect=lambda a:(self.directory/a['name']).read_bytes()),\
             patch.object(shared,'verify_public') as install,patch.object(shared,'github') as github:
            result=self.pub.check();self.assertFalse(result['writes']);github.assert_not_called()
            install.assert_called_once_with(self.directory,source_root=self.source)
        redirect=publisher.AssetRedirect().redirect_request(Request('https://api.github.com/asset',headers={'Authorization':'Bearer private-fixture'}),None,302,'',{},'https://release-assets.githubusercontent.com/file?signature=fixture')
        self.assertFalse(redirect.has_header('Authorization'))


if __name__=='__main__':unittest.main()
