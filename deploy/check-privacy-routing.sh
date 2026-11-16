set -eu
grep -n -A4 -B2 'privacy\|terms' /etc/nginx/sites-available/orange-rewards || true
