set -eu
sudo python3 - <<'PY'
from pathlib import Path
path=Path('/etc/orange-rewards/server.env')
raw=path.read_text() if path.exists() else ''
lines=[line for line in raw.splitlines() if not line.startswith(('ORANGE_REVIEW_LOGIN_PHONE=','ORANGE_REVIEW_LOGIN_CODE='))]
lines.extend([
    'ORANGE_REVIEW_LOGIN_PHONE=19866051435',
    'ORANGE_REVIEW_LOGIN_CODE=888888',
])
path.write_text('\n'.join(lines)+'\n')
PY
sudo chmod 600 /etc/orange-rewards/server.env
sudo systemctl restart orange-rewards
sudo systemctl is-active orange-rewards
