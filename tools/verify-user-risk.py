"""Controlled business/API integration with synthetic P-256 reports, not a device attestation test."""
import base64, hashlib, json, os, secrets, struct, urllib.error, urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from cryptography.x509.oid import NameOID

root=Path(__file__).resolve().parents[1]
base=os.environ.get('ORANGE_VERIFY_URL','http://127.0.0.1:38181')
config=json.loads((root/'private/device-trust.json').read_text(encoding='utf-8'))
env=dict(line.split('=',1) for line in (root/'private/server.env').read_text().splitlines() if line and not line.startswith('#'))
opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
checks=[]
def call(path,body=None,token=None,admin=False,expected=200,method=None,risk=None,host=None):
    headers={'Content-Type':'application/json'}
    if token: headers['Authorization']='Bearer '+token
    if admin: headers['x-admin-token']=env['ORANGE_ADMIN_TOKEN']
    if risk: headers['x-risk-event']=risk
    raw=None if body is None else json.dumps(body).encode()
    request=urllib.request.Request((host or base)+path,data=raw,headers=headers,method=method)
    try: response=opener.open(request,timeout=25)
    except urllib.error.HTTPError as error: response=error
    data=json.loads(response.read())
    assert response.status==expected, (path,response.status,expected,data)
    checks.append(path.split('?')[0])
    return data

key=ec.generate_private_key(ec.SECP256R1())
public=key.public_key().public_bytes(serialization.Encoding.DER,serialization.PublicFormat.SubjectPublicKeyInfo)
key_id=hashlib.sha256(public).hexdigest()
name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'ORANGE INTEGRATION TEST ONLY')])
certificate=x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key()).serial_number(x509.random_serial_number()).not_valid_before(datetime.now(timezone.utc)-timedelta(minutes=5)).not_valid_after(datetime.now(timezone.utc)+timedelta(hours=1)).sign(key,hashes.SHA256())
enrolled=False
failure_checked=False
def frame(kind,challenge,payload):
    ephemeral=ec.generate_private_key(ec.SECP256R1())
    server=serialization.load_der_public_key(base64.b64decode(challenge['serverPublicKey']))
    shared=ephemeral.exchange(ec.ECDH(),server)
    aes_key=HKDF(algorithm=hashes.SHA256(),length=32,salt=base64.b64decode(challenge['nonce']),info=b'device-trust/frame/v1').derive(shared)
    plain=json.dumps(payload,separators=(',',':')).encode()
    nonce=os.urandom(12)
    header=b'DTP4'+bytes([kind])+bytes.fromhex(challenge['id'])+bytes.fromhex(key_id)+ephemeral.public_key().public_bytes(serialization.Encoding.X962,serialization.PublicFormat.UncompressedPoint)+nonce+struct.pack('>I',len(plain)+16)
    signed=header+AESGCM(aes_key).encrypt(nonce,plain,header)
    signature=key.sign(signed,ec.ECDSA(hashes.SHA256()))
    return signed+struct.pack('>H',len(signature))+signature
def binary(data,expected):
    request=urllib.request.Request(config['base_url']+'/v1/secure',data=data,headers={'Content-Type':'application/octet-stream'})
    try: response=opener.open(request,timeout=20)
    except urllib.error.HTTPError as error: response=error
    result=json.loads(response.read())
    assert response.status==expected,(response.status,expected,result)
    checks.append('DTP4')
    return result
def report(token,kind,risky=False):
    global enrolled,failure_checked
    prepared=call('/api/risk/prepare',{'kind':kind,'key_id':key_id if enrolled else ''},token)
    challenge=prepared['challenge']
    if not enrolled:
        binary(frame(1,challenge,{'certificateChain':[base64.b64encode(certificate.public_bytes(serialization.Encoding.DER)).decode()]}),201)
        enrolled=True
    payload={'appId':config['app_id'],'package':'com.lingzidev.xiaochengyouxi','installationId':'controlled-integration-test',
             'signals':{'debugger':{'status':'available','value':risky},'emulatorHint':{'status':'available','value':risky}}}
    raw=frame(2,challenge,payload)
    uploaded=binary(raw,201)
    assert binary(raw,200)['reportId']==uploaded['reportId']
    tampered=bytearray(raw);tampered[142]^=1
    binary(bytes(tampered),403)
    if not failure_checked:
        call('/api/risk/failure',{'event_id':prepared['event_id'],'kind':kind,'stage':'upload','code':'TestTimeout'},token)
        call('/api/risk/complete',{'event_id':prepared['event_id'],'report_id':'0'*64},token,expected=503)
        failure_checked=True
    result=call('/api/risk/complete',{'event_id':prepared['event_id'],'report_id':uploaded['reportId']},token)
    assert set(result)=={'status','reference'},result
    assert call('/api/risk/complete',{'event_id':prepared['event_id'],'report_id':uploaded['reportId']},token)==result
    return prepared['event_id'],result
def account():
    phone='135'+''.join(secrets.choice('0123456789') for _ in range(8))
    code=call('/api/auth/sms',{'phone':phone,'purpose':'register'})['code']
    result=call('/api/auth/register',{'phone':phone,'password':secrets.token_hex(12),'code':code,'nickname':'风控联调'})
    call('/api/invite/bind',{'invite_code':call('/api/admin/invitations',admin=True)['platform_code']},result['token'])
    return result

a=account();token=a['token'];uid=a['user']['id']
call('/api/admin/users/'+str(uid)+'/detail',token=token,expected=401)
call('/api/check-in',{},token,expected=423)
login,_=report(token,'login')
call('/api/check-in',{},token,risk=login,expected=503)
rid,result=report(token,'check_in');assert result['status']=='recorded'
claimed=call('/api/check-in',{},token,risk=rid)
call('/api/check-in',{},token,risk=rid,expected=409)
assert claimed['user']['coins']==call('/api/me',token=token)['coins']
reward_id,_=report(token,'reward')
session=call('/api/rewards/start',{'task_id':'welcome'},token,risk=reward_id)
assert call('/api/rewards/start',{'task_id':'welcome'},token,risk=reward_id)['id']==session['id']
b=account()
call('/api/check-in',{},b['token'],risk=rid,expected=503)
call('/api/risk/failure',{'event_id':rid,'kind':'check_in','stage':'upload','code':'TestTimeout'},b['token'],expected=400)
report(b['token'],'login')
review_id,result=report(token,'reward',risky=True);assert result['status']=='review',result
blocked=call('/api/rewards/start',{'task_id':'welcome'},token,risk=review_id,expected=423)
assert not any(k in json.dumps(blocked) for k in ['debugger','emulator','score','rule','keyId'])
detail=call('/api/admin/users/'+str(uid)+'/detail',admin=True)
for section in ['timeline','ledger','ads','callbacks','withdrawals','risk','devices','audit','invitations']:
    records=call(f'/api/admin/users/{uid}/records?section={section}&page=1&page_size=100',admin=True)
    if section=='timeline':
        assert any(item['kind']=='risk_assessment' and item['status']=='failed' for item in records['items'])
        assert any(item['kind']=='risk_sdk_failure' for item in records['items'])
    if section=='risk' and base.startswith('https://'):
        assert all(item['client_ip'] not in ['127.0.0.1','::1'] for item in records['items'])
call(f'/api/admin/users/{uid}/restriction',{'status':'active','reason':'受控联调核验完成'},admin=True,method='PUT')
call('/api/auth/logout',{},token)
(root/'private/risk-test-users.json').write_text(json.dumps({'user_id':uid,'phone':a['user']['phone'],'other_user_id':b['user']['id'],'key_id':key_id},indent=2),encoding='utf-8')
(root/'output/user-risk-verification.json').write_text(json.dumps({'target':base,'checks':len(checks),'verified':['signed-report','tamper-rejection','report-retry','business-idempotency','account-binding','risk-hold','public-redaction','admin-auth','advanced-records','manual-review'],'synthetic_protocol_test':True},indent=2),encoding='utf-8')
print(json.dumps({'checks':len(checks),'user_id':uid,'status':'passed'},ensure_ascii=False))
