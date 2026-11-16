# 小橙有喜

在线测试版已部署，Android 发布版为 0.3.6，包名 `com.lingzidev.xiaochengyouxi`。网址、服务器目录、签名保管位置与验收范围见 [部署交接](docs/DEPLOYMENT.md)。固定名称、公钥和证书信息见 [发布身份](docs/RELEASE-IDENTITY.md)。

Android 原生 App + Rust / SQLite 服务 + Vue 预览与管理后台。

## 本地启动

```powershell
cd D:\工作\android-apps\orange-rewards
.\scripts\start-local.ps1
```

- App 浏览器预览：<http://127.0.0.1:38180>
- Vue 管理后台：<http://127.0.0.1:38180/admin>
- 演示账号：`13800138000` / `Orange123!`
- 注册、找回密码：先获取模拟短信验证码，验证码为 `123456`。
- 日志：`output/server.log`、`output/server.error.log`。

初次启动会构建前后端。已构建后可使用 `-SkipBuild`。需要 Node.js、Rust 和 Windows C++ 构建工具。npm 使用项目内的工具配置，不修改机器的全局配置。

## Android

```powershell
cd D:\工作\android-apps\orange-rewards\android
.\build-debug.ps1
```

APK：`android/build-output/app/outputs/apk/debug/app-debug.apk`。

通过 USB 或 adb 连接的设备安装并接入本地服务：

```powershell
cd D:\工作\android-apps\orange-rewards
adb devices
.\scripts\install-android.ps1 -Device '<设备序列号>' -SkipBuild
```

脚本执行 `adb reverse tcp:38180 tcp:38180`，App 默认访问 `http://127.0.0.1:38180`。机器上有多个设备时必须指定目标设备。

## 当前功能

- 大厅：登录并绑定邀请后，按服务端策略推送欢迎语、视频红包、返佣商品和 Android 原生信息流；消息保留在会话内存。
- 奖励：发布版使用穿山甲开屏和激励视频，Debug 使用模拟视频；金币入账和签到共用洒落音效。
- 欢迎与登录：首次欢迎动效、带标签的登录表单和密码显隐；未签到入口高亮并轻跳，签到后按服务端状态停止动画并禁用。
- 捡漏：商品列表、分类、商品详情预览。
- 我的：手机号密码账号、余额、收益明细、模拟提现、邀请统计、个人邀请链接与二维码海报。
- 邀请：6 位大写字母数字邀请码；推广页手机号预绑定；直接注册强制绑定，拒绝返回登录；服务端拦截未绑定账号。
- 后台：账号密码登录，右上角账户设置修改账号和密码；概览、用户、邀请管理、奖励记录、提现处理、奖励设置及三类内容的推送比例。
- 账号密码使用 Argon2 哈希；余额、奖励和提现记录保存在 SQLite。
- 更新：后台版本发布、可选与必要升级、APP 检查更新、签名安装包下载，以及大厅公告在线刷新。操作说明见 [APP 更新](docs/APP-UPDATES.md)。

短信、购物返佣和提现目前为测试流程。`1000 金币 = 1 元` 是当前测试设置。穿山甲开屏、激励视频和信息流 SDK 已接入，实际广告填充待联网手机验收。

百度、快手、优量汇仍待接入；购物需要推广身份和订单接口，短信与真实付款通道后续配置。四家广告示例索引见 [接入参考](docs/AD-INTEGRATION.md)。

## 目录

| 目录 | 内容 |
| --- | --- |
| `android/` | Java 原生 Android App |
| `server/` | Rust API 和 SQLite 数据 |
| `web/` | Vue 手机预览及管理后台 |
| `assets/` | 生成的原始素材图集和切图 |
| `scripts/` | 本地启动、安装和接口验证 |
| `output/` | 构建交付件、截图、运行日志 |

接口说明见 [API](docs/API.md)。美术通过内置 imagegen 生成，提示词和切图说明见 [素材说明](docs/ART.md)。

## 已验证

- Android Debug 构建通过，实机完成登录、红包领取、5 秒模拟播放、奖励到账与页面浏览。
- Rust 构建和格式检查通过，新增邀请及推送集成检查 58 项通过，覆盖旧库迁移、强制绑定、预绑定承接、码唯一性和信息流能力隔离。
- Vue 生产构建通过，Chrome 检查了领奖余额变化、商品详情、三个菜单、手机宽度布局和管理台。
- 浏览器账号流程通过：注册、模拟验证码、密码重置、新密码登录、退出、过期会话清理。
- 模拟提现扣除 1000 金币，经管理台退回后恢复 1000 金币。
- 金币音效验证：签到、视频奖励各播放一次；重复签到和关闭弹层保持安静，浏览器音频上下文运行正常。
- 大厅推送验证：欢迎语、红包、商品到达；切换菜单保留消息，刷新后消息 ID 全新，退出停止接收。大厅内容仅保留在客户端内存，服务端仅保存策略配置。

0.3.0 门禁、保存海报和分享页已在 Android 实际验证；海报 PNG 1080×1600 已独立解码。0.3.1 欢迎/登录/签到已通过浏览器交互验证及 Android 构建签名检查；该版本 Android 安装截图验收因工具自动审批拒绝而未执行。交付包为 `output/小橙有喜-0.3.1-release.apk`，平台海报为 `output/小橙有喜-邀请海报.png`。

API 验证：`scripts/smoke.ps1`，会创建测试账号并操作本地测试数据。浏览器验证脚本为 `tools/verify-web.cjs`、`tools/verify-operations.cjs`、`tools/verify-coin-sound.cjs`、`web/tests/hall-ui.cjs`、`web/tests/auth-ui.cjs`；需要在 `tools/` 安装 Playwright，并使用机器已有 Chrome。

本地开发服务默认仅监听本机；线上服务已配置 HTTPS、后台鉴权和 systemd。邀请码流程说明见 [邀请说明](docs/INVITATIONS.md)。

DeviceTrust 设备风控的后续接入要求见 [接入约定](docs/DEVICE-TRUST-INTEGRATION.md)，当前实施阶段为方案记录与独立 SDK 部署，App 尚待接入。
