set -eu
target=/opt/orange-rewards/current/web/dist/docs/PRODUCT-API.md
sudo rm -f "$target"
test ! -e "$target"
echo removed
