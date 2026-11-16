set -eu
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
sudo install -m 644 /home/ubuntu/orange-service /etc/systemd/system/orange-rewards.service
sudo systemctl daemon-reload
sudo systemctl restart orange-rewards
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
sha256sum src/main.rs
tail -n 5 /tmp/orange-build.log
