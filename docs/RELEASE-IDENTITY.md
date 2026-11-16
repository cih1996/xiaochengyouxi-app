# 小橙有喜 · 固定发布身份

| 项目 | 固定值 |
| --- | --- |
| 正式应用名称 | 小橙有喜 |
| Android 包名 | `com.lingzidev.xiaochengyouxi` |
| 发布证书算法 | RSA 4096 位 |
| Alias | `release_5603cecffc2142b0` |
| 证书 SHA-1 | `80:DB:79:D3:8B:E6:58:88:D4:12:A1:63:0C:31:23:06:88:72:F2:71` |
| 证书 SHA-256 | `E8:C6:93:62:5B:A7:88:F0:69:BD:77:4C:83:8A:58:99:04:6E:1E:F1:83:75:E3:D9:72:62:52:99:36:EF:E0:D3` |

发布构建沿用现有证书，后续仅递增版本号。私钥、密码与公共证书保存位置：

- 工作目录：`D:\工作\android-apps\orange-rewards\android\signing\`。
- 独立备份：`D:\工作\密钥备份\orange-rewards\`。
- 私钥：`orange-release.jks`；构建凭据：`keystore.properties`。
- 公共证书：`release-certificate.der`、`release-certificate.pem`。
- 公钥：`release-public-key.pem`。

以上公钥文件由现有证书导出，发布私钥保持原样。源码库忽略签名私钥和凭据，项目约定在 `AGENTS.md` 中固定了发布身份和恢复方式。

后续软著及应用市场资料统一使用上述应用名称和包名。源码、说明书和界面截图按正式版本准备；私钥与凭据单独保管。
