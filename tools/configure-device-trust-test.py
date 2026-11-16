"""Dedicated DeviceTrust application for isolated 0.5 Guard integration tests."""
import json
from pathlib import Path
from urllib.request import Request,urlopen

root=Path(__file__).resolve().parents[1]
access=json.loads(Path(r'C:\Users\Arlo\Documents\Codex\2026-11-10\device-trust-platform-work\outputs\DeviceTrust-线上登录凭据.json').read_text(encoding='utf-8-sig'))
target=root/'private/device-trust-test.json'
def call(path,body=None,method=None):
    request=Request(access['console'].rstrip('/')+'/api/'+path,
        data=None if body is None else json.dumps(body).encode(),
        headers={'Authorization':'Bearer '+access['adminToken'],'Content-Type':'application/json'},
        method=method or ('GET' if body is None else 'PUT'))
    with urlopen(request,timeout=25) as response:return json.load(response)
if target.exists():
    config=json.loads(target.read_text(encoding='utf-8'))
else:
    if any(a['name']=='小橙有喜联调' for a in call('apps')):
        raise RuntimeError('Restore the saved test application key instead of rotating it')
    created=call('apps',{'name':'小橙有喜联调','package':'com.lingzidev.xiaochengyouxi'},'POST')
    config={'app_id':created['app']['id'],'app_key':created['key'],'base_url':'https://devicetrust-sdk-api.lingzidev.com','spki_pins':['AU7LGDAFJUgs4HkcpRf1O7/6OU9avVMvle6wTsAK5m4=']}
    target.write_text(json.dumps(config,ensure_ascii=False,indent=2),encoding='utf-8')
app=next(a for a in call('apps') if a['id']==config['app_id'])
security={**app['security'],'requireRequestBinding':True,'requireSignedReports':True}
call('apps/'+config['app_id']+'/security',security)
policy={**app['policy'],'mode':'enforce'}
policy=call('apps/'+config['app_id']+'/policy',policy)
print(json.dumps({'test_app_id':config['app_id'],'mode':policy['mode'],'requireRequestBinding':True}))
