"""Register this business in DeviceTrust; private keys stay outside public artifacts."""
import json
from pathlib import Path
from urllib.request import Request, urlopen

root = Path(__file__).resolve().parents[1]
credentials = Path(r'C:\Users\Arlo\Documents\Codex\2026-11-10\device-trust-platform-work\outputs\DeviceTrust-线上登录凭据.json')
admin = json.loads(credentials.read_text(encoding='utf-8-sig'))
private = root / 'private/device-trust.json'

def call(path, data=None):
    request = Request(admin['console'].rstrip('/') + '/api/' + path,
                      data=None if data is None else json.dumps(data).encode(),
                      headers={'Authorization': 'Bearer ' + admin['adminToken'], 'Content-Type': 'application/json'},
                      method='GET' if data is None else 'POST')
    with urlopen(request, timeout=30) as response:
        return json.load(response)

if private.exists():
    config = json.loads(private.read_text(encoding='utf-8'))
else:
    apps = call('apps')
    if any(a['package'] == 'com.lingzidev.xiaochengyouxi' for a in apps):
        raise RuntimeError('Application already exists; restore its existing private key instead of rotating it')
    result = call('apps', {'name': '小橙有喜', 'package': 'com.lingzidev.xiaochengyouxi'})
    config = {'app_id': result['app']['id'], 'app_key': result['key'],
              'base_url': 'https://devicetrust-sdk-api.lingzidev.com',
              'spki_pins': ['AU7LGDAFJUgs4HkcpRf1O7/6OU9avVMvle6wTsAK5m4=']}
    private.write_text(json.dumps(config, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps({'app_id': config['app_id'], 'base_url': config['base_url'], 'private_config': str(private)}, ensure_ascii=False))
