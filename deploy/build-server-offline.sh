set -eu
# Replace the stalled registry fetch with the exact Cargo.lock source bundle.
build_pid=$(pgrep -u ubuntu -f '^cargo build --release --locked --jobs 2$' || true)
if [ -n "$build_pid" ]; then kill "$build_pid"; fi
tar -xzf /home/ubuntu/orange-vendor.tar.gz -C /opt/orange-rewards/current
mkdir -p /opt/orange-rewards/current/server/.cargo
cat > /opt/orange-rewards/current/server/.cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "../vendor"
EOF
cd /opt/orange-rewards/current/server
cargo build --release --locked --offline --jobs 2 > /tmp/orange-build.log 2>&1
tail -n 20 /tmp/orange-build.log
