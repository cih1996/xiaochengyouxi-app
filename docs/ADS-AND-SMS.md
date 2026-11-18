# 广告聚合与短信通道

## 广告：谁决定拉哪家

App 每次要广告，先由服务器给出“本次可以试哪几家”的计划（plan），App 按顺序尝试，第一家没填充就换下一家：

- 激励视频：`POST /api/rewards/start` 的返回里带 `plan: [{network, app_id, code_id}]`。
- 大厅信息流：大厅推送的 `native_ad` 卡片带 `ads: [...]`。

计划由后台“广告管理 → 广告配置”决定：

- 总开关；每家平台的开关、AppID、激励视频代码位、信息流代码位、回调密钥。
- 每种广告形式可选“按概率”（滑块权重，按权重随机选首选，其余按权重从高到低做备选；权重 0 不参与）或“按顺序”。
- 激励视频平台必须配置回调密钥才会被分配（金币只在平台服务器回调验签通过后到账）。

平台：`csj` 穿山甲 7.8.1.4、`kuaishou` 快手 5.9.10.1、`baidu` 百度百青藤 9.460、`gdt` 腾讯优量汇 4.640.1510。AAR 原包放在 `android/app/libs/`（不入库），SHA-256 见 `android/docs/AD-NETWORKS.md`。

## 风控：静默不出广告

“命中风控不返回广告”开启时，以下情况计划为空：

- 账户限制状态不是 `active`（复核或限制中）；
- 设定时间窗口内（默认 24 小时）有 DeviceTrust 决策为 review/deny/reject/block 或动作为 verify/hold/block/deny 的已上报事件。

App 对空计划与“所有平台都没填充”完全一致：只提示“暂时没有可观看的视频，请稍后再来”，不出现风控字样，金币报价不变。命中原因只记录在广告统计明细（事件 `risk_block`）。

0.3.20 及更早版本只认识 `ad_app_id/ad_code_id`（穿山甲）。为避免它们弹出加载错误，服务器继续给它们下发这两个字段；静默拦截在用户升级到 0.3.21 后生效。

## 奖励回调

| 平台 | 回调地址（后台广告配置页可复制） | 签名 | 返回 |
|---|---|---|---|
| 穿山甲 | `https://orange-api.lingzidev.com/csj/callback` | `sha256(密钥:trans_id)` | `{"isValid":true}` |
| 快手 | `https://orange-api.lingzidev.com/api/ads/callback/kuaishou?userId=__UID__&transId=__TRANSID__&sign=__SIGN__&amount=__RAMOUNT__&name=__RNAME__&extra=__EXTRA__` | `md5(appSecurityKey:transId)` | `{"isValid":true}` |
| 百度 | `https://orange-api.lingzidev.com/api/ads/callback/baidu` | `sha256(trans_id:密钥)` | `{"isValid":"1","code":0,"msg":"success"}` |
| 优量汇 | `https://orange-api.lingzidev.com/api/ads/callback/gdt` | `sha256(transid:密钥)` | `{"isValid":true}` |

App 把账号编号作为平台的用户 ID、把 `{"reward_session_id": ...}` 作为透传参数（穿山甲 media_extra、快手 extraData、百度 setExtraInfo、优量汇 customData）。服务器核对会话归属、30 分钟有效期、交易号唯一后标记已验证；非穿山甲交易号以 `平台:` 前缀存储。平台重试同一笔回调返回成功但统计为 `callback_retry`，不计入成功率。

## 统计

后台“广告管理 → 广告统计”：请求、填充、展示、点击、回调通过/拒绝、风控拦截；填充率=填充÷请求，点击率=点击÷展示，回调成功率=回调通过÷展示。含每日趋势、展示占比、转化漏斗、各平台成功率表、失败原因 Top 20 和可筛选明细。App 事件经 `POST /api/ads/events` 批量上报（每次最多 30 条、每人每天最多 3000 条），数据保留 90 天。

## 短信通道

后台“系统设置 → 短信通道”配置腾讯云短信（SendSms 2021-01-11，TC3-HMAC-SHA256 签名）：SecretId/SecretKey（SecretKey 只写不读）、地域、SdkAppId、签名、默认模板及注册/登录/找回密码模板、模板参数（`{code}`、`{minutes}`）、有效期与防刷（同号码间隔、同号码每日上限、同 IP 每小时上限）。

- 通道开启：`/api/auth/sms` 生成随机验证码并真实下发，失败时用户只看到“短信发送失败，请稍后再试”，后台发送记录保留平台错误码和 RequestId。
- 通道关闭：生产环境仍生成随机码但不下发（记录为“未开启通道”）；本地开发返回固定 123456。
- App 不需要改动：只在服务器返回 `code` 时自动填入（仅本地演示）。
- 审核账号继续走 `ORANGE_REVIEW_LOGIN_PHONE/CODE`，与短信通道无关。
