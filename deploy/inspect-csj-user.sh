set -eu
sudo python3 - <<'PY'
import sqlite3,json
c=sqlite3.connect('/var/lib/orange-rewards/orange.sqlite3');c.row_factory=sqlite3.Row
for table,query in [
('sessions',"SELECT id,user_id,task_id,reward,provider,started_at,completed_at,verified_at,csj_trans_id FROM reward_sessions WHERE user_id=2 ORDER BY started_at DESC LIMIT 10"),
('callbacks',"SELECT id,reward_session_id,trans_id,status,reason_code,created_at FROM callback_events WHERE user_id=2 ORDER BY created_at DESC LIMIT 20"),
('risk',"SELECT id,kind,status,error,created_at FROM risk_events WHERE user_id=2 ORDER BY created_at DESC LIMIT 8")]:
 print(json.dumps({table:[dict(r) for r in c.execute(query)]},ensure_ascii=False))
PY
