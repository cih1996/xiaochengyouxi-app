set -eu
cd /opt/orange-rewards/current
tar -xzf /home/ubuntu/orange-server.tar.gz
tar -xzf /home/ubuntu/orange-vendor.tar.gz
cd server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
cd ..
tar -xzf /home/ubuntu/orange-web.tar.gz
sudo install -m 644 /home/ubuntu/orange-nginx /etc/nginx/sites-available/orange-rewards
sudo nginx -t
sudo systemctl restart orange-rewards
sudo systemctl reload nginx
curl -fsS --retry 5 --retry-delay 1 --retry-connrefused http://127.0.0.1:38180/api/health
printf '\n'
sha256sum server/src/main.rs server/src/invitations.rs
tail -n 6 /tmp/orange-build.log
