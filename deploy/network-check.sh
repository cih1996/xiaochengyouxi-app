set -eu
curl -I --connect-timeout 5 --max-time 10 https://index.crates.io/config.json
sudo ss -lntp
