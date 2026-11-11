"""CI progress stays bound to its original Web Release and cannot mutate anything."""
import unittest
import contextlib
import io
from unittest.mock import patch
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

    def test_safe_configuration_diagnostic_reaches_ci_stderr_without_private_reason(self):
        failure={**self.result,'complete':False,'failed':True,'phase':'attention_required',
                 'error_code':'configuration_changed','recovery':'new_request','reason':'DO-NOT-EXPOSE'}
        def observe():return web.observe(self.env,fetch=lambda:failure,emit=lambda *a,**kw:None)
        # The CLI must preserve this guidance instead of replacing it with the
        # generic exception handler's workflow/availability message.
        with self.assertRaises(web.ReleaseStopped) as caught:observe()
        output=io.StringIO()
        with patch.object(web,'observe',side_effect=caught.exception),contextlib.redirect_stderr(output):
            self.assertEqual(web.main(),1)
        self.assertIn('empty RELEASE_RESUME',output.getvalue())
        self.assertNotIn('DO-NOT-EXPOSE',output.getvalue())

    def test_unknown_errors_require_inspection_and_mismatched_run_never_supplies_guidance(self):
        failure={**self.result,'failed':True,'reason':'DO-NOT-EXPOSE','error_code':'private-details',
                 'recovery':'new_request'}
        with self.assertRaises(web.ReleaseStopped) as caught:
            web.observe(self.env,fetch=lambda:failure,emit=lambda *a,**kw:None)
        self.assertIn('confirming the original local task exists',str(caught.exception))
        self.assertNotIn('DO-NOT-EXPOSE',str(caught.exception))
        with self.assertRaisesRegex(ValueError,'another repository/run/source'):
            web.observe(self.env,fetch=lambda:{**failure,'pipeline':45},emit=lambda *a,**kw:None)


if __name__=='__main__':unittest.main()
