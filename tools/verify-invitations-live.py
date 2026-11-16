import json
import secrets
import string
import urllib.error
import urllib.request
from pathlib import Path
import cv2
import numpy as np
from PIL import Image

root=Path(__file__).resolve().parents[1]
base='https://orange-api.lingzidev.com'
opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
checks=0
def call(path,body=None,token=None,status=200):
    global checks
    headers={'Content-Type':'application/json'}
    if token: headers['Authorization']='Bearer '+token
    request=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers=headers)
    try: response=opener.open(request,timeout=20)
    except urllib.error.HTTPError as error: response=error
    assert response.status==status,(path,response.status,status)
    checks+=1
    return json.loads(response.read())
def account(prebound=None):
    phone='136'+''.join(secrets.choice(string.digits) for _ in range(8))
    if prebound: call('/api/invite/prebind',{'phone':phone,'invite_code':prebound})
    code=call('/api/auth/sms',{'phone':phone,'purpose':'register'})['code']
    password=secrets.token_hex(10)
    result=call('/api/auth/register',{'phone':phone,'password':password,'code':code,'nickname':'邀请验收'})
    return phone,password,result

config=call('/api/config')
assert config['app_name']=='小橙有喜' and config['android_package']=='com.lingzidev.xiaochengyouxi'
phone,password,a=account()
assert a['user']['invitation_required']
token=a['token']
for endpoint,body in [('/api/ledger',None),('/api/check-in',{}),('/api/rewards/start',{'task_id':'welcome'}),('/api/hall/stream',None),('/api/invite/summary',None)]:
    assert call(endpoint,body,token,status=403)['code']=='INVITE_REQUIRED'
call('/api/auth/logout',{},token)
call('/api/me',token=token,status=401)
a=call('/api/auth/login',{'phone':phone,'password':password});token=a['token']
call('/api/invite/bind',{'invite_code':a['user']['invite_code']},token,status=400)
platform='JZ8F6A'
assert not call('/api/invite/bind',{'invite_code':platform},token)['user']['invitation_required']
summary=call('/api/invite/summary',token=token)
qr=call('/api/invite/qr/'+summary['invite_code'])
scale=10
image=Image.new('L',((qr['size']+8)*scale,)*2,255)
pixels=image.load()
for y,row in enumerate(qr['modules']):
    for x,cell in enumerate(row):
        if cell=='1':
            for yy in range((y+4)*scale,(y+5)*scale):
                for xx in range((x+4)*scale,(x+5)*scale):pixels[xx,yy]=0
decoded,_,_=cv2.QRCodeDetector().detectAndDecode(np.asarray(image))
assert decoded==summary['invite_url']
image.save(root/'output/invite-qr-live-check.png')
bphone,bpass,b=account(summary['invite_code'])
assert not b['user']['invitation_required'] and b['user']['inviter_code']==summary['invite_code']
assert b['user']['coins']==0
call('/api/invite/bind',{'invite_code':platform},b['token'],status=409)
call('/api/invite/prebind',{'phone':bphone,'invite_code':platform},status=409)
assert call('/api/invite/summary',token=token)['registered_count']==1
request=urllib.request.Request(base+'/api/hall/stream?native_ads=1',headers={'Authorization':'Bearer '+token})
with opener.open(request,timeout=20) as stream:
    events=[]
    while len(events)<4:
        line=stream.readline().decode()
        if line.startswith('data:'):events.append(json.loads(line[5:]))
assert [event['kind'] for event in events]==['text','red_packet','product','native_ad']
assert events[-1]['ad_code_id']=='993606636'
result={'passed':True,'apiChecks':checks,'unboundBlocked':True,'phonePrebindAutoConsumed':True,'bindingDoesNotAwardCoins':True,'rebindingBlocked':True,'qrDecoded':True,'nativeCapabilityEvent':True}
(root/'output/live-invitations-verification.json').write_text(json.dumps(result,indent=2))
print(json.dumps(result))
