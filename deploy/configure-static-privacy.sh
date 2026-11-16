set -eu
sudo python3 - <<'PY'
from pathlib import Path
path=Path('/etc/nginx/sites-available/orange-rewards')
raw=path.read_text()
raw=raw.replace('location = /privacy { try_files /index.html =404; }','location = /privacy { try_files /privacy.html =404; }')
raw=raw.replace('location = /terms { try_files /index.html =404; }','location = /terms { try_files /terms.html =404; }')
path.write_text(raw)
PY
sudo nginx -t
sudo systemctl reload nginx
grep -n -A1 'location = /privacy\|location = /terms' /etc/nginx/sites-available/orange-rewards
