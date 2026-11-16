# Android 发布签名

当前包名为 `com.lingzidev.xiaochengyouxi`，应用名为“小橙有喜”。发布签名已创建，后续更新继续使用这份签名。

## 文件位置

- 工作密钥：`D:\工作\android-apps\orange-rewards\android\signing\orange-release.jks`
- 构建凭据：`D:\工作\android-apps\orange-rewards\android\signing\keystore.properties`
- 公共证书：同目录 `release-certificate.der`
- PEM 公钥：同目录 `release-public-key.pem`；PEM 证书为 `release-certificate.pem`。
- 第二份备份：`D:\工作\密钥备份\orange-rewards\`，包括相同的私钥、凭据及公开证书文件。

密钥和密码文件已加入 Android 忽略规则。备份目录在项目之外。构建时由 Gradle 读取密码文件，APK 包含公共证书；回调密钥仅由服务端管理。

## 证书

- 算法：RSA 4096 位。
- 有效期：10000 天，自创建日期起。
- Alias：`release_5603cecffc2142b0`。
- SHA-1：`80:DB:79:D3:8B:E6:58:88:D4:12:A1:63:0C:31:23:06:88:72:F2:71`。
- SHA-256：`E8:C6:93:62:5B:A7:88:F0:69:BD:77:4C:83:8A:58:99:04:6E:1E:F1:83:75:E3:D9:72:62:52:99:36:EF:E0:D3`。

## 构建和核验

运行 `./build-release.ps1`，产物为 `build-output/app/outputs/apk/release/app-release.apk`。发布构建使用 HTTPS 服务和真实穿山甲广告，Debug 使用本地服务与演示广告。

签名已通过 Android Build Tools 35.0.0 的 `apksigner verify`。v2、v3 均有效；使用 `--min-sdk-version 23` 检验也确认 v1 有效（应用实际最低版本为 API 24）。两份 JKS 的 SHA-256 文件校验一致。

发布版切换到真实服务并且不预填体验密码。Debug 和 Release 使用不同证书，同包名覆盖安装时会显示签名不匹配；测试设备可先卸载原来的 Debug 应用，正式发布后的更新继续使用这份 Release 证书。

签名密码保存在上述私有文件中，不在文档和构建输出中重复记录。
