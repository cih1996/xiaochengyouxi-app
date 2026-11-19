use super::*;
use rand_core::RngCore;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct RainConfig {
    pub enabled: bool,
    pub start_hour: i64,
    pub every_hours: i64,
    pub duration_seconds: i64,
    pub packet_count: i64,
    pub small_min_coins: i64,
    pub small_max_coins: i64,
    pub grand_probability: f64,
    pub grand_coins: i64,
    pub max_total_coins: i64,
    pub overlay_opacity: f64,
    pub fall_speed: f64,
    pub max_visible: i64,
}
impl Default for RainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            start_hour: 0,
            every_hours: 1,
            duration_seconds: 45,
            packet_count: 24,
            small_min_coins: 1,
            small_max_coins: 5,
            grand_probability: 0.04,
            grand_coins: 88,
            max_total_coins: 300,
            overlay_opacity: 0.64,
            fall_speed: 1.0,
            max_visible: 8,
        }
    }
}
fn validate(c: &RainConfig) -> Result<(), ApiError> {
    if !(0..=23).contains(&c.start_hour)
        || !(1..=24).contains(&c.every_hours)
        || !(10..=180).contains(&c.duration_seconds)
        || !(5..=60).contains(&c.packet_count)
        || !(1..=10).contains(&c.small_min_coins)
        || !(c.small_min_coins..=10).contains(&c.small_max_coins)
        || !c.grand_probability.is_finite()
        || !(0.0..=1.0).contains(&c.grand_probability)
        || !(1..=10000).contains(&c.grand_coins)
        || !(c.packet_count * c.small_min_coins..=600000).contains(&c.max_total_coins)
        || !c.overlay_opacity.is_finite()
        || !(0.0..=0.9).contains(&c.overlay_opacity)
        || !c.fall_speed.is_finite()
        || !(0.4..=3.0).contains(&c.fall_speed)
        || !(1..=20).contains(&c.max_visible)
    {
        return Err(invalid("红包雨参数超出允许范围，请检查配置"));
    }
    Ok(())
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS rain_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS rain_rounds(id TEXT PRIMARY KEY,start_ms INTEGER NOT NULL,end_ms INTEGER NOT NULL,source TEXT NOT NULL,config_json TEXT NOT NULL,created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_rain_round_window ON rain_rounds(start_ms,end_ms);
        CREATE TABLE IF NOT EXISTS rain_joins(round_id TEXT NOT NULL,user_id INTEGER NOT NULL,risk_event_id TEXT,last_claim_ms INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL,PRIMARY KEY(round_id,user_id));
        CREATE TABLE IF NOT EXISTS rain_packets(id TEXT PRIMARY KEY,round_id TEXT NOT NULL,user_id INTEGER NOT NULL,kind TEXT NOT NULL,coins INTEGER NOT NULL,reveal_ms INTEGER NOT NULL,expire_ms INTEGER NOT NULL,status TEXT NOT NULL DEFAULT 'available',opened_at INTEGER,session_id TEXT UNIQUE,claimed_at INTEGER);
        CREATE INDEX IF NOT EXISTS idx_rain_packets_user_round ON rain_packets(user_id,round_id,reveal_ms);")?;
    db.execute(
        "INSERT OR IGNORE INTO rain_config(id,config_json) VALUES(1,?1)",
        [serde_json::to_string(&RainConfig::default())?],
    )?;
    Ok(())
}
pub(super) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}
fn instant(value: i64) -> String {
    chrono::DateTime::from_timestamp_millis(value)
        .unwrap()
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn config(db: &Connection) -> Result<RainConfig, ApiError> {
    let raw: String = db.query_row("SELECT config_json FROM rain_config WHERE id=1", [], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&raw)?)
}
#[derive(Clone)]
struct Round {
    id: String,
    start: i64,
    end: i64,
    source: String,
    config: RainConfig,
}
impl Round {
    fn json(&self) -> Value {
        json!({"id":self.id,"start_at":instant(self.start),"end_at":instant(self.end),"source":self.source})
    }
}
fn round_row(row: &rusqlite::Row) -> Result<(String, i64, i64, String, String), rusqlite::Error> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}
fn round_from(row: (String, i64, i64, String, String)) -> Result<Round, ApiError> {
    Ok(Round {
        id: row.0,
        start: row.1,
        end: row.2,
        source: row.3,
        config: serde_json::from_str(&row.4)?,
    })
}
fn choose_round(db: &Connection, c: &RainConfig, at: i64) -> Result<Round, ApiError> {
    let active=db.query_row("SELECT id,start_ms,end_ms,source,config_json FROM rain_rounds WHERE start_ms<=?1 AND end_ms>?1 ORDER BY start_ms DESC LIMIT 1",[at],round_row).optional()?;
    if let Some(row) = active {
        return round_from(row);
    }
    let manual=db.query_row("SELECT id,start_ms,end_ms,source,config_json FROM rain_rounds WHERE source='manual' AND start_ms>?1 ORDER BY start_ms LIMIT 1",[at],round_row).optional()?.map(round_from).transpose()?;
    let interval = c.every_hours * 3600000;
    let anchor = (c.start_hour - 8) * 3600000;
    let mut start = (at - anchor).div_euclid(interval) * interval + anchor;
    if at >= start + c.duration_seconds * 1000 {
        start += interval;
    }
    let overlaps_manual:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM rain_rounds WHERE source='manual' AND start_ms<?1 AND end_ms>?2)",params![start+c.duration_seconds*1000,start],|r|r.get(0))?;
    if overlaps_manual {
        start += interval;
    }
    if let Some(manual) = &manual {
        if manual.start < start {
            return Ok(manual.clone());
        }
    }
    let scheduled = Round {
        id: format!("hourly:{start}"),
        start,
        end: start + c.duration_seconds * 1000,
        source: "scheduled".into(),
        config: c.clone(),
    };
    if start <= at {
        db.execute("INSERT OR IGNORE INTO rain_rounds(id,start_ms,end_ms,source,config_json,created_at) VALUES(?1,?2,?3,'scheduled',?4,?5)",params![scheduled.id,scheduled.start,scheduled.end,serde_json::to_string(c)?,now()])?;
        let stored = db.query_row(
            "SELECT id,start_ms,end_ms,source,config_json FROM rain_rounds WHERE id=?1",
            [&scheduled.id],
            round_row,
        )?;
        let stored = round_from(stored)?;
        if stored.end <= at {
            let start = start + interval;
            if let Some(manual) = manual {
                if manual.start < start {
                    return Ok(manual);
                }
            }
            return Ok(Round {
                id: format!("hourly:{start}"),
                start,
                end: start + c.duration_seconds * 1000,
                source: "scheduled".into(),
                config: c.clone(),
            });
        }
        return Ok(stored);
    }
    Ok(scheduled)
}
fn status_json(db: &Connection, uid: Option<i64>, at: i64) -> Result<Value, ApiError> {
    let c = config(db)?;
    if !c.enabled {
        return Ok(
            json!({"enabled":false,"server_now":instant(at),"phase":"disabled","next_start_at":Value::Null,"round":Value::Null,"joined":false,"config":c}),
        );
    }
    let round = choose_round(db, &c, at)?;
    let active = round.start <= at && at < round.end;
    let joined = if let Some(uid) = uid {
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM rain_joins WHERE round_id=?1 AND user_id=?2)",
            params![round.id, uid],
            |r| r.get::<_, bool>(0),
        )?
    } else {
        false
    };
    let next = if active {
        choose_round(db, &c, round.end)?.start
    } else {
        round.start
    };
    Ok(
        json!({"enabled":true,"server_now":instant(at),"phase":if active{"active"}else{"waiting"},"next_start_at":instant(next),"round":round.json(),"joined":joined,"config":if active{round.config}else{c}}),
    )
}
async fn status(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = if headers.contains_key("authorization") {
        Some(auth(&db, &headers)?)
    } else {
        None
    };
    Ok(Json(status_json(&db, uid, now_ms())?))
}
fn join_json(db: &Connection, uid: i64, round: &Round, at: i64) -> Result<Value, ApiError> {
    let mut stmt=db.prepare("SELECT id,kind,reveal_ms,expire_ms,status FROM rain_packets WHERE round_id=?1 AND user_id=?2 ORDER BY reveal_ms,id")?;
    let packets=stmt.query_map(params![round.id,uid],|r|Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"reveal_at":instant(r.get(2)?),"expires_at":instant(r.get(3)?),"status":r.get::<_,String>(4)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(
        json!({"operation_id":round.id,"server_now":instant(at),"round":round.json(),"config":round.config,"packets":packets}),
    )
}
async fn join(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<JoinInput>,
) -> ApiResult<Value> {
    let (uid, existing) = {
        let db = open(&state)?;
        let uid = auth(&db, &headers)?;
        risk::check_account(&db, uid)?;
        let c = config(&db)?;
        if !c.enabled {
            return Err(conflict("红包雨暂未开启"));
        }
        let round = choose_round(&db, &c, now_ms())?;
        if round.id != input.round_id {
            return Err(conflict("场次已更新，请重新进入"));
        }
        let existing: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM rain_joins WHERE round_id=?1 AND user_id=?2)",
            params![round.id, uid],
            |r| r.get(0),
        )?;
        (uid, existing)
    };
    if !existing {
        business_guard::ensure_permit(
            &state,
            &headers,
            uid,
            "rain",
            json!({"round_id":input.round_id}),
        )
        .await?;
    }
    join_at(&state, &context, &headers, now_ms())
}
#[derive(Deserialize)]
struct JoinInput {
    round_id: String,
}
pub(super) fn binding_summary(db: &Connection, body: &Value) -> Result<Value, ApiError> {
    let c = config(db)?;
    if !c.enabled {
        return Err(conflict("红包雨暂未开启"));
    }
    let round = choose_round(db, &c, now_ms())?;
    if body["round_id"].as_str() != Some(round.id.as_str()) {
        return Err(conflict("场次已更新，请重新进入"));
    }
    Ok(
        json!({"round_id":round.id,"start_at":instant(round.start),"end_at":instant(round.end),"config":round.config}),
    )
}
fn join_at(
    state: &AppState,
    context: &RequestContext,
    headers: &HeaderMap,
    at: i64,
) -> ApiResult<Value> {
    let mut db = open(state)?;
    let uid = auth(&db, headers)?;
    let tx = db.transaction()?;
    risk::check_account(&tx, uid)?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("红包雨暂未开启"));
    }
    let round = choose_round(&tx, &c, at)?;
    if at < round.start || at >= round.end {
        return Err(conflict("本场红包雨尚未开始"));
    }
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM rain_joins WHERE round_id=?1 AND user_id=?2)",
        params![round.id, uid],
        |r| r.get(0),
    )?;
    if exists {
        return Ok(Json(join_json(&tx, uid, &round, at)?));
    }
    let event = business_guard::validate_local(
        state,
        &tx,
        headers,
        uid,
        "rain",
        json!({"round_id":round.id}),
    )?;
    tx.execute(
        "INSERT INTO rain_joins(round_id,user_id,risk_event_id,created_at) VALUES(?1,?2,?3,?4)",
        params![round.id, uid, event, now()],
    )?;
    risk::consume(&tx, &event, &format!("rain:{}:{uid}", round.id))?;
    let rc = &round.config;
    let interval = rc.duration_seconds * 1000 / rc.packet_count;
    let lifetime = ((10000.0 / rc.fall_speed) as i64)
        .min(interval * rc.max_visible)
        .max(1);
    let mut budget = rc.max_total_coins;
    for index in 0..rc.packet_count {
        let reserve = (rc.packet_count - index - 1) * rc.small_min_coins;
        let large = (OsRng.next_u64() as f64 / (u64::MAX as f64 + 1.0)) < rc.grand_probability
            && budget - rc.grand_coins >= reserve;
        let coins = if large {
            rc.grand_coins
        } else {
            let max = rc.small_max_coins.min(budget - reserve);
            rc.small_min_coins + (OsRng.next_u64() % ((max - rc.small_min_coins + 1) as u64)) as i64
        };
        budget -= coins;
        let reveal = round.start + index * interval;
        let expire = (reveal + lifetime).min(round.end);
        tx.execute("INSERT INTO rain_packets(id,round_id,user_id,kind,coins,reveal_ms,expire_ms) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![Uuid::new_v4().to_string(),round.id,uid,if large{"large"}else{"small"},coins,reveal,expire])?;
    }
    user_admin::behavior(
        &tx,
        context,
        Some(uid),
        None,
        "rain_join",
        "success",
        &round.id,
        None,
        json!({"packet_count":rc.packet_count,"max_total_coins":rc.max_total_coins}),
    )?;
    let result = join_json(&tx, uid, &round, at)?;
    let reference = format!("rain:{}:{uid}", round.id);
    business_guard::record_receipt(&tx, uid, &reference)?;
    tx.commit()?;
    business_guard::dispatch(state.clone(), reference);
    Ok(Json(result))
}
#[derive(Deserialize)]
struct ClaimInput {
    packet_id: String,
}
async fn claim(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<ClaimInput>,
) -> ApiResult<Value> {
    claim_at(&state, &context, &headers, &input.packet_id, now_ms())
}
fn claim_at(
    state: &AppState,
    context: &RequestContext,
    headers: &HeaderMap,
    pid: &str,
    at: i64,
) -> ApiResult<Value> {
    let mut db = open(state)?;
    let uid = auth(&db, headers)?;
    let tx = db.transaction()?;
    risk::check_account(&tx, uid)?;
    if !config(&tx)?.enabled {
        return Err(conflict("本场红包雨已结束"));
    }
    let packet:Option<(String,String,i64,i64,i64,String,i64,i64)>=tx.query_row("SELECT p.round_id,p.kind,p.coins,p.reveal_ms,p.expire_ms,p.status,r.start_ms,r.end_ms FROM rain_packets p JOIN rain_rounds r ON r.id=p.round_id WHERE p.id=?1 AND p.user_id=?2",params![pid,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).optional()?;
    let Some((round, kind, coins, reveal, expire, status, start, end)) = packet else {
        return Err(ApiError(StatusCode::NOT_FOUND, "红包不存在".into(), None));
    };
    if at < start || at >= end || at < reveal || at >= expire {
        return Err(conflict("红包已失效或尚未出现"));
    }
    if status == "claimed" {
        if kind == "small" {
            return Ok(Json(
                json!({"kind":"small","packet_id":pid,"operation_id":format!("rain:{pid}"),"reward":coins,"user":user(&tx,uid)?,"already_claimed":true,"credited":false}),
            ));
        }
        return Err(conflict("这个红包已经领取啦"));
    }
    if kind == "large" && ["opened", "ad_started"].contains(&status.as_str()) {
        return Ok(Json(large_payload(pid, coins)));
    }
    let last: i64 = tx.query_row(
        "SELECT last_claim_ms FROM rain_joins WHERE round_id=?1 AND user_id=?2",
        params![round, uid],
        |r| r.get(0),
    )?;
    if at - last < 200 {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "点得太快啦，稍后再试".into(),
            None,
        ));
    }
    tx.execute(
        "UPDATE rain_joins SET last_claim_ms=?1 WHERE round_id=?2 AND user_id=?3",
        params![at, round, uid],
    )?;
    let result = if kind == "small" {
        credit(&tx, uid, coins, "红包雨奖励", &format!("rain:{pid}"))?;
        tx.execute(
            "UPDATE rain_packets SET status='claimed',claimed_at=?1 WHERE id=?2",
            params![at, pid],
        )?;
        json!({"kind":"small","packet_id":pid,"operation_id":format!("rain:{pid}"),"reward":coins,"user":user(&tx,uid)?,"already_claimed":false,"credited":true})
    } else {
        tx.execute(
            "UPDATE rain_packets SET status='opened',opened_at=?1 WHERE id=?2",
            params![at, pid],
        )?;
        large_payload(pid, coins)
    };
    user_admin::behavior(
        &tx,
        context,
        Some(uid),
        None,
        if kind == "small" {
            "rain_small_claim"
        } else {
            "rain_large_open"
        },
        "success",
        pid,
        if kind == "small" {
            Some(coins as f64 / 1000.0)
        } else {
            None
        },
        json!({"round_id":round}),
    )?;
    tx.commit()?;
    Ok(Json(result))
}
fn large_payload(pid: &str, coins: i64) -> Value {
    json!({"kind":"large","packet_id":pid,"reward":coins,"task":{"id":format!("rain:{pid}"),"title":"红包雨大奖","subtitle":"看视频领取这份好运","reward":coins,"kind":"rewarded_video","claimed_today":0}})
}
pub(super) struct GrandTask {
    pub packet_id: String,
    pub coins: i64,
    pub session_id: Option<String>,
}
pub(super) fn grand_task(
    state: &AppState,
    db: &Connection,
    uid: i64,
    task: &str,
) -> Result<Option<GrandTask>, ApiError> {
    if let Some(pid) = task.strip_prefix("newbie:") {
        return newbie::video_task(state, db, uid, pid).map(Some);
    }
    if let Some(id) = task.strip_prefix("wheel:") {
        return wheel::video_task(state, db, uid, id).map(Some);
    }
    let Some(pid) = task.strip_prefix("rain:") else {
        return Ok(None);
    };
    if state.config.ad_mode != "csj" {
        return Err(conflict("请在已接入广告的 APP 中领取大奖"));
    }
    let row:Option<(i64,String,String,Option<String>,i64)>=db.query_row("SELECT p.coins,p.kind,p.status,p.session_id,r.end_ms FROM rain_packets p JOIN rain_rounds r ON r.id=p.round_id WHERE p.id=?1 AND p.user_id=?2",params![pid,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    let Some((coins, kind, status, session, end)) = row else {
        return Err(ApiError(StatusCode::NOT_FOUND, "红包不存在".into(), None));
    };
    if kind != "large" || !["opened", "ad_started"].contains(&status.as_str()) {
        return Err(conflict("请先打开这份大奖红包"));
    }
    if session.is_none() && (!config(db)?.enabled || now_ms() >= end) {
        return Err(conflict("本场红包雨已结束"));
    }
    Ok(Some(GrandTask {
        packet_id: pid.into(),
        coins,
        session_id: session,
    }))
}
pub(super) fn attach_session(
    db: &Connection,
    uid: i64,
    packet: &str,
    sid: &str,
) -> Result<(), ApiError> {
    if newbie::attach_session(db, uid, packet, sid)? || wheel::attach_session(db, uid, packet, sid)? {
        return Ok(());
    }
    let changed=db.execute("UPDATE rain_packets SET status='ad_started',session_id=?1 WHERE id=?2 AND user_id=?3 AND kind='large' AND status='opened' AND session_id IS NULL",params![sid,packet,uid])?;
    if changed != 1 {
        return Err(conflict("该红包已处理，请刷新后重试"));
    }
    Ok(())
}
/// A video session that expired unverified (user quit the ad, or the network never called back)
/// must not pin its packet forever: unbind it so the next attempt starts a fresh session.
pub(super) fn release_session(db: &Connection, uid: i64, task: &str, sid: &str) -> Result<(), ApiError> {
    let changed = if let Some(id) = task.strip_prefix("newbie:") {
        db.execute("UPDATE newbie_packets SET status='revealed',session_id=NULL WHERE id=?1 AND user_id=?2 AND kind='video' AND status='ad_started' AND session_id=?3", params![id, uid, sid])?
    } else if let Some(id) = task.strip_prefix("wheel:") {
        db.execute("UPDATE wheel_chances SET status='locked',session_id=NULL WHERE id=?1 AND user_id=?2 AND source='video' AND status='ad_started' AND session_id=?3", params![id, uid, sid])?
    } else if let Some(id) = task.strip_prefix("rain:") {
        db.execute("UPDATE rain_packets SET status='opened',session_id=NULL WHERE id=?1 AND user_id=?2 AND kind='large' AND status='ad_started' AND session_id=?3", params![id, uid, sid])?
    } else {
        0
    };
    if changed != 1 {
        return Err(conflict("该红包已处理，请刷新后重试"));
    }
    Ok(())
}
pub(super) fn completion_packet(
    db: &Connection,
    uid: i64,
    task: &str,
    sid: &str,
) -> Result<Option<String>, ApiError> {
    if let Some(pid) = task.strip_prefix("newbie:") {
        return newbie::completion_packet(db, uid, pid, sid).map(Some);
    }
    if let Some(id) = task.strip_prefix("wheel:") {
        return wheel::completion_packet(db, uid, id, sid).map(Some);
    }
    let Some(pid) = task.strip_prefix("rain:") else {
        return Ok(None);
    };
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM rain_packets WHERE id=?1 AND user_id=?2 AND kind='large' AND status='ad_started' AND session_id=?3)",params![pid,uid,sid],|r|r.get(0))?;
    if !valid {
        return Err(conflict("红包奖励状态不匹配"));
    }
    Ok(Some(pid.into()))
}
pub(super) fn finish_grand(
    db: &Connection,
    uid: i64,
    pid: &str,
    sid: &str,
    coins: i64,
) -> Result<(), ApiError> {
    let changed=db.execute("UPDATE rain_packets SET status='claimed',claimed_at=?1 WHERE id=?2 AND user_id=?3 AND session_id=?4 AND coins=?5 AND status='ad_started'",params![now_ms(),pid,uid,sid,coins])?;
    if changed != 1 {
        return Err(conflict("红包奖励状态不匹配"));
    }
    Ok(())
}
async fn admin_config(State(state): State<AppState>) -> ApiResult<RainConfig> {
    Ok(Json(config(&open(&state)?)?))
}
async fn update_config(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<RainConfig>,
) -> ApiResult<RainConfig> {
    validate(&input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = config(&tx)?;
    tx.execute(
        "UPDATE rain_config SET config_json=?1 WHERE id=1",
        [serde_json::to_string(&input)?],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "rain_config",
        "rain",
        "修改红包雨配置",
        serde_json::to_value(before)?,
        serde_json::to_value(&input)?,
    )?;
    tx.commit()?;
    Ok(Json(input))
}
#[derive(Deserialize)]
struct StartInput {
    countdown_seconds: i64,
}
async fn start_round(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<StartInput>,
) -> ApiResult<Value> {
    if !(10..=300).contains(&input.countdown_seconds) {
        return Err(invalid("开场倒计时应为 10 至 300 秒"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("请先开启红包雨"));
    }
    let at = now_ms();
    let current = choose_round(&tx, &c, at)?;
    if current.start <= at && at < current.end {
        return Err(conflict("已有一场红包雨正在进行"));
    }
    let pending: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM rain_rounds WHERE source='manual' AND start_ms>?1)",
        [at],
        |r| r.get(0),
    )?;
    if pending {
        return Err(conflict("已有一场红包雨等待开始"));
    }
    let id = Uuid::new_v4().to_string();
    let start = at + input.countdown_seconds * 1000;
    let end = start + c.duration_seconds * 1000;
    tx.execute("INSERT INTO rain_rounds(id,start_ms,end_ms,source,config_json,created_at) VALUES(?1,?2,?3,'manual',?4,?5)",params![id,start,end,serde_json::to_string(&c)?,now()])?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "rain_start",
        &id,
        "安排红包雨场次",
        json!({}),
        json!({"start_at":instant(start),"end_at":instant(end)}),
    )?;
    let result = status_json(&tx, None, at)?;
    tx.commit()?;
    Ok(Json(result))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/rain/status", get(status))
        .route("/rain/join", post(join))
        .route("/rain/claim", post(claim))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/rain-config", get(admin_config).put(update_config))
        .route("/rain/start", post(start_round))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture(sdk: bool, large: bool) -> (AppState, RequestContext, HeaderMap, i64) {
        let state = AppState {
            db_path: std::env::temp_dir()
                .join(format!("orange-rain-test-{}.sqlite3", Uuid::new_v4())),
            config: RuntimeConfig {
                production: false,
                ad_mode: if large { "csj" } else { "mock" }.into(),
                csj_reward_secret: if large {
                    Some("rain-test-secret".into())
                } else {
                    None
                },
                admin_username: "admin".into(),
                admin_password: None,
                mail: mail_admin::Config::default(),
                mail_events: mail_events::Config::default(),
                app_download_url: None,
                privacy_url: None,
                invite_base_url: "https://orange.lingzidev.com".into(),
                device_trust: if sdk {
                    Some(risk::Config::test_config())
                } else {
                    None
                },
                review_login_phone: None,
                review_login_code: None,
            },
        };
        init(&state).unwrap();
        let db = open(&state).unwrap();
        let at = now_ms();
        for (uid, phone, code, token) in [
            (2, "13900139000", "AB7C8D", "rain-user"),
            (3, "13900139001", "EF4G5H", "rain-other"),
        ] {
            db.execute("INSERT INTO users(id,phone,password_hash,nickname,coins,created_at,invite_code,inviter_code,invitation_bound_at) VALUES(?1,?2,'unused','红包雨测试',0,?3,?4,'JZ8F6A',?3)",params![uid,phone,now(),code]).unwrap();
            db.execute(
                "INSERT INTO sessions(token,user_id,expires_at) VALUES(?1,?2,?3)",
                params![token, uid, now() + 3600],
            )
            .unwrap();
        }
        let c = RainConfig {
            duration_seconds: 60,
            packet_count: 5,
            small_min_coins: 1,
            small_max_coins: 1,
            grand_probability: if large { 1.0 } else { 0.0 },
            grand_coins: 20,
            max_total_coins: 100,
            ..Default::default()
        };
        db.execute(
            "UPDATE rain_config SET config_json=?1 WHERE id=1",
            [serde_json::to_string(&c).unwrap()],
        )
        .unwrap();
        db.execute("INSERT INTO rain_rounds(id,start_ms,end_ms,source,config_json,created_at) VALUES('test-round',?1,?2,'manual',?3,?4)",params![at-500,at+59500,serde_json::to_string(&c).unwrap(),now()]).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer rain-user"),
        );
        if sdk {
            business_guard::test_event(
                &state,
                "rain-proof",
                2,
                "rain",
                json!({"round_id":"test-round"}),
            );
            headers.insert("x-risk-event", HeaderValue::from_static("rain-proof"));
        }
        (
            state,
            RequestContext {
                request_id: Uuid::new_v4().to_string(),
                client_ip: "203.0.113.5".into(),
                admin_username: None,
            },
            headers,
            at,
        )
    }
    fn epoch(raw: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .timestamp_millis()
    }
    #[test]
    fn beijing_hourly_schedule_crosses_midnight_and_never_reopens_ended_round() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let mut c = RainConfig::default();
        let midnight = epoch("2026-11-12T00:00:00+08:00");
        assert_eq!(choose_round(&db, &c, midnight - 1).unwrap().start, midnight);
        c.start_hour = 2;
        c.every_hours = 3;
        assert_eq!(
            choose_round(&db, &c, midnight - 1).unwrap().start,
            midnight + 2 * 3600000
        );
        c.start_hour = 0;
        c.every_hours = 1;
        c.duration_seconds = 10;
        let round = choose_round(&db, &c, midnight + 1).unwrap();
        assert_eq!(round.start, midnight);
        c.duration_seconds = 60;
        assert!(choose_round(&db, &c, round.end).unwrap().start > round.end);
        let meta = status_json(&db, None, midnight - 1).unwrap();
        assert!(meta.get("packets").is_none());
    }
    #[test]
    fn join_is_fixed_private_capped_and_consumes_only_one_sdk_event() {
        let (state, context, headers, at) = fixture(true, true);
        let db = open(&state).unwrap();
        let mut c = config(&db).unwrap();
        c.grand_coins = 10;
        c.max_total_coins = 20;
        let raw = serde_json::to_string(&c).unwrap();
        db.execute("UPDATE rain_config SET config_json=?1", [&raw])
            .unwrap();
        db.execute("UPDATE rain_rounds SET config_json=?1", [raw])
            .unwrap();
        business_guard::test_event(
            &state,
            "rain-proof",
            2,
            "rain",
            json!({"round_id":"test-round"}),
        );
        let first = join_at(&state, &context, &headers, at).unwrap().0;
        let mut plain = headers.clone();
        plain.remove("x-risk-event");
        let again = join_at(&state, &context, &plain, at + 1).unwrap().0;
        assert_eq!(first["packets"], again["packets"]);
        assert_eq!(first["packets"].as_array().unwrap().len(), 5);
        assert!(
            first["packets"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p.get("coins").is_none() && p.get("reward").is_none())
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM rain_packets WHERE user_id=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
        assert!(
            db.query_row(
                "SELECT SUM(coins) FROM rain_packets WHERE user_id=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap()
                <= 20
        );
        assert_eq!(
            db.query_row(
                "SELECT operation_ref FROM risk_events WHERE id='rain-proof'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "rain:test-round:2"
        );
        assert!(join_at(&state, &context, &headers, at - 1000).is_err());
        assert!(join_at(&state, &context, &headers, at + 60000).is_err());
    }
    #[test]
    fn small_claim_rejects_forgery_owner_time_rate_and_shutdown_without_extra_money() {
        let (state, context, headers, at) = fixture(false, false);
        let group = join_at(&state, &context, &headers, at).unwrap().0;
        let db = open(&state).unwrap();
        let first = group["packets"][0]["id"].as_str().unwrap();
        let future = group["packets"][4]["id"].as_str().unwrap();
        assert!(claim_at(&state, &context, &headers, "fake-id", at).is_err());
        assert!(claim_at(&state, &context, &headers, future, at).is_err());
        let mut other = headers.clone();
        other.insert(
            "authorization",
            HeaderValue::from_static("Bearer rain-other"),
        );
        assert_eq!(
            claim_at(&state, &context, &other, first, at).unwrap_err().0,
            StatusCode::NOT_FOUND
        );
        let input: ClaimInput = serde_json::from_value(
            json!({"packet_id":first,"coins":999999,"server_now":"2099-01-01"}),
        )
        .unwrap();
        let paid = claim_at(&state, &context, &headers, &input.packet_id, at)
            .unwrap()
            .0;
        assert_eq!(paid["reward"], 1);
        assert_eq!(paid["credited"], true);
        let repeat = claim_at(&state, &context, &headers, first, at + 1)
            .unwrap()
            .0;
        assert_eq!(repeat["already_claimed"], true);
        assert_eq!(repeat["user"]["coins"], 1);
        let second = group["packets"][1]["id"].as_str().unwrap();
        db.execute(
            "UPDATE rain_packets SET reveal_ms=?1 WHERE id=?2",
            params![at - 1, second],
        )
        .unwrap();
        assert_eq!(
            claim_at(&state, &context, &headers, second, at + 100)
                .unwrap_err()
                .0,
            StatusCode::TOO_MANY_REQUESTS
        );
        let expire = epoch(group["packets"][0]["expires_at"].as_str().unwrap());
        assert!(claim_at(&state, &context, &headers, first, expire).is_err());
        let mut c = config(&db).unwrap();
        c.enabled = false;
        db.execute(
            "UPDATE rain_config SET config_json=?1",
            [serde_json::to_string(&c).unwrap()],
        )
        .unwrap();
        assert!(claim_at(&state, &context, &headers, second, at + 1000).is_err());
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM ledger WHERE user_id=2 AND source LIKE 'rain:%'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    #[tokio::test]
    async fn large_reward_requires_separate_proof_and_callback_but_started_session_survives_shutdown()
     {
        let (state, context, mut headers, at) = fixture(true, true);
        let group = join_at(&state, &context, &headers, at).unwrap().0;
        let db = open(&state).unwrap();
        let pid = group["packets"][0]["id"].as_str().unwrap();
        let task = format!("rain:{pid}");
        assert!(
            reward_start(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(RewardInput {
                    task_id: task.clone(),
                    quote_id: "not-yet-quoted".into()
                })
            )
            .await
            .is_err()
        );
        let opened = claim_at(&state, &context, &headers, pid, at).unwrap().0;
        assert_eq!(opened["kind"], "large");
        assert_eq!(user(&db, 2).unwrap().coins, 0);
        let quote = reward_policy::test_quote(&state, &headers, &task, None).await;
        let qid = quote["quote_id"].as_str().unwrap().to_owned();
        assert!(
            reward_start(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(RewardInput {
                    task_id: task.clone(),
                    quote_id: qid.clone()
                })
            )
            .await
            .is_err()
        );
        business_guard::test_event(
            &state,
            "reward-proof",
            2,
            "reward",
            json!({"task_id":task,"quote_id":qid}),
        );
        headers.insert("x-risk-event", HeaderValue::from_static("reward-proof"));
        let session = reward_start(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RewardInput {
                task_id: task,
                quote_id: qid,
            }),
        )
        .await
        .unwrap()
        .0;
        let sid = session["id"].as_str().unwrap();
        assert!(
            reward_complete(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Path(sid.into())
            )
            .await
            .is_err()
        );
        assert_eq!(user(&db, 2).unwrap().coins, 0);
        let mut c = config(&db).unwrap();
        c.enabled = false;
        db.execute(
            "UPDATE rain_config SET config_json=?1",
            [serde_json::to_string(&c).unwrap()],
        )
        .unwrap();
        db.execute("UPDATE rain_rounds SET end_ms=?1", [now_ms() - 1])
            .unwrap();
        let trans = "rain-callback-transaction";
        let mut query = HashMap::from([
            ("user_id".into(), "2".into()),
            ("trans_id".into(), trans.into()),
            ("reward_name".into(), "金币".into()),
            ("reward_amount".into(), "99999".into()),
            (
                "extra".into(),
                session["media_extra"].as_str().unwrap().into(),
            ),
            ("sign".into(), "forged".into()),
        ]);
        assert!(!verify_csj_callback(&state, &query).unwrap().valid);
        query.insert(
            "sign".into(),
            format!(
                "{:x}",
                Sha256::digest(format!("rain-test-secret:{trans}").as_bytes())
            ),
        );
        // The platform's reward amount (99999 here) is ignored: a correctly signed callback is
        // accepted and the coins paid are the server-locked amount on the session.
        assert!(verify_csj_callback(&state, &query).unwrap().valid);
        let paid = reward_complete(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Path(sid.into()),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(paid["reward"], 20);
        assert_eq!(paid["user"]["coins"], 20);
        let repeat = reward_complete(State(state), Extension(context), headers, Path(sid.into()))
            .await
            .unwrap()
            .0;
        assert_eq!(repeat["already_completed"], true);
        assert_eq!(repeat["user"]["coins"], 20);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM ledger WHERE source=?1",
                [format!("rain:{pid}")],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
