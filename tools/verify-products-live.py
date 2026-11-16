"""Verify public product routes and admin isolation without creating fake inventory."""
import json
import uuid
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request,urlopen

root=Path(__file__).resolve().parents[1]
checks=[]

def call(base,path,method='GET',data=None,headers=None):
    try:
        with urlopen(Request(base+path,data,headers or {},method=method),timeout=20) as response:
            return response.status,response.headers,response.read()
    except HTTPError as error:
        return error.code,error.headers,error.read()

for host in ['orange-api.lingzidev.com','orange.lingzidev.com']:
    base='https://'+host
    code,headers,body=call(base,'/api/products')
    assert code==200 and isinstance(json.loads(body),list)
    checks.append({'host':host,'public_catalog':code,'count':len(json.loads(body))})
    for path in ['/api/admin/products','/api/admin/products/config','/api/admin/products/api-key']:
        code,_,_=call(base,path)
        assert code==401,(path,code)
    for path in ['/api/products/9223372036854775807','/media/products/'+str(uuid.uuid4())+'.png']:
        code,headers,body=call(base,path)
        assert code==404 and headers.get_content_type()=='application/json',(path,code)
    boundary='orange-'+uuid.uuid4().hex
    data=('--'+boundary+'\r\nContent-Disposition: form-data; name="title"\r\n\r\nUnauthorized probe\r\n--'+boundary+'--\r\n').encode()
    code,_,_=call(base,'/api/integration/products','POST',data,{'Content-Type':'multipart/form-data; boundary='+boundary,'X-Product-Api-Key':'invalid'})
    assert code==401,code

(root/'output/products-live-verification.json').write_text(json.dumps({'checks':checks,'admin_protected':True,'integration_wrong_key':401,'fake_products_created':False},indent=2))
print('Both public catalogs, product/media routing and protected admin/upload endpoints passed.')
