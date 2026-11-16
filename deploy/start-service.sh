set -eu
sudo systemctl start orange-rewards
sudo systemctl --no-pager --full status orange-rewards | head -n 16
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
curl -fsS http://127.0.0.1:38180/api/config
printf '\n'
sha256sum /opt/orange-rewards/current/server/src/main.rs
