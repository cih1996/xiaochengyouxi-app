"""Check the deployed service using a private test account and a synthetic callback."""
import hashlib
import json
import secrets
import string
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

root = Path(__file__).resolve().parents[1]
api = 'https://orange-api.lingzidev.com'
web = 'https://orange.lingzidev.com'
env = dict(line.split('=', 1) for line in (root/'private/server.env').read_text().splitlines() if line)
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
checks = 0
def call(path, body=None, token=None, admin=None, status=200, host=api):
    global checks
    headers = {'Content-Type': 'application/json'}
    if token: headers['Authorization'] = 'Bearer ' + token
    if admin: headers['x-admin-token'] = admin
    request = urllib.request.Request(host+path, data=None if body is None else json.dumps(body).encode(), headers=headers)
    try:
        response = opener.open(request, timeout=20)
    except urllib.error.HTTPError as error:
        response = error
    assert response.status == status, f'{path.split("?")[0]} expected {status}, got {response.status}'
    result = json.loads(response.read())
    checks += 1
    return result

assert call('/api/health')['ok']
config = call('/api/config')
assert config['ad_mode'] == 'csj' and config['admin_auth_required']
call('/api/admin/overview', status=401)
call('/api/admin/overview', status=401, host=web)
call('/api/admin/login', {'password':'incorrect'}, status=401)
admin = call('/api/admin/login', {'password':env['ORANGE_ADMIN_PASSWORD']})['token']
call('/api/admin/overview', admin=admin)
call('/api/auth/login', {'phone':'13800138000','password':'Orange123!'}, status=401)
access_path = root/'private/deployment-access.json'
access = json.loads(access_path.read_text())
if 'app_phone' not in access:
    phone = '139' + ''.join(secrets.choice(string.digits) for _ in range(8))
    password = ''.join(secrets.choice(string.ascii_letters+string.digits) for _ in range(20))
    code = call('/api/auth/sms', {'phone':phone,'purpose':'register'})['code']
    account = call('/api/auth/register', {'phone':phone,'password':password,'code':code,'nickname':'发布验收账号'})
    access.update(app_phone=phone, app_password=password)
    access_path.write_text(json.dumps(access,indent=2),encoding='utf-8')
else:
    account = call('/api/auth/login', {'phone':access['app_phone'],'password':access['app_password']})
token = account['token']
if account['user'].get('invitation_required'):
    platform = call('/api/admin/invitations', admin=admin)['platform_code']
    call('/api/invite/bind', {'invite_code':platform}, token)
before = account['user']['coins']
session = call('/api/rewards/start', {'task_id':'welcome'}, token)
assert session['mode']=='csj'
call('/api/rewards/'+session['id']+'/complete', {}, token, status=409)
query = {'user_id':session['user_id'],'trans_id':'deployment-'+uuid.uuid4().hex,'reward_name':'金币','reward_amount':str(session['reward']),'extra':session['media_extra'],'sign':'invalid'}
assert not call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
query['sign'] = hashlib.sha256((env['CSJ_REWARD_SECRET']+':'+query['trans_id']).encode()).hexdigest()
assert call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
assert call('/api/me',token=token)['coins'] == before
assert call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
result = call('/api/rewards/'+session['id']+'/complete', {}, token)
assert result['user']['coins'] == before+session['reward']
assert call('/api/rewards/'+session['id']+'/complete', {}, token)['already_completed']
call('/api/hall/stream', status=401)
request = urllib.request.Request(api+'/api/hall/stream',headers={'Authorization':'Bearer '+token})
with opener.open(request,timeout=15) as stream:
    events=[]
    while len(events)<3:
        line=stream.readline().decode()
        if line.startswith('data:'): events.append(json.loads(line[5:]))
assert [event['kind'] for event in events]==['text','red_packet','product']
summary={'passed':True,'httpsApiChecks':checks,'adminAuthentication':True,'syntheticCallbackOnly':True,'duplicateCreditPrevented':True,'sseKinds':[event['kind'] for event in events]}
(root/'output/deployment-verification.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary))
