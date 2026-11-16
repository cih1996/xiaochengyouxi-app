set -eu
sudo python3 - <<'PY'
from pathlib import Path
path=Path('/etc/orange-rewards/server.env')
key='ORANGE_APP_API_SPKI_PINS'
value='leP8ENoVNTlFedrK3rN6UoodDYWghpr57islBR6PZAo='
lines=[line for line in path.read_text().splitlines() if line.split('=',1)[0]!=key]
lines.append(key+'='+value)
path.write_text('\n'.join(lines)+'\n'); path.chmod(0o600)
PY
