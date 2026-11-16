"""Check 0.5-bound rain joins and quoted grand prizes using controlled SDK/CSJ fixtures."""
import hashlib,json,os,urllib.parse,uuid
from datetime import datetime,timezone
from device_trust_probe import Probe
p=Probe(os.environ.get('ORANGE_VERIFY_URL','http://127.0.0.1:38183'))
old=p.call('/api/admin/rain-config',admin=True);settings=p.call('/api/admin/settings',admin=True)
try:
    cfg={**old,'enabled':True,'start_hour':(datetime.now(timezone.utc).hour+20)%24,'every_hours':24,
         'duration_seconds':15,'packet_count':5,'small_min_coins':1,'small_max_coins':1,
         'grand_probability':1,'grand_coins':14,'max_total_coins':70,'max_visible':5}
    p.call('/api/admin/rain-config',cfg,admin=True,method='PUT')
    status=p.call('/api/rain/status')
    if status['phase']=='active':p.wait_until(status['round']['end_at'])
    status=p.call('/api/admin/rain/start',{'countdown_seconds':10},admin=True)
    a=p.account();token=a['token'];round_id=status['round']['id'];body={'round_id':round_id}
    proof=p.assess(token,'rain',{'method':'POST','path':'/api/rain/join','body':body})
    p.call('/api/rain/join',body,token,risk=proof,expected=[400,409])
    p.wait_until(status['round']['start_at'])
    p.call('/api/rain/join',{'round_id':str(uuid.uuid4())},token,risk=proof,expected=[400,409,503])
    assignment=p.call('/api/rain/join',body,token,risk=proof)
    assert [v['id'] for v in assignment['packets']]==[v['id'] for v in p.call('/api/rain/join',body,token)['packets']]
    packet=min(assignment['packets'],key=lambda v:v['reveal_at']);p.wait_until(packet['reveal_at'])
    opened=p.call('/api/rain/claim',{'packet_id':packet['id']},token);assert opened['kind']=='large'
    q=p.call('/api/rewards/quote',{'task_id':opened['task']['id'],'key_id':p.key_id},token);assert q['reward']==14
    reward_body={'task_id':q['task_id'],'quote_id':q['quote_id']}
    reward_proof=p.assess(token,'reward',{'method':'POST','path':'/api/rewards/start','body':reward_body})
    session=p.call('/api/rewards/start',reward_body,token,risk=reward_proof);assert session['reward']==14
    p.call('/api/rewards/'+session['id']+'/complete',{},token,expected=409)
    p.call('/api/admin/rain-config',{**cfg,'enabled':False},admin=True,method='PUT')
    trans='sdk05-rain-'+uuid.uuid4().hex
    query={'user_id':session['user_id'],'trans_id':trans,'reward_name':settings['csj_expected_reward_name'],
           'reward_amount':str(settings['csj_expected_reward_amount']),'extra':session['media_extra'],
           'sign':hashlib.sha256((p.env['CSJ_REWARD_SECRET']+':'+trans).encode()).hexdigest()}
    assert p.call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
    assert p.call('/api/rewards/'+session['id']+'/complete',{},token)['user']['coins']==14
    assert p.call('/api/rewards/'+session['id']+'/complete',{},token)['user']['coins']==14
    summary={'passed':True,'checks':p.checks,'target':p.base,'synthetic_protocol_and_callback':True,
             'verified':['bound-round','stable-join','grand-price-preserved','s2s-required','completed-after-shutdown','no-duplicate-credit']}
    (p.root/'output/sdk05-rain-verification.json').write_text(json.dumps(summary,indent=2),encoding='utf-8');print(json.dumps(summary))
finally:p.call('/api/admin/rain-config',old,admin=True,method='PUT')
