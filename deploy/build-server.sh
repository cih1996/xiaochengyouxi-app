set -eu
tar -xzf /home/ubuntu/orange-server.tar.gz -C /opt/orange-rewards/current
cd /opt/orange-rewards/current/server
cargo build --release --locked --jobs 2 > /tmp/orange-build.log 2>&1
tail -n 20 /tmp/orange-build.log
