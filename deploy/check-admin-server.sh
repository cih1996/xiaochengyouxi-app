set -eu
sudo install -d -o ubuntu -g ubuntu -m 755 /opt/orange-rewards/verify-admin
tar -xzf /home/ubuntu/orange-server-check.tar.gz -C /opt/orange-rewards/verify-admin
mkdir -p /opt/orange-rewards/verify-admin/server/.cargo
cat > /opt/orange-rewards/verify-admin/server/.cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "/opt/orange-rewards/current/vendor"
EOF
cd /opt/orange-rewards/verify-admin/server
if ! cargo test --locked --offline --jobs 2 > /tmp/orange-check-admin.log 2>&1; then
    cat /tmp/orange-check-admin.log
    exit 1
fi
tail -n 55 /tmp/orange-check-admin.log
