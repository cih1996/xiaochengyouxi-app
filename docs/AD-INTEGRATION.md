# 广告接入参考

Android 发布版已接入穿山甲 SDK 7.8.1.4：应用 5931950、开屏 899677611、激励视频 993606639、大厅模板信息流 993606636。Debug 保留模拟奖励流程。其余三家使用以下旧项目作为后续接入参考。

参考项目：`D:\工作\android-games\sheep-home-oem2`。

| 联盟 | 旧项目示例 | 旧项目 SDK 版本 |
| --- | --- | --- |
| 百度 | `BaiduProvider.java` | 9.460 |
| 快手 | `KuaishouProvider.java` | 5.9.10.1 |
| 腾讯优量汇 | `TencentProvider.java` | 4.640.1510 |
| 穿山甲 | `PangleProvider.java` | 7.8.1.4 |

以上文件位于旧项目 `app/src/main/java/com/huafuzhi/game/ganyanghuiquan/`，原有 `AdProvider.java` 展示了初始化、加载、展示、获得奖励和关闭的接口。

新 App 通过 `RewardedAdProvider` 调用激励视频，发布版实现为穿山甲，服务端验证奖励会话后单次入账。真实信息流通过 `TTNativeExpressAd` 展示，按消息复用并在会话结束时释放。广告网络的实际展示验证需联网手机。

购物入口当前展示本地商品样例。正式返佣需要配置各平台推广身份、商品推广链接和订单同步，再将预计返佣转成已结算收益。
