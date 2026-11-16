"""Publish verified APK metadata and record the deployment audit."""
import hashlib
import json
import sqlite3
import sys
import time
import uuid
from pathlib import Path

metadata = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
artifact = Path(sys.argv[2])
if artifact.stat().st_size != metadata['apk_size_bytes']:
    raise ValueError('APK size does not match publication metadata')
if hashlib.sha256(artifact.read_bytes()).hexdigest() != metadata['apk_sha256']:
    raise ValueError('APK SHA256 does not match publication metadata')
db = sqlite3.connect('/var/lib/orange-rewards/orange.sqlite3')
with db:
    raw, revision = db.execute('SELECT config_json,revision FROM app_release WHERE id=1').fetchone()
    before = {**json.loads(raw), 'revision': revision}
    after = {**metadata, 'revision': revision + 1}
    stamp = int(time.time())
    db.execute('UPDATE app_release SET config_json=?,revision=?,updated_at=? WHERE id=1',
               (json.dumps(metadata, ensure_ascii=False), revision + 1, stamp))
    db.execute('''INSERT INTO admin_audit
        (id,user_id,actor,action,related_id,client_ip,request_id,reason,filters_json,before_json,after_json,http_status,created_at)
        VALUES(?,NULL,'deployment','app_release_update','app-release','127.0.0.1',?,?, '{}',?,?,200,?)''',
        (str(uuid.uuid4()), str(uuid.uuid4()), '发布已校验安装包 ' + metadata['latest_version_name'],
         json.dumps(before, ensure_ascii=False), json.dumps(after, ensure_ascii=False), stamp))
db.close()
print('Verified APK published:', metadata['latest_version_name'], 'revision', revision + 1)
