set -eu
sudo python3 - <<'PY'
import sqlite3,json
c=sqlite3.connect('/var/lib/orange-rewards/orange.sqlite3');c.row_factory=sqlite3.Row
ref='0fa244e9-7f68-47b2-9cf5-3d67b8b84115'
rows=c.execute('SELECT id,user_id,kind,key_id,report_id,status,decision,action,error,created_at,updated_at FROM risk_events WHERE id=?',(ref,)).fetchall()
print(json.dumps({'reference':ref,'events':[dict(r) for r in rows]},ensure_ascii=False))
rows=c.execute("SELECT id,user_id,kind,status,related_id,detail_json,created_at FROM user_behavior WHERE related_id=? ORDER BY created_at",(ref,)).fetchall()
print(json.dumps({'behavior':[dict(r) for r in rows]},ensure_ascii=False))
rows=c.execute("SELECT id,user_id,kind,status,error,created_at FROM risk_events WHERE status='assessment_error' ORDER BY created_at DESC LIMIT 8").fetchall()
print(json.dumps({'recent_errors':[dict(r) for r in rows]},ensure_ascii=False))
PY
