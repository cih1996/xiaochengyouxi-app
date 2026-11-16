"""Package only deployable application files; never include private configuration."""
import argparse, hashlib, json, tarfile
from pathlib import Path
parser=argparse.ArgumentParser(description='Generate deployment archives and publication metadata for an explicit release.')
parser.add_argument('version')
parser.add_argument('version_code',type=int)
parser.add_argument('sdk_version')
parser.add_argument('release_notes')
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
version=args.version
version_code=args.version_code
sdk_version=args.sdk_version
apk=root/f'output/小橙有喜-{version}-release.apk'
apk_hash=hashlib.sha256(apk.read_bytes()).hexdigest()
with tarfile.open(root/'output/orange-risk-source.tar.gz','w:gz') as tar:
    for file in ['server/Cargo.toml','server/Cargo.lock','server/src']:
        tar.add(root/file,arcname=file)
with tarfile.open(root/'output/orange-web.tar.gz','w:gz') as tar:
    tar.add(root/'web/dist',arcname='web/dist')
manifest={'name':'小橙有喜','version':version,'version_code':version_code,
    'package':'com.lingzidev.xiaochengyouxi','apk_sha256':apk_hash,
    'signing_sha256':'E8:C6:93:62:5B:A7:88:F0:69:BD:77:4C:83:8A:58:99:04:6E:1E:F1:83:75:E3:D9:72:62:52:99:36:EF:E0:D3',
    'admin_url':'https://orange.lingzidev.com/admin','apk_url':'https://orange.lingzidev.com/download/orange-rewards.apk',
    'sdk':'DeviceTrust '+sdk_version,'hardware_attestation_enabled':False}
(root/f'output/小橙有喜-{version}-交付.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
release={'enabled':True,'latest_version_code':version_code,'latest_version_name':version,
    'min_supported_version_code':0,'title':'小橙有喜有新版本啦',
    'release_notes':args.release_notes,
    'apk_url':f'https://orange.lingzidev.com/download/xiaochengyouxi-{version}-{version_code}.apk',
    'apk_sha256':manifest['apk_sha256'],'apk_size_bytes':apk.stat().st_size}
(root/f'output/小橙有喜-{version}-更新发布.json').write_text(json.dumps(release,ensure_ascii=False,indent=2),encoding='utf-8')
print('Deployable source/web archives, delivery manifest and update publication metadata prepared.')
