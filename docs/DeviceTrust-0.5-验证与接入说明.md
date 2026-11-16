# 小橙有喜 DeviceTrust 0.5 接入与奖励验证

本说明对应本仓库 0.3.5 服务端与 Android 接口。平台原始交付文档及平台仓库仅作为协议依据读取；应用集成代码位于 `server/src/risk.rs`、`business_guard.rs`、`reward_policy.rs`。

## 证明与执行

平台应用须配置 `requireSignedReports=true`、`requireRequestBinding=true`、策略 `mode=enforce`。当前普通签名接入使用 `requireHardware=false`。服务端环境变量见 API 文档；平台 app key、执行 token 与 CSJ 回调密钥只在服务端保存。

Android 使用 0.5 AAR，原样传递服务端 challenge，包含 `hardwareRequired`。普通签名不主动申请硬件证明。ResourceCollector 在用户授权并实际 collect 后使用；平台返回的 `deviceId` 与 `reportProof.keyId` 才是服务端关联依据。它们不能证明清除数据后仍是同一物理设备。

prepare 示例：

```json
{
  "kind": "reward",
  "key_id": "现有SDK密钥ID，可省略",
  "intent": {
    "method": "POST",
    "path": "/api/rewards/start",
    "body": {"task_id":"welcome","quote_id":"服务端报价UUID"}
  }
}
```

实际收益请求必须使用相同 POST body 和 `x-risk-event`。服务端从持久化报价、账号、收款对象、自然日与雨场次补齐业务摘要。规范摘要采用 `dtp-request-1`：先对排序后的 JSON body 计算 SHA256，再对按协议字段顺序组成的 `{v,method,path,account,bodyHash}` 计算 SHA256。客户端不决定 hash。

| kind | intent.path | intent.body |
| --- | --- | --- |
| login | `/api/auth/login/report` | `{}`，登录后的报告语义 |
| reward | `/api/rewards/start` | `{task_id,quote_id}` |
| check_in | `/api/check-in` | `{}` |
| withdraw | `/api/withdrawals` | `{amount_yuan,channel,account}` |
| rain | `/api/rain/join` | `{round_id}` |

真实报告返回后，服务端核对平台 event、account、kind、requestHash 和已验签 proof。收益执行前调用平台 claim，核验 `appId/eventId/operationId/requestHash/token/mode/expiresAt`，其中 eventId 为 `SHA256(appId+":"+业务事件ID)`。许可必须 enforce 且未过期，shadow 不构成执行授权。

网络请求位于 SQLite 写事务外。业务事务内重新核验摘要和许可，消费事件并写入实际业务及账本。奖励开始时占用一个名额并关联许可，最终有效回调确认后完成原金额；许可在开始时有效的已预留会话可在广告会话有效期内完成。已完成重试直接返回原结果。

业务提交时同时写入回执行，随后直接发送平台 executions。失败保留原始安全错误与待补报状态。管理员查询 `/api/admin/risk-receipts/{event_id}`，调用同路径 `/retry` 仅补报原执行，不能重新发币。回执 ID 与引用固定，补报核验平台响应绑定。令牌不出现在查询接口、用户调查、公开配置或客户端。

平台 verify/hold 仍映射到本地 review/restricted。账号恢复前需同步原始平台事件 approved，失败保留后台审计并不更改本地限制。平台复核是单事件最终决定，已开始业务绑定不替换为其他事件。

## 设备阶梯奖励

`GET/PUT /api/admin/reward-policy` 使用以下完整对象，保存时由服务端递增 policy_version 并记录审计：

```json
{
  "enabled": true,
  "fixed_coins": 40,
  "quote_ttl_seconds": 300,
  "new_device": {"max_days":3,"max_completed":5,"min_coins":100,"max_coins":150},
  "active_device": {"max_days":30,"max_completed":30,"min_coins":40,"max_coins":80},
  "mature_device": {"min_coins":15,"max_coins":40},
  "unknown_device": {"min_coins":10,"max_coins":20},
  "policy_version": 1
}
```

金币范围为 1–10000，最大值不低于最小值；报价有效期 30–300 秒。新设备、活跃设备的天数和次数上限为正数，活跃上限不低于新设备上限。依次匹配新设备、活跃设备，其余为成熟设备。档位条件为年龄不超过天数，且“完成+有效在途”次数小于次数上限，即默认前 5 次占新设备名额。关闭阶梯时明确使用 fixed_coins。

阶段只用于普通激励红包；签到、雨小包和雨大奖继续使用原独立配置。跨账号按已验证设备聚合；0.4 keyId 升级 0.5 deviceId 时保留首次记录与历史已归属会话。首次记录日期是本服务接入以来的真实时间，不是手机购买日期。

客户端先 `POST /api/rewards/quote {task_id,key_id?}`：

```json
{
  "quote_id":"报价UUID",
  "task_id":"welcome",
  "reward":17,
  "reward_min":10,
  "reward_max":20,
  "expires_at":"2026-11-11T06:05:00Z",
  "task":{"id":"welcome","title":"见面红包","subtitle":"看视频领取这份小惊喜","reward":17,"reward_min":17,"reward_max":17,"kind":"rewarded_video","claimed_today":0}
}
```

相同有效报价复用，区间内金额由服务端随机确定；开始会话将其持久化。卡片显示预计区间，打开后显示准确金额。关开弹层不能重抽有效报价。已开始会话不因策略调整重定价；未开始报价在策略、名额或关联设备变化时返回 `410/QUOTE_EXPIRED`，需要明确重新打开。

key_id 仅为已有已验证关联的提示。未传时用账号最近已验证关联；已有关联却提交伪造提示返回通用 400，原因仅私有行为记录。真正首次未关联可锁定 unknown 保守价；验证后首次新设备仍保持该价。实际设备已有完成/在途，或已超过新设备年龄上限时，该 unknown 报价失效。所有真实设备的后续报价和占位继续跨账号聚合，不能以省略 key 或预建多份报价重置阶段。

后台广告记录保留 quote_id、档位、policy_version、区间及 reward_basis，其中含首次已验签时间、真实完成数、在途数、分配前次数和关联来源。用户端不返回这些分类依据。

## 穿山甲奖励与诊断

广告位 `993606639` 的平台奖励配置与业务报价分别处理。管理设置 `csj_expected_reward_name/amount` 默认「金币/100」，必须与穿山甲广告位实际值一致。有效签名、会话归属、30 分钟窗口、唯一 trans_id 和正确平台奖励共同确认会话。回调只标 verified；业务 complete 按会话锁定金币入账。平台 100 对应业务 17 时，到账仍为 17。

`GET /api/rewards/{id}/status` 仅本人查询 pending/verified/completed、原金额和有效期。未确认 complete 返回 `409/REWARD_PENDING`。Android 使用 SDK 官方错误 Bundle 常量，关闭后查一次权威状态，允许用户主动查询同一会话；SDK false/valid 及客户端诊断不授币。

客户端可向 `POST /api/rewards/{id}/client-event` 提交四类广告事件及实际存在的布尔/错误字段，message 最长 500 字符。记录明确标记 client_reported，只供后台排查。SDK 错误因果摘要可通过 `risk/failure.detail` 提交，最长 2048 字节，Bearer token 会剔除，不收集密钥、报告正文或 HTTP 原始正文。

UID2 的历史 `upload:ProviderException` 记录只反映 SDK 上报失败。旧版本误走硬件证明的具体 cause 需真实设备新版本诊断确认；不将其伪造为风险命中。0.5 普通签名 hardwareRequired=false 已通过挑战与真平台协议联调，Android KeyStore 行为仍以用户真机为准。

## 验证

`cargo test --manifest-path server/Cargo.toml` 当前 21 项通过，覆盖报价与跨账号占位、旧身份迁移、未知档套利边界、完整摘要和许可字段、enforce/expiry、失败消费回滚、CSJ 固定数量与动态金额分离，以及既有邀请、雨和调查功能。

根代理使用独立 enforce/binding 测试 App 运行 `tools/verify-sdk05-rewards.py`，首轮真实平台联调 81 项通过，包含挑战 hardwareRequired=false、签名报告、exact intent、设备跨账号、claim、CSJ 回调、状态、执行回执与公开字段检查。正式平台开关切换及部署由根代理统一执行；测试应用与生产应用独立。
