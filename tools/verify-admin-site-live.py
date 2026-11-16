"""Verify the published documentation and JSON API access boundaries."""
import hashlib
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request,urlopen

root=Path(__file__).resolve().parents[1]

def call(url,data=None,headers=None):
    try:
        with urlopen(Request(url,data,headers or {}),timeout=20) as response:
            return response.status,response.headers,response.read()
    except HTTPError as error:
        return error.code,error.headers,error.read()

for host in ['orange.lingzidev.com','orange-api.lingzidev.com']:
    origin='https://'+host
    code,headers,body=call(origin+'/docs/PRODUCT-API.md')
    assert code==404
    code,headers,body=call(origin+'/docs/product-api')
    assert code==200 and headers.get_content_type()=='text/html'
    for path in ['/api/integration/products','/api/integration/product-images']:
        payload=json.dumps({'image_base64':'iVBORw0KGgo='}).encode()
        code,_,_=call(origin+path,payload,{'Content-Type':'application/json','X-Product-Api-Key':'invalid'})
        assert code==401,(host,path,code)
    code,_,_=call(origin+'/api/admin/user-center/stats?days=30')
    assert code==401,(host,code)

source=(root/'docs/PRODUCT-API.md').read_bytes()
result={'documentation_sha256':hashlib.sha256(source).hexdigest(),'documentation_bytes':len(source),
        'markdown_public':False,'json_upload_protected':True,'user_center_protected':True}
(root/'output/admin-site-live-verification.json').write_text(json.dumps(result,indent=2))
print('Internal documentation boundary, JSON upload endpoints and user-center protection passed.')
