set -eu
snapshot=$(date +%Y%m%d-%H%M%S)
sudo -u orange sqlite3 /var/lib/orange-rewards/orange.sqlite3 ".backup '/var/lib/orange-rewards/before-user-investigation-${snapshot}.sqlite3'"
sudo install -d -m 700 /opt/orange-rewards/backups
sudo cp /etc/nginx/sites-available/orange-rewards "/opt/orange-rewards/backups/nginx-before-investigation-${snapshot}.conf"
sudo cp /etc/orange-rewards/server.env "/opt/orange-rewards/backups/env-before-investigation-${snapshot}"
sudo tar -xzf /home/ubuntu/orange-risk-source.tar.gz -C /opt/orange-rewards/current
sudo tar -xzf /home/ubuntu/orange-risk-vendor.tar.gz -C /opt/orange-rewards/current
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
sudo tar -xzf /home/ubuntu/orange-web.tar.gz -C /opt/orange-rewards/current
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
sudo install -m 644 /home/ubuntu/orange-release.apk /var/www/orange-downloads/orange-rewards.apk
sudo python3 - <<'PY'
from pathlib import Path
p=Path('/etc/orange-rewards/server.env')
env=dict(line.split('=',1) for line in p.read_text().splitlines() if line and not line.startswith('#'))
fragment=Path('/home/ubuntu/orange-risk-env')
env.update(dict(line.split('=',1) for line in fragment.read_text().splitlines() if line))
p.write_text(''.join(f'{k}={v}\n' for k,v in env.items()))
p.chmod(0o600)
fragment.unlink()
PY
sudo install -m 600 /home/ubuntu/orange-proxy-token /etc/orange-rewards/proxy-token.conf
sudo install -m 644 /home/ubuntu/orange-nginx /etc/nginx/sites-available/orange-rewards
sudo nginx -t
sudo systemctl restart orange-rewards
sudo systemctl reload nginx
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
tail -n 3 /tmp/orange-build.log
