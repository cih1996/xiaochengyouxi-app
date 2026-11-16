"""Exercise real rain allocation and claim boundaries with controlled accounts and synthetic CSJ callbacks."""
import hashlib,json,os,time,urllib.parse,uuid
from datetime import datetime,timezone
from device_trust_probe import Probe

p=Probe(os.environ.get('ORANGE_VERIFY_URL','http://127.0.0.1:38183'))
old_rain=p.call('/api/admin/rain-config',admin=True)
old_requests=p.call('/api/admin/hall-request-config',admin=True)
config=p.call('/api/config')
assert config['device_trust_enabled'] and config['ad_mode']=='csj','Use an isolated CSJ-mode service or the deployed service'
denied=[400,404,409,410,423,429]
def save_rain(value): return p.call('/api/admin/rain-config',value,admin=True,method='PUT')
def coins(token): return p.call('/api/me',token=token)['coins']
def ledger(token):
    uid=p.call('/api/me',token=token)['id']
    return p.call(f'/api/admin/users/{uid}/records?section=ledger&page_size=100',admin=True)['items']
def start(value):
    save_rain(value)
    current=p.call('/api/rain/status')
    if current['phase']=='active':
        p.wait_until(current['round']['end_at'])
    return p.call('/api/admin/rain/start',{'countdown_seconds':10},admin=True)
def claim(token,packet,**extra): return p.call('/api/rain/claim',{'packet_id':packet['id'],**extra},token)
def active_round(status):
    round=status['round'];assert round
    p.wait_until(round['start_at'])
    return round

try:
    a=p.account();b=p.account();token=a['token'];other=b['token']
    cfg={**old_rain,'enabled':False}
    save_rain(cfg)
    assert p.call('/api/rain/status')['phase']=='disabled'
    p.call('/api/rain/join',{},token,expected=denied)
    p.call('/api/rain/claim',{'packet_id':str(uuid.uuid4()),'coins':999999},token,expected=denied)
    assert coins(token)==0
    hour=(datetime.now(timezone.utc).hour+8+12)%24
    cfg.update(enabled=True,start_hour=hour,every_hours=24,duration_seconds=10,packet_count=5,
               small_min_coins=2,small_max_coins=2,grand_probability=0,grand_coins=10,max_total_coins=50,
               max_visible=3,fall_speed=1,overlay_opacity=.64)
    status=start(cfg)
    proof=p.assess(token,'rain')
    p.call('/api/rain/join',{},token,risk=proof,expected=denied)
    assert 'packets' not in p.call('/api/rain/status')
    active_round(status)
    p.call('/api/rain/join',{},other,expected=423)
    joined=p.call('/api/rain/join',{},token,risk=proof)
    repeat=p.call('/api/rain/join',{},token)
    ids=[packet['id'] for packet in joined['packets']]
    assert len(ids)==5 and len(set(ids))==5 and ids==[packet['id'] for packet in repeat['packets']]
    assert all(packet['kind']=='small' and 'reward' not in packet and 'coins' not in packet for packet in joined['packets'])
    packets=sorted(joined['packets'],key=lambda packet:packet['reveal_at'])
    last=packets[-1]
    if datetime.fromisoformat(last['reveal_at'].replace('Z','+00:00')).timestamp()>time.time():
        p.call('/api/rain/claim',{'packet_id':last['id']},token,expected=denied)
    p.wait_until(packets[0]['reveal_at'])
    p.call('/api/rain/claim',{'packet_id':packets[0]['id']},other,expected=denied)
    result=claim(token,packets[0],coins=999999)
    assert result['reward']==2 and coins(token)==2
    result=claim(token,packets[0]);assert result['reward']==2 and coins(token)==2
    assert len([entry for entry in ledger(token) if entry['source']=='rain:'+packets[0]['id']])==1
    save_rain({**cfg,'enabled':False})
    p.call('/api/rain/claim',{'packet_id':last['id']},token,expected=denied)
    assert coins(token)==2
    p.wait_until(joined['round']['end_at'])
    p.call('/api/rain/claim',{'packet_id':last['id']},token,expected=denied)
    assert coins(token)==2

    cfg.update(grand_probability=1)
    status=start(cfg);proof=p.assess(token,'rain');active_round(status)
    joined=p.call('/api/rain/join',{},token,risk=proof)
    assert all(packet['kind']=='large' for packet in joined['packets'])
    packet=min(joined['packets'],key=lambda packet:packet['reveal_at']);p.wait_until(packet['reveal_at'])
    large=claim(token,packet);assert large['kind']=='large' and coins(token)==2
    p.call('/api/rewards/start',{'task_id':large['task']['id']},token,expected=423)
    proof=p.assess(token,'reward')
    session=p.call('/api/rewards/start',{'task_id':large['task']['id']},token,risk=proof)
    p.call('/api/rewards/'+session['id']+'/complete',{},token,expected=409)
    assert coins(token)==2
    save_rain({**cfg,'enabled':False})
    query={'user_id':session['user_id'],'trans_id':'rain-test-'+uuid.uuid4().hex,'reward_name':'金币','reward_amount':str(session['reward']),'extra':session['media_extra'],'sign':'invalid'}
    assert not p.call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
    query['sign']=hashlib.sha256((p.env['CSJ_REWARD_SECRET']+':'+query['trans_id']).encode()).hexdigest()
    assert p.call('/csj/callback?'+urllib.parse.urlencode(query))['isValid']
    completed=p.call('/api/rewards/'+session['id']+'/complete',{},token)
    assert completed['user']['coins']==12
    assert p.call('/api/rewards/'+session['id']+'/complete',{},token)['user']['coins']==12
    assert len([entry for entry in ledger(token) if entry['source']=='rain:'+packet['id']])==1
    p.call('/api/admin/hall-request-config',{'enabled':True,'packet_enabled':True,'product_enabled':True,'cooldown_seconds':5,'daily_limit':2},admin=True,method='PUT')
    first=p.call('/api/hall/request',{'kind':'packet'},token);assert first['event']['kind']=='red_packet'
    p.call('/api/hall/request',{'kind':'product'},token,expected=429)
    time.sleep(5.1)
    second=p.call('/api/hall/request',{'kind':'product'},token);assert second['event']['kind']=='product'
    time.sleep(5.1)
    p.call('/api/hall/request',{'kind':'packet'},token,expected=denied)
    assert coins(token)==12
    records=p.call(f"/api/admin/users/{a['user']['id']}/records?section=timeline&page_size=100",admin=True)
    assert any('rain' in row['kind'] for row in records['items'])
    summary={'target':p.base,'checks':p.checks,'passed':True,'synthetic_protocol_and_callback':True,
             'verified':['outside-window','disabled','unpredictable-ids','cross-account','early-reveal','join-stability','duplicate-claim','server-amount','large-requires-callback','post-shutdown-completion','unique-ledger','proactive-cooldown-and-limit']}
    (p.root/'output/rain-verification.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
    print(json.dumps(summary))
finally:
    save_rain(old_rain)
    p.call('/api/admin/hall-request-config',old_requests,admin=True,method='PUT')
