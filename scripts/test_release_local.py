"""Local publication boundaries/recovery; no account mutations or platform builds."""
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, patch
import release_local as local
import release_publish as publisher
import release_actions as shared
import test_release_woodpecker as preparation


class LocalPublicationTests(unittest.TestCase):
    def setUp(self):
        self.prep = preparation.PreparationTests();self.prep.setUp();self.addCleanup(self.prep.doCleanups)
        self.candidate = self.prep.seal(); self.work = self.prep.directory.parent/'local-operation';self.work.mkdir()
        shutil.copytree(self.prep.destination,self.work/'candidate')
        self.ctx = {'provider':'local','repository':publisher.REPOSITORY,'workflow_commit':'a'*40,
                    'task':'finalize','invocation':'b'*32,'created':1700000001,'branch':'main','github_user':'fixture'}
        self.args = SimpleNamespace(operation='stage',pipeline='42',expected_github_user='fixture',expected_npm_user='fixture',npm_userconfig=self.work/'npmrc')
        self.args.npm_userconfig.write_text('registry=https://registry.npmjs.org\n');self.args.npm_userconfig.chmod(0o600)
        self.client = MagicMock();self.client.identity.return_value='fixture';self.client.request.return_value={'sha':'a'*40}
        self.client.listing.return_value=[];self.client.check_tag.return_value=False

    def test_local_admission_rejects_ci_and_rehearsal_before_using_personal_login(self):
        with patch.object(publisher,'git',return_value='a'*40) as git,self.assertRaises(ValueError):
            local.local_context('stage','a'*40,env={'CI':'woodpecker'})
        git.assert_not_called()
        gh = local.GitHubCLI()
        gh.before_write = lambda: (_ for _ in ()).throw(ValueError('source changed'))
        with patch.object(local,'cli') as command,self.assertRaises(ValueError):
            gh.request('releases',method='POST',payload={'draft':True})
        command.assert_not_called()
        dev=copy.deepcopy(self.candidate);dev['development']=True
        with self.assertRaises(ValueError):
            local.operate(self.args,self.work,dev,self.prep.root,self.ctx,client=self.client)
        self.client.identity.assert_not_called()
        self.client.request.return_value={'sha':'c'*40}
        with patch.object(local,'npm_identity') as npm,self.assertRaises(ValueError):
            local.operate(self.args,self.work,self.candidate,self.prep.root,self.ctx,client=self.client)
        npm.assert_not_called()
        with patch.dict(os.environ,{'GH_TOKEN':'fake-github','NPM_STAGE_TOKEN':'fake-stage','npm_config_registry':'https://invalid.example'},clear=False),local.npm_environment(self.args.npm_userconfig,self.work) as env:
            self.assertNotIn('GH_TOKEN',env);self.assertNotIn('NPM_STAGE_TOKEN',env)
            self.assertNotIn('npm_config_registry',env)
            self.assertEqual(env['npm_config_userconfig'],str(self.args.npm_userconfig))
            self.assertNotIn('NODE_AUTH_TOKEN',env)

    def test_local_staging_resumes_exact_bytes_via_cli_and_never_approves_or_creates_release(self):
        stages={}; metadata=publisher.metadata.check_candidate(self.work/'candidate',self.prep.root)
        for index,key in enumerate(local.PACKAGES):
            entry=metadata[key];stage_id=f'00000000-0000-4000-8000-{index:012d}'
            stages[stage_id]=(entry,self.work/'candidate'/entry['tarball'])
        calls=[]
        def command(argv,*,env,cwd,**kwargs):
            argv=list(map(str,argv));calls.append(argv)
            self.assertEqual(env['npm_config_userconfig'],str(self.args.npm_userconfig))
            self.assertNotIn('GH_TOKEN',env);self.assertNotIn('NPM_STAGE_TOKEN',env)
            operation=argv[2]
            if operation=='list':
                return json.dumps([{'id':s,'packageName':e['name'],'version':e['version'],'tag':'next'} for s,(e,p) in stages.items() if e['name']==argv[3]]).encode()
            entry,path=stages[argv[3]]
            if operation=='view':
                return json.dumps({'id':argv[3],'packageName':entry['name'],'version':entry['version'],'tag':'next',
                                   'shasum':hashlib.sha1(path.read_bytes()).hexdigest(),'status':'staged'}).encode()
            self.assertEqual(operation,'download');(Path(cwd)/'staged.tgz').write_bytes(path.read_bytes());return b''
        with patch.object(shared,'public_package',return_value=None),patch.object(shared,'check_channel',return_value={}),\
             patch.object(local,'npm_identity',return_value='fixture'),patch.object(local,'cli',side_effect=command):
            self.args.operation='stage';self.ctx['task']='stage'
            result=local.operate(self.args,self.work,self.candidate,self.prep.root,self.ctx,client=self.client)
        self.assertEqual(result['status'],'awaiting_maintainer_2fa')
        self.assertEqual([v[2] for v in calls],['list','view','download']*2)
        self.assertFalse(any(c.kwargs.get('method') in ('POST','PATCH') for c in self.client.request.call_args_list))
        self.client.upload.assert_not_called()
        # The download callback still passes through the existing SHA-256 boundary.
        with patch.object(shared,'command'):
            sample=next(iter(stages.values()))
            def fake(argv,**kwargs):
                if argv[2]=='list':return json.dumps([{'id':next(iter(stages)),'packageName':sample[0]['name'],'version':sample[0]['version'],'tag':'next'}])
                return json.dumps({'id':next(iter(stages)),'packageName':sample[0]['name'],'version':sample[0]['version'],'tag':'next','shasum':hashlib.sha1(sample[1].read_bytes()).hexdigest(),'status':'staged'})
            with self.assertRaises(ValueError):shared.stage_package(sample[0],sample[1],'next',{},run=fake,download=lambda *a:b'wrong bytes',report=lambda *a:None)

    def test_local_finalization_requires_public_bytes_and_preserves_honest_receipts_on_retry(self):
        publication=publisher.Publication(self.work/'candidate',self.candidate,self.prep.root,self.ctx,client=self.client)
        with patch.object(publisher,'public_bytes',side_effect=ValueError('approval pending')),self.assertRaises(ValueError):publication.finalize()
        self.client.request.assert_not_called()
        stored={};body=publication.body
        release={'id':77,'draft':True,'target_commitish':'a'*40,'tag_name':'v'+publication.meta['version'],'body':body,'prerelease':True}
        self.client.request.return_value=release
        def upload(identity,path):
            data=path.read_bytes();asset={'name':path.name,'id':len(stored)+1,'size':len(data),'digest':'sha256:'+hashlib.sha256(data).hexdigest()};stored[path.name]=(asset,data);return asset
        self.client.upload.side_effect=upload
        self.client.asset_bytes.side_effect=lambda asset:stored[asset['name']][1]
        with patch.object(publisher,'public_bytes'):
            publication.finalize()
        receipt_name='local-publish-receipt-'+'b'*32+'-1700000001.json'
        receipt=json.loads(stored[receipt_name][1]);self.assertEqual(receipt['publisher'],self.ctx)
        self.assertEqual(receipt['candidate'],self.candidate['execution']);self.assertNotIn('run',receipt['publisher'])
        self.client.upload.reset_mock();self.client.request.reset_mock()
        self.client.listing.side_effect=lambda path:[{**release,'draft':False}] if path=='releases' else [a for a,data in stored.values()]
        with patch.object(publisher,'public_bytes'):
            publication.finalize()
        self.client.upload.assert_not_called();self.client.request.assert_not_called()


if __name__=='__main__':unittest.main()
