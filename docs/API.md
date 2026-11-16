# API 与运行配置

营销活动与幸运转盘接口见 [CAMPAIGN-API.md](CAMPAIGN-API.md)。

默认服务地址：`http://127.0.0.1:38180`。Rust + Axum，数据库使用 SQLite；前端构建文件由同一个服务提供。广告支持本地 `mock` 和穿山甲 `csj` 两种模式；短信与提现当前为演示模式，商品库接受真实商品资料与图片上传。

本地演示账号：`13800138000` / `Orange123!`，初始余额 2680 金币。新账号从 0 金币开始。生产环境首次启动仅创建表与配置，账号通过注册接口建立。

## 运行

在 `server` 目录执行 `cargo run`。或者运行编译产物 `server/target/debug/orange-rewards-server.exe`，工作目录不限。

| 环境变量 | 默认值 |
| --- | --- |
| `ORANGE_BIND` | `127.0.0.1:38180` |
| `ORANGE_DB_PATH` | `server/data/orange.sqlite3` 的绝对路径 |
| `ORANGE_WEB_DIST` | `web/dist` 的绝对路径 |
| `ORANGE_ENV` | `local`；生产设置 `production` |
| `ORANGE_AD_MODE` | `mock`；生产设置 `csj` |
| `CSJ_REWARD_SECRET` | 穿山甲广告位 Security Key，`csj` 模式必填 |
| `ORANGE_ADMIN_USERNAME` | 首次创建后台账号的名称，默认 `admin`；已有 SQLite 账号时不覆盖 |
| `ORANGE_ADMIN_PASSWORD` | 首次创建后台账号的引导密码；已有 SQLite 账号时不覆盖 |
| `ORANGE_MAIL_SSO_SECRET` | 可选，邮件内部票据兑换密钥，仅后端与本机 Roundcube 插件共用 |
| `ORANGE_MAIL_USERNAME` | 可选，后台邮件服务账号，只有内部票据兑换响应提供给本机 PHP |
| `ORANGE_MAIL_PASSWORD` | 可选，邮件服务账号密码，不进入浏览器或公开配置 |
| `ORANGE_MAIL_TENCENT_CALLBACK_TOKEN` | 可选，腾讯邮件事件回调 URL 的独立随机 token；配合 `ORANGE_MAIL_USERNAME` 启用 |
| `ORANGE_CORS_ORIGINS` | 逗号分隔的允许来源；默认本地 5173、38180 |
| `ORANGE_APP_DOWNLOAD_URL` | 可选，公开 APK 下载地址 |
| `ORANGE_PRIVACY_URL` | 默认 `/privacy` |
| `ORANGE_INVITE_BASE_URL` | 默认 `https://orange.lingzidev.com`，用于个性化邀请链接 |
| `ORANGE_PROXY_TOKEN` | 服务与本机 Nginx 共用的私有随机值，用于确认覆盖的真实 IP 头 |
| `ORANGE_DEVICE_TRUST_APP_ID` | 可选，启用设备证明集成时配置 |
| `ORANGE_DEVICE_TRUST_APP_KEY` | 设备证明平台服务端密钥，随 APP_ID 配套配置 |
| `ORANGE_DEVICE_TRUST_URL` | 设备证明平台 HTTPS 地址 |
| `ORANGE_DEVICE_TRUST_PINS` | 平台公开 SPKI pin，逗号分隔 |
| `ORANGE_DEVICE_TRUST_MODE` | 业务执行授权使用 `enforce`；配置其他值直接拒绝启动 |

路径默认按 Cargo manifest 所在目录解析。退出和重启保留账号、会话、余额及流水。进程从环境变量读取配置；生产通过 systemd 的私有 EnvironmentFile 注入秘密。`production` 要求 `csj` 模式与穿山甲回调密钥；首次启动还需要后台引导密码。已有数据库管理员时可移除引导字段，继续使用 SQLite 中的凭据和会话鉴权。服务监听 loopback，由 Nginx 提供 HTTPS 入口。

`CSJ_REWARD_SECRET` 仅服务端使用。广告应用 ID 与广告位 ID通过领奖接口提供给客户端，回调密钥与管理凭据不出现在公开配置、APK 或前端构建文件中。

受保护接口携带 `Authorization: Bearer <token>`。会话有效期 30 天；退出删除当前会话，重置密码删除该账号全部会话。密码使用独立随机 salt 的 Argon2 哈希，不保存明文。

成功响应为表中原始对象或数组；业务错误响应为 `{"error":"具体原因"}`，HTTP 状态：400 参数/条件不满足，401 未登录或凭据错误，403 待绑定邀请关系，404 记录不存在，409 重复操作，500 数据库等内部错误。邀请门槛错误固定为 `{"error":"请先绑定邀请码","code":"INVITE_REQUIRED"}`。未知 `/api/*` 返回 JSON 404，不返回前端页面。

## 账号

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `GET /api/health` | 无 | `{ok:true}` |
| `GET /api/config` | 无 | `{app_name:"小橙有喜",android_package:"com.lingzidev.xiaochengyouxi",environment,admin_auth_required,ad_mode,sms_mode:"mock",payment_mode:"mock",invite_required:true,invite_sms_required:false,device_trust_enabled,app_download_url?,privacy_url?}` |
| `POST /api/auth/sms` | `{phone,purpose:"register"或"reset"}` | `{code:"123456",expires_in:300,mode:"mock"}` |
| `POST /api/auth/register` | `{phone,password,code,nickname?}` | `{token,user}` |
| `POST /api/auth/login` | `{phone,password}` | `{token,user}` |
| `POST /api/auth/reset-password` | `{phone,code,password}` | `{ok:true}` |
| `POST /api/auth/logout` | 需登录 | `{ok:true}` |
| `GET /api/me` | 需登录 | `user` |

手机号使用 11 位大陆手机号形式，密码至少 8 位。注册/找回须先请求对应用途验证码，5 分钟有效，使用后删除；`reset-password` 也接受为验证码用途别名。

`user`：

```json
{
  "id": 1,
  "phone": "13800138000",
  "nickname": "小橙同学",
  "coins": 2680,
  "total_earned": 2680,
  "today_earned": 2680,
  "cash_yuan": 2.68,
  "withdraw_min_yuan": 1.0,
  "checked_in_today": false,
  "invite_code": "AB7C8D",
  "invitation_required": false,
  "inviter_code": "EF4G5H"
}
```

1000 金币折合 1 元，金额展示不代表真实打款。`total_earned`、`today_earned` 统计奖励，不包含提现退回。每日按北京时间零点切分。

`checked_in_today` 由当天签到流水计算，使用与签到接口相同的北京时间日期和账号来源标识。账号、登录、签到及奖励等接口中的 `user` 快照均返回此字段；客户端据此显示“已签到”并停用当日签到按钮。重复签到仍返回 409。

## 应用版本与运营公告

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `GET /api/app/update?version_code=N` | 公开，N 为当前非负整数版本号 | 发布配置、revision、update_available、force_update |
| `GET /api/admin/app-release` | 需后台会话 | 发布配置与 revision |
| `PUT /api/admin/app-release` | 完整发布配置，revision 只读 | 保存后的发布配置与新 revision |
| `GET /api/app/runtime` | 公开 | `{revision,hall_notice,updated_at}` |
| `GET /api/admin/app-runtime` | 需后台会话 | `{revision,hall_notice,updated_at}` |
| `PUT /api/admin/app-runtime` | `{hall_notice}` | 保存后的 `{revision,hall_notice,updated_at}` |

发布配置默认停用：

```json
{
  "enabled": false,
  "latest_version_code": 0,
  "latest_version_name": "",
  "min_supported_version_code": 0,
  "title": "发现新版本",
  "release_notes": "",
  "apk_url": "",
  "apk_sha256": "",
  "apk_size_bytes": 0,
  "revision": 1
}
```

`release_notes` 为字符串。启用前需填写正的 `latest_version_code`、非空版本名称、有效 HTTPS 安装包下载地址、64 位十六进制 SHA256 与正的真实字节大小。SHA256 保存为小写。最低支持版本为非负整数，不能大于最新版本，0 表示不要求强更。停用时可保存未完成草稿；负版本号或负安装包大小仍拒绝保存。

版本比较使用 `version_code` 数字，不解析版本名称：只有 enabled 且 N 小于 latest_version_code 时 update_available=true；在此基础上 N 还低于 min_supported_version_code 才 force_update=true。等于或高于最新版本时两项都为 false，停用时也均为 false。客户端显示运营填写的标题、说明和安装包元数据。

公告默认空字符串，最多 200 个字符，可保存空值清除；updated_at 为服务器 ISO 时间。发布与公告各自持有独立递增 revision，重启保留 SQLite 配置。公告刷新不执行动态代码；雨场、红包报价与大厅策略继续从原有接口读取实时规则。

本模块公开检测、公告和对应后台配置响应携带 `Cache-Control: no-store`。

保存发布和公告配置分别写入 `app_release_update`、`app_runtime_update` 管理审计，与配置更新同事务提交，actor 为当前后台会话的真实用户名。公开检测接口只读；PUT 接口继承现有管理会话鉴权，不操作管理员凭据。

## 邀请关系

每个账号具有固定的 6 位邀请码，包含大写字母与数字，排除 `I`、`O`、`0`、`1`。迁移已有账号时仅补齐邀请码和邀请状态，余额及昵称保持原值。

直接注册或登录的账号在 `invitation_required:true` 时进入绑定页。该会话可调用 `/api/me`、`/api/auth/logout`、`/api/invite/bind`；大厅、金币、提现和邀请统计等业务接口返回带 `INVITE_REQUIRED` 的 403。匿名商品/任务预览可访问，携带未绑定账号会话时同样受门槛约束。

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `POST /api/invite/prebind` | 公开；`{phone,invite_code}` | `{status:"pending",invite_code,download_url}` |
| `POST /api/invite/bind` | 需账号会话；`{invite_code}` | `{user}` |
| `GET /api/invite/summary` | 需已绑定账号 | `{invite_code,invite_url,invited_count,registered_count,pending_count,inviter_code?}` |
| `GET /api/invite/qr/{code}` | 公开，校验邀请码 | `{code,url,size,modules:["0101..."]}` |

分享链接为 `https://orange.lingzidev.com/i/CODE`。网页手机号预绑定使用首次来源，相同手机号和邀请码重复提交保持同一记录，改填其他来源返回 409。预绑定暂记为 `pending`，在手机号对应账号成功注册或密码登录时，与账号和会话创建一并在事务中绑定。已有绑定关系的手机号在公共预绑定接口返回 409。

邀请码来源须为已绑定账号或管理端配置的平台引导码。账号不能邀请自己、使用尚未绑定账号的邀请码或更换已有邀请关系；相同绑定的重复请求返回现有用户。邀请关系独立持久化，仅记录来源和人数。`registered_count` 统计已绑定账号，`pending_count` 统计尚未消费的预绑定手机号，`invited_count` 为两者之和。

二维码采用 QR Code M 级纠错，返回原始正方形模块矩阵，`1` 为黑色、`0` 为白色。客户端渲染时在四周增加至少 4 格白色静区，并使用整数像素比例绘制。矩阵编码的是当前用户的邀请 URL，可供海报和页面复用。手机号预绑定的 `download_url` 来自 `ORANGE_APP_DOWNLOAD_URL`；未配置时为空字符串。

后端提供 `/invite` 与 `/i/{code}` 的前端页面入口。本地测试脚本为 `server/tests/invitations.ps1`；核心迁移及归属测试执行 `cargo test`。

## 大厅与金币

商品库、图片文件上传、专用 API 密钥、平台分成及有效期接口见 [商品上传接口](PRODUCT-API.md)。公开商品列表仍为数组，来源为持久商品库；当前没有有效商品时返回空数组，大厅只推送实际有效上架商品。

商品点击/曝光上报与后台统计、捡漏顶部展示图配置见 [商品上传接口](PRODUCT-API.md) 的“点击、曝光与统计”章节。公开运行配置包含主体名称 `operator_name:"广西奇易电子商务有限公司"`。

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `GET /api/tasks` | 可匿名；携带会话则返回领取次数 | `[{id,title,subtitle,reward,reward_min,reward_max,kind,claimed_today}]` |
| `POST /api/rewards/quote` | 需登录；`{task_id,key_id?}` | `{quote_id,task_id,reward,reward_min,reward_max,expires_at,task}` |
| `POST /api/rewards/start` | 需登录；`{task_id,quote_id}` | 返回当前广告模式的奖励会话，见下文 |
| `POST /api/rewards/{id}/complete` | 需登录 | `{reward,user}`；重复完成额外含 `already_completed:true` |
| `GET /api/rewards/{id}/status` | 需本人会话 | `{id,status,reward,verified_at,completed_at,expires_at,expired}` |
| `POST /api/rewards/{id}/client-event` | 需本人会话；见下文安全诊断 | `{ok:true}` |
| `POST /api/check-in` | 需登录 | `{reward:50,user}` |
| `GET /api/ledger` | 需登录 | `[{id,title,coins,created_at}]`，最近 200 笔 |
| `GET /api/hall/stream` | 需登录；SSE 长连接 | 按大厅策略实时下发文字、红包或商品 |

任务 ID 为 `welcome`、`afternoon`、`lucky`，`kind` 为 `rewarded_video`。普通红包采用设备阶梯奖励，每人每日默认最多 20 次。卡片 `reward_min/reward_max` 表示预计区间，`reward` 为区间下限；打开后先获取限时报价，使用返回的精确金额和 `quote_id` 开始会话。报价中的 `task` 为该次精确金额卡片。已开始会话锁定奖励，完成必须是当前账号创建的记录。同一次完成重复请求仅返回结果，账本唯一来源保证不重复入账。签到每日 50 金币，重复返回 409。

`key_id` 仅作为已有 SDK 密钥的提示；服务端使用该账号已验证关联决定档位。未传时使用账号最近已验证关联；已有设备却显式提交未关联提示返回通用 400 并记录私有原因。真正首次未登记设备使用保守区间，首次验证后不更改该报价金额。报价、设备、名额或策略已变化时返回 410 `{error,code:"QUOTE_EXPIRED"}`，客户端应明确重新打开。设备策略、档位与证据不进入公共响应。

本地 `mock` 会话返回 `{id,reward,duration_seconds:5,title,mode:"mock"}`，5 秒后允许完成。

### 穿山甲激励视频

`csj` 模式开始会话返回：

```json
{
  "id":"奖励会话UUID",
  "reward":100,
  "title":"见面红包",
  "mode":"csj",
  "user_id":"账号ID字符串",
  "ad_app_id":"5931950",
  "ad_code_id":"993606639",
  "media_extra":"{\"reward_session_id\":\"奖励会话UUID\"}"
}
```

Android 将 `user_id` 传给 SDK 的 `setUserID`，将完整 `media_extra` 字符串直接传给 `setMediaExtra`。广告关闭后查询本次权威状态，`verified/completed` 才调用完成接口；`pending` 显示等待确认并允许主动重查。SDK 回调和客户端上报本身不会授币。

在穿山甲广告位开启服务端激励回调，将回调地址设置为部署域名的 `/csj/callback`。该接口为公开 GET，接收 `user_id`、`trans_id`、`reward_name`、`reward_amount`、`extra`、`sign`。签名为 `SHA256(SecurityKey + ":" + trans_id)` 的小写十六进制，响应为 `{"isValid":true}` 或 `{"isValid":false}`。[穿山甲官方回调文档](https://www.csjplatform.com/supportcenter/5416)

服务端验证签名、`extra.reward_session_id`、账号归属、30 分钟会话有效期、广告位预期奖励名称/数量和交易编号唯一性。平台回调只更新验证状态；金币由客户端完成接口在验证通过后入账，发放数量使用会话创建时的奖励快照。相同正确回调可重复确认，同一交易编号只能关联一份奖励。未验证会话调用完成接口返回 409 `{error,code:"REWARD_PENDING"}`。

穿山甲 `reward_amount` 是广告位配置值，与本地报价独立。管理设置 `csj_expected_reward_name`、`csj_expected_reward_amount` 默认 `金币`、`100`，必须与平台配置一致。平台回调 100 可以确认业务报价 17，但只发放会话锁定的 17；名称或数量不符会明确拒绝验证。

状态 `pending/verified/completed`、奖励和有效期均来自服务端。安全诊断请求 `{event,valid?,reward_type?,is_server_verify?,error_code?,error_message?}` 的 event 为 `reward_arrived/reward_verify/ad_closed/video_error`，message 最长 500 字符；只进入受保护时间线，标记 `client_reported:true`，不会更新验证或余额。

奖励表启动时补齐 `provider`、`verified_at` 和 `csj_trans_id` 三列，并为平台交易编号建立唯一索引。原有本地会话保持 `mock` 标记，客户端需要使用当前广告通道创建的新会话。

### 大厅推送

登录后携带 Bearer token 请求 `GET /api/hall/stream`，响应类型为 `text/event-stream`。普通事件使用默认 SSE `message` 类型，单个 `data:` 行为 JSON：

```json
{"id":"独立UUID","kind":"red_packet","text":"给你送来一份红包，点开看看吧。","task":{"id":"welcome","title":"见面红包","subtitle":"一份小心意，开心收下吧","reward":100,"kind":"rewarded_video","claimed_today":0},"created_at":"2026-11-10T16:00:00Z"}
```

普通客户端的 `kind` 为 `text`、`red_packet` 或 `product`。红包事件带完整 `task`，商品事件带与 `/api/products` 相同结构的 `product`，文字事件仅有基础字段。

连接建立后先发欢迎文字，约 1 秒发送第一份红包，约 6 秒推荐第一件商品；对应权重为 0 时跳过该类型。之后按 `interval_seconds` 间隔和两项权重选择类型。每次读取当前策略，修改配置会作用于已打开的连接。`enabled:false` 时连接维持，仅发心跳；重新启用后继续按策略推送。SSE 心跳间隔为 15 秒。

支持穿山甲原生信息流的 Android 客户端使用 `/api/hall/stream?native_ads=1`。只有服务端为 `csj` 模式且 `native_ad_weight>0` 时，该连接才会在约 10 秒收到第一条 `native_ad`，随后参与加权选择：

```json
{"id":"独立UUID","kind":"native_ad","text":"为你精选","ad_code_id":"993606636","created_at":"2026-11-11T00:00:00Z"}
```

客户端通过该代码位加载真实 SDK 信息流素材；事件本身仅为展示指令。普通网页及 `mock` 模式会排除原生信息流权重。信息流事件与金币账本分别处理。

事件只存在于当前连接和客户端会话内存。关闭 App 后清空消息，重新打开建立新连接。服务端持久化大厅策略配置，金币和提现仍使用各自账本。客户端接收一条后直接展示一条。

请求阶段账号无效返回 HTTP 401 JSON；连接期间出现错误，先发送 `event:error`、`data:{"error":"具体原因","status":401}`，随后关闭连接。客户端应显示该原因，让用户主动重新连接。退出登录或重置密码后，原连接在下一次检查时结束。

后续用户兴趣推荐可在服务端 `hall_item` 的商品选择处接入，当前使用商品目录和配置权重。

## 捡漏

`GET /api/products` 公开返回当前有效上架的持久商品数组；`GET /api/products/{id}` 提供有效详情，无效返回 404。客户端跳转前重新读取详情和最新链接。商品来源、预计金币规则及上传示例见 [商品上传接口](PRODUCT-API.md)。

## 提现

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `POST /api/withdrawals` | 需登录；`{amount_yuan,channel,account}` | `{id,amount_yuan,channel,account,status:"模拟申请",created_at,mode:"mock",user}` |
| `GET /api/withdrawals` | 需登录 | `[{id,user_id,phone,amount_yuan,channel,account,status,created_at}]` |

`channel` 支持 `alipay` / `wechat` 或 `支付宝` / `微信`。默认满 1 元可申请，最多两位小数，申请即扣减金币，写入同一笔数据库事务。管理端模拟退回时返还金币；模拟完成只改变状态。这里没有支付接口，也不会真实转账。

## 管理接口

后台为单个管理员账号，账号和 Argon2 密码哈希保存于 SQLite。`ORANGE_ADMIN_USERNAME`（默认 `admin`）与 `ORANGE_ADMIN_PASSWORD` 仅用于数据库首次创建管理员；后续重启不覆盖修改后的凭据。原 `ORANGE_ADMIN_TOKEN` 环境值不再读取，也不能授权后台请求。

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| `POST /api/admin/login` | `{username,password}` | `{token,username,expires_at}` |
| `GET /api/admin/account` | 当前后台会话 | `{username,auth_required:true,session_expires_at}` |
| `PUT /api/admin/account` | `{username,current_password,new_password?}` | `{username,reauthenticate:true}` |
| `POST /api/admin/logout` | 当前后台会话 | `{ok:true}` |

除登录外所有 `/api/admin/*` 通过统一 middleware 校验 `x-admin-token`。每次登录生成独立的随机会话，有效期 12 小时，数据库只保存会话 token 的 SHA256。错账号或登录密码、失效/撤销的会话返回 401。用户名支持 2–40 位文字、数字和 `_ . -`，密码至少 8 位。

修改凭据先核验当前密码；填错返回 400 并保留原会话。用户名或密码修改成功时，与安全审计同事务撤销全部会话，包含当前会话；客户端清除 token 后使用新凭据重新登录。未传 `new_password` 时保留原密码。凭据变更审计只保存前后用户名和密码是否修改，不保存密码、哈希或 token。退出只撤销当前会话。

仅本地、未接设备证明且数据库从未创建管理员的演示模式可直接预览，`GET account` 返回 `{username:"",auth_required:false,session_expires_at:null}`。数据库已有账号时，即使删除所有环境引导字段，公开 `admin_auth_required` 仍为 true，后台继续要求有效会话。

### 后台邮箱

邮件由独立 Stalwart 与 Roundcube 提供；应用只实现一次性 SSO 桥，不实现 SMTP/IMAP 协议。前端与邮箱位于后台相同来源，邮箱入口为 `/mailbox/`。

| 方法与路径 | 鉴权与请求 | 响应 |
| --- | --- | --- |
| `GET /api/admin/mail/status` | 现有 `x-admin-token` | `{enabled,entry_path:"/mailbox/",error?}` |
| `POST /api/admin/mail/session` | 现有真实后台数据库会话 | `{ticket,entry_path:"/mailbox/"}`，附下述 Cookie |
| `POST /api/mail-sso/redeem` | 仅内部；`x-mail-sso-secret`；`{ticket}` | `{username,password}`，只给本机 PHP |
| `GET /api/mail-sso/access` | 仅 Nginx 内部；转发原请求 Cookie | 有效后台会话 `204`，无效 `401` |

三个邮件环境变量必须同时配置。未配置时 status 明确返回 `enabled:false` 与缺失配置名；签票或兑换返回 503。enabled 表示 SSO 配置状态，实际邮件连接结果由 Roundcube 展示。

签票仅接受真实后台登录会话，本地免登录预览不签票。票据为独立随机值，只将 SHA256、真实管理员名和原后台会话 SHA256 保存至 `mail_sso_tickets`。票据最多有效 60 秒，且不超过原会话剩余期限；内部兑换以单条条件更新在事务中消费一次，复核原后台会话仍然有效。重放、过期、错误内部密钥或后台会话已撤销均返回 401，错误不会回显票据、密钥或密码。

签票响应设置：

```text
Set-Cookie: orange_mail_admin=<本次x-admin-token>; Path=/mailbox/; Secure; HttpOnly; SameSite=Lax; Max-Age=<不超过原会话剩余秒数>
```

前端收到票据后用同源 POST 表单将 `ticket` 提交至 `/mailbox/`，不放进 URL。Roundcube 插件仅向固定 loopback 地址 `http://127.0.0.1:38180/api/mail-sso/redeem` 发送票据和内部密钥，取得服务邮箱凭据完成登录。浏览器只接触一次性票据和正常 Roundcube 页面，邮件密码留在服务端。

Nginx 对 `/mailbox/` 的每个请求通过内部 `auth_request` 调用 access，沿用 `admin_account` 的同一数据库会话校验。后台退出、修改账号或密码撤销原会话后，邮箱 Cookie 即使仍保留在浏览器，下次访问也会被拒绝。这个 Cookie 仅用于邮箱保护，不授权其他后台 API。

Nginx 必须禁止公网直达 `/api/mail-sso/redeem` 与 `/api/mail-sso/access`；前者只接受本机 PHP，后者只用于内部子请求。端口 38180 继续只监听 loopback。所有邮件桥响应携带 `Cache-Control:no-store`。签票和兑换分别写 `mail_session_create`、`mail_session_redeem` 安全审计，actor 为真实管理员，不保存原始票据、后台 token、邮箱密码或内部密钥。

### 腾讯邮件事件回调

协议来源：[腾讯云 SES 邮件通知事件](https://cloud.tencent.com/document/product/1288/52368)。控制台回调地址填写 HTTPS 地址 `/mail/tencent/callback?token=<独立随机token>`。该文档未声明数字签名协议，本实现使用 **HTTPS 与私有 URL token 鉴权**，不将其描述为腾讯数字验签。

| 方法与路径 | 请求/响应 |
| --- | --- |
| `GET /mail/tencent/callback?token=...` | 连通性验证；正确配置与 token 返回 HTTP 200 `{ok:true}` |
| `POST /mail/tencent/callback?token=...` | 单个 JSON 事件；持久化成功或完全相同内容重试均返回 HTTP 200 `{ok:true}` |
| `GET /api/admin/mail/events?limit=50&event=&email=` | 原有后台会话鉴权；返回 `{items,limit}`，默认 50，最多 100 条，按接收顺序倒序 |

配置缺失返回 503，错误或缺失 token 返回 401，错误 JSON 或事件字段返回 400。POST 必须有非空字符串 `event`、`email`、`from`，`from` 与当前 `ORANGE_MAIL_USERNAME` 不区分大小写匹配；`timestamp` 必须是有效的正整数 Unix 秒时间戳。其余字段完整保留，包括 `bulkId`、`reason`、`bounceType`、`username`、`fromDomain`、`templateId`、`subject`、`messageId`、`sentTimestamp`、`link`、`useragent`、`eventid`、`taskId` 与 `X-Tencentcloudses-Cb-` 扩展。

已知事件为 `deferred/delivered/dropped/open/click/bounce/spamreport/unsubscribe`；非空的后续新事件也正常记录。事件 JSON 按键排序、去除无意义空白后计算 SHA256，SQLite `mail_events.payload_hash` 唯一约束去重。同一封邮件的打开或点击在不同 `timestamp` 下分别保存。该表仅保存邮件事件，不修改用户、金币、广告会话或其他业务状态。

管理员事件项为 `{id,event,email,from,timestamp,received_at,subject,reason,bulk_id,message_id,bounce_type,raw_json}`；时间为 UTC ISO 字符串，`raw_json` 为规范化原始 JSON 字符串，未提供的可选展示字段为 `null`。`event` 精确筛选，`email` 不区分大小写精确筛选。原始数据只通过后台接口读取，公开确认响应不包含邮件内容、配置或凭据。所有事件接口响应携带 `Cache-Control:no-store`。

Nginx 需转发 `/mail/tencent/callback`，并关闭该位置访问日志，或使用仅含 `$uri`、不含 `$request`/`$request_uri`/`$args` 的日志格式，避免将 URL token 写进日志。Rust 不记录回调查询字符串或 token；邮件事件正文也不进入管理员审计。

| 方法与路径 | 请求/响应 |
| --- | --- |
| `GET /api/admin/overview` | `{users,total_users,today_users,rewards,today_rewards,coins_issued,today_coins,withdrawal_count,withdrawal_amount,pending_withdrawals,risk_review_users,risk_restricted_users,risk_assessment_errors_24h,mode,ad_platforms:[{name,status}]}` |
| `GET /api/admin/users` | `[{id,phone,nickname,coins,created_at}]` |
| `GET /api/admin/rewards` | `[{id,user_id,phone,task_id,reward,started_at,completed_at,status}]` |
| `GET /api/admin/withdrawals` | 同提现列表，所有账号最近 200 笔 |
| `PUT /api/admin/withdrawals/{id}` | 请求 `{status:"approved"或"rejected"}`；响应 `{id,status,mode:"mock",user}` |
| `GET /api/admin/settings` | `{coins_per_yuan:1000,min_withdraw_yuan,daily_reward_limit,reward_amount,sms_mode:"mock",ad_mode,payment_mode:"mock"}` |
| `PUT /api/admin/settings` | `{min_withdraw_yuan,daily_reward_limit,reward_amount}`；响应更新后的配置 |
| `GET /api/admin/hall-strategy` | 返回大厅推送策略对象 |
| `PUT /api/admin/hall-strategy` | 提交完整大厅推送策略对象；响应保存后的对象 |
| `GET /api/admin/invitations` | `{platform_code,platform_url,invited_count,registered_count,pending_count,unbound_count,records}` |
| `PUT /api/admin/invitations` | `{platform_code}`；响应更新后的邀请概览 |

邀请概览 `records` 包含 `{id?,phone,invite_code,inviter_code,status,created_at,bound_at}`。状态为“已绑定”“待绑定”“待登录”“待注册”；尚未注册的预绑定记录没有账号 ID，`invite_code` 为 `null`。平台引导码通过管理接口配置，公开运行配置只提供绑定能力开关。更换平台码会保留之前已确认的预绑定来源和用户归属。

大厅推送策略默认值：

```json
{
  "enabled": true,
  "interval_seconds": 20,
  "video_weight": 70,
  "product_weight": 30,
  "native_ad_weight": 15,
  "welcome_text": "欢迎回来，今天也有小惊喜。",
  "packet_text": "给你送来一份红包，点开看看吧。",
  "product_text": "这件好物值得看看，还能省一点。"
}
```

间隔至少为 1 秒，权重为非负整数。旧版策略 JSON 缺少 `native_ad_weight` 时读取默认值 15；每个连接只使用其支持的类型权重，支持的权重全部为 0 时仅发送欢迎文字。

提现管理 `approved`→`模拟已完成`，`rejected`→`模拟已退回`，处理完成的申请不可再次处理。金币兑换比例固定，避免更改比例后既有账本出现歧义。概览中的穿山甲配置状态随广告模式展示，其余联盟等待后续接入。

## 红包雨与主动请求

红包雨使用服务端场次和时间窗，时间字段统一为带毫秒的 UTC ISO 字符串。整点排期以北京时间为基准：`start_hour` 指定整点锚点，之后每 `every_hours` 小时一场。

| 方法与路径 | 请求与响应 |
| --- | --- |
| `GET /api/rain/status` | 公开元数据；带账号会话时同时计算 `joined`。返回 `{enabled,server_now,phase,next_start_at,round,joined,config}` |
| `POST /api/rain/join` | 已绑定账号，启用设备证明时首次参与携带 kind=`rain` 的 `x-risk-event`；返回 `{server_now,round,config,packets}` |
| `POST /api/rain/claim` | 已绑定账号，`{packet_id}`；服务端决定金额，见下文 |
| `GET/PUT /api/admin/rain-config` | 读取或提交完整雨配置，修改动作留后台审计 |
| `POST /api/admin/rain/start` | `{countdown_seconds:10}`，范围 10 至 300，安排一场手动雨并返回状态；进行中或已有待开场时返回 409 |

`phase` 为 `disabled`、`waiting` 或 `active`；`round` 为 `null` 或 `{id,start_at,end_at,source}`。状态接口只提供场次元数据。只有进行中的场次允许 join，返回 `packets:[{id,kind,reveal_at,expires_at,status}]`。每个账号每场获得固定一组不可猜测 ID，重复 join 返回原组，已有参与记录会检查账号状态和当前场次窗口。包类型 `kind` 为 `small` 或 `large`，状态为 `available/opened/ad_started/claimed`；包金额在打开前保留在服务端。

雨配置默认值与范围：

| 字段 | 默认值 | 范围/单位 |
| --- | --- | --- |
| enabled | true | 开关 |
| start_hour | 0 | 北京时间 0 至 23 点 |
| every_hours | 1 | 1 至 24 小时 |
| duration_seconds | 45 | 10 至 180 秒 |
| packet_count | 24 | 每人每场 5 至 60 个 |
| small_min_coins / small_max_coins | 1 / 5 | 1 至 10 金币，最大值不低于最小值 |
| grand_probability | 0.04 | 0 至 1，预算约束内服务端随机分配 |
| grand_coins | 88 | 1 至 10000 金币 |
| max_total_coins | 300 | 每人每场分配上限，不低于 packet_count×small_min_coins，上限 600000 |
| overlay_opacity | 0.64 | 0 至 0.9 |
| fall_speed | 1 | 0.4 至 3 倍速 |
| max_visible | 8 | 最多同时可见 1 至 20 个 |

数量、概率、金额和展示窗口按场次保存快照，关闭开关则立即停止新参与、新领取及新广告会话。随机分配预留后续小包最低金额，分配总额受 `max_total_coins` 限制。服务端按速度与可见数量排出 `reveal_at` 到 `expires_at`，客户端应在该窗口内绘制，不延长有效期。

小包首次成功返回 `{kind:"small",packet_id,reward,user,already_claimed:false,credited:true}`。有效窗口内同一包重试返回原奖励收据与当前用户快照，并标记 `already_claimed:true,credited:false`，客户端只同步余额。禁用、未出现、过场、失效、跨账号及伪造 ID 均不能产生新入账；新领取还受合理点击频率约束。

大包 claim 返回 `{kind:"large",packet_id,reward,task}`，`task.id` 为 `rain:PACKET_ID`，该步骤不入账。客户端在本场结束前调用现有 `/api/rewards/start`，使用独立 kind=`reward` 的设备证明，播放真实激励视频；有效 CSJ 服务端回调确认后调用完成接口。已经开始的广告会话可在关闭雨开关或场次结束后，按原有效期和账号状态完成一次。雨大奖在 `mock` 广告模式下不会通过计时直接入账。

小包领取或大奖完成时，金币入账、包状态及唯一 `ledger.source=rain:PACKET_ID` 在同一事务处理；调查账本类型为 `rain_reward`，标题区分“红包雨奖励”和“红包雨大奖”。参与、打开和入账动作进入用户行为时间线。该机制限制每人数量、窗口、频率和实际发币上限，客户端动画与请求本身不能证明物理点击。

主动请求入口：

| 方法与路径 | 请求与响应 |
| --- | --- |
| `GET /api/hall/request-config` | 公开 `{enabled,packet_enabled,product_enabled,cooldown_seconds,daily_limit,remaining_seconds,remaining_today}`；带会话时返回本人剩余时间与次数 |
| `POST /api/hall/request` | `{kind:"packet"或"product"}`，返回 `{event,cooldown_seconds,remaining_today}`；event 沿用大厅 SSE 事件结构 |
| `GET/PUT /api/admin/hall-request-config` | `{enabled,packet_enabled,product_enabled,cooldown_seconds,daily_limit}`，默认开启、20 秒、每日 20 次；范围 5 至 3600 秒、每日 1 至 200 次 |

冷却中返回 HTTP 429 `{error,retry_after_seconds}`；次数用尽或入口关闭返回明确业务错误。按北京时间自然日计数，两类请求共用冷却和每日额度。运营表保存计数和时间，返回的消息由客户端加入内存大厅；服务端不保存消息内容。该入口只请求消息，后续领取继续走原有收益验证。

## 用户调查

调查接口沿用管理鉴权。原始 `/api/admin/users` 列表仍保留；分页筛选使用以下接口：

| 方法与路径 | 用途 |
| --- | --- |
| `GET /api/admin/users/search` | 用户搜索和分页 |
| `GET /api/admin/users/{id}/detail` | 资料、汇总、上级邀请关系、账号处理状态、采集范围 |
| `GET /api/admin/users/{id}/records` | 分页查看指定记录类型 |
| `PUT /api/admin/users/{id}/restriction` | `{status:"active"或"review"或"restricted",reason}`，理由必填，返回更新后的处理状态 |

搜索参数为 `q`、`invitation_status=bound|unbound`、`account_status=active|review|restricted`、`from`、`to`、`min_amount`、`max_amount`、`page`、`page_size`。`q` 匹配账号 ID、手机号、昵称及邀请码；搜索中的金额范围表示当前余额，单位元。返回 `{items,total,page,page_size}`，每行包含用户基本资料、`account_status`、`risk_count`、`latest_risk_at`、`last_risk_decision`、`last_risk_score`。最近风险字段来自真实事件。

详情返回 `{user,summary,inviter,restriction,availability}`。`user` 为现有用户快照加 `created_at`、`last_login_at`；`summary` 包括 `ledger_count`、`ad_sessions`、`ad_verified`、`ad_completed`、`withdrawal_count`、`withdrawal_total_yuan`、`withdrawal_pending_yuan`、`invite_registered`、`invite_pending`、`device_count`、`risk_count`。`inviter` 保留真实邀请账号或平台码关系。

记录参数包含 `section`、`from`、`to`、`kind`、`status`、`min_amount`、`max_amount`、`related`、`page`、`page_size`。日期可用 `YYYY-MM-DD`（北京时间自然日）或带时区的 ISO 时间；金额统一按绝对值元筛选，类型和状态精确匹配，关联号包含匹配。默认第 1 页、每页 20 条，每页最多 100 条。返回 `{items,total,page,page_size,availability,note}`，每行共有 `id,user_id,kind,status,created_at,amount_yuan,related_id`。

| section | 附加字段与含义 |
| --- | --- |
| `ledger` | `title,coins,source`；类型为 reward/rain_reward/check_in/withdrawal/withdrawal_refund/other，状态 credited/debited |
| `ads` | `task_id,provider,reward,started_at,verified_at,completed_at,csj_trans_id,quote_id,reward_tier,policy_version,device_first_seen,device_age_days,device_completed_before,reward_min,reward_max,reward_basis`；状态 started/verified/completed |
| `callbacks` | `reward_session_id,trans_id,reason_code,client_ip,request_id,source`；状态 accepted/rejected；`source=business` 表示已存在的业务验证状态回溯，`captured` 表示实际采集的请求 |
| `withdrawals` | `channel,account,coins,reviewed_at,reviewed_by`；保留实际处理状态，未采集的历史审核时间为空 |
| `invitations` | `direction,related_user_id,phone,nickname,invite_code,bound_at`；direction 为 upstream/downstream/pending，支持打开关联账号 |
| `timeline` | `title,client_ip,request_id,source,detail`；组合业务记录、采集后的操作及 `risk_assessment`，后者保留 SDK 场景、决策、动作、分数和业务引用 |
| `risk` | `decision,action,score,key_id,report_id,client_ip,context,detail,error,updated_at,request_hash,operation_id,receipt_status,receipt_error,receipt_updated_at`；展示实际平台记录，状态 pending/reported/assessment_error |
| `devices` | `device_id,platform,model,os_version,key_id,report_id,evidence_status,first_seen,last_seen,account_count,related_users`；关联依据为已验证签名 keyId，机型/系统空值表示未采集 |
| `audit` | `actor,action,client_ip,request_id,reason,filters,before,after,http_status`；区分访问、账号状态变更和提现审核 |

`availability` 包括 `behavior_since,callbacks_since,audit_since,device_trust_configured,notes`。历史登录、请求 IP、失败回调及管理员操作在采集启用前缺失，页面应显示空缺；交易和邀请使用真实持久化业务回溯。设备记录表示已验证密钥的关联，不能直接视为物理设备唯一标识。待核查、受限及平台评估错误分别表示当前处理状态。

管理员访问及白名单筛选条件会记录审计。账号状态变更、提现处理与对应操作审计在同一 SQLite 事务中提交；操作主体来自当前已验证后台会话的真实用户名，本地免登录预览标记为 `local-preview`。审计不保存密码、验证码、Bearer token、管理 token、回调签名、平台密钥或原始请求正文。原生大厅消息仍保留在客户端会话内存。

请求 IP 默认使用 TCP 对端地址。只有 TCP 对端为本机 loopback 且 `x-orange-proxy-token` 与 `ORANGE_PROXY_TOKEN` 相符，才采用 Nginx 覆盖的单个 `x-real-ip`；`X-Forwarded-For` 不作为可信来源。Nginx 必须覆盖私有认证头和真实 IP 头，服务端保持 loopback 监听。

## 设备证明与收益动作

配置了设备证明平台后，公开配置仅返回 `device_trust_enabled:true`。普通账号登录与首页仍可使用；收益动作需要真实、已确认的同账号同场景证明。

| 方法与路径 | 请求与响应 |
| --- | --- |
| `POST /api/risk/prepare` | 账号会话；`{kind:"login"或"reward"或"check_in"或"withdraw"或"rain",key_id?,intent:{method:"POST",path,body}}`，返回 `{event_id,challenge,sdk}`，sdk 仅包含公开 app_id/base_url/spki_pins |
| `POST /api/risk/complete` | `{event_id,report_id}`，服务端向平台核实后返回 `{status,reference}` |
| `POST /api/risk/failure` | `{event_id?,kind?,stage,code,detail?}`，detail 最多 2048 字节，仅私有调查保存，返回 `{reference}` |

`POST /api/check-in`、`POST /api/rewards/start`、`POST /api/withdrawals`、`POST /api/rain/join` 分别使用 `check_in`、`reward`、`withdraw`、`rain` 事件，并携带 `x-risk-event`。prepare 的 intent 为同一次实际 POST 路径及请求体：签到 `{}`；奖励 `{task_id,quote_id}`；提现 `{amount_yuan,channel,account}`；雨参与 `{round_id}`。登录报告使用虚拟路径 `/api/auth/login/report` 和空对象。服务端补齐报价金额、收款对象、币种、日期及场次快照后计算规范 SHA256，不接受客户端自行指定摘要。

首次执行要求真实报告同账号/同场景/同 requestHash，并取得平台 `mode=enforce` 的有效执行许可，核验 appId、哈希 eventId、operationId、requestHash、token、expiresAt。网络 claim 在 SQLite 写事务外；进入事务后再次核验本地摘要和许可，事件消费、业务操作、账本、回执行同事务提交。平台 `shadow` 许可不执行收益动作。同事件同参数重试返回既有奖励会话或提现申请，避免再次扣账；每日签到仍保留重复 409。

`/api/rewards/{id}/complete` 使用开始会话关联的证明，首次完成还检查账号状态；已完成结果保持幂等。账号处于 `review` 或 `restricted` 时暂缓收益参与。客户端只收到通用提示与参考号；规则、分数、证据和处理理由通过受保护的调查接口查看。设备证明平台的真实事件和本地限制均持久化，失败后成功重试仍保留先前失败行为记录。提现继续使用模拟流程。

执行后向平台提交稳定幂等回执。发送失败保存 `pending/failed` 与原始安全错误，管理员可 `GET /api/admin/risk-receipts/{event_id}` 查询、`POST /api/admin/risk-receipts/{event_id}/retry` 补报；该操作只补报原业务，不重发金币。奖励在最终入账后提交成功回执；签到、雨参与和模拟提现在各自事务提交后报告。接口及调查 DTO 不返回 permit token 或平台 app key。人工放行 DeviceTrust 来源的受限账号时，先同步平台该原事件 approved，再提交本地状态与审计；平台失败保留私有失败审计。

阶梯策略 `GET/PUT /api/admin/reward-policy`，完整字段、默认值与验证范围见 [0.5 接入说明](DeviceTrust-0.5-验证与接入说明.md)。策略调整写 `reward_policy` 管理审计。阶段使用本服务端首次已验证记录、跨账号完成数与未过期在途会话；清除数据后不能保证识别物理同机。旧 keyId 记录迁移到已验证平台 deviceId 时保留年龄、完成数和账号关联。旧已开始广告会话继续使用原锁定金额。

## 验证

启动服务后执行 `scripts/smoke.ps1`。脚本创建独立测试账号，检查注册、密码登录、错误验证码、重置密码、会话失效、签到、观看时长、奖励归属、重复发奖、提现扣减和退回、管理统计，以及大厅策略保存、SSE 事件、禁用推送和类型权重。脚本会留下测试记录；可通过独立 `ORANGE_DB_PATH` 和端口进行隔离验证。

`csj` 模式自动执行签名与防重检查分支，测试进程需通过 `ORANGE_TEST_CSJ_SECRET`、`ORANGE_TEST_ADMIN_PASSWORD` 注入对应测试服务的私有配置。检查包括伪造签名、会话归属、未验证完成、重复交易、平台回调验证后单次入账，以及管理接口鉴权。测试脚本不输出秘密。

`cargo test` 当前覆盖 21 项：调查筛选、可信 IP、秘密审计、原子限制、报价阶段/年龄/跨账号/在途窗口、未知保守报价与伪造 key、策略锁价、摘要和 permit 篡改、失败事务回滚、奖励及提现重试、雨安全窗口和 CSJ 平台数量独立验证。`scripts/smoke.ps1` 适用于未配置设备证明平台的隔离服务；真实 0.5 SDK、claim/receipt 与 CSJ 端到端验证使用 `tools/verify-sdk05-rewards.py`。

后台账号改造另有 5 项针对测试：`cargo test --manifest-path server/Cargo.toml admin_account::tests`，验证错误凭据、过期与旧静态 token、改凭据撤销所有会话、环境引导不覆盖已有账号、无秘密审计、真实 HTTP 管理路由统一鉴权和真实用户名记录。

版本与公告使用 `cargo test --manifest-path server/Cargo.toml app_config::tests`，3 项覆盖数字版本比较、最低版本与新版本强更边界、发布停用、启用元数据校验、200 字公告边界、清空公告、重启持久化和真实 actor 审计。

邮件 SSO 使用 `cargo test --manifest-path server/Cargo.toml mail_admin::tests`，3 项覆盖一次性兑换、重放与过期、内部密钥错误、缺少后台会话、原会话撤销后 Cookie/票据同步失效、Cookie 作用域和寿命、未配置状态，以及凭据不进入公开响应和审计。

腾讯邮件事件使用 `cargo test --manifest-path server/Cargo.toml mail_events::tests`，覆盖回调鉴权与未配置状态、错误字段和发信地址、规范化重复通知去重、多次打开及后续事件保留、后台筛选和条数上限，以及资金与审计不受事件影响。
