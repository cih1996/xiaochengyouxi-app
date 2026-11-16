set -eu
sudo ss -lntp
sudo ufw status 2>/dev/null || true
sudo nft list ruleset 2>/dev/null || true
curl -I --connect-timeout 3 --max-time 5 --resolve orange.lingzidev.com:443:127.0.0.1 https://orange.lingzidev.com/
tail -n 12 /tmp/orange-build.log
