set -eu
sudo tail -n 12 /tmp/orange-build-admin-site.log 2>/dev/null || true
sudo systemctl is-active orange-rewards || true
curl -sS http://127.0.0.1:38180/api/health || true
