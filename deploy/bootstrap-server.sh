set -eu
export DEBIAN_FRONTEND=noninteractive
sudo apt-get update > /tmp/orange-bootstrap.log 2>&1
sudo apt-get install -y --no-install-recommends nginx cargo rustc build-essential pkg-config libssl-dev ca-certificates sqlite3 >> /tmp/orange-bootstrap.log 2>&1
if ! id orange >/dev/null 2>&1; then
  sudo useradd --system --home-dir /var/lib/orange-rewards --shell /usr/sbin/nologin orange
fi
sudo install -d -m 755 /opt/orange-rewards/current /var/www/orange-downloads /etc/orange-rewards
sudo install -d -m 750 -o orange -g orange /var/lib/orange-rewards
sudo install -d -m 700 /etc/nginx/ssl/orange.lingzidev.com /etc/nginx/ssl/orange-api.lingzidev.com
sudo chown ubuntu:ubuntu /opt/orange-rewards/current
rustc --version
cargo --version
nginx -v
tail -n 15 /tmp/orange-bootstrap.log
