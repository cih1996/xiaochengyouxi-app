# 首页入口、网页活动与提现档位

## 首页入口（已移除）

后台“首页入口”（图标宫格 + 悬浮入口）和旧版转盘已移除。`GET /api/app/home` 仅为旧版 App 保留，固定返回空布局。活动入口改为大厅右上角的专属悬浮入口：新人礼包、幸运转盘（`docs/NEWBIE-GIFT.md`）。

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
| `OrangeApp.sharePoster(target, pngBase64, text)` | 0.3.24 起：`wechat` 发微信好友 / `moments` 存相册并打开微信 / `save` 存相册 |
| `OrangeApp.hasWeChat()` | 0.3.24 起：是否安装微信 |

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
