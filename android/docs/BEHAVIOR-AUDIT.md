# 行为采集与 DeviceTrust 0.6

Android `0.3.17` 使用 DeviceTrust 0.7.3 启动登记。隐私同意保存键为 `device_trust_consent_v4`；同意前不调用 SDK。启动后从 `/api/device/config` 获取公开应用 ID、业务前缀和证书 SPKI pins，再由 SDK 在后台调用 `/api/device/prepare` 完成设备密钥登记与报告上报。SDK 生成的 deviceId 只用于服务端设备关联，客户端不把它作为登录凭据或收益授权。

登录成功后，若服务端行为策略开启，Android 生成独立 Android Keystore P-256 签名密钥，并通过 `kind=behavior` 的真实 DeviceTrust challenge 绑定公钥。会话返回 OBA1 的 AES-256-GCM 密钥、4 字节 nonce 前缀、挑战、过期时间和采集策略；密钥只保留在内存中，退出账号、账号切换、过期或 Activity 销毁时清除。

每个行为包包含大端固定头、AES-GCM 密文和 ECDSA DER 签名。头部签名覆盖 `OBA1`、会话 UUID、连续序号、服务端挑战、nonce、前一包哈希和密文长度；AAD 是完整108字节头，packet hash 是 `SHA-256(header + ciphertext)`。服务端确认后返回下一挑战和链哈希，重复的同序号包可安全重试一次；网络失败不会写离线文件，也不会无限重试。

采集范围限于登录后的本应用：页面进入/离开、前后台、稳定控件点击、归一化触点和手势时长、前台心跳，以及服务端操作编号对应的业务事件。触点为0至10000的相对坐标，停留使用 `SystemClock.elapsedRealtime()`。密码、短信、输入框文字、屏幕截图、其他应用和金额文本均不采集；广告等待不算大厅停留。业务事件只携带服务端操作编号，到账状态由服务端账本对账后写入回放记录。

稳定屏幕枚举：`0 unknown`、`1 hall`、`2 shop`、`3 mine`、`4 packet`、`5 reward_video`、`6 product_detail`、`7 withdrawal`、`8 invite`、`9 rain`、`10 login`、`11 register`、`12 settings`、`13 check_in`、`14 reward_result`。采集策略缺失、协议不兼容、加密或上报错误会写后台诊断并记录 `Log.e`；普通用户只看到通用状态。

验证：`tools/BehaviorProtocolCheck.java` 使用 `server/src/behavior-vector-v1.json` 的公开测试向量，纯 Java 验证 AES-GCM 解密、ECDSA 签名和 packet hash；本次输出 `OBA1 vector verified: header=108, payload=88, ECDSA, AES-GCM, packet hash`。Debug/Release 构建均通过，未安装设备。
