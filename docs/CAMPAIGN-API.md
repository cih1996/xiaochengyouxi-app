# 营销活动与幸运转盘

活动机会来自服务端记录的每日登录、已完成激励视频和真实邀请关系。客户端不能提交机会数量或奖励金额；每次转盘由服务端随机并持久化实例。奖励先是 `pending`，同时满足技术等待时间 `settle_delay_seconds`（默认 60 秒）和金币门槛 `arrival_threshold_coins`（默认 0）后才能结算入账。

## 用户接口

| 接口 | 说明 |
| --- | --- |
| `GET /api/campaign` | 已绑定账号；返回配置、今日机会、待结算奖励和 `can_spin` |
| `POST /api/campaign/spin` | `{opportunity_id}`；机会只可使用一次，返回 pending 转盘实例 |
| `POST /api/campaign/rewards/{id}/settle` | 达到门槛后结算；重复调用返回已到账状态 |

`GET /api/campaign` 响应包括 `enabled,title,copy,entry_label,hero_image,starts_at,expires_at,arrival_threshold,reward_tiers,opportunities,pending_rewards,revision,can_spin,spin_path`。机会项含 `{id,source,source_ref,created_at,expires_at,used}`；source 为 `login`、`invite` 或 `video`。待结算奖励含 `{id,prize_coins,created_at,status,settled_at?}`。当前不按固定每日转盘上限拦截，剩余次数等于服务端生成的未使用有效机会数。

抽奖响应：

```json
{
  "id": "…",
  "status": "pending",
  "prize_coins": 20,
  "settle_after": "2026-11-13T04:01:00Z",
  "message": "奖励待结算，请稍后查看"
}
```

结算成功后状态为 `settled`，并通过唯一 `ledger.source=campaign:{id}` 增加金币。配置关闭、机会重复、每日次数用尽、机会过期和结算门槛未到分别返回明确业务错误；这些错误不会写入余额。

## 后台配置

后台会话接口：

```text
GET /api/admin/campaign
PUT /api/admin/campaign
```

配置字段：`enabled,title,copy/subtitle,entry_label,hero_image,starts_at,expires_at,arrival_threshold/arrival_threshold_coins,settle_delay_seconds,login_opportunities,video_opportunities_per_day,invite_opportunities_per_invitee,reward_tiers/prizes,revision`。奖励档位为金币整数数组，服务端限制 0–100000；`arrival_threshold_coins` 为当前账户达到后可领取的金币门槛，`settle_delay_seconds` 为技术等待时间。旧版 `daily_limit/daily_spin_limit` 字段仅为兼容保留，不再作为转盘拦截条件。更新会递增 revision 并写管理员审计，已有 pending 转盘保留创建时的金额，不随新配置重抽。

后台会话还提供 `GET /api/admin/campaign/stats`、`GET /api/admin/campaign/users?page=1&page_size=20` 和 `GET /api/admin/campaign/users/{id}`，用于查看参与用户、机会来源、转盘次数、待结算/已到账奖励、金币门槛和用户活动记录。

活动后台明细接口：

| 接口 | 返回 |
| --- | --- |
| `GET /api/admin/campaign/stats` | 全局用户、机会、转盘、pending/settled 数量和已结算金币 |
| `GET /api/admin/campaign/users?page=&page_size=&q=` | 用户分页，包含余额、机会数、转盘数、pending/settled 数 |
| `GET /api/admin/campaign/users/{id}` | 用户资料、机会来源和每次转盘、门槛状态 |
| `GET /api/admin/campaign/users/{id}/records` | 与用户详情相同的活动记录别名 |

管理员看到的 `settlement` 包括当前金币、`arrival_threshold_coins`、`settle_delay_seconds` 和 `threshold_met`。pending 奖励不计入用户余额，只有 settled 记录对应 ledger 才显示到账。

活动图应使用本服务登记的媒体路径，空值时客户端隐藏活动图。活动状态、机会来源、奖励门槛和到账状态均以服务端响应为准。

