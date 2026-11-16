set -eu
whoami
uname -a
cat /etc/os-release
printf '\n--- resources ---\n'
free -h
df -h /
printf '\n--- installed tools ---\n'
command -v nginx || true
command -v cargo || true
command -v rustc || true
command -v docker || true
command -v node || true
printf '\n--- services / ports ---\n'
sudo -n true
sudo ss -lntp
printf '\n--- existing web configuration ---\n'
sudo ls -la /etc/nginx/sites-enabled /opt /var/www 2>/dev/null || true
