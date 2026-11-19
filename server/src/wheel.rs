//! 幸运转盘 (rebuilt in the 新人礼包 style).
//!
//! * Runs in rounds (期); the admin starts a new round to reset invite chances.
//! * Chances: free spins per day, video spins per day (blind box: the server locks the result,
//!   the rewarded-video pipeline pays it only after the ad network's verified callback, then the
//!   page spins to it) and invite spins (per invited friend who did a real first action).
//! * The landing slot is decided here: weighted among slots whose coins do not exceed the cap of
//!   the player's DeviceTrust risk tier, so black devices can only ever land on the smallest prize.
//!   Ledger source `wheel:{chance_id}` is unique.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Slot {
    pub label: String,
    pub coins: i64,
    pub weight: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct CapTier {
    pub max_score: i64,
    pub max_coins: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Config {
    pub enabled: bool,
    pub title: String,
    pub entry_label: String,
    pub round: i64,
    pub round_started_at: i64,
    pub free_per_day: i64,
    pub video_per_day: i64,
    pub invite_per_invitee: i64,
    pub slots: Vec<Slot>,
    pub caps: Vec<CapTier>,
    pub unscored_score: i64,
    #[serde(default)]
    pub revision: i64,
}

fn slot(label: &str, coins: i64, weight: i64) -> Slot {
    Slot { label: label.into(), coins, weight }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            title: "幸运转盘".into(),
            entry_label: "幸运转盘".into(),
            round: 1,
            round_started_at: now(),
            free_per_day: 1,
            video_per_day: 3,
            invite_per_invitee: 1,
            slots: vec![
                slot("10金币", 10, 30),
                slot("5000金币", 5000, 1),
                slot("50金币", 50, 25),
                slot("1000金币", 1000, 4),
                slot("100金币", 100, 18),
                slot("2000金币", 2000, 2),
                slot("200金币", 200, 12),
                slot("500金币", 500, 8),
            ],
            caps: vec![
                CapTier { max_score: 10, max_coins: 5000 },
                CapTier { max_score: 30, max_coins: 2000 },
                CapTier { max_score: 60, max_coins: 200 },
                CapTier { max_score: 100, max_coins: 10 },
            ],
            unscored_score: 60,
            revision: 0,
        }
    }
}

fn validate(c: &Config) -> Result<(), ApiError> {
    for (name, text, max) in [("活动名称", &c.title, 12), ("入口文字", &c.entry_label, 6)] {
        let n = text.trim().chars().count();
        if n == 0 || n > max {
            return Err(invalid(&format!("{name}需为 1–{max} 个字")));
        }
    }
    if !(6..=12).contains(&c.slots.len()) {
        return Err(invalid("转盘需要 6–12 个奖项"));
    }
    for s in &c.slots {
        let n = s.label.trim().chars().count();
        if n == 0 || n > 8 {
            return Err(invalid("奖项名称需为 1–8 个字"));
        }
        if !(1..=100_000).contains(&s.coins) || !(0..=10_000).contains(&s.weight) {
            return Err(invalid("奖项金币需为 1–100000，权重 0–10000"));
        }
    }
    if c.slots.iter().all(|s| s.weight == 0) {
        return Err(invalid("至少一个奖项的权重大于 0"));
    }
    let mut last = -1;
    for t in &c.caps {
        if t.max_score <= last || !(0..=100).contains(&t.max_score) || t.max_coins < 1 {
            return Err(invalid("风险档位的分数上限需在 0–100 之间递增，金币上限至少 1"));
        }
        last = t.max_score;
    }
    if last != 100 {
        return Err(invalid("最后一个风险档位的分数上限必须是 100"));
    }
    for (name, v, max) in [("每日免费次数", c.free_per_day, 10), ("每日看视频次数", c.video_per_day, 20), ("每邀请 1 人得次数", c.invite_per_invitee, 10)] {
        if !(0..=max).contains(&v) {
            return Err(invalid(&format!("{name}需在 0–{max} 之间")));
        }
    }
    if !(0..=100).contains(&c.unscored_score) {
        return Err(invalid("无评分时按的分数需在 0–100 之间"));
    }
    Ok(())
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS wheel_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS wheel_chances(id TEXT PRIMARY KEY,round INTEGER NOT NULL,user_id INTEGER NOT NULL,source TEXT NOT NULL,source_ref TEXT NOT NULL,
           status TEXT NOT NULL DEFAULT 'ready',slot INTEGER,coins INTEGER,score INTEGER,session_id TEXT,created_at INTEGER NOT NULL,done_at INTEGER,
           UNIQUE(round,user_id,source,source_ref));
         CREATE INDEX IF NOT EXISTS idx_wheel_user ON wheel_chances(user_id,round,status);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO wheel_config VALUES(1,?1,?2)",
        params![serde_json::to_string(&Config::default())?, now()],
    )?;
    Ok(())
}

fn config(db: &Connection) -> Result<Config, ApiError> {
    Ok(serde_json::from_str(&db.query_row::<String, _, _>(
        "SELECT config_json FROM wheel_config WHERE id=1",
        [],
        |r| r.get(0),
    )?)?)
}

fn score(db: &Connection, uid: i64, c: &Config) -> Result<i64, ApiError> {
    if risk::check_account(db, uid).is_err() {
        return Ok(100);
    }
    let s: Option<i64> = db
        .query_row(
            "SELECT score FROM risk_events WHERE user_id=?1 AND score IS NOT NULL ORDER BY created_at DESC LIMIT 1",
            [uid],
            |r| r.get(0),
        )
        .optional()?;
    Ok(s.map(|v| v.clamp(0, 100)).unwrap_or(c.unscored_score))
}

/// Weighted pick among the slots allowed by the player's risk cap.
fn pick(c: &Config, score: i64) -> usize {
    let cap = c
        .caps
        .iter()
        .find(|t| score <= t.max_score)
        .map(|t| t.max_coins)
        .unwrap_or(0);
    let allowed: Vec<usize> = (0..c.slots.len())
        .filter(|&i| c.slots[i].coins <= cap && c.slots[i].weight > 0)
        .collect();
    if allowed.is_empty() {
        // Nothing fits the cap: the smallest prize.
        return (0..c.slots.len()).min_by_key(|&i| c.slots[i].coins).unwrap_or(0);
    }
    let total: i64 = allowed.iter().map(|&i| c.slots[i].weight).sum();
    let mut roll = (Uuid::new_v4().as_u128() % total as u128) as i64;
    for &i in &allowed {
        roll -= c.slots[i].weight;
        if roll < 0 {
            return i;
        }
    }
    allowed[0]
}

fn beijing_day() -> String {
    chrono::DateTime::from_timestamp(now() + 8 * 3600, 0)
        .unwrap()
        .format("%Y-%m-%d")
        .to_string()
}

/// Creates today's free and video chances and the invite chances earned so far.
fn sync(tx: &Connection, uid: i64, c: &Config) -> Result<(), ApiError> {
    let d = beijing_day();
    for (source, n) in [("free", c.free_per_day), ("video", c.video_per_day)] {
        for i in 0..n {
            tx.execute(
                "INSERT OR IGNORE INTO wheel_chances(id,round,user_id,source,source_ref,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![Uuid::new_v4().to_string(), c.round, uid, source, format!("{d}:{i}"), now()],
            )?;
        }
    }
    if c.invite_per_invitee > 0 {
        // Friends who joined through my code during this round and then did something real.
        let friends: Vec<i64> = tx
            .prepare(
                "SELECT u.id FROM users u WHERE u.inviter_id=?1 AND u.invitation_bound_at>=?2 AND (
                   EXISTS(SELECT 1 FROM ledger l WHERE l.user_id=u.id AND l.created_at>=u.invitation_bound_at AND l.title IN ('每日签到','连续签到补签'))
                   OR EXISTS(SELECT 1 FROM reward_sessions s WHERE s.user_id=u.id AND s.completed_at>=u.invitation_bound_at))",
            )?
            .query_map(params![uid, c.round_started_at], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for f in friends {
            for i in 0..c.invite_per_invitee {
                tx.execute(
                    "INSERT OR IGNORE INTO wheel_chances(id,round,user_id,source,source_ref,created_at) VALUES(?1,?2,?3,'invite',?4,?5)",
                    params![Uuid::new_v4().to_string(), c.round, uid, format!("{f}:{i}"), now()],
                )?;
            }
        }
    }
    Ok(())
}

/// Chances usable now: today's free/video ones and all unused invite ones of this round.
fn usable_filter() -> &'static str {
    "round=?1 AND user_id=?2 AND status IN ('ready','locked','ad_started') AND (source='invite' OR source_ref LIKE ?3)"
}

fn state_json(state: &AppState, db: &Connection, uid: i64, c: &Config) -> Result<Value, ApiError> {
    let today = format!("{}:%", beijing_day());
    let count = |source: &str| -> Result<i64, ApiError> {
        Ok(db.query_row(
            &format!("SELECT COUNT(*) FROM wheel_chances WHERE {} AND source=?4", usable_filter()),
            params![c.round, uid, today, source],
            |r| r.get(0),
        )?)
    };
    let (free, video, invite) = (count("free")?, count("video")?, count("invite")?);
    // Next chance: free first, then invite, then video (video asks for a rewarded ad).
    let next: Option<(String, String, String)> = db
        .query_row(
            &format!(
                "SELECT id,source,status FROM wheel_chances WHERE {} ORDER BY CASE source WHEN 'free' THEN 0 WHEN 'invite' THEN 1 ELSE 2 END, CASE status WHEN 'ready' THEN 1 ELSE 0 END, created_at LIMIT 1",
                usable_filter()
            ),
            params![c.round, uid, today],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (won, spins): (i64, i64) = db.query_row(
        "SELECT COALESCE(SUM(coins),0),COUNT(*) FROM wheel_chances WHERE round=?1 AND user_id=?2 AND status='done'",
        params![c.round, uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let friends: i64 = db.query_row(
        "SELECT COUNT(*) FROM users WHERE inviter_id=?1 AND invitation_bound_at>=?2",
        params![uid, c.round_started_at],
        |r| r.get(0),
    )?;
    let code: String = db.query_row("SELECT invite_code FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    Ok(json!({
        "enabled": c.enabled,
        "round": c.round,
        "title": c.title,
        "entry_label": c.entry_label,
        "slots": c.slots.iter().map(|s| json!({"label":s.label,"coins":s.coins})).collect::<Vec<_>>(),
        "max_coins": c.slots.iter().map(|s| s.coins).max().unwrap_or(0),
        "free_left": free,
        "video_left": video,
        "invite_left": invite,
        "available": free + video + invite,
        "free_per_day": c.free_per_day,
        "video_per_day": c.video_per_day,
        "invite_per_invitee": c.invite_per_invitee,
        "next": next.map(|(id, source, status)| json!({"id":id,"source":source,"status":status})),
        "won_coins": won,
        "spins": spins,
        "friends": friends,
        "invite_code": code,
        "invite_url": invitations::share_url(state, &code),
    }))
}

async fn runtime(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let c = config(&db)?;
    if !c.enabled {
        return Ok(Json(json!({"enabled":false})));
    }
    let tx = db.transaction()?;
    sync(&tx, uid, &c)?;
    let result = state_json(&state, &tx, uid, &c)?;
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct SpinInput {
    chance_id: String,
}

async fn spin(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<SpinInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    invitations::require_bound(&db, uid)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("活动已结束，敬请期待下一期"));
    }
    sync(&tx, uid, &c)?;
    let today = format!("{}:%", beijing_day());
    let row: Option<(String, String, Option<i64>)> = tx
        .query_row(
            &format!("SELECT source,status,slot FROM wheel_chances WHERE id=?4 AND {}", usable_filter()),
            params![c.round, uid, today, input.chance_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((source, status, locked)) = row else {
        return Err(conflict("今天的次数已用完，邀请好友可再得次数"));
    };
    let score = score(&tx, uid, &c)?;
    let index = locked.map(|v| v as usize).unwrap_or_else(|| pick(&c, score));
    let coins = c.slots.get(index).map(|s| s.coins).unwrap_or(c.slots[0].coins);
    if source == "video" {
        // Blind box: lock the result; it is paid through the rewarded-video pipeline (task
        // wheel:{id}) and shown only after that, when the page spins to it.
        if status == "ready" {
            tx.execute(
                "UPDATE wheel_chances SET status='locked',slot=?1,coins=?2,score=?3 WHERE id=?4 AND status='ready'",
                params![index as i64, coins, score, input.chance_id],
            )?;
        }
        let result = json!({"needs_video":true,"task_id":format!("wheel:{}",input.chance_id)});
        tx.commit()?;
        return Ok(Json(result));
    }
    let changed = tx.execute(
        "UPDATE wheel_chances SET status='done',slot=?1,coins=?2,score=?3,done_at=?4 WHERE id=?5 AND status='ready'",
        params![index as i64, coins, score, now(), input.chance_id],
    )?;
    if changed == 0 {
        return Err(conflict("正在转动，请稍候"));
    }
    credit(&tx, uid, coins, &c.title, &format!("wheel:{}", input.chance_id))?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "wheel_spin",
        "success",
        &input.chance_id,
        Some(coins as f64 / COINS_PER_YUAN as f64),
        json!({"round":c.round,"source":source,"slot":index,"score":score}),
    )?;
    let result = json!({"slot":index,"coins":coins,"user":user(&tx,uid)?,"state":state_json(&state,&tx,uid,&c)?});
    tx.commit()?;
    Ok(Json(result))
}

/// After a paid video spin: where the wheel stops (only once it is paid).
async fn result(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let row: Option<(String, Option<i64>, Option<i64>)> = db
        .query_row(
            "SELECT status,slot,coins FROM wheel_chances WHERE id=?1 AND user_id=?2",
            params![id, uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        Some((status, Some(slot), Some(coins))) if status == "done" => Ok(Json(json!({"slot":slot,"coins":coins}))),
        Some(_) => Err(conflict("奖励确认中，请稍后再看")),
        None => Err(ApiError(StatusCode::NOT_FOUND, "记录不存在".into(), None)),
    }
}

// ---------- rewarded-video pipeline hooks (task "wheel:{chance_id}") ----------
pub(super) fn video_task(state: &AppState, db: &Connection, uid: i64, id: &str) -> Result<rain::GrandTask, ApiError> {
    if state.config.ad_mode != "csj" {
        return Err(conflict("请在已接入广告的 APP 中领取"));
    }
    let row: Option<(i64, String, Option<i64>, Option<String>)> = db
        .query_row(
            "SELECT round,status,coins,session_id FROM wheel_chances WHERE id=?1 AND user_id=?2 AND source='video'",
            params![id, uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((round, status, coins, session)) = row else {
        return Err(ApiError(StatusCode::NOT_FOUND, "记录不存在".into(), None));
    };
    let c = config(db)?;
    if session.is_none() && (!c.enabled || round != c.round) {
        return Err(conflict("本期活动已结束"));
    }
    if !["locked", "ad_started"].contains(&status.as_str()) {
        return Err(conflict("请重新打开转盘"));
    }
    Ok(rain::GrandTask { packet_id: id.into(), coins: coins.unwrap_or(0), session_id: session })
}

pub(super) fn attach_session(db: &Connection, uid: i64, id: &str, sid: &str) -> Result<bool, ApiError> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM wheel_chances WHERE id=?1 AND user_id=?2 AND source='video')",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(false);
    }
    let changed = db.execute(
        "UPDATE wheel_chances SET status='ad_started',session_id=?1 WHERE id=?2 AND user_id=?3 AND status='locked' AND session_id IS NULL",
        params![sid, id, uid],
    )?;
    if changed != 1 {
        return Err(conflict("该次数已处理，请刷新后重试"));
    }
    Ok(true)
}

pub(super) fn completion_packet(db: &Connection, uid: i64, id: &str, sid: &str) -> Result<String, ApiError> {
    let valid: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM wheel_chances WHERE id=?1 AND user_id=?2 AND source='video' AND status='ad_started' AND session_id=?3)",
        params![id, uid, sid],
        |r| r.get(0),
    )?;
    if !valid {
        return Err(conflict("奖励状态不匹配"));
    }
    Ok(id.into())
}

pub(super) fn finish_video(db: &Connection, uid: i64, id: &str, sid: &str, coins: i64) -> Result<(), ApiError> {
    let changed = db.execute(
        "UPDATE wheel_chances SET status='done',done_at=?1 WHERE id=?2 AND user_id=?3 AND session_id=?4 AND coins=?5 AND status='ad_started'",
        params![now(), id, uid, sid, coins],
    )?;
    if changed != 1 {
        return Err(conflict("奖励状态不匹配"));
    }
    Ok(())
}

pub(super) fn title(db: &Connection) -> String {
    config(db).map(|c| c.title).unwrap_or_else(|_| "幸运转盘".into())
}

// ---------- winners board (cached 5 minutes per round) ----------
static BOARD: std::sync::Mutex<Option<(i64, i64, Value)>> = std::sync::Mutex::new(None);

async fn board(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let c = config(&db)?;
    if !c.enabled {
        return Ok(Json(json!({"items":[]})));
    }
    if let Some((round, at, value)) = BOARD.lock().unwrap().as_ref() {
        if *round == c.round && now() - at < 300 {
            return Ok(Json(value.clone()));
        }
    }
    let items: Vec<Value> = db
        .prepare(
            "SELECT u.phone,SUM(w.coins) total FROM wheel_chances w JOIN users u ON u.id=w.user_id
             WHERE w.round=?1 AND w.status='done' GROUP BY w.user_id ORDER BY total DESC LIMIT 20",
        )?
        .query_map([c.round], |r| Ok(json!({"name":newbie::mask_phone(&r.get::<_,String>(0)?),"coins":r.get::<_,i64>(1)?})))?
        .collect::<Result<_, _>>()?;
    let value = json!({"items":items});
    *BOARD.lock().unwrap() = Some((c.round, now(), value.clone()));
    Ok(Json(value))
}

/// Hall badge: whether the wheel runs and how many spins are left today.
pub(super) fn badge(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let c = config(db)?;
    if !c.enabled {
        return Ok(json!({"enabled":false}));
    }
    sync(db, uid, &c)?;
    let today = format!("{}:%", beijing_day());
    let available: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM wheel_chances WHERE {}", usable_filter()),
        params![c.round, uid, today],
        |r| r.get(0),
    )?;
    Ok(json!({"enabled":true,"entry_label":c.entry_label,"available":available}))
}

async fn badge_route(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    Ok(Json(badge(&db, uid)?))
}

async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let c = config(&db)?;
    let one = |sql: &str| -> Result<i64, ApiError> { Ok(db.query_row(sql, [c.round], |r| r.get(0))?) };
    let stats = json!({
        "players": one("SELECT COUNT(DISTINCT user_id) FROM wheel_chances WHERE round=?1 AND status='done'")?,
        "spins": one("SELECT COUNT(*) FROM wheel_chances WHERE round=?1 AND status='done'")?,
        "video_spins": one("SELECT COUNT(*) FROM wheel_chances WHERE round=?1 AND status='done' AND source='video'")?,
        "invite_spins": one("SELECT COUNT(*) FROM wheel_chances WHERE round=?1 AND status='done' AND source='invite'")?,
        "coins": one("SELECT COALESCE(SUM(coins),0) FROM wheel_chances WHERE round=?1 AND status='done'")?,
    });
    let recent: Vec<Value> = db
        .prepare(
            "SELECT w.done_at,w.source,w.coins,w.score,u.id,u.username,u.phone FROM wheel_chances w JOIN users u ON u.id=w.user_id
             WHERE w.round=?1 AND w.status='done' ORDER BY w.done_at DESC LIMIT 50",
        )?
        .query_map([c.round], |r| {
            Ok(json!({"done_at":iso(r.get(0)?),"source":r.get::<_,String>(1)?,"coins":r.get::<_,i64>(2)?,"score":r.get::<_,Option<i64>>(3)?,
                "user_id":r.get::<_,i64>(4)?,"username":r.get::<_,Option<String>>(5)?,"phone":r.get::<_,String>(6)?}))
        })?
        .collect::<Result<_, _>>()?;
    let mut value = serde_json::to_value(&c)?;
    value["round_started_at"] = json!(iso(c.round_started_at));
    Ok(Json(json!({"config":value,"stats":stats,"recent":recent})))
}

#[derive(Deserialize)]
struct AdminInput {
    #[serde(flatten)]
    config: Value,
    #[serde(default)]
    new_round: bool,
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<AdminInput>,
) -> ApiResult<Value> {
    {
        let mut db = open(&state)?;
        let tx = db.transaction()?;
        let old = config(&tx)?;
        let mut body = input.config;
        body["round"] = json!(old.round);
        body["round_started_at"] = json!(old.round_started_at);
        body["revision"] = json!(old.revision);
        let mut next: Config = serde_json::from_value(body).map_err(|e| invalid(&format!("配置格式错误：{e}")))?;
        validate(&next)?;
        if input.new_round {
            next.round = old.round + 1;
            next.round_started_at = now();
        }
        next.revision = old.revision + 1;
        tx.execute(
            "UPDATE wheel_config SET config_json=?1,updated_at=?2 WHERE id=1",
            params![serde_json::to_string(&next)?, now()],
        )?;
        user_admin::action_audit(
            &tx,
            &context,
            None,
            if input.new_round { "wheel_new_round" } else { "wheel_config_update" },
            "wheel",
            if input.new_round { "开启幸运转盘新一期" } else { "修改幸运转盘配置" },
            serde_json::to_value(&old)?,
            serde_json::to_value(&next)?,
        )?;
        tx.commit()?;
    }
    admin_get(State(state)).await
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/wheel", get(runtime))
        .route("/wheel/spin", post(spin))
        .route("/wheel/result/{id}", get(result))
        .route("/wheel/board", get(board))
        .route("/wheel/badge", get(badge_route))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/wheel", get(admin_get).put(admin_put))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::collections::HashMap;

    /// Free spin pays at once; a video spin locks its result without revealing it, and pays only
    /// after the verified ad callback — once — then /wheel/result reveals where it stopped.
    #[tokio::test]
    async fn video_spin_pays_only_after_verified_callback() {
        let (state, context, mut headers, _) = rain::tests::fixture(true, true);
        let db = open(&state).unwrap();
        let mut c = config(&db).unwrap();
        c.enabled = true;
        db.execute("UPDATE wheel_config SET config_json=?1", [serde_json::to_string(&c).unwrap()]).unwrap();
        let st = runtime(State(state.clone()), headers.clone()).await.unwrap().0;
        assert_eq!(st["free_left"], 1);
        assert_eq!(st["video_left"], 3);
        assert_eq!(st["next"]["source"], "free");
        let free = st["next"]["id"].as_str().unwrap().to_string();
        let won = spin(State(state.clone()), Extension(context.clone()), headers.clone(), Json(SpinInput { chance_id: free.clone() })).await.unwrap().0;
        let coins = won["coins"].as_i64().unwrap();
        assert_eq!(user(&db, 2).unwrap().coins, coins);
        assert!(spin(State(state.clone()), Extension(context.clone()), headers.clone(), Json(SpinInput { chance_id: free })).await.is_err());
        let st = won["state"].clone();
        assert_eq!(st["next"]["source"], "video");
        let vid = st["next"]["id"].as_str().unwrap().to_string();
        let locked = spin(State(state.clone()), Extension(context.clone()), headers.clone(), Json(SpinInput { chance_id: vid.clone() })).await.unwrap().0;
        assert_eq!(locked["needs_video"], true);
        assert!(locked.get("coins").is_none() && locked.get("slot").is_none());
        assert!(result(State(state.clone()), headers.clone(), Path(vid.clone())).await.is_err());
        let prize: i64 = db.query_row("SELECT coins FROM wheel_chances WHERE id=?1", [&vid], |r| r.get(0)).unwrap();
        let task = format!("wheel:{vid}");
        let quote = reward_policy::test_quote(&state, &headers, &task, None).await;
        assert_eq!(quote["reward"], prize);
        let qid = quote["quote_id"].as_str().unwrap().to_owned();
        business_guard::test_event(&state, "wheel-proof", 2, "reward", json!({"task_id":task,"quote_id":qid}));
        headers.insert("x-risk-event", HeaderValue::from_static("wheel-proof"));
        let session = super::super::reward_start(State(state.clone()), Extension(context.clone()), headers.clone(), Json(super::super::RewardInput { task_id: task, quote_id: qid })).await.unwrap().0;
        let sid = session["id"].as_str().unwrap().to_string();
        assert!(super::super::reward_complete(State(state.clone()), Extension(context.clone()), headers.clone(), Path(sid.clone())).await.is_err());
        let trans = "wheel-callback-transaction";
        let mut query = HashMap::from([
            ("user_id".to_string(), "2".to_string()),
            ("trans_id".to_string(), trans.to_string()),
            ("reward_name".to_string(), "金币".to_string()),
            ("reward_amount".to_string(), "1".to_string()),
            ("extra".to_string(), session["media_extra"].as_str().unwrap().to_string()),
            ("sign".to_string(), format!("{:x}", Sha256::digest(format!("rain-test-secret:{trans}").as_bytes()))),
        ]);
        assert!(super::super::verify_csj_callback(&state, &query).unwrap().valid);
        query.insert("sign".into(), "forged".into());
        assert!(!super::super::verify_csj_callback(&state, &query).unwrap().valid);
        let paid = super::super::reward_complete(State(state.clone()), Extension(context.clone()), headers.clone(), Path(sid.clone())).await.unwrap().0;
        assert_eq!(paid["reward"], prize);
        assert_eq!(user(&db, 2).unwrap().coins, coins + prize);
        let again = super::super::reward_complete(State(state.clone()), Extension(context), headers.clone(), Path(sid)).await.unwrap().0;
        assert_eq!(again["already_completed"], true);
        let shown = result(State(state.clone()), headers.clone(), Path(vid.clone())).await.unwrap().0;
        assert_eq!(shown["coins"], prize);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM ledger WHERE source=?1", [format!("wheel:{vid}")], |r| r.get::<_, i64>(0)).unwrap(), 1);
        let st = runtime(State(state), headers).await.unwrap().0;
        assert_eq!(st["free_left"], 0);
        assert_eq!(st["video_left"], 2);
    }

    #[test]
    fn risk_cap_limits_the_landing_slot() {
        let c = Config::default();
        for _ in 0..300 {
            assert_eq!(c.slots[pick(&c, 100)].coins, 10);
            assert!(c.slots[pick(&c, 50)].coins <= 200);
            assert!(c.slots[pick(&c, 0)].coins <= 5000);
        }
        assert!(validate(&c).is_ok());
    }

    #[test]
    fn expired_video_session_releases_the_chance_for_a_new_one() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE wheel_chances(id TEXT PRIMARY KEY,user_id INTEGER,source TEXT,status TEXT,session_id TEXT);
            INSERT INTO wheel_chances VALUES('c1',7,'video','ad_started','old');").unwrap();
        assert!(rain::release_session(&db, 7, "wheel:c1", "other").is_err());
        rain::release_session(&db, 7, "wheel:c1", "old").unwrap();
        assert!(attach_session(&db, 7, "c1", "new").unwrap());
        let sid: String = db.query_row("SELECT session_id FROM wheel_chances WHERE id='c1'", [], |r| r.get(0)).unwrap();
        assert_eq!(sid, "new");
    }
}
