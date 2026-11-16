set -eu
sudo tar -xzf /home/ubuntu/orange-web.tar.gz -C /opt/orange-rewards/current
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
sudo install -m 644 /home/ubuntu/orange-release.apk /var/www/orange-downloads/orange-rewards.apk
curl -fsS --resolve orange.lingzidev.com:443:127.0.0.1 https://orange.lingzidev.com/ > /dev/null
sha256sum /var/www/orange-downloads/orange-rewards.apk
