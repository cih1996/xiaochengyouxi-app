"""Check production admin login without changing the administrator's credentials."""
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

root = Path(__file__).resolve().parents[1]
access = json.loads((root / 'private/deployment-access.json').read_text(encoding='utf-8-sig'))
base = 'https://orange.lingzidev.com/api'


def call(path, method='GET', body=None, token=None):
    headers = {'Content-Type': 'application/json'}
    if token:
        headers['x-admin-token'] = token
    request = Request(base + path, method=method, headers=headers,
                      data=json.dumps(body).encode() if body is not None else None)
    try:
        with urlopen(request, timeout=30) as response:
            return response.status, json.load(response)
    except HTTPError as error:
        return error.code, error.read().decode()


status, config = call('/config')
assert status == 200 and config['admin_auth_required']
assert call('/admin/account')[0] == 401
assert call('/admin/login', 'POST', {'password': access['admin_password']})[0] == 422
assert call('/admin/login', 'POST', {
    'username': access['admin_username'] + '_incorrect',
    'password': access['admin_password'],
})[0] == 401
env = dict(line.split('=', 1) for line in (root / 'private/server.env').read_text(encoding='utf-8-sig').splitlines()
           if line and not line.startswith('#'))
if env.get('ORANGE_ADMIN_TOKEN'):
    assert call('/admin/account', token=env['ORANGE_ADMIN_TOKEN'])[0] == 401
status, login = call('/admin/login', 'POST', {
    'username': access['admin_username'], 'password': access['admin_password'],
})
assert status == 200, f'Login status: {status}'
token = login['token']
assert token != env.get('ORANGE_ADMIN_TOKEN')
try:
    status, account = call('/admin/account', token=token)
    assert status == 200 and account['username'] == access['admin_username']
    assert set(account) == {'username', 'auth_required', 'session_expires_at'}
    assert call('/admin/overview', token=token)[0] == 200
finally:
    assert call('/admin/logout', 'POST', token=token)[0] == 200
assert call('/admin/account', token=token)[0] == 401
print('Production admin checks passed: username/password login, protected account, old token denied, logout revoked.')
