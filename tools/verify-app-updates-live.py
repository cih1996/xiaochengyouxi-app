"""Verify public update metadata and the immutable APK download."""
import argparse
import hashlib
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import urlopen

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description='Verify the deployment against its publication metadata.')
parser.add_argument('release_file', type=Path)
args = parser.parse_args()
expected = json.loads(args.release_file.read_text(encoding='utf-8'))
base = 'https://orange-api.lingzidev.com/api'


def get(path):
    with urlopen(base + path, timeout=30) as response:
        assert response.headers.get('Cache-Control') == 'no-store'
        return json.load(response)


latest = expected['latest_version_code']
minimum = expected['min_supported_version_code']
boundaries = {0, max(0, latest - 1), latest, latest + 1, max(0, minimum - 1), minimum}
for current in sorted(boundaries):
    available = expected['enabled'] and current < latest
    result = get(f'/app/update?version_code={current}')
    assert result['update_available'] == available
    assert result['force_update'] == (available and current < minimum)
    for key, value in expected.items():
        assert result[key] == value, f'Publication mismatch: {key}'
try:
    get('/app/update?version_code=-1')
    raise AssertionError('Negative version accepted')
except HTTPError as error:
    assert error.code == 400
runtime = get('/app/runtime')
assert isinstance(runtime['revision'], int) and runtime['revision'] > 0
assert isinstance(runtime['hall_notice'], str) and runtime['updated_at']
for path in ['/admin/app-release', '/admin/app-runtime']:
    try:
        get(path)
        raise AssertionError('Admin route unprotected')
    except HTTPError as error:
        assert error.code == 401
digest = hashlib.sha256()
size = 0
with urlopen(expected['apk_url'], timeout=60) as response:
    while block := response.read(256 * 1024):
        digest.update(block)
        size += len(block)
assert size == expected['apk_size_bytes']
assert digest.hexdigest() == expected['apk_sha256']
summary = {'version': expected['latest_version_name'], 'version_code': expected['latest_version_code'],
           'apk_sha256': digest.hexdigest(), 'apk_size_bytes': size, 'public_checks_passed': True}
(root / 'output/app-updates-live-verification.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print('Published version boundaries, update requirements, protected admin routes and APK integrity passed.')
