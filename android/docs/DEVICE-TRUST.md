# DeviceTrust 0.7.6 Android 接入

应用包名 `com.lingzidev.xiaochengyouxi`，原发布签名继续使用。SDK 公共应用 ID：`24befb89afb1186d8dc492c722ba302d40b148e8f5ca5a5f`。Android 0.3.17（versionCode 20）起使用 0.7.6。

## 固定二进制

来源为独立项目 `~/work/device-trust-platform` 提交 `58e11ab` 的 release 构建（DeviceTrust 会话 clean build 交付，lint 0 错误），本应用未修改 SDK。完整发行包 `devicetrust-sdk-0.7.6.zip`：`d0dfbbadef8c144eae20125db5c4e5e40cd7d7ab4a7c0c9a803cfd29c1651a33`。

- `app/libs/device-trust-base-0.7.6.aar`（sdk-0.7.6）：`59c82e230700d3f2a3d6c3784cca4f70df4374dd4317d8803d1b5b3917d8ad11`。
- `app/libs/device-trust-secure-0.7.6.aar`（sdk-secure-0.7.6，与 0.7.4 字节相同）：`ad4193ba123932e659b525cadfd38214205c8e9bcef74364944bd0ecde6ee4e9`。
- secure AAR 包含 arm64-v8a、armeabi-v7a、x86、x86_64 的 `libdtpsecure.so`。

只引入基础与 secure 两个模块，无第三方运行库新增。未引入 environment、rootbeer 或定位模块。

## 授权和采集

0.7.2 扩大了采集范围，隐私同意键升级为 `device_trust_consent_v4`，已同意 v3 的用户升级后重新确认。未授权时不读取设备密钥、不采集、不上报。

基础采集包含应用 ID、随机安装 UUID、包名、设备厂商/型号、Android API 级别、安全补丁、系统构建指纹、调试与模拟器线索、VPN/网络类型、HTTP 代理、默认网卡名、注册运营商与 SIM 运营商、漫游状态，以及应用可调试标记、ADB/开发者选项、无障碍服务数量和应用沙箱（用户 ID、数据目录）。`includeAndroidId=true`：SDK 0.7 启动登记固定上报 ANDROID_ID（按签名证书隔离，用于平台跨应用设备关联），业务报告保持一致。蜂窝详情、Wi-Fi RSSI/BSSID、位置、应用列表、蓝牙和路由器 MAC 未启用；本应用没有新增权限。

0.7.4 起采集 `lan`（第一个 Wi-Fi/以太网的 IPv4 地址与前缀、默认网关、IPv4 DNS、DHCP 服务器、网卡名，仅用 `ACCESS_NETWORK_STATE`）；挑战带 `lanTargets` 时，上报前在 3 秒预算内探测本机网段内指定地址是否可达及 5555 端口（ADB 无线调试）。服务端须把平台挑战原样转给 App（`device_bootstrap.rs`、`risk.rs` 已原样透传）。0.7.6 起 `includeAndroidId=true` 同时上报 Widevine `DEVICE_UNIQUE_ID` 的 SHA-256（`drmId`，原值不上传），平台升级到 0.7.6 后用于重装后找回原 deviceId；线上平台为 0.7.5 时照常登记，只是重装会分配新 deviceId。隐私政策 1.3 与启动同意弹窗已披露上述项目。

`ResourceCollector.collect` 在 0.7.2 内部调用 RiskCollector：通过 sdk-secure manifest 的 `<queries>` 逐个查询 42 个固定风险工具包名是否安装（不申请 `QUERY_ALL_PACKAGES`），读取已开启无障碍服务的包名与手势能力、`isUserAMonkey`、TouchAudit 计数、进程内 Hook 类与调用栈；原生层读取 TracerPid、maps 中的 Hook/注入代码、模拟器属性，并对 127.0.0.1 全端口做约 2 秒的连接扫描识别 frida-server 等调试服务。

`MainActivity.dispatchTouchEvent` 及 BehaviorAudit 包装的弹窗窗口回调调用 `TouchAudit.record(event)`，只累计按下次数、模拟点击形态和虚拟输入设备次数。

SDK 使用独立 Android Keystore P-256 密钥注册公钥证书链，原生层生成 DTP4 加密签名帧，私钥保留在 Keystore。0.7 起新密钥生成时总是附带挑战值请求硬件证明证书链，`hardwareRequired` 不再影响客户端密钥生成；是否拒绝未通过证明的设备由平台“接入保护”配置决定。0.7.3：`hardwareRequired=false` 时若带证明生成密钥抛 `ProviderException`，SDK 删除残留别名后不带证明重新生成，并在登记包附 `attestationError`，平台记为 `signature_only`、`issue=keystore_failed`；`hardwareRequired=true` 时直接失败、不降级。该降级登记需要 DeviceTrust 0.7.3 服务端，线上仍为 0.7.2 时会以 400 拒绝。已有密钥继续沿用；唯一例外是系统抛出 `KeyPermanentlyInvalidatedException`（密钥被 Android 永久作废，例如修改锁屏后），应用调用 `SecureReporter.reset` 并用 `/device/config` 的参数重新登记，原事件记为 `key_reset` 核验错误后重试本次操作一次，同一安装 24 小时内最多重置 2 次（平台对 24 小时内 3 次重登记 frequent_reinstall）。SDK 返回的不透明 `deviceId` 由平台验证签名和服务端绑定，Android 不把自报 ID 当作业务身份或通行凭据。

## 业务协议

1. `SecureReporter.keyId(context, appId)` 返回已登记设备密钥 ID，首次为空。
2. 登录会话调用 `POST /api/risk/prepare {kind,key_id,intent:{method,path,body}}`，服务端返回 `{event_id,challenge,sdk:{app_id,base_url,spki_pins}}`。`path` 使用 `/api/...` 完整业务路径，`body` 与随后收益请求复用同一个 JSON 对象。账号、规范化请求摘要、币种与准确金额由服务器绑定，Android 不提交自算摘要。
3. 后台线程调用 `DeviceTrust.collect(context,new Options(appId,true),consent)` 和 `ResourceCollector.collect(context,report,consent)`，再将 challenge 原样传给 `SecureReporter.upload(context,base,pins,challenge,report)`。SDK 负责首次登记和 `/v1/secure` 加密上报，保留证书链、主机名和 SPKI 验证。
4. 客户端提交 `POST /api/risk/complete {event_id,report_id}`，只消费通用业务状态与追踪号。
5. `recorded` 状态的收益请求携带 `x-risk-event: event_id`。红包开始、签到、提现申请、红包雨加入分别使用 reward/check_in/withdraw/rain；红包完成沿用开始时绑定的事件。平台 claim、permit 和执行关口由服务器处理，不进入客户端。

| 场景 | intent 路径 | body |
| --- | --- | --- |
| 登录证据 | `/api/auth/login/report` | `{}`（仅登录报告语义，不另发此业务请求） |
| 红包奖励 | `/api/rewards/start` | `{task_id,quote_id}` |
| 签到 | `/api/check-in` | `{}` |
| 提现 | `/api/withdrawals` | `{amount_yuan,channel,account}` |
| 红包雨加入 | `/api/rain/join` | `{round_id}` |

红包打开前调用 `POST /api/rewards/quote {task_id,key_id?}`。SDK 开启时只读既有密钥 ID，首次尚未登记时为空；SDK 关闭的本地服务省略该字段。服务器返回 `{quote_id,reward,task,...}`，客户端用报价中的准确金币展示弹层和领取按钮，开始请求必须带原 `quote_id`。大厅卡片按任务 `reward_min/reward_max` 显示范围，报价返回后替换为该次锁定任务；任务刷新同时更新金额、标题和领取状态。红包雨大奖也经过报价，金额仍由原场次分配决定。

普通登录在页面展示后独立提交 login 证据。邀请未绑定会话仍可提交登录证据。收益动作须先完成本次 SDK 报告，review/restricted 或采集上报失败时不发起收益请求；业务后端同时强制验证同用户、同场景、时效及一次消费。

没有自动重试或后台队列。主线程的一次 SDK 请求状态避免首次密钥登记并行；已在处理时用户可以稍后重新操作。除上述密钥被永久作废的情况外，SDK 密钥异常不会重置或重新登记以掩盖错误。

## 错误与边界

SDK 异常不进入普通界面或发布版日志。`POST /api/risk/failure` 提交 event_id、kind、stage 与异常类名或 HTTP 状态码；事件尚未创建时 event_id 为空。用户只看到通用文案和随机处理编号。规则、分数、设备证据和平台原始响应由受保护服务端处理。

本地 SDK 异常另附私有 `detail`：最多四层异常类与消息摘要、总计最多 600 个字符（UTF-8 不超过 2KB），去掉当前会话 token，不复制 HTTP 响应正文或 JSON 解析正文。底层 Keystore 错误码可在后台排查，明细不进入消费者页面或发布日志。

应用私密 API 密钥、管理凭据和规则未放入 APK。SPKI 与公共应用 ID 属于公开接入配置。SDK 采集失败和已确认风险分别留痕。

本次只进行源码、AAR/ABI、构建与固定签名检查；设备安装和真实 Keystore/TLS 上报联调未执行，先前自动审批拒绝的设备操作未重试。
