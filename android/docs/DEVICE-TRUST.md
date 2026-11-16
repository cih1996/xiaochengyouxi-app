# DeviceTrust 0.5.0 Android 接入

应用包名 `com.lingzidev.xiaochengyouxi`，原发布签名继续使用。SDK 公共应用 ID：`24befb89afb1186d8dc492c722ba302d40b148e8f5ca5a5f`。

## 固定二进制

来源为独立项目正式交付包 `C:\Users\Arlo\Documents\Codex\2026-11-10\device-trust-platform-work\outputs\devicetrust-sdk-0.5.0.zip`。该源项目只读，本应用未修改 SDK。

- 交付包 SHA-256：`3c72848cac0520cadd4f9f0c0fb18c21afe46ebc2bcaf56e376f477895d44754`。
- `app/libs/device-trust-base-0.5.0.aar`：`e69c724f5117d37702dddd3514c3dd9de7a08a922317fe6b780d1f9cf7fce64d`。
- `app/libs/device-trust-secure-0.5.0.aar`：`5a33fabacb32039195783180e408a4f08b7beb13d5551b7119c0d1c9aa16d52d`。
- secure AAR 包含 arm64-v8a、armeabi-v7a、x86、x86_64 的 `libdtpsecure.so`。

只引入基础与 secure 两个模块，无第三方运行库新增。未引入 environment、rootbeer 或定位模块。

## 授权和采集

首次和升级用户通过 `device_trust_consent_v2` 明确确认隐私政策；Debug 也经过确认。SDK 构造器仅保存对象引用，读取设备密钥、采集和上报都在授权与登录之后。未授权时不会调用这三步。

授权后先加载业务服务 `/api/config`，`device_trust_enabled` 为必填布尔值。明确为 false 的本地 mock 服务沿用原收益流程，不调用 DeviceTrust；为 true 时登录提交证据、收益请求绑定验证事件。配置加载失败或字段缺失时显示错误，收益请求不使用未加载的默认值。生产后端开启 SDK 后也独立强制校验证据。

基础采集包含应用 ID、随机安装 UUID、包名、设备厂商/型号、Android API 级别、安全补丁、系统构建指纹、调试与模拟器线索、VPN/网络类型和运营商代码。`includeAndroidId=false`。蜂窝详情、Wi-Fi RSSI/BSSID、位置、应用列表、蓝牙和路由器 MAC 未启用；本应用没有新增位置、电话、短信、录音或存储权限。

SDK 使用独立 Android Keystore P-256 密钥注册公钥证书链，原生层生成 DTP4 加密签名帧，私钥保留在 Keystore。服务器证明等级仍由平台验证；持钥签名不等同已经完成硬件设备认证。SDK 设备密钥与 APK 发布签名分别管理。

基础采集后显式调用 `ResourceCollector.collect(context,report,consent)`。资源信号包括 Java/NDK 两路品牌、型号、设备代号、硬件平台、ABI、内存、核心数、SoC 信息，及加速度计、陀螺仪、磁场和气压传感器的名称/厂商/版本元信息；不读取实时传感器数据。secure AAR 通过明确 queries 读取 13 个指定系统组件的可见状态、版本、系统标记和当前 APK 签名摘要，不扩展到全量应用列表。

服务端 challenge 的 `hardwareRequired` 原样交给 SDK。0.5.0 的普通签名路径仅生成持钥签名密钥；硬件证明路径由 SDK 严格执行服务端要求。设备密钥保持既有登记，应用不调用 `reset`。SDK 返回的不透明 `deviceId` 由平台验证签名和服务端绑定，Android 不把自报 ID 当作业务身份或通行凭据。

## 业务协议

1. `SecureReporter.keyId(context, appId)` 返回已登记设备密钥 ID，首次为空。
2. 登录会话调用 `POST /api/risk/prepare {kind,key_id,intent:{method,path,body}}`，服务端返回 `{event_id,challenge,sdk:{app_id,base_url,spki_pins}}`。`path` 使用 `/api/...` 完整业务路径，`body` 与随后收益请求复用同一个 JSON 对象。账号、规范化请求摘要、币种与准确金额由服务器绑定，Android 不提交自算摘要。
3. 后台线程调用 `DeviceTrust.collect(context,new Options(appId,false),consent)` 和 `ResourceCollector.collect(context,report,consent)`，再将 challenge 原样传给 `SecureReporter.upload(context,base,pins,challenge,report)`。SDK 负责首次登记和 `/v1/secure` 加密上报，保留证书链、主机名和 SPKI 验证。
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

没有自动重试或后台队列。主线程的一次 SDK 请求状态避免首次密钥登记并行；已在处理时用户可以稍后重新操作。SDK 密钥异常不会重置或重新登记以掩盖错误。

## 错误与边界

SDK 异常不进入普通界面或发布版日志。`POST /api/risk/failure` 提交 event_id、kind、stage 与异常类名或 HTTP 状态码；事件尚未创建时 event_id 为空。用户只看到通用文案和随机处理编号。规则、分数、设备证据和平台原始响应由受保护服务端处理。

本地 SDK 异常另附私有 `detail`：最多四层异常类与消息摘要、总计最多 600 个字符（UTF-8 不超过 2KB），去掉当前会话 token，不复制 HTTP 响应正文或 JSON 解析正文。底层 Keystore 错误码可在后台排查，明细不进入消费者页面或发布日志。

应用私密 API 密钥、管理凭据和规则未放入 APK。SPKI 与公共应用 ID 属于公开接入配置。SDK 采集失败和已确认风险分别留痕。

本次只进行源码、AAR/ABI、构建与固定签名检查；设备安装和真实 Keystore/TLS 上报联调未执行，先前自动审批拒绝的设备操作未重试。
