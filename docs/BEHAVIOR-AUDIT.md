# 行为审计与回放

行为审计记录 App 内结构化页面、停留、点击、手势和业务操作，用于后台画像、回放和人工核查。它不读取密码、短信、输入框文本、其他 App 内容、截图或录屏，也不直接产生金币。奖励金额与到账状态始终来自现有服务端会话、回调和 ledger。

## 设备绑定与会话

用户同意采集后，客户端在已认证账号下取得 DeviceTrust `kind=behavior` 证明，再调用：

```text
POST /api/behavior/session
Authorization: Bearer <账号会话>
x-risk-event: <已完成的 behavior 事件>
Content-Type: application/json
```

请求体为 `{public_key,client_nonce,consent:true}`，`public_key` 是 Android Keystore P-256 公钥的 DER SPKI Base64，`client_nonce` 是 UUID。可选 `device_model`、`os_version`、`app_version` 为不超过 128 字符的客户端元信息；它们不构成硬件证明。服务端只接受同账号、同 requestHash、已完成签名报告、有效设备身份的证明；受限或待核查账号也可继续记录行为，收益限制保持原规则。

返回一次性内存会话：

```json
{
  "session_id":"…",
  "key_base64":"…",
  "nonce_prefix":"…",
  "challenge":"…",
  "expires_at":"2026-11-11T12:30:00Z",
  "server_now":"2026-11-11T12:00:00Z",
  "protocol":"OBA1",
  "next_seq":1,
  "previous_hash":"0000…0000",
  "elapsed_offset_ms":0,
  "batch_interval_seconds":15,
  "max_events":200,
  "max_elapsed_ms":1800000
}
```

会话密钥只通过已鉴权 HTTPS 返回给当前账号，服务端 SQLite 仅保存会话密钥用于解密；APK 不包含静态会话密钥。前台恢复或账号切换应新建会话，客户端不把行为包写入离线队列。单个上传帧失败最多按相同帧重试一次，之后暂停本会话并等待新的证明。

## OBA1 二进制帧

上传路径是 `POST /api/behavior/batch`（`/api/behavior/batches` 兼容别名），请求体 `application/octet-stream`，服务端限制 8MiB，客户端单帧建议不超过 64KiB。所有整数网络字节序（大端）。

固定 108 字节头：

```text
magic OBA1 4
session UUID 原始字节 16
seq uint64 8（从服务端 next_seq 开始）
challenge 32
nonce 12（session nonce_prefix 4 + seq uint64 8）
previous_hash 32（首包为 32 个 0）
cipher_len uint32 4（AES-GCM 密文含 16 字节 tag）
```

AES-256-GCM 的 AAD 是完整 108 字节头。签名是 Android Keystore P-256 SHA256withECDSA DER，签名内容为 `header || ciphertext_and_tag`；尾部为 `signature_len uint16 || signature`。`packet_hash=SHA256(header || ciphertext_and_tag)`，不包含签名。服务端检查公钥、账号会话、challenge、nonce、seq、previous_hash、签名、GCM tag、时间窗口和事件范围。

解密 payload：

```text
client_wall_ms uint64（仅诊断线索）
viewport_width uint16
viewport_height uint16
dropped_events uint32
event_count uint16（最多 200）
重复 event_count 次：
  elapsed_ms uint32（以单调时钟为准）
  type uint8
  screen uint8
  control uint8
  action uint8
  x,y,end_x,end_y uint16（0–10000，65535 表示无触点）
  duration_ms uint32
  ref_len uint8
  operation_ref ASCII（最多 96 字节，字符仅 A–Z/a–z/0–9/:/_/-）
```

事件 type：`1 screen_enter`、`2 screen_leave`、`3 foreground`、`4 background`、`5 click`、`6 gesture`、`7 business`、`8 heartbeat`。screen 14 是 `reward_result`；它只表示页面样式，不能证明到账。行为页面和控制枚举固定在代码与共享向量中。服务端不使用客户端墙钟决定顺序、有效期或金币。

成功响应为 `{accepted,duplicate,session_id,seq,packet_hash,next_challenge,server_now,status:"received"}`。完全相同的序号和 packet hash 重试返回 `duplicate:true`；同序号不同密文、链断裂、签名错误、账号不匹配、过期或单调时间越界返回业务错误并留下安全诊断。重复包不会重复计数、评分或写入回放。

共享密码学向量位于 `server/src/behavior-vector-v1.json`，用于 Android 与 Rust 互通测试，不含生产密钥。

## 公开配置与客户端诊断

登录后 `GET /api/behavior/config` 返回采集能力：`enabled`、`protocol`、`batch_interval_seconds`、`heartbeat_seconds`、`touch_sample_percent`、`max_events`、`session_ttl_seconds`、`consent_required`。客户端可按 touch_sample_percent 抽样触点；服务端仍记录实际样本数，不外推未采集触点。

上传故障可 `POST /api/behavior/failure`，请求 `{session_id?,stage:"session"|"encode"|"encrypt"|"upload",code,detail?}`。detail 最多 1024 字节，只保存清理后的诊断摘要；不发送 token、密钥、原始 payload、输入框内容或网络正文。普通网络失败不计入行为风险分。

## 后台接口

所有以下接口使用现有后台数据库会话：

| 接口 | 内容 |
| --- | --- |
| `GET/PUT /api/admin/behavior/policy` | 策略、阈值、权重、采集开关和 revision；保存写真实管理员审计 |
| `GET /api/admin/users/{id}/behavior/profile?days=7|30|90` | 真实会话聚合、停留/点击/间隔、业务对账、风险摘要；缺失返回 unknown/空数组 |
| `GET /api/admin/users/{id}/behavior/sessions?page=&page_size=` | 账号会话分页 |
| `GET /api/admin/users/{id}/behavior/sessions/{sid}?page=&page_size=` | 事件分页、server_state、批次状态和回放元数据；最多 5000 条/页请求上限 |
| `GET /api/admin/behavior/alerts?page=&page_size=` | 行为规则命中、分数、策略版本与处置动作 |

回放事件字段包括相对 `time_ms`、type、screen、control_id、坐标、duration_ms、operation_ref、`server_confirmed`、`server_state`、`credited_coins`、服务端接收时间和 batch_seq。`server_state` 为 `client_reported`、`created`、`pending`、`credited` 或 `unmatched`：合法已开始广告、红包雨参与及未完成会话可以是 created/pending；只有服务端真实 ledger 对账成功才是 credited。后台不根据客户端传入金额发放奖励。

行为策略默认 `observe`。点击频率、重复间隔、短停留、业务关联和已签名批次异常只有达到最小样本后才形成证据；网络错误、丢失数据和未采集历史不被视为确认作弊。策略可调为 review/hold，并通过现有 `account_restrictions` 与管理员审计处置；客户端只收到通用状态和参考号。

风险摘要 `risk_summary` 返回 SDK 与行为来源的真实分数、level、证据数量、策略版本和 factors。缺少证据时 score 为 null、level 为 unknown；这些分数是可解释规则结果，不是概率，也不代替服务端账本、广告回调或提现对账。
