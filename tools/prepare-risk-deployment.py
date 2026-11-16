import json, secrets, tarfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
config = json.loads((root/'private/device-trust.json').read_text(encoding='utf-8'))
env_path = root/'private/server.env'
env = dict(line.split('=',1) for line in env_path.read_text(encoding='utf-8').splitlines() if line and not line.startswith('#'))
env.setdefault('ORANGE_PROXY_TOKEN',secrets.token_hex(32))
new = {'ORANGE_PROXY_TOKEN':env['ORANGE_PROXY_TOKEN'],
       'ORANGE_DEVICE_TRUST_APP_ID':config['app_id'],'ORANGE_DEVICE_TRUST_APP_KEY':config['app_key'],
       'ORANGE_DEVICE_TRUST_URL':config['base_url'],'ORANGE_DEVICE_TRUST_PINS':','.join(config['spki_pins'])}
env.update(new)
env_path.write_text(''.join(f'{k}={v}\n' for k,v in env.items()),encoding='utf-8')
(root/'private/risk-env-fragment').write_text(''.join(f'{k}={v}\n' for k,v in new.items()),encoding='utf-8')
(root/'private/proxy-token.conf').write_text('proxy_set_header X-Orange-Proxy-Token "'+env['ORANGE_PROXY_TOKEN']+'";\n',encoding='utf-8')
backup = Path(r'D:\工作\密钥备份\orange-rewards')
for name in ['server.env','device-trust.json']:
    (backup/name).write_bytes((root/'private'/name).read_bytes())
with tarfile.open(root/'output/orange-risk-source.tar.gz','w:gz') as tar:
    for file in ['server/Cargo.toml','server/Cargo.lock']:
        tar.add(root/file,arcname=file)
    tar.add(root/'server/src',arcname='server/src')
with tarfile.open(root/'output/orange-risk-vendor.tar.gz','w:gz') as tar:
    tar.add(root/'private/vendor',arcname='vendor')
print('Source, vendor and private environment prepared; credentials backed up.')
