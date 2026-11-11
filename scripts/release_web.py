#!/usr/bin/env python3
"""Observe this exact Web-triggered Release; no account credentials or mutation API."""
import json
import os
import re
import ssl
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import build_opener, HTTPSHandler, HTTPRedirectHandler, Request


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self,*args,**kwargs):
        raise ValueError('Release progress redirects are refused')


class ReleaseStopped(ValueError):
    """Fixed public diagnostics; never print arbitrary coordinator output."""


def failure_message(value):
    if value.get('error_code')=='configuration_changed' and value.get('recovery')=='new_request':
        return ('Release stopped: release configuration changed. Review the current configuration, then start '
                'a new main / release pipeline with the intended dry-run value and an empty RELEASE_RESUME. '
                'Original records and candidates are preserved.')
    return ('Release stopped. Inspect the original deployment record before recovery. Use RELEASE_RESUME '
            'only after confirming the original local task exists and its intent is unchanged. '
            'No unknown write was repeated.')


def observe(env=os.environ,*,fetch=None,sleep=time.sleep,clock=time.monotonic,emit=print):
    if env.get('CI')!='woodpecker' or env.get('CI_PIPELINE_EVENT')!='manual' or env.get('CI_COMMIT_BRANCH')!='main':
        raise ValueError('Release must be selected manually on main')
    repo=env.get('CI_REPO_ID','');number=env.get('CI_PIPELINE_NUMBER','');revision=env.get('CI_COMMIT_SHA','')
    if not all(re.fullmatch('[1-9][0-9]*',v) for v in (repo,number)) or not re.fullmatch('[0-9a-f]{40}',revision):
        raise ValueError('Actual Server run identity is required')
    endpoint=env.get('RELEASE_CONTROL_ENDPOINT','').rstrip('/');parts=urlsplit(endpoint)
    if parts.scheme!='https' or not parts.hostname or parts.username or parts.password or parts.query or parts.fragment or parts.path:
        raise ValueError('Configured HTTPS Release coordinator is required')
    if fetch is None:
        context=ssl.create_default_context(cadata=env.get('RELEASE_CONTROL_CA',''))
        opener=build_opener(HTTPSHandler(context=context),NoRedirect())
        def fetch():
            with opener.open(Request(endpoint+'/status/'+repo+'/'+number),timeout=15) as response:
                data=response.read(65537)
            if len(data)>65536:raise ValueError('Release progress exceeded its bound')
            return json.loads(data)
    deadline=clock()+3*3600;previous=None;last=clock();unavailable=clock()
    emit('Release started from Woodpecker Web UI. Default dry-run; official login/2FA prompts appear on the deployment desktop.',flush=True)
    while clock()<deadline:
        try:value=fetch()
        except (HTTPError,URLError,TimeoutError,OSError):
            if clock()-unavailable>90:raise ValueError('Release progress unavailable; inspect the original run, do not resubmit preparation') from None
            sleep(3);continue
        unavailable=clock()
        if (value.get('repository_id'),value.get('pipeline'),value.get('commit'))!=(int(repo),int(number),revision):
            raise ValueError('Progress belongs to another repository/run/source')
        state={k:value[k] for k in ('phase','candidate','operations','dry_run') if k in value}
        if state!=previous or clock()-last>=45:
            emit(json.dumps(state,ensure_ascii=False),flush=True);previous=state;last=clock()
        if value.get('candidate'):
            candidate=value['candidate']
            if type(candidate) is not int or candidate<=0:raise ValueError('Invalid original candidate identity')
        if value.get('failed'):raise ReleaseStopped(failure_message(value))
        if value.get('complete'):
            if value.get('phase') not in ('dry_run_complete','release_complete'):raise ValueError('Unexpected terminal Release state')
            emit('Dry-run complete: no npm upload or Tag/Release creation.' if value['phase']=='dry_run_complete'
                 else 'Release complete: public original bytes and isolated installation verified.',flush=True)
            return value
        sleep(3)
    raise ValueError('Observation timed out; original candidates/stages were preserved. Resume the same request.')


def main():
    try:observe()
    except ReleaseStopped as error:
        print(str(error),file=sys.stderr)
        return 1
    except (ValueError,KeyError,TypeError):
        print('Release stopped: verify selected workflow/main, parameters, coordinator availability and original run. No unknown write was repeated.',file=sys.stderr)
        return 1
    return 0


if __name__=='__main__':sys.exit(main())
