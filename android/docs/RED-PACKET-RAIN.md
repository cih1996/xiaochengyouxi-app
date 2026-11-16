# 红包雨 · Android 0.3.4

大厅固定顶部条显示“距下场 hh:mm:ss”，位于标题下方、消息 ScrollView 外，滚动消息时保持可见。图片使用本项目实际透明素材：`rain_small`、`rain_large`、`rain_sky`；大型红包使用独立皇冠、金翅、金色边缘图案和原生脉冲动画。

## 服务端协议

- `GET /api/rain/status`：服务器时间、waiting/active/disabled、下一场时间、当前场次、是否已加入、当前配置。waiting 每 5 秒、active 每 2 秒刷新；进入大厅立即校准。
- `POST /api/rain/join {round_id}`：首次加入通过 DeviceTrust `rain` 事件并带精确场次的业务证明；服务器已标记 joined 的恢复请求直接取原分配。每用户每场由服务器固定随机 packet IDs。
- `POST /api/rain/claim {packet_id}`：小包只有 `credited:true` 的成功响应更新声音和飘字，实际余额使用服务端 user。重复领取 `credited:false` 仅同步状态。客户端不提交金额。
- 大包 claim 只返回 `rain:PACKET_ID` 任务，取得该包准确报价后打开专属弹层明确提示看视频；开始奖励带 `quote_id`，走独立 `reward` DeviceTrust 证明及真实激励视频，完成仍须平台回调验证。

## 时钟与生命周期

时间按服务器 ISO 毫秒 UTC 解析，以 `elapsedRealtime` 建立单调时钟；每次状态刷新重新校准。下落窗口严格使用服务端 reveal_at→expires_at，服务端已结合 fall_speed 倍率、持续时间和并发数量排定，客户端不再次延长窗口。

最后 10 秒显示可关闭倒计时，关闭只影响该次提示。雨层有“收起本场”按钮，展开时继续使用同一组 ID 和剩余有效窗口，不补发红包。大小类型来自服务器 kind，横向位置只是 ID 派生的视觉位置，不影响奖励。

切换菜单、退出、进入后台时清除轮询、倒计时和掉落/飘字动画。返回后立即请求状态并按服务器时间恢复同一分配；退出账号会清空内存分配。已开始的合法大包广告沿现有会话完成，客户端不在后台发起新广告。

系统关闭动画时使用静态红包位置，仍按服务端窗口移除过期红包。单包协议或领取错误只反馈该包，不阻止其余分配。

## 验证

`tools/RainTimingCheck.java` 对 ISO UTC、倒计时向上取整、恢复进度、过期边界和错误时间窗进行 JVM 检查。实际设备操作未执行，未重试先前被自动审查拒绝的设备操作。构建和固定签名另行核验。
