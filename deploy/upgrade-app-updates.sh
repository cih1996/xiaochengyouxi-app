set -eu
apk_filename=$(python3 - <<'PY'
import json, re
from urllib.parse import urlsplit
with open('/home/ubuntu/orange-app-release.json',encoding='utf-8') as source:
    release=json.load(source)
path=urlsplit(release['apk_url']).path
filename=path.rsplit('/',1)[-1]
if path != '/download/'+filename or not re.fullmatch(r'xiaochengyouxi-[0-9A-Za-z._-]+-[0-9]+\.apk',filename):
    raise ValueError('Invalid versioned APK download path')
print(filename)
PY
)
apk_target="/var/www/orange-downloads/$apk_filename"
if sudo test -f "$apk_target" && ! sudo cmp -s /home/ubuntu/orange-release.apk "$apk_target"; then
    printf '%s\n' 'Versioned APK already exists with different content; increment the version.' >&2
    exit 1
fi
snapshot=$(date +%Y%m%d-%H%M%S)
sudo -u orange sqlite3 /var/lib/orange-rewards/orange.sqlite3 ".backup '/var/lib/orange-rewards/before-app-updates-${snapshot}.sqlite3'"
sudo tar -xzf /home/ubuntu/orange-risk-source.tar.gz -C /opt/orange-rewards/current
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
sudo install -m 644 /home/ubuntu/orange-release.apk "$apk_target"
sudo install -m 644 /home/ubuntu/orange-release.apk /var/www/orange-downloads/orange-rewards.apk
sudo install -m 644 /home/ubuntu/orange-app-release.json /opt/orange-rewards/current/app-release.json
sudo install -m 644 /home/ubuntu/orange-publish-release.py /opt/orange-rewards/current/publish-app-release.py
sudo tar -xzf /home/ubuntu/orange-web.tar.gz -C /opt/orange-rewards/current
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
sudo systemctl restart orange-rewards
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
sudo -u orange python3 /opt/orange-rewards/current/publish-app-release.py /opt/orange-rewards/current/app-release.json "$apk_target"
tail -n 3 /tmp/orange-build.log
sudo systemctl is-active orange-rewards
