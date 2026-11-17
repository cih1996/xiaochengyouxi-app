# 首页入口、网页活动与提现档位

## 首页入口（后台“运营管理 → 首页入口”）

- 大厅欢迎卡下方显示图标入口宫格（最多 10 个，4 个以内排一行 4 列，否则 5 列），右下角显示一个悬浮入口。
- 默认两者都不显示，只用于运营活动；签到、捡漏、提现、金币明细已有固定入口，不要重复添加。名称、角标、图标均按活动填写或在后台上传图片（复用商品图片存储，返回 `/media/products/...`），App 不内置任何活动文案；悬浮入口名称留空时只显示图片。
- 每个入口：名称（1–6 字）、角标（≤4 字）、内置图标或图标图片、打开方式、开始/结束时间（RFC3339，留空不限）、显示开关。
- 打开方式：
  - `native`：App 内置页面，可选 `checkin`、`invite`、`activity`、`withdraw`、`shop`、`ledger`、`me`。
  - `web`：网页活动。填站内路径（如 `/activity/demo.html`），或 `ORANGE_ACTIVITY_HOSTS` 允许域名下的 https 地址（默认 `orange.lingzidev.com,orange-api.lingzidev.com`）。
- 悬浮入口动画：`float` 上下漂浮、`bounce` 弹跳、`shake` 摇摆、`none` 静止；用户可点 × 隐藏，后台再次保存（revision 变化）后重新出现。
- 站内路径的活动页按 `WEB_BASE_URL`（正式包 `https://orange.lingzidev.com`）打开，页面内 `/api/` 由该站点代理到后端。
- 接口：App 读取 `GET /api/app/home`（只返回当前生效的项）；后台 `GET/PUT /api/admin/home-layout`，修改写入管理审计。App 每 45 秒随运行时配置刷新，无需发版。

## 网页活动容器（Android `WebActivity`）

- 活动页放在 `web/public/activity/`，随网页发布；上线新活动只需发布网页并在后台加入口。
- 安全约束：
  - 只能打开允许域名（含 API 域名）的页面；其他域名的跳转、脚本、iframe、图片请求一律拦截，因此活动页的所有资源必须放在自己的站点上。
  - 禁止文件访问和混合内容；UA 追加 `OrangeApp/<版本>`。
  - 桥接对象 `window.OrangeApp` 每次调用都会重新检查当前页面域名。
- 桥接方法：

| 方法 | 说明 |
|---|---|
| `OrangeApp.token()` | 当前登录凭证（未登录为空串），用于调用 `/api/*` |
| `OrangeApp.version()` | App 版本号 |
| `OrangeApp.open(target)` | 关闭活动页并打开内置页面（取值同上 `native`） |
| `OrangeApp.share(text)` | 调起系统分享，最多 300 字 |
| `OrangeApp.toast(text)` | 轻提示，最多 60 字 |
| `OrangeApp.close()` | 关闭活动页 |

- 金币、机会、奖励一律由服务端接口结算，网页不能直接加币。激励视频桥接留待后续按需实现。
- 示例页：`web/public/activity/demo.html`。

## 提现档位（后台“资金管理 → 提现档位”）

- 用户只能选择启用的固定金额；余额满档位金额才可提。
- 每档可设“每天次数”和“累计次数”（0 为不限）；北京时间自然日计算，被驳回退回的申请不占次数。
- 默认：0.5 元每天 1 次；20 元、50 元不限次数。
- 服务端在申请前和事务内各校验一次，并发请求不能突破每日次数。
- 接口：App `GET /api/withdraw/options`（含每档可用状态和原因）；后台 `GET/PUT /api/admin/withdraw-rules`。
- 兼容：旧版 App 仍是手填金额，`withdraw_min_yuan` 返回最小档位金额；填写非档位金额时服务端提示可选档位。原“运营设置 → 最低提现金额”不再生效。

## 发布顺序

先部署服务端与网页，再发布带首页入口和活动容器的 APK；旧版 APK 不受首页配置影响。
