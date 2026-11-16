set -eu
ps -eo pid,etime,%cpu,%mem,args | grep -E 'cargo build|rustc|cc1' | grep -v grep || true
tail -n 30 /tmp/orange-build.log
curl -sS --connect-timeout 3 --max-time 8 https://index.crates.io/ar/go/argon2 -o /tmp/orange-index-check
wc -c /tmp/orange-index-check
