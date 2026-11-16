set -eu
sudo -u orange sh -c 'umask 077; sqlite3 /var/lib/orange-rewards/orange.sqlite3 ".backup /var/lib/orange-rewards/before-invitations.sqlite3"'
sudo stat -c 'Database backup ready: %n (%s bytes, permissions %a)' /var/lib/orange-rewards/before-invitations.sqlite3
