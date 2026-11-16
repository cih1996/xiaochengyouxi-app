"""Verify production with a labeled test account; leave operating policies unchanged."""
import hashlib,json,urllib.parse,uuid,time
from device_trust_probe import Probe
p=Probe('https://orange-api.lingzidev.com')
config=p.call('/api/config');assert config['device_trust_enabled'] and config['ad_mode']=='csj'
policy=p.call('/api/admin/reward-policy',admin=True);settings=p.call('/api/admin/settings',admin=True)
assert settings['csj_expected_reward_name']=='金币' and settings['csj_expected_reward_amount']==100
a=p.account('SDK发布验收');token=a['token'];uid=a['user']['id']
q=p.call('/api/rewards/quote',{'task_id':'welcome'},token)
assert q['reward_min']<=q['reward']<=q['reward_max']
body={'task_id':'welcome','quote_id':q['quote_id']}
event=p.assess(token,'reward',{'method':'POST','path':'/api/rewards/start','body':body})
session=p.call('/api/rewards/start',body,token,risk=event);assert session['reward']==q['reward']
assert p.call('/api/rewards/start',body,token,risk=event)['id']==session['id']
p.call('/api/rewards/'+session['id']+'/client-event',{'event':'reward_arrived','valid':True,'reward_type':0,'is_server_verify':True},token)
assert p.call('/api/rewards/'+session['id']+'/status',token=token)['status']=='pending'
assert p.call('/api/rewards/'+session['id']+'/complete',{},token,expected=409)['code']=='REWARD_PENDING'
assert p.call('/api/me',token=token)['coins']==0
trans='release05-controlled-'+uuid.uuid4().hex
query={'user_id':session['user_id'],'trans_id':trans,'reward_name':'金币','reward_amount':'100','extra':session['media_extra'],
       'sign':hashlib.sha256((p.env['CSJ_REWARD_SECRET']+':'+trans).encode()).hexdigest()}
assert p.call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
assert p.call('/api/rewards/'+session['id']+'/complete',{},token)['user']['coins']==q['reward']
assert p.call('/api/rewards/'+session['id']+'/complete',{},token)['user']['coins']==q['reward']
receipt_path='/api/admin/risk-receipts/'+event
receipt=p.call(receipt_path,admin=True)
if receipt['status']!='sent':receipt=p.call(receipt_path+'/retry',{},admin=True)
assert receipt['status']=='sent' and 'token' not in receipt
records=p.call(f'/api/admin/users/{uid}/records?section=risk&page_size=100',admin=True)['items']
record=next(row for row in records if row['id']==event)
assert record['detail']['mode']=='enforce' and record['detail']['deviceId']
assert not any(key in json.dumps(record) for key in ['permit_json','"token":','app_key'])
assert record['client_ip'] not in ['127.0.0.1','::1']
assert p.call('/api/admin/reward-policy',admin=True)==policy
p.call('/api/auth/logout',{},token)
result={'passed':True,'live':True,'checks':p.checks,'sdk':'0.5.0','mode':'enforce','reward':q['reward'],
        'synthetic_protocol_and_callback':True,'operator_policy_unchanged':True,'server_device_id':True,
        'client_notification_cannot_credit':True,'quote_session_ledger_equal':True,'receipt_sent':True}
(p.root/'output/sdk05-live-verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps(result))
