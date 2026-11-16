"""Inspect or enable 0.5 request binding for this application only."""
import argparse,json
from pathlib import Path
from urllib.request import Request,urlopen

root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--enable-binding',action='store_true');parser.add_argument('--enforce',action='store_true');args=parser.parse_args()
access=json.loads(Path(r'C:\Users\Arlo\Documents\Codex\2026-11-10\device-trust-platform-work\outputs\DeviceTrust-线上登录凭据.json').read_text(encoding='utf-8-sig'))
business=json.loads((root/'private/device-trust.json').read_text(encoding='utf-8'))
def call(path,body=None):
    req=Request(access['console'].rstrip('/')+'/api/'+path,
        data=None if body is None else json.dumps(body).encode(),
        headers={'Authorization':'Bearer '+access['adminToken'],'Content-Type':'application/json'},
        method='GET' if body is None else 'PUT')
    with urlopen(req,timeout=20) as response:return json.load(response)
app=next(a for a in call('apps') if a['id']==business['app_id'])
security=app['security']
policy=app['policy']
if args.enable_binding:
    backup=root/'private/device-trust-security-before-05.json'
    if not backup.exists():backup.write_text(json.dumps(security,indent=2),encoding='utf-8')
    security={**security,'requireSignedReports':True,'requireRequestBinding':True}
    result=call('apps/'+business['app_id']+'/security',security)
    assert result['requireSignedReports'] and result['requireRequestBinding']
if args.enforce:
    backup=root/'private/device-trust-policy-before-05.json'
    if not backup.exists():backup.write_text(json.dumps(policy,indent=2),encoding='utf-8')
    policy=call('apps/'+business['app_id']+'/policy',{**policy,'mode':'enforce'})
    assert policy['mode']=='enforce'
print(json.dumps({'application':app['name'],'app_id':business['app_id'],'security':security,'policy_mode':policy['mode']},ensure_ascii=False))
