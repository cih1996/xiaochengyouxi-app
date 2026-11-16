set -eu
stamp=$(date +%Y%m%d-%H%M%S)
sudo cp /etc/nginx/sites-available/orange-rewards "/etc/nginx/sites-available/orange-rewards.before-products-${stamp}"
sudo python3 - <<'PY'
from pathlib import Path
import re
path=Path('/etc/nginx/sites-available/orange-rewards')
raw=path.read_text()
marker='    location ^~ /api/mail-sso/ { return 404; }'
for location in ['= /api/integration/products','= /api/integration/product-images','^~ /api/admin/products','^~ /media/products/']:
    block='''    location %s {
        client_max_body_size 8m;
        proxy_pass http://127.0.0.1:38180;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        include /etc/orange-rewards/proxy-token.conf;
        proxy_set_header X-Forwarded-Proto https;
    }
''' % location
    pattern=r'    location '+re.escape(location)+r' \{\n.*?\n    \}\n'
    if re.search(pattern,raw,re.S):
        raw=re.sub(pattern,lambda _:block,raw,flags=re.S)
    else:
        raw=raw.replace(marker,block+marker)
docs='''    location = /docs/product-api { try_files /index.html =404; }
    location = /docs/PRODUCT-API.md {
        types { text/markdown md; }
        charset utf-8;
        add_header Content-Disposition 'attachment; filename="PRODUCT-API.md"';
        add_header X-Content-Type-Options nosniff;
        add_header Cache-Control no-cache;
        try_files $uri =404;
    }
'''
if 'location = /docs/product-api ' not in raw:
    raw=raw.replace(marker,docs+marker)
if raw.count(marker)!=2:
    raise RuntimeError('Expected both Orange HTTPS servers')
path.write_text(raw)
PY
sudo nginx -t
sudo systemctl reload nginx
