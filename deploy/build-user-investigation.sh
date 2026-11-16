set -eu
sudo tar -xzf /home/ubuntu/orange-risk-source.tar.gz -C /opt/orange-rewards/current
sudo tar -xzf /home/ubuntu/orange-risk-vendor.tar.gz -C /opt/orange-rewards/current
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
tail -n 3 /tmp/orange-build.log
