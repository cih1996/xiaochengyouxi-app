# 部署与发布交接

## 在线入口

| 用途 | 地址 |
| --- | --- |
| 网页与注册入口 | https://orange.lingzidev.com/ |
| 管理后台 | https://orange.lingzidev.com/admin |
| App API | https://orange-api.lingzidev.com/api/ |
| 穿山甲回调 | https://orange-api.lingzidev.com/csj/callback |
| 签名 APK | https://orange.lingzidev.com/download/orange-rewards.apk |
| 隐私政策 | https://orange.lingzidev.com/privacy |
| 用户协议 | https://orange.lingzidev.com/terms |

当前 Android 发布版为 `0.3.18`（versionCode 21），正式名称“小橙有喜”，固定包名 `com.lingzidev.xiaochengyouxi`。继续使用原发布证书。版本发布、在线公告与升级流程见 [APP 更新](APP-UPDATES.md)。

## 私密资料

后台初始账号 `admin`，初始密码及专用验收账号保存在 `D:\工作\android-apps\orange-rewards\private\deployment-access.json`，副本在 `D:\工作\密钥备份\orange-rewards\deployment-access.json`。登录后点击右上角账号进入账户设置，可修改账号和密码；修改后所有管理会话失效，需要重新登录。修改后的凭据保存在 SQLite，环境变量及初始信息文件不覆盖它们。

账号功能已部署：5 项服务端针对测试通过；公网验证账号密码登录、账户查询、旧静态凭据失效及退出撤销；浏览器验证登录、账户设置、密码显隐、390px 布局和退出，未修改生产账号凭据。线上只读复核脚本为 `tools/verify-admin-account-live.py`，其中登录资料仅适用于用户尚未修改初始账号的阶段。

服务端环境变量与回调密钥保存在本地 `private/server.env`，服务器上为 `/etc/orange-rewards/server.env`。这些文件和签名目录均已排除版本控制。私密目录已限制为当前 Windows 用户访问。文档与 APK 不包含回调密钥或管理密码。

发布签名及备份详见 [签名说明](../android/docs/SIGNING.md)。后续更新使用相同签名。

## 服务器

- Ubuntu 26.04，Nginx 反向代理与 TLS。
- Rust 服务使用独立 `orange` 用户，监听 `127.0.0.1:38180`，通过 systemd 常驻并随开机启动。
- 程序与网页：`/opt/orange-rewards/current/`。
- 数据库：`/var/lib/orange-rewards/orange.sqlite3`。
- 安装包：`/var/www/orange-downloads/orange-rewards.apk`。
- Nginx 配置：`/etc/nginx/sites-available/orange-rewards`。
- TLS 证书：`/etc/nginx/ssl/<域名>/`。本次提供的两张证书均于北京时间 **2027-02-08 23:59:59** 到期；续期后替换对应证书及私钥并重载 Nginx。
- Nginx 日志：`/var/log/nginx/orange*.log`。日志记录路径，省略回调参数。
- 服务日志：`sudo journalctl -u orange-rewards -n 100 --no-pager`。

常用命令：

```sh
sudo systemctl status orange-rewards --no-pager
sudo systemctl restart orange-rewards
sudo nginx -t && sudo systemctl reload nginx
```

依赖按 `Cargo.lock` 固定。服务器已保存相同版本的构建依赖源码，可在 `server/` 使用 `cargo build --release --locked --offline --jobs 2` 构建。

## 广告配置

| 项目 | ID |
| --- | --- |
| 穿山甲应用 | 5931950 |
| 激励视频代码位 | 993606639 |
| 开屏代码位 | 899677611 |
| 大厅模板信息流代码位 | 993606636 |

本次使用旧项目提供的穿山甲 SDK `7.8.1.4`。激励视频通过 `user_id` 和 JSON `media_extra.reward_session_id` 关联服务端奖励会话。服务端按官方 `SHA256(SecurityKey:trans_id)` 验签，只确认该会话的奖励资格；App 收到有效奖励事件并关闭广告后请求入账。交易与账本防重复。

穿山甲后台应为激励视频代码位开启服务端奖励回调，填写上述 **HTTPS 回调地址**，密钥使用用户提供的值。平台登记信息统一为“小橙有喜”和 `com.lingzidev.xiaochengyouxi`。

信息流使用穿山甲原始模板，保留广告标识。后台“推送策略”的信息流占比控制推送比例，仅具有原生 SDK 能力的 Android 发布版请求此类型。

官方协议参考：[奖励回调](https://www.csjplatform.com/supportcenter/5416)、[开屏接入](https://www.csjplatform.com/supportcenter/5398)。

## 验证结果与当前范围

- Android 签名 v1/v2/v3 验证通过，双份私钥文件哈希一致。
- Release / Debug 构建通过；实机确认隐私确认页与发布版登录页正常启动。
- 服务端 28 项 CSJ/后台鉴权检查及 49 项本地回归检查通过。
- 公网 19 项 HTTPS 接口验证通过，覆盖后台鉴权、账号、合成签名回调、防重复入账与三类 SSE 推送。
- 浏览器实际完成公网后台登录、账号登录、推送与政策页检查，无运行错误。
- 本次回调测试使用专用验收账号及**合成回调请求**；不能视为穿山甲实际广告回调的实播验收。
- 所有当前 ADB 设备均无默认网络，开屏请求返回 SDK 602；真实开屏展示、激励视频播放及平台实际回调需由正常联网手机继续验收。
- 短信、购物返佣与提现继续使用测试流程；生产配置已关闭浏览器模拟视频发奖，网页红包引导下载 App。

检查输出位于 `output/deployment-verification.json` 与 `output/live-web-verification.json`。
