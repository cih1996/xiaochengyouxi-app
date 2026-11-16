set -eu
sudo python3 - <<'PY'
import sqlite3,json
c=sqlite3.connect('/var/lib/orange-rewards/orange.sqlite3');c.row_factory=sqlite3.Row
rows=c.execute("SELECT user_id,status,reason_code,count(*) count,max(created_at) latest FROM callback_events GROUP BY user_id,status,reason_code ORDER BY latest DESC LIMIT 20").fetchall()
print(json.dumps({'callback_summary':[dict(r) for r in rows]},ensure_ascii=False))
PY
sudo sh -c "rg 'csj/callback' /var/log/nginx/orange-api.access.log | tail -n 10"
