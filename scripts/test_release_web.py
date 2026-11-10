"""CI progress stays bound to its original Web Release and cannot mutate anything."""
import unittest
import release_web as web


class WebProgressTests(unittest.TestCase):
    def setUp(self):
        self.env={'CI':'woodpecker','CI_PIPELINE_EVENT':'manual','CI_COMMIT_BRANCH':'main',
            'CI_REPO_ID':'1','CI_PIPELINE_NUMBER':'46','CI_COMMIT_SHA':'a'*40,
            'RELEASE_CONTROL_ENDPOINT':'https://coordinator.example.invalid'}
        self.result={'repository_id':1,'pipeline':46,'commit':'a'*40,'phase':'dry_run_complete',
            'complete':True,'failed':False,'candidate':42,'dry_run':True}
    def test_success_requires_exact_run_and_truthful_complete_phase(self):
        messages=[]
        result=web.observe(self.env,fetch=lambda:self.result,emit=lambda *a,**kw:messages.append(a))
        self.assertTrue(result['complete']);self.assertTrue(messages)
        for bad in ({'pipeline':45},{'commit':'b'*40},{'repository_id':2},{'phase':'preparing'},{'failed':True}):
            with self.assertRaises(ValueError):web.observe(self.env,fetch=lambda:{**self.result,**bad},emit=lambda *a,**kw:None)
    def test_push_event_and_authenticated_or_redirecting_progress_urls_are_refused(self):
        for bad in ({'CI_PIPELINE_EVENT':'push'},{'CI_COMMIT_BRANCH':'feature'},
                    {'RELEASE_CONTROL_ENDPOINT':'http://plain.example.invalid'},
                    {'RELEASE_CONTROL_ENDPOINT':'https://user:secret@example.invalid'}):
            with self.assertRaises(ValueError):web.observe({**self.env,**bad},fetch=lambda:self.fail('read attempted'))
        with self.assertRaises(ValueError):web.NoRedirect().redirect_request(None,None,None,None,None,None)


if __name__=='__main__':unittest.main()
