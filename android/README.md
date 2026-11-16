# 小橙有喜 Android

Java 原生界面，Android 系统动画。最低 Android 7.0，编译和目标 SDK 35。广告使用穿山甲 7.8.1.4 及其 AndroidX 支持依赖。

## 本地运行

先启动项目根目录的本地服务，再执行 `./build-debug.ps1`。APK 位于 `build-output/app/outputs/apk/debug/app-debug.apk`。

连接单台指定设备后：

```powershell
adb -s <设备序列号> reverse tcp:38180 tcp:38180
adb -s <设备序列号> install -r ./build-output/app/outputs/apk/debug/app-debug.apk
adb -s <设备序列号> shell am start -n com.lingzidev.xiaochengyouxi/.MainActivity
```

体验账号：`13800138000`，密码 `Orange123!`。注册、密码重置先获取验证码，当前短信返回 `123456`。会话保存到应用私有存储。

`./build-release.ps1` 生成签名发布 APK，连接 `https://orange-api.lingzidev.com` 并启用真实开屏、激励视频。发布版登录页为空白输入框。签名和备份位置见 [docs/SIGNING.md](docs/SIGNING.md)。

## 界面和动效

- 欢迎：首次展示小橙角色、浮动红包与稀疏金色星光，“开启小欢喜”之后进入已有隐私确认和账号流程。`welcome_seen` 只记录欢迎页是否已看过，退出账号不重复展示欢迎页。
- 登录：登录、注册、找回密码使用统一暖色品牌插画，带清晰字段标签和密码可见切换。
- 大厅：策略推送的助手消息、红包和商品推荐，签到、余额。
- 红包雨：大厅顶部固定下场倒计时、最后十秒提示、透明天空雨层、小红包直接入账与金喜大包激励视频。协议和生命周期见 [docs/RED-PACKET-RAIN.md](docs/RED-PACKET-RAIN.md)。
- 签到：未签到展示橙金按钮、金币轻跳和微光，成功后静态显示“今日已签到”并禁用；状态使用服务端 `checked_in_today`，失败后恢复可点击，409 时刷新服务端用户状态。
- 领奖：红包轻摇、翻转打开、5 秒激励广告演示、奖励弹层、金币沿曲线飞入余额；金币到账和签到成功播放金币落袋音效。
- 金额：大厅红包卡片按服务端任务显示奖励范围，打开时先取该次准确报价；领取按钮、视频奖励参数和会话金额使用服务器锁定的报价。
- 捡漏：分类、商品预览、预计返佣。
- 我的：金币、今日收益、累计收益、模拟提现、明细、账号设置。
- 更新：隐私同意后检查版本，设置页显示当前版本并可手动检查；下载带进度、取消及失败重试，校验发布文件和安装身份后调用系统安装。详情见 [docs/APP-UPDATES.md](docs/APP-UPDATES.md)。
- 公告：大厅固定顶部公告有内容时展示；启动、回到前台及前台每 45 秒获取最新公告，直接更新文字，不重建大厅或广告。
- 账号：手机号密码登录、注册、验证码密码重置、退出。
- 账户安全：授权后使用 DeviceTrust 0.5.0 基础与 secure 模块，采集必要设备配置和指定系统组件签名，以独立 Keystore 密钥加密签名上报；登录关联账号，收益操作绑定经验证的业务事件。协议、采集字段与二进制校验见 [docs/DEVICE-TRUST.md](docs/DEVICE-TRUST.md)。
- 邀请：首次登录必须绑定邀请码；从邀请页面登记手机号后，注册时由服务端自动绑定。“我的”可生成 1080×1600 邀请海报，复制链接或邀请码、保存图片、调用系统分享。

美术来源为根目录 `assets` 的生成图片，`syncArtwork` 构建任务将八张独立资源复制并转换为 Android 资源名。主界面在 `MainActivity.java`，图标由 `LineIcon.java` 原生绘制，HTTP 在 `Api.java`。实际错误直接展示。

大厅通过带 Bearer 会话的 `/api/hall/stream` 接收 SSE。`HallStream.java` 使用单个原生 HTTP 读取线程，将事件直接交给页面。消息只在当前进程的账号会话内存中保留，切换底部菜单继续展示，退出、重置密码、重新登录及进程结束后清空。只在浏览位置靠近底部时跟随新消息；连接失败直接显示错误，不自动重连。

金币到账音效为 `res/raw/coin_arrival.wav`，由原生 SoundPool 预载，成功入账时播放一次，遵循媒体音量。重复到账确认和领取按钮不重复播放。

欢迎和签到动画由 `WelcomeArt.java`、`CheckInButton.java` 的原生属性动画完成。视图隐藏、移除或销毁后取消动画；Android 8.0 以上读取系统动画启用状态，系统关闭动画时使用静态画面。

运行 `./format-java.ps1` 可用 google-java-format 1.24.0 格式化 Java 源码。首次执行会下载开发工具到 `tools/`，不进入 APK，JAR 已加入忽略规则。

广告入口经过 `RewardedAdProvider`：Debug 为演示，Release 使用 `PangleAds`。首次使用先明确同意隐私政策后初始化 SDK 和展示开屏，设置页可查看隐私政策及用户协议。激励视频关闭后查询原会话的服务端状态，服务器已验证且金额匹配才确认入账；待确认可手动查询，实际 SDK 通知和错误记录到受保护后台。协议见 [docs/CSJ-REWARDS.md](docs/CSJ-REWARDS.md)。购物入口为商品预览，提现为模拟流程。

Release 的大厅 SSE 使用 `native_ads=1`，策略可推送 `native_ad`。`NativeFeedAds.java` 按官方模板渲染信息流方案获取 SDK 视图，高度自适应为 0，保留 SDK 广告内容、标识与不感兴趣入口。每条事件在当前账号会话复用同一 SDK 广告和视图，切换菜单不重新请求；退出、重新登录、销毁及不感兴趣时释放资源。应用后台不发起信息流请求，错误直接展示。真实开屏、激励与信息流填充仍需联网手机核验；当前 ADB 测试设备无外网。

`InvitePoster.java` 使用现有美术和服务端二维码矩阵绘制海报，`QrLayout.java` 保持整像素模块和四模块白边。海报缓存为 `cache/invite-posters/invite-CODE.png`，分享由独立的 `${applicationId}.invite-files` FileProvider 授权读取。Android 10 以上直接保存到相册，较早版本保存到应用图片目录并支持分享。未绑定用户不会启动大厅或 SSE，返回时清除会话并回到登录页；后端也独立限制业务接口。

SDK AAR 来源于旧项目 `sheep-home-oem2/app/libs/open_ad_sdk_7.8.1.4.aar`，已复制到本项目 `app/libs/`。SDK 在应用首次隐私确认之前不调用初始化和广告请求，位置、应用列表、电话状态、Wi-Fi 状态、外部存储、Android ID、录音、短信与运动信息访问开关均关闭。

SDK 7.8.1.4 的 `TTFileProvider` 仍引用旧版 support-v4，工程启用 Jetifier 将其转为 AndroidX 引用。Manifest 保留 SDK Demo 声明的 provider 和私有下载目录；不能只打开 AndroidX 而遗漏 Jetifier。
