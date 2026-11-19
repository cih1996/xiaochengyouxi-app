# 测试账号接口（接口令牌）

用于测试分发：持有接口令牌的一方调用一个接口，传手机号和登录密码，即得到一个可直接登录、已绑定邀请码、已加入白名单的账号。

## 后台

“系统设置 → 接口令牌”：

- 生成令牌（填一个名称，如“测试分发”“渠道A”）。令牌形如 `oat_` + 64 位十六进制，**只在生成时显示一次**；数据库只存 SHA-256 摘要。
- 列表显示令牌开头、创建人、最近使用、调用次数、新建账号数；可随时停用（不可恢复）。
- 下方列出最近 50 条通过接口处理的账号。
- 完整接口文档页：`https://orange.lingzidev.com/admin?doc=test-accounts`（页面里“完整接口文档”按钮），与商品接入文档一样只在后台登录后可见，不公开。
- 生成、停用都写入后台操作审计。

## 接口

`POST https://orange-api.lingzidev.com/api/integration/test-accounts`

请求头：`Authorization: Bearer <令牌>`（或 `X-Api-Token: <令牌>`），`Content-Type: application/json`

```json
{"phone": "13800000000", "password": "Test12345", "nickname": "可选"}
```

- `phone`：11 位手机号；`password`：至少 8 位；`nickname` 可选，默认“测试”+手机尾号 4 位。

成功（200）：

```json
{"user_id": 123, "phone": "13800000000", "created": true, "whitelisted": true,
 "inviter_code": "JZ8F6A", "invite_code": "G2CCNB", "message": "账号已创建并加入白名单"}
```

| 状态 | 含义 |
|---|---|
| 401 | 令牌无效或已停用 |
| 400 | 手机号或密码不合规 |
| 409 | 该账号已被后台人工限制（不会被接口覆盖，需在后台处理） |
| 429 | 该令牌当日新建账号已达 500 个 |

## 账号状态

- **邀请码**：绑定后台“邀请管理”的平台邀请码（`inviter_id` 为空），登录后不再要求填写邀请码。
- **白名单**：写入 `account_restrictions`，状态 `active`、操作人 `接口令牌:<名称>`、原因“测试白名单（接口创建）”。这与后台“人工放行”是同一机制（`trusted_override`）：DeviceTrust / 行为审计命中时自动放行并回传平台，广告不因风控被拦截；风险事件照常记录。金额策略（设备阶梯、活动风控上限）和提现审核不变。
- **手机号已注册**：只加入白名单，**不修改密码**，返回 `created: false`。后台人工限制过的账号返回 409，不会被接口放行。
- 新建账号记一条 `register` 行为（`method: api_token`），已有账号记 `whitelist`。

实现：`server/src/test_accounts.rs`，后台页面 `web/src/ApiTokenAdmin.vue`，文档页 `web/src/TestAccountsApiDocs.vue`。
