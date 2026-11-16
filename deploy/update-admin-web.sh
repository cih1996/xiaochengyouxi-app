set -eu
sudo tar -xzf /home/ubuntu/orange-admin-update.tar.gz -C /opt/orange-rewards/current web/dist
sudo find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
sudo find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
