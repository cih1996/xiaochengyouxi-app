set -eu
tar -xzf /home/ubuntu/orange-web.tar.gz -C /opt/orange-rewards/current
find /opt/orange-rewards/current/web/dist -type d -exec chmod 755 {} +
find /opt/orange-rewards/current/web/dist -type f -exec chmod 644 {} +
curl -sS -I --resolve orange.lingzidev.com:443:127.0.0.1 https://orange.lingzidev.com/
curl -sS -I --resolve orange.lingzidev.com:443:127.0.0.1 https://orange.lingzidev.com/admin
tail -n 15 /tmp/orange-build.log
