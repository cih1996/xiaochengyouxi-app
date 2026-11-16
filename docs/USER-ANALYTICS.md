# 用户中心统计

管理员接口 `GET /api/admin/user-center/stats` 直接查询 SQLite 中的用户和成功行为，沿用后台管理员鉴权。统计不创建用户、行为或趋势数据，不查询订单或收入。

## 日期

默认 `?days=30`，也支持 `days=7`、`days=90`。自选使用 `?start_date=2026-11-01&end_date=2026-11-11`，两个日期必须同时提供，不与 days 混用。日期为北京时间（固定 UTC+8）自然日，含开始日和结束日，最多366天；结束日不能晚于今天。今天统计截至响应的 `metadata.generated_at`。

## 返回字段

`selected_range`：`start_date`、`end_date`、`days`。

| summary 字段 | 口径 |
| --- | --- |
| total_users | 截至查询时的累计注册人数 |
| today_registrations / yesterday_registrations | 北京时间今日 / 昨日新增注册 |
| range_registrations | 所选日期区间新增注册 |
| today_active_users | 今日去重活跃用户 |
| active_users_7d / active_users_30d | 含今天的近7 / 30个自然日去重活跃用户，与自选区间分别计算 |
| range_active_users | 所选日期区间去重活跃用户 |
| range_total_users | 截至所选结束日已注册的用户数 |
| range_inactive_users | range_total_users 减 range_active_users，未把区间结束之后才注册的人算作历史未活跃 |

`daily` 按日期升序，含 `date`、`registrations`、`active_users`、`total_users`、`returning_active`。空天新增/活跃补0，累计注册人数延续前一天。`returning_active` 是注册日期早于该日的活跃人数。

`activity_days` 是所选区间活跃天数分布，每位活跃用户只落入一个区间：`{key:"1",label:"1天",users}`、`2_3`、`4_7`、`8_plus`。无活跃记录用户由 summary 的 range_inactive_users 单独展示。

## 活跃口径

`user_behavior.status='success'`，有真实用户且行为不早于注册时间，按用户及北京时间日期去重。白名单包含：

`register`、`login`、`app_open`、`password_reset`、`logout`、`invite_bind`、`hall_request`、`reward_start`、`reward_complete`、`check_in`、`withdrawal`、`rain_join`、`rain_small_claim`、`rain_large_open`、`rain_large_claim`。

成功 `GET /me` 记录 `app_open`，覆盖保留登录状态打开应用的用户。一天内刷新自身资料或多个业务操作仍只计一个活跃用户。风控 SDK 自动上报、广告 SDK 诊断、被动推送、后台查看、失败请求不算活跃。`invite_prebind` 可能由其他人提交某手机号，不能据此认定该手机号所属用户活跃，因此排除。

响应顶层 `activity_definition` 为简短中文口径，metadata 返回 `timezone`、`generated_at`、`behavior_collection_since`、`returning_active_definition`、`inactive_definition`、`history_note`、`active_kinds`、`active_status`。未采集的历史活跃不补算；注册趋势仍以真实 users.created_at 为准。

## 实现与验证

`server/src/user_stats.rs` 无初始化表和统计任务。固定两条批量聚合 SQL 分别统计注册自然日与活跃用户自然日；行为 CTE 复用于日趋势、7/30日去重、所选区间去重和活跃天数分布，不按用户或日期循环查询。一次只读事务保持统计快照一致，另读取既有采集开始时间。

模块测试仅用独立内存 SQLite，覆盖空库、空天补零、中国午夜与区间末日、当前秒、成功业务白名单、用户/日去重、风险/失败/匿名/其他人预绑定排除、历史期间未活跃分母、回访用户、活跃天数分布、7/30日边界和日期范围校验。运行 `cargo test user_stats::tests`。
