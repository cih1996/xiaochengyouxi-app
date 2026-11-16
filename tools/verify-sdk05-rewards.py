"""Controlled integration of DeviceTrust 0.5 Guard, device pricing, and CSJ verification."""
import hashlib,json,os,time,urllib.parse,uuid
from device_trust_probe import Probe

p=Probe(os.environ.get('ORANGE_VERIFY_URL','http://127.0.0.1:38183'))
old=p.call('/api/admin/reward-policy',admin=True)
settings=p.call('/api/admin/settings',admin=True)
def save(value):return p.call('/api/admin/reward-policy',value,admin=True,method='PUT')
def quote(token,task='welcome',key=None):
    value={'task_id':task}
    if key is not None:value['key_id']=key
    return p.call('/api/rewards/quote',value,token)
def intent(quote):return {'method':'POST','path':'/api/rewards/start','body':{'task_id':quote['task_id'],'quote_id':quote['quote_id']}}
def start(token,quote,event):return p.call('/api/rewards/start',intent(quote)['body'],token,risk=event)
def coins(token):return p.call('/api/me',token=token)['coins']
def callback(session,amount=None,valid=True):
    trans='sdk05-'+uuid.uuid4().hex
    query={'user_id':session['user_id'],'trans_id':trans,'reward_name':settings['csj_expected_reward_name'],
        'reward_amount':str(settings['csj_expected_reward_amount'] if amount is None else amount),
        'extra':session['media_extra'],'sign':hashlib.sha256((p.env['CSJ_REWARD_SECRET']+':'+trans).encode()).hexdigest() if valid else 'invalid'}
    return p.call('/csj/callback?'+urllib.parse.urlencode(query))
def finish(token,session):
    before=coins(token)
    pending=p.call('/api/rewards/'+session['id']+'/status',token=token)
    assert pending['status']=='pending'
    error=p.call('/api/rewards/'+session['id']+'/complete',{},token,expected=409)
    assert error['code']=='REWARD_PENDING'
    assert not callback(session,valid=False)['isValid']
    assert not callback(session,amount=session['reward'] if session['reward']!=settings['csj_expected_reward_amount'] else -1)['isValid']
    assert coins(token)==before
    assert callback(session)['isValid']
    assert p.call('/api/rewards/'+session['id']+'/status',token=token)['status']=='verified'
    complete=p.call('/api/rewards/'+session['id']+'/complete',{},token)
    assert complete['reward']==session['reward'] and complete['user']['coins']==before+session['reward']
    repeat=p.call('/api/rewards/'+session['id']+'/complete',{},token)
    assert repeat['user']['coins']==complete['user']['coins']
    return complete
def receipt(event):
    path='/api/admin/risk-receipts/'+event
    result=p.call(path,admin=True)
    if result['status']!='sent':result=p.call(path+'/retry',{},admin=True)
    assert result['status']=='sent'
    assert not any(field in result for field in ['token','permit_json','app_key'])
    return result

try:
    policy={**old,'enabled':True,'quote_ttl_seconds':300,'fixed_coins':19,
        'new_device':{'max_days':3,'max_completed':2,'min_coins':11,'max_coins':13},
        'active_device':{'max_days':30,'max_completed':4,'min_coins':5,'max_coins':6},
        'mature_device':{'min_coins':3,'max_coins':4},'unknown_device':{'min_coins':7,'max_coins':9}}
    policy=save(policy)
    a=p.account();token=a['token'];b=p.account();other=b['token']
    q=quote(token);assert 7<=q['reward']<=9
    assert quote(token)['quote_id']==q['quote_id'],'Reopening must not reroll a live price'
    assert quote(token)['reward']==q['reward']
    prepared_intent=intent(q)
    p.call('/api/risk/prepare',{'kind':'reward','key_id':'','intent':{**prepared_intent,'path':'/api/withdrawals'}},token,expected=[400,409])
    event=p.assess(token,'reward',prepared_intent)
    p.call('/api/rewards/start',prepared_intent['body'],token,expected=423)
    p.call('/api/rewards/start',prepared_intent['body'],other,risk=event,expected=[400,409,410,423,503])
    p.call('/api/rewards/start',{**prepared_intent['body'],'task_id':'afternoon'},token,risk=event,expected=[400,409,410,503])
    session=start(token,q,event);assert session['reward']==q['reward']
    assert start(token,q,event)['id']==session['id']
    finish(token,session);receipt(event)
    assert p.call('/api/rewards/'+session['id']+'/status',token=other,expected=[401,404])
    events=p.call(f"/api/admin/users/{a['user']['id']}/records?section=risk&page_size=100",admin=True)['items']
    record=next(row for row in events if row['id']==event)
    assert record['detail'].get('deviceId'),'SDK0.5 must return a server-registered identity'
    assert record['detail']['reportProof']['level']=='signature_only'
    assert record['detail']['context']['requestHash']
    assert not any(field in json.dumps(record) for field in ['permit_json','"token":','app_key'])
    p.assess(other,'login')
    q2=quote(other,key=p.key_id);assert 11<=q2['reward']<=13,'Same key across accounts must retain completed count'
    event2=p.assess(other,'reward',intent(q2));session2=start(other,q2,event2)
    q3=quote(token,key=p.key_id);assert 5<=q3['reward']<=6,'In-flight session reserves the next tier position'
    omitted=quote(token);assert 5<=omitted['reward']<=6,'Omitting an existing key must not select the unknown tier'
    p.call('/api/rewards/quote',{'task_id':'welcome','key_id':'f'*64},token,expected=400)
    changed=save({**policy,'enabled':False,'fixed_coins':19})
    p.call('/api/rewards/start',intent(q3)['body'],token,risk=event,expected=[400,409,410,503])
    finish(other,session2);receipt(event2)
    fixed=quote(token,key=p.key_id);assert fixed['reward']==19
    fixed_event=p.assess(token,'reward',intent(fixed));fixed_session=start(token,fixed,fixed_event)
    save({**changed,'fixed_coins':23})
    finish(token,fixed_session);receipt(fixed_event)
    assert fixed_session['reward']==19 and quote(token,key=p.key_id)['reward']==23
    for endpoint,body,kind in [('/api/check-in',{},'check_in')]:
        proof=p.assess(token,kind,{'method':'POST','path':endpoint,'body':body})
        p.call(endpoint,body,token,risk=proof);receipt(proof)
    profile=p.call('/api/admin/users/'+str(a['user']['id'])+'/detail',admin=True)
    assert profile['summary']['ad_completed']>=2
    summary={'target':p.base,'checks':p.checks,'passed':True,'synthetic_protocol_and_callback':True,
        'verified':['hardware-required-false','server-device-id','enforce-permit','full-intent-binding','quote-stability','unknown-device','cross-account-device-tier','inflight-reservation','session-price-lock','static-provider-validation','callback-before-credit','ledger-idempotency','private-receipts']}
    (p.root/'output/sdk05-rewards-verification.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
    print(json.dumps(summary))
finally:
    save(old)
