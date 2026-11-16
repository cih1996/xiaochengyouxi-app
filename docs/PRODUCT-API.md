# 商品上传接口

上传商品资料与图片后，商品按自己的开始、到期时间进入商品库，客户端展示预计金币并跳转购物平台。支持 JSON 一步上传、先上传图片再引用，以及 multipart 文件上传。本接口维护商品资料，不创建购物订单或结算金币。

## 地址与密钥

```text
POST https://orange-api.lingzidev.com/api/integration/products
X-Product-Api-Key: <商品API密钥>
Content-Type: application/json
```

在后台商品库生成并查看专用商品 API 密钥；轮换后旧密钥立即失效。该密钥只用于商品及图片上传，不具备后台登录、账号、金币或提现权限。上传程序从服务端环境变量 PRODUCT_API_KEY 读取密钥。

## JSON 一步上传

五项业务内容必须提供：商品名称、商品链接、价格、佣金比例、图片。

| 字段 | 类型与内容 |
| --- | --- |
| title | 字符串，商品名称，非空，最多 200 字 |
| url | 字符串，HTTP/HTTPS 链接，不接受自定义 scheme 或含用户名密码的链接 |
| price | 元，十进制字符串或普通 JSON 数字，最多两位小数，例如 `"100.00"`、`99.99`；建议用字符串 |
| commission_percent | 百分比，十进制字符串或普通 JSON 数字，0–100，最多两位小数，例如 `"50"` 表示 50% |
| image_base64 | 图片字节的标准 Base64 字符串，或 `data:image/png;base64,...` 等图片 data URL |
| image_id | 可替代 image_base64，先上传图片接口返回的本机登记 UUID |

image_base64 与 image_id 必须二选一。图片解码后必须为 PNG、JPEG 或 WebP，大于 0 且不超过 **5MiB（5242880 字节）**；类型按真实字节判断。Base64 使用标准字母表与填充，不使用 URL-safe Base64，不包含额外换行。JSON 总请求体上限为 **8MiB**。

数值按十进制原文精确解析，拒绝超过两位小数和科学计数法。服务端不会将价格先转换为浮点数再舍入。

每件商品分别指定有效期：

| 可选字段 | 类型与规则 |
| --- | --- |
| external_id | 上传方稳定商品编号；相同编号更新同一件商品，保留商品 ID；省略则每次新增 |
| platform | 字符串，例如淘宝、京东；新建默认空 |
| category | 字符串，例如日用、家居；新建默认空 |
| starts_at | RFC3339 时间，可精确到秒；新建省略时使用服务器当前时间 |
| expires_at | RFC3339 时间，必须晚于 starts_at；建议每件商品显式填写自己的到期时间 |
| enabled | JSON 布尔值 true/false；新建默认 true |

例如甲商品在 `2027-01-15T10:23:45+08:00` 到期，乙商品可在同日 `18:00:00+08:00` 到期，分别保存与判断。每件商品独立修改自己的有效期。真正新建且省略 expires_at 的兼容调用，使用 starts_at 加默认天数（初始 7 天）；已有商品沿用各自保存的到期时间。

相同 external_id 重报仍需完整主字段和一项图片，省略的 starts_at、expires_at、enabled、platform、category **保留原商品值**。显式传入上述字段修改相应值。后台 PATCH 继续局部修改，未传字段保留。

将下面内容保存为 product.json，替换图片 Base64、链接与该商品时间：

```json
{
  "title": "日用好物",
  "url": "https://example.com/item/123",
  "price": "100.00",
  "commission_percent": "50",
  "image_base64": "<图片文件的标准Base64>",
  "external_id": "shop-123",
  "platform": "淘宝",
  "category": "日用",
  "starts_at": "2027-01-15T10:00:00+08:00",
  "expires_at": "2027-01-15T10:23:45+08:00",
  "enabled": true
}
```

```bash
curl --fail-with-body \
  'https://orange-api.lingzidev.com/api/integration/products' \
  -H "X-Product-Api-Key: ${PRODUCT_API_KEY}" \
  -H 'Content-Type: application/json' \
  --data-binary @product.json
```

## Python：直接从图片字节上传

只使用 Python 标准库，无需安装依赖。图片也可以直接来自程序内存中的 bytes。示例给这件商品单独设置 2 小时 15 分钟有效期。

```python
import base64
import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.request import Request, urlopen

BASE = "https://orange-api.lingzidev.com/api/integration"

def post(path, data):
    request = Request(
        BASE + path,
        data=json.dumps(data, ensure_ascii=False).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "X-Product-Api-Key": os.environ["PRODUCT_API_KEY"],
        },
        method="POST",
    )
    with urlopen(request, timeout=30) as response:
        return json.load(response)

now = datetime.now(timezone.utc).replace(microsecond=0)
image_bytes = Path("product.png").read_bytes()
product = {
    "title": "日用好物",
    "url": "https://example.com/item/123",
    "price": "100.00",
    "commission_percent": "50",
    "image_base64": base64.b64encode(image_bytes).decode("ascii"),
    "external_id": "shop-123",
    "platform": "淘宝",
    "category": "日用",
    "starts_at": now.isoformat(),
    "expires_at": (now + timedelta(hours=2, minutes=15)).isoformat(),
}
print(post("/products", product))
```

## 先上传图片，再提交商品

```text
POST /api/integration/product-images
X-Product-Api-Key: <同一个商品API密钥>
Content-Type: application/json

{"image_base64":"<标准Base64或图片data URL>"}
```

成功返回 HTTP 200：

```json
{
  "image_id": "12345678-1234-4234-8234-123456789abc",
  "image": "/media/products/12345678-1234-4234-8234-123456789abc.png",
  "content_type": "image/png",
  "size_bytes": 428620
}
```

在上面的 Python 示例中，用以下片段替换最后一行的一步上传：

```python
uploaded = post("/product-images", {"image_base64": product.pop("image_base64")})
product["image_id"] = uploaded["image_id"]
print(post("/products", product))
```

图片 ID 必须在本服务登记且文件仍存在；外链、媒体路径、任意 UUID 或已经丢失的文件会明确报错。图片可复用于多件商品。独立图片上传仅记录图片与安全审计，不新增商品或金币。

## 兼容 multipart 文件上传

原有 multipart 继续支持，字段含义和逐件有效期相同。图片字段名为 image，内容为真实文件字节；数字和 enabled 均通过表单字符串传递。

```bash
curl --fail-with-body \
  'https://orange-api.lingzidev.com/api/integration/products' \
  -H "X-Product-Api-Key: ${PRODUCT_API_KEY}" \
  -F 'title=日用好物' \
  -F 'url=https://example.com/item/123' \
  -F 'price=100.00' \
  -F 'commission_percent=50' \
  -F 'image=@./product.png;type=image/png' \
  -F 'external_id=shop-123' \
  -F 'starts_at=2027-01-15T10:00:00+08:00' \
  -F 'expires_at=2027-01-15T10:23:45+08:00'
```

curl 自动生成 multipart boundary，不要另写固定 Content-Type。该路径不能把图片 URL 当作文件发送。

## 商品响应与金币计算

成功返回 HTTP 200 商品对象；以下为 100 元、50% 佣金、平台分成 20% 的例子：

```json
{
  "id": 123,
  "title": "日用好物",
  "url": "https://example.com/item/123",
  "price": 100.0,
  "price_text": "100.00",
  "price_fen": 10000,
  "commission_percent": 50.0,
  "commission_bps": 5000,
  "reward_coins": 40000,
  "commission": 40.0,
  "image": "/media/products/12345678-1234-4234-8234-123456789abc.png",
  "platform": "淘宝",
  "category": "日用",
  "external_id": "shop-123",
  "starts_at": "2027-01-15T02:00:00Z",
  "expires_at": "2027-01-15T02:23:45Z",
  "server_now": "2027-01-15T02:01:00Z",
  "seconds_until_start": 0,
  "seconds_until_expiry": 1365,
  "enabled": true,
  "status": "active",
  "revision": 1
}
```

平台分成初始 20%，系统兑换比例为 1000 金币/元，复用账号余额的 COINS_PER_YUAN 常量。价格按分、比例按万分比保存，使用 i128 计算，只在最后向下取整：

```text
reward_coins = floor(
  price_fen × commission_bps × (10000 - platform_share_bps) × coins_per_yuan
  / (100 × 10000 × 10000)
)
```

100 元相同条件为 40000 金币；99.99 元为 39996 金币。commission 仅兼容旧客户端的预计奖励元，消费者页面直接使用 reward_coins。调整平台分成后，后续商品查询按新比例计算，不影响账号余额或商品有效期。

时间统一返回 UTC ISO，seconds_until_* 为按服务端当前时间计算的非负整数秒。有效条件为 enabled=true 且 starts_at ≤ 服务端时间 < expires_at，到期这一秒即失效。图片路径相对 API origin，响应提供真实 Content-Type 与 X-Content-Type-Options:nosniff。

## 后台与公开读取

### 点击、曝光与统计

目录读取本身不写曝光。客户端在商品卡片或详情真正展示时可调用：

```text
POST /api/products/{id}/impression
Authorization: Bearer <可选>
{}

POST /api/products/{id}/click
Authorization: Bearer <可选>
{"kind":"detail"或"outbound"}
```

服务端只接受当前有效商品；失效商品返回 404。事件只记录统计，不创建订单、不发金币。匿名事件保留不可逆 session hash，管理员不查看原始 IP。CTR 口径固定为 `outbound_clicks / detail_opens`，这是跳转相对详情打开的转化率，不代表广告曝光率；没有目录曝光时不虚构曝光数据。

| 接口 | 用途 |
| --- | --- |
| `GET /api/admin/products/analytics?days=7|30|90&product_id=` | 返回 `{days,since,definition,summary,summary_ctr,products,impressions}`；按商品列出 details、outbound、unique_users、impressions、ctr；可选 product_id 只查看单件 |
| `GET /api/products/runtime` | `{hero_image,revision}`；默认 `hero_image:null`，空值时客户端隐藏捡漏顶部图 |
| `GET/PUT /api/admin/products/display-config` | 管理员读取或设置 `{image_id}`；image_id 必须是本服务已登记的图片 UUID，传 null 清空，响应同 runtime |

统计中的 `impressions` 仅来自客户端显式上报；`unique_users` 只统计已登录用户，匿名访问不伪造用户数。统计事件按服务器时间记录，后台按近 7/30/90 天筛选。

后台接口沿用 x-admin-token 数据库会话鉴权。

| 接口 | 用途 |
| --- | --- |
| `GET /api/admin/products?q=&status=all&page=1&page_size=20` | `{items,total,page,page_size}`；状态 all/active/scheduled/expired/disabled，每页最多 100 条 |
| `POST /api/admin/products` | 原有完整 multipart 上传表单 |
| `PATCH /api/admin/products/{id}` | 局部 multipart，分别调整本商品的文字、金额、图片、enabled 与起止时间 |
| `POST /api/admin/products/images` | 后台 JSON `{image_base64}` 上传，返回相同图片 DTO |
| `GET/PUT /api/admin/products/config` | `{platform_share_percent,default_valid_days,revision}`；默认天数仅用于新建时省略到期时间的兼容调用 |
| `GET /api/admin/products/api-key` | `{enabled,key,header_name,endpoint,created_at,rotated_at}`；未生成时 key/时间为 null |
| `POST /api/admin/products/api-key` | 生成或轮换，空请求即可；旧密钥失效 |
| `GET /api/products` | 当前有效上架商品数组，空库为 `[]` |
| `GET /api/products/{id}` | 当前有效商品详情，无效为 404；客户端跳转前重新读取 |

配置分成允许 0–100%、最多两位小数，兼容默认天数为 1–365 天。大厅只选有效商品；空库不发伪造商品，不扣主动请求次数。商品、图片、配置与密钥操作均有安全审计；图像 Base64、文件字节和专用密钥不写审计。外部上传 actor 为 product-api，后台操作记录真实管理员。

## 错误与部署

业务错误统一为 `{"error":"具体原因"}`：

| HTTP | 含义 |
| --- | --- |
| 400 | 错误 JSON、Base64、图片、金额、字段、时间，缺少图片，图片两种方式冲突或 image_id 无效 |
| 401 | 专用密钥或后台会话无效 |
| 404 | 公开商品已下架、未开始、到期或不存在 |
| 409 | 后台将 external_id 改为另一件商品已经使用的编号 |
| 413 | JSON 总请求体超过 8MiB 或代理请求体上限 |
| 500 | 数据库或文件系统错误 |

例如：`{"error":"image_base64与image_id只能选择一项"}`、`{"error":"image_id不存在，请先上传图片"}`。

SQLite 保存商品、图片登记与配置，图片文件位于数据库所在目录的 product-media。备份同时保留数据库和该目录。代理商品与图片上传端点的请求体上限为 8MiB，并转发 `/media/products/`；图片使用独立 UUID 文件名，旧图片清理留作存储维护。商品 API 响应 no-store，图片文件使用 immutable 缓存。

测试：`cargo test --manifest-path server/Cargo.toml product_admin::tests`，覆盖真实 JSON/图片字节上传、登记引用、非法输入、逐件有效期隔离、重复上传不复活、精确金额与 multipart 兼容；测试使用临时数据库，不写生产商品。
