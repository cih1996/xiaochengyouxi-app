"""Synthetic signed-protocol fixture for controlled business integration tests only."""
import base64, hashlib, json, os, secrets, struct, time, urllib.error, urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from cryptography.x509.oid import NameOID

class Probe:
    def __init__(self, base):
        self.root=Path(__file__).resolve().parents[1]
        self.base=base.rstrip('/')
        self.env=dict(line.split('=',1) for line in (self.root/'private/server.env').read_text().splitlines() if line and not line.startswith('#'))
        sdk_path=Path(os.environ.get('ORANGE_PROBE_SDK_CONFIG',str(self.root/'private/device-trust.json')))
        self.sdk=json.loads(sdk_path.read_text(encoding='utf-8'))
        self.opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
        self.checks=0
        self.admin_token=None
        self.admin_auth_required=None
        self.admin_access=json.loads((self.root/'private/deployment-access.json').read_text(encoding='utf-8'))
        self.key=ec.generate_private_key(ec.SECP256R1())
        self.key_id=hashlib.sha256(self.key.public_key().public_bytes(serialization.Encoding.DER,serialization.PublicFormat.SubjectPublicKeyInfo)).hexdigest()
        name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'CONTROLLED BUSINESS TEST ONLY')])
        self.cert=x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(self.key.public_key()).serial_number(x509.random_serial_number()).not_valid_before(datetime.now(timezone.utc)-timedelta(minutes=5)).not_valid_after(datetime.now(timezone.utc)+timedelta(hours=1)).sign(self.key,hashes.SHA256())
        self.enrolled=False
    def call(self,path,body=None,token=None,admin=False,method=None,risk=None,expected=200):
        headers={'Content-Type':'application/json'}
        if token: headers['Authorization']='Bearer '+token
        if admin:
            if self.admin_auth_required is None:self.admin_auth_required=self.call('/api/config')['admin_auth_required']
            if self.admin_auth_required:
                if self.admin_token is None:
                    self.admin_token=self.call('/api/admin/login',{'username':self.admin_access['admin_username'],'password':self.admin_access['admin_password']})['token']
                headers['x-admin-token']=self.admin_token
        if risk: headers['x-risk-event']=risk
        request=urllib.request.Request(self.base+path,data=None if body is None else json.dumps(body).encode(),headers=headers,method=method)
        try: response=self.opener.open(request,timeout=25)
        except urllib.error.HTTPError as error: response=error
        data=json.loads(response.read());self.checks+=1
        allowed=expected if isinstance(expected,(list,tuple)) else [expected]
        assert response.status in allowed,(path.split('?')[0],response.status,allowed,data.get('error'))
        return data
    def frame(self,kind,challenge,payload):
        ephemeral=ec.generate_private_key(ec.SECP256R1())
        server=serialization.load_der_public_key(base64.b64decode(challenge['serverPublicKey']))
        shared=ephemeral.exchange(ec.ECDH(),server)
        aes_key=HKDF(algorithm=hashes.SHA256(),length=32,salt=base64.b64decode(challenge['nonce']),info=b'device-trust/frame/v1').derive(shared)
        plain=json.dumps(payload,separators=(',',':')).encode();nonce=os.urandom(12)
        header=b'DTP4'+bytes([kind])+bytes.fromhex(challenge['id'])+bytes.fromhex(self.key_id)+ephemeral.public_key().public_bytes(serialization.Encoding.X962,serialization.PublicFormat.UncompressedPoint)+nonce+struct.pack('>I',len(plain)+16)
        signed=header+AESGCM(aes_key).encrypt(nonce,plain,header)
        signature=self.key.sign(signed,ec.ECDSA(hashes.SHA256()))
        return signed+struct.pack('>H',len(signature))+signature
    def binary(self,frame):
        request=urllib.request.Request(self.sdk['base_url']+'/v1/secure',data=frame,headers={'Content-Type':'application/octet-stream'})
        with self.opener.open(request,timeout=20) as response: return json.load(response)
    def assess(self,token,kind,intent=None):
        if intent is None:
            if kind!='login':raise ValueError('Pass the exact business intent for DeviceTrust 0.5')
            intent={'method':'POST','path':'/api/auth/login/report','body':{}}
        prepared=self.call('/api/risk/prepare',{'kind':kind,'key_id':self.key_id if self.enrolled else '', 'intent':intent},token)
        challenge=prepared['challenge']
        assert challenge['hardwareRequired'] is False,'Fixture is not a hardware-attestation test'
        assert len(challenge['context']['requestHash'])==64,'Business request binding is required'
        if not self.enrolled:
            self.binary(self.frame(1,challenge,{'certificateChain':[base64.b64encode(self.cert.public_bytes(serialization.Encoding.DER)).decode()]}))
            self.enrolled=True
        uploaded=self.binary(self.frame(2,challenge,{'appId':self.sdk['app_id'],'package':'com.lingzidev.xiaochengyouxi','installationId':'controlled-rain-test','signals':{'debugger':{'status':'available','value':False},'emulatorHint':{'status':'available','value':False}}}))
        result=self.call('/api/risk/complete',{'event_id':prepared['event_id'],'report_id':uploaded['reportId']},token)
        assert result['status']=='recorded',result['status']
        return prepared['event_id']
    def account(self,nickname='红包雨联调'):
        phone='134'+''.join(secrets.choice('0123456789') for _ in range(8))
        code=self.call('/api/auth/sms',{'phone':phone,'purpose':'register'})['code']
        result=self.call('/api/auth/register',{'phone':phone,'password':secrets.token_hex(12),'code':code,'nickname':nickname})
        code=self.call('/api/admin/invitations',admin=True)['platform_code']
        self.call('/api/invite/bind',{'invite_code':code},result['token'])
        return result
    @staticmethod
    def wait_until(iso,extra=0.05):
        target=datetime.fromisoformat(iso.replace('Z','+00:00')).timestamp()+extra
        remaining=target-time.time()
        assert remaining<61,'Do not block for a long schedule; use an immediate test round'
        if remaining>0: time.sleep(remaining)
