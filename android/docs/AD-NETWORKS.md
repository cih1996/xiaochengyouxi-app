# 广告 SDK 文件

官方 AAR 原包，未修改，放在 `android/app/libs/`（`.gitignore` 排除）。来源：`private/ad-sdks/广告SDK接入包-四家-2026-11-18.zip`。

| 平台 | 文件 | SHA-256 |
|---|---|---|
| 穿山甲 7.8.1.4 | `open_ad_sdk_7.8.1.4.aar` | `4bf7551684c845b846d4b44e72c5ade6bea33ba7d815239cf835ea2a00c47efa` |
| 快手 5.9.10.1 | `kssdklite-ad-5.9.10.1.aar` | `81438ac1ad626db325e0f8d95ae744eed71ccc87f0979c2166b679a49ee0679f` |
| 百度 9.460 | `Baidu_MobAds_SDK-release_v9.460.aar` | `d47a04bfba5242dc88a50b092650cbb56bb4506519b31441e2ff08bff9c4dabf` |
| 优量汇 4.640.1510 | `GDTSDK.unionNormal.4.640.1510.aar` | `00850730de97532e132d1fa8b69dac957613a919b84cd4756d2ee9797e5e3311` |

代码：`AdNetwork`（统一接口）、`CsjNetwork`/`KsNetwork`/`BdNetwork`/`GdtNetwork`（适配）、`AdMediator`（按服务器计划依次尝试，15 秒超时换下一家）、`AdEvents`（统计上报）、`NativeFeedAds`（大厅信息流卡片）。

隐私配置：四家都关闭个性化广告；不读位置、电话状态、MAC、应用列表、存储；清单移除 `READ_PHONE_STATE`，保留我们自己的 `usesCleartextTraffic=false`（覆盖优量汇清单里的 true）。优量汇的 FileProvider 在 App 清单中声明，快手、百度由 SDK 清单自带。

AppID 由服务器 `/api/config` 的 `ads.networks` 下发，App 启动（同意隐私政策后）初始化这些平台；改 AppID 需要用户重启 App。
