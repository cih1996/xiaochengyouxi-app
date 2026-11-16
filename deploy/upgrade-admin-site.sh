set -eu
stamp=$(date +%Y%m%d-%H%M%S)
sudo -u orange sqlite3 /var/lib/orange-rewards/orange.sqlite3 ".backup '/var/lib/orange-rewards/before-admin-site-${stamp}.sqlite3'"
sudo tar -xzf /home/ubuntu/orange-admin-update.tar.gz -C /opt/orange-rewards/current server/Cargo.toml server/Cargo.lock server/src
cd /opt/orange-rewards/current/server
if ! cargo build --release --locked --offline --jobs 2 > /tmp/orange-build-admin-site.log 2>&1; then
    cat /tmp/orange-build-admin-site.log
    exit 1
fi
sudo systemctl restart orange-rewards
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
sudo tar -xzf /home/ubuntu/orange-admin-update.tar.gz -C /opt/orange-rewards/current web/dist
sudo rm -f /opt/orange-rewards/current/web/dist/docs/PRODUCT-API.md
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
tail -n 4 /tmp/orange-build-admin-site.log
sudo systemctl is-active orange-rewards
