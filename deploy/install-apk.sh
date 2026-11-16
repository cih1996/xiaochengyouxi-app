set -eu
sudo install -m 644 /home/ubuntu/orange-release.apk /var/www/orange-downloads/orange-rewards.apk
sha256sum /var/www/orange-downloads/orange-rewards.apk
tail -n 8 /tmp/orange-build.log
