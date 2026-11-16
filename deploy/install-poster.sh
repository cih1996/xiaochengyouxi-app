set -eu
sudo install -m 644 /home/ubuntu/invite-platform.png /opt/orange-rewards/current/web/dist/assets/invite-platform.png
curl -fsS -I --resolve orange.lingzidev.com:443:127.0.0.1 https://orange.lingzidev.com/assets/invite-platform.png
