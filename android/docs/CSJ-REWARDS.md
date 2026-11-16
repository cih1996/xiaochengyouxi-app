# 激励奖励确认 · Android 0.3.5

使用穿山甲 7.8.1.4 的 `TTRewardVideoAd.REWARD_EXTRA_KEY_*` 常量解析回调。缺失字段保持缺失，不创建错误码 0 或文本 null。旧 `onRewardVerify` 只记录诊断，当前 `onRewardArrived` 记录每种奖励通知；后续进阶任务失败不会清除已收到的有效通知。

`RewardedAdProvider.Notice` 只表达客户端观察，不授予奖励。广告关闭立即回到应用，释放 SDK listener；真实广告仅单次查询 `GET /api/rewards/:id/status`，`verified/completed` 且会话金额匹配后才提交 `complete`。SDK valid、视频完成或跳过状态都不能替代服务器验签结果。

pending 使用原会话 ID 显示待确认页面，用户点击“查询奖励”重新查询同一会话。没有自动轮询。切入后台关闭页面，返回前台恢复尚未关闭的确认页；退出账号清理会话页面。模拟广告仍按本地服务的演示完成规则处理。

`POST /api/rewards/:id/client-event` 记录 reward_arrived、reward_verify、ad_closed、video_error。只发送当前会话的通知布尔值、奖励类型、服务端回调标记和实际错误码/限长错误信息，不发送整个 Bundle。错误详情只供受保护后台诊断，不影响平台回调验签或实际入账。

业务金币由 quote 锁定。`AdSlot.setRewardAmount` 可用于 SDK 展示，平台服务器回调的 reward_name/reward_amount 对照代码位配置校验，不能用其数量覆盖业务会话金币。

依据：[Android 激励广告文档](https://www.csjplatform.com/supportcenter/5399)、[服务器奖励回调文档](https://www.csjplatform.com/supportcenter/5416)，以及交付 AAR 的公开常量与回调派发字节码。
