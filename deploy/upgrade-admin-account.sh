set -eu
snapshot=$(date +%Y%m%d-%H%M%S)
sudo -u orange sqlite3 /var/lib/orange-rewards/orange.sqlite3 ".backup '/var/lib/orange-rewards/before-admin-account-${snapshot}.sqlite3'"
sudo tar -xzf /home/ubuntu/orange-risk-source.tar.gz -C /opt/orange-rewards/current
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
sudo tar -xzf /home/ubuntu/orange-web.tar.gz -C /opt/orange-rewards/current
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
sudo systemctl restart orange-rewards
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
tail -n 3 /tmp/orange-build.log
sudo systemctl is-active orange-rewards
