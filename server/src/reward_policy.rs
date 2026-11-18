use super::*;
use rand_core::RngCore;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Range {
    min_coins: i64,
    max_coins: i64,
}
#[derive(Clone, Serialize, Deserialize)]
struct Tier {
    max_days: i64,
    max_completed: i64,
    min_coins: i64,
    max_coins: i64,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Policy {
    enabled: bool,
    fixed_coins: i64,
    quote_ttl_seconds: i64,
    new_device: Tier,
    active_device: Tier,
    mature_device: Range,
    unknown_device: Range,
    #[serde(default)]
    policy_version: i64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: true,
            fixed_coins: 40,
            quote_ttl_seconds: 300,
            new_device: Tier {
                max_days: 3,
                max_completed: 5,
                min_coins: 100,
                max_coins: 150,
            },
            active_device: Tier {
                max_days: 30,
                max_completed: 30,
                min_coins: 40,
                max_coins: 80,
            },
            mature_device: Range {
                min_coins: 15,
                max_coins: 40,
            },
            unknown_device: Range {
                min_coins: 10,
                max_coins: 20,
            },
            policy_version: 1,
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS reward_policy(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS reward_device_profiles(identity TEXT PRIMARY KEY,identity_kind TEXT NOT NULL,first_seen INTEGER NOT NULL,last_seen INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS reward_device_keys(key_id TEXT PRIMARY KEY,identity TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS reward_device_aliases(old_identity TEXT PRIMARY KEY,new_identity TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS reward_quotes(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,task_id TEXT NOT NULL,coins INTEGER NOT NULL,min_coins INTEGER NOT NULL,max_coins INTEGER NOT NULL,tier TEXT NOT NULL,device_identity TEXT NOT NULL,allocation_count INTEGER NOT NULL,policy_version INTEGER NOT NULL,basis_json TEXT NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,session_id TEXT UNIQUE);
      CREATE INDEX IF NOT EXISTS idx_reward_quotes_owner ON reward_quotes(user_id,task_id,expires_at);
      CREATE INDEX IF NOT EXISTS idx_reward_quotes_device ON reward_quotes(device_identity,allocation_count,policy_version);")?;
    db.execute(
        "INSERT OR IGNORE INTO reward_policy(id,config_json) VALUES(1,?1)",
        [serde_json::to_string(&Policy::default())?],
    )?;
    let columns = db
        .prepare("PRAGMA table_info(reward_sessions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    for (name, kind) in [
        ("quote_id", "TEXT"),
        ("device_identity", "TEXT NOT NULL DEFAULT ''"),
        ("reward_basis_json", "TEXT NOT NULL DEFAULT '{}'"),
        ("guard_event_id", "TEXT"),
    ] {
        if !columns.iter().any(|v| v == name) {
            db.execute_batch(&format!(
                "ALTER TABLE reward_sessions ADD COLUMN {name} {kind}"
            ))?;
        }
    }
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS idx_reward_session_quote ON reward_sessions(quote_id) WHERE quote_id IS NOT NULL;")?;
    let historical=db.prepare("SELECT key_id,MIN(first_seen),MAX(last_seen) FROM account_devices WHERE key_id!='' GROUP BY key_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?)))?.collect::<Result<Vec<_>,_>>()?;
    for (key, first, last) in historical {
        let identity = format!("key:{key}");
        db.execute("INSERT OR IGNORE INTO reward_device_profiles(identity,identity_kind,first_seen,last_seen) VALUES(?1,'verified_key',?2,?3)",params![identity,first,last])?;
        db.execute(
            "INSERT OR IGNORE INTO reward_device_keys(key_id,identity) VALUES(?1,?2)",
            params![key, identity],
        )?;
    }
    db.execute("UPDATE reward_sessions SET device_identity=(SELECT k.identity FROM risk_events e JOIN reward_device_keys k ON k.key_id=e.key_id WHERE e.operation_ref=reward_sessions.id AND e.user_id=reward_sessions.user_id AND e.status='reported' LIMIT 1) WHERE device_identity='' AND EXISTS(SELECT 1 FROM risk_events e JOIN reward_device_keys k ON k.key_id=e.key_id WHERE e.operation_ref=reward_sessions.id AND e.user_id=reward_sessions.user_id AND e.status='reported')",[])?;
    Ok(())
}
fn expired_quote() -> ApiError {
    ApiError(StatusCode::GONE, "报价已失效，请重新打开红包".into(), None)
}
fn policy(db: &Connection) -> Result<Policy, ApiError> {
    let raw: String = db.query_row(
        "SELECT config_json FROM reward_policy WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}
pub(super) fn resolve_identity(db: &Connection, identity: &str) -> Result<String, ApiError> {
    Ok(db
        .query_row(
            "SELECT new_identity FROM reward_device_aliases WHERE old_identity=?1",
            [identity],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_else(|| identity.into()))
}
pub(super) fn observe_verified(
    db: &Connection,
    _uid: i64,
    key: &str,
    device_id: &str,
) -> Result<String, ApiError> {
    let old: Option<String> = db
        .query_row(
            "SELECT identity FROM reward_device_keys WHERE key_id=?1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    let target = if device_id.is_empty() {
        old.clone().unwrap_or_else(|| format!("key:{key}"))
    } else {
        format!("device:{device_id}")
    };
    if old
        .as_ref()
        .is_some_and(|value| value.starts_with("device:") && value != &target)
    {
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "已验证设备关联发生冲突".into(),
            None,
        ));
    }
    let seen: Option<i64> = db.query_row(
        "SELECT MIN(first_seen) FROM account_devices WHERE key_id=?1",
        [key],
        |r| r.get(0),
    )?;
    let previous = if let Some(old) = &old {
        db.query_row(
            "SELECT first_seen FROM reward_device_profiles WHERE identity=?1",
            [old],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
    } else {
        None
    };
    let first = seen.into_iter().chain(previous).min().unwrap_or_else(now);
    db.execute("INSERT INTO reward_device_profiles(identity,identity_kind,first_seen,last_seen) VALUES(?1,?2,?3,?4) ON CONFLICT(identity) DO UPDATE SET first_seen=MIN(reward_device_profiles.first_seen,excluded.first_seen),last_seen=excluded.last_seen",params![target,if target.starts_with("device:"){"stable_device_id"}else{"verified_key"},first,now()])?;
    if let Some(old) = old.filter(|old| old != &target) {
        db.execute("INSERT INTO reward_device_aliases(old_identity,new_identity) VALUES(?1,?2) ON CONFLICT(old_identity) DO UPDATE SET new_identity=excluded.new_identity",params![old,target])?;
        db.execute(
            "UPDATE reward_sessions SET device_identity=?1 WHERE device_identity=?2",
            params![target, old],
        )?;
        db.execute(
            "UPDATE reward_quotes SET device_identity=?1 WHERE device_identity=?2",
            params![target, old],
        )?;
    }
    db.execute("INSERT INTO reward_device_keys(key_id,identity) VALUES(?1,?2) ON CONFLICT(key_id) DO UPDATE SET identity=excluded.identity",params![key,target])?;
    if !device_id.is_empty() && device_id != key {
        // A verified 0.5 identity upgrades the old key-only observation without resetting its age.
        db.execute("UPDATE account_devices SET first_seen=MIN(first_seen,COALESCE((SELECT MIN(old.first_seen) FROM account_devices old WHERE old.user_id=account_devices.user_id AND old.key_id=?1),first_seen)),last_seen=MAX(last_seen,COALESCE((SELECT MAX(old.last_seen) FROM account_devices old WHERE old.user_id=account_devices.user_id AND old.key_id=?1),last_seen)) WHERE device_id=?2 AND key_id=?1",params![key,device_id])?;
        db.execute("DELETE FROM account_devices WHERE key_id=?1 AND device_id=?1 AND EXISTS(SELECT 1 FROM account_devices current WHERE current.user_id=account_devices.user_id AND current.device_id=?2)",params![key,device_id])?;
        db.execute(
            "UPDATE account_devices SET device_id=?2 WHERE key_id=?1 AND device_id=?1",
            params![key, device_id],
        )?;
    }
    Ok(target)
}
fn trusted_identity(
    db: &Connection,
    uid: i64,
    key: Option<&str>,
) -> Result<(String, String), ApiError> {
    let recent: Option<String> = if key.filter(|key| !key.is_empty()).is_none() {
        db.query_row("SELECT key_id FROM account_devices WHERE user_id=?1 AND key_id!='' ORDER BY last_seen DESC LIMIT 1",[uid],|r|r.get(0)).optional()?
    } else {
        None
    };
    let Some(key) = key.filter(|key| !key.is_empty()).or(recent.as_deref()) else {
        return Ok(("unknown".into(), String::new()));
    };
    let identity:Option<String>=db.query_row("SELECT k.identity FROM reward_device_keys k WHERE k.key_id=?1 AND EXISTS(SELECT 1 FROM account_devices d WHERE d.user_id=?2 AND d.key_id=k.key_id)",params![key,uid],|r|r.get(0)).optional()?;
    if identity.is_none()
        && db.query_row(
            "SELECT EXISTS(SELECT 1 FROM account_devices WHERE user_id=?1 AND key_id!='')",
            [uid],
            |r| r.get::<_, bool>(0),
        )?
    {
        return Err(invalid("账户信息正在更新，请稍后重试"));
    }
    Ok((identity.unwrap_or_else(|| "unknown".into()), key.into()))
}
fn counts(db: &Connection, identity: &str) -> Result<(i64, i64), ApiError> {
    counts_at(db, identity, now())
}
fn counts_at(db: &Connection, identity: &str, at: i64) -> Result<(i64, i64), ApiError> {
    let completed=db.query_row("SELECT COUNT(*) FROM reward_sessions WHERE device_identity=?1 AND completed_at IS NOT NULL AND task_id NOT LIKE 'rain:%'",[identity],|r|r.get(0))?;
    let reserved=db.query_row("SELECT COUNT(*) FROM reward_sessions WHERE device_identity=?1 AND completed_at IS NULL AND started_at>=?2 AND task_id NOT LIKE 'rain:%'",params![identity,at-1800],|r|r.get(0))?;
    Ok((completed, reserved))
}
fn selection(
    db: &Connection,
    p: &Policy,
    identity: &str,
    key: &str,
) -> Result<(String, i64, i64, Value), ApiError> {
    let profile: Option<(String, i64)> = db
        .query_row(
            "SELECT identity_kind,first_seen FROM reward_device_profiles WHERE identity=?1",
            [identity],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (completed, reserved) = if profile.is_some() {
        counts(db, identity)?
    } else {
        (0, 0)
    };
    let allocation = completed + reserved;
    let age = profile.as_ref().map(|v| (now() - v.1).max(0));
    let (tier, min, max, tier_config) = if !p.enabled {
        (
            "fixed",
            p.fixed_coins,
            p.fixed_coins,
            json!({"fixed_coins":p.fixed_coins}),
        )
    } else if profile.is_none() {
        (
            "unknown_device",
            p.unknown_device.min_coins,
            p.unknown_device.max_coins,
            serde_json::to_value(&p.unknown_device)?,
        )
    } else if age.unwrap() <= p.new_device.max_days * 86400
        && allocation < p.new_device.max_completed
    {
        (
            "new_device",
            p.new_device.min_coins,
            p.new_device.max_coins,
            serde_json::to_value(&p.new_device)?,
        )
    } else if age.unwrap() <= p.active_device.max_days * 86400
        && allocation < p.active_device.max_completed
    {
        (
            "active_device",
            p.active_device.min_coins,
            p.active_device.max_coins,
            serde_json::to_value(&p.active_device)?,
        )
    } else {
        (
            "mature_device",
            p.mature_device.min_coins,
            p.mature_device.max_coins,
            serde_json::to_value(&p.mature_device)?,
        )
    };
    let basis = json!({"device_identity":identity,"identity_kind":profile.as_ref().map(|v|v.0.as_str()).unwrap_or("unknown"),"key_id":key,"first_seen":profile.as_ref().map(|v|iso(v.1)),"age_days":age.map(|v|v as f64/86400.0),"completed_before":completed,"reserved_before":reserved,"allocation_count_before":allocation,"evidence_source":if profile.is_some(){"server_verified_device_history"}else{"unverified_current_device"},"quote_created_at":iso(now()),"policy_version":p.policy_version,"tier_config":tier_config});
    Ok((tier.into(), min, max, basis))
}
pub(super) fn preview(db: &Connection, uid: i64) -> Result<(i64, i64), ApiError> {
    let key:Option<String>=db.query_row("SELECT key_id FROM account_devices WHERE user_id=?1 AND key_id!='' ORDER BY last_seen DESC LIMIT 1",[uid],|r|r.get(0)).optional()?;
    let (identity, key) = trusted_identity(db, uid, key.as_deref())?;
    let (_, min, max, _) = selection(db, &policy(db)?, &identity, &key)?;
    Ok((min, max))
}
#[derive(Deserialize)]
struct QuoteInput {
    task_id: String,
    key_id: Option<String>,
}
pub(super) struct Quote {
    pub id: String,
    pub task_id: String,
    pub coins: i64,
    pub min: i64,
    pub max: i64,
    pub tier: String,
    pub device: String,
    pub allocation: i64,
    pub version: i64,
    pub basis: Value,
    pub expires: i64,
    pub session_id: Option<String>,
}
pub(super) fn read_quote(db: &Connection, uid: i64, id: &str) -> Result<Quote, ApiError> {
    let row:Option<(String,String,i64,i64,i64,String,String,i64,i64,String,i64,Option<String>)>=db.query_row("SELECT id,task_id,coins,min_coins,max_coins,tier,device_identity,allocation_count,policy_version,basis_json,expires_at,session_id FROM reward_quotes WHERE id=?1 AND user_id=?2",params![id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional()?;
    let Some(r) = row else {
        return Err(expired_quote());
    };
    Ok(Quote {
        id: r.0,
        task_id: r.1,
        coins: r.2,
        min: r.3,
        max: r.4,
        tier: r.5,
        device: r.6,
        allocation: r.7,
        version: r.8,
        basis: serde_json::from_str(&r.9)?,
        expires: r.10,
        session_id: r.11,
    })
}
async fn quote(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<QuoteInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let (identity, key) = match trusted_identity(&db, uid, input.key_id.as_deref()) {
        Ok(identity) => identity,
        Err(error) => {
            user_admin::behavior(
                &db,
                &context,
                Some(uid),
                None,
                "reward_quote",
                "rejected",
                "",
                None,
                json!({"reason":"key_not_verified_for_account","key_id":input.key_id}),
            )?;
            return Err(error);
        }
    };
    let tx = db.transaction()?;
    risk::check_account(&tx, uid)?;
    let p = policy(&tx)?;
    let grand = rain::grand_task(&state, &tx, uid, &input.task_id)?;
    let title = if input.task_id.starts_with("newbie:") {
        "新人礼包红包"
    } else if grand.is_some() {
        "红包雨大奖"
    } else {
        task_info(&input.task_id)?.1
    };
    let (tier, min, max, mut basis) = if let Some(grand) = &grand {
        (
            "fixed".into(),
            grand.coins,
            grand.coins,
            json!({"scope":if input.task_id.starts_with("newbie:") {"newbie_video"} else {"rain_large"},"device_identity":identity,"allocation_count_before":0,"policy_version":p.policy_version}),
        )
    } else {
        selection(&tx, &p, &identity, &key)?
    };
    let allocation = basis["allocation_count_before"].as_i64().unwrap_or(0);
    let existing:Option<String>=tx.query_row("SELECT id FROM reward_quotes WHERE user_id=?1 AND task_id=?2 AND device_identity=?3 AND allocation_count=?4 AND policy_version=?5 AND expires_at>?6 AND session_id IS NULL ORDER BY created_at DESC LIMIT 1",params![uid,input.task_id,identity,allocation,p.policy_version,now()],|r|r.get(0)).optional()?;
    let id = if let Some(id) = existing {
        id
    } else {
        let bucket = if identity == "unknown" {
            format!("unknown:{uid}")
        } else {
            identity.clone()
        };
        let previous:Option<i64>=tx.query_row("SELECT coins FROM reward_quotes WHERE (device_identity=?1 OR (device_identity='unknown' AND user_id=?2)) AND allocation_count=?3 AND policy_version=?4 AND tier=?5 AND min_coins=?6 AND max_coins=?7 AND expires_at>?8 ORDER BY created_at DESC LIMIT 1",params![bucket,uid,allocation,p.policy_version,tier,min,max,now()],|r|r.get(0)).optional()?;
        let coins =
            previous.unwrap_or_else(|| min + (OsRng.next_u64() % ((max - min + 1) as u64)) as i64);
        let id = Uuid::new_v4().to_string();
        basis["quote_created_at"] = json!(iso(now()));
        tx.execute("INSERT INTO reward_quotes(id,user_id,task_id,coins,min_coins,max_coins,tier,device_identity,allocation_count,policy_version,basis_json,created_at,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,uid,input.task_id,coins,min,max,tier,identity,allocation,p.policy_version,basis.to_string(),now(),now()+p.quote_ttl_seconds])?;
        id
    };
    let q = read_quote(&tx, uid, &id)?;
    let response = json!({"quote_id":q.id,"task_id":q.task_id,"reward":q.coins,"reward_min":q.min,"reward_max":q.max,"expires_at":iso(q.expires),"task":{"id":q.task_id,"title":title,"subtitle":"看视频领取这份小惊喜","reward":q.coins,"reward_min":q.coins,"reward_max":q.coins,"kind":"rewarded_video","claimed_today":0}});
    // The hall uses this to know the last pushed packet was opened (quotes can be reused).
    tx.execute(
        "INSERT INTO hall_activity(user_id,last_open_at) VALUES(?1,?2) ON CONFLICT(user_id) DO UPDATE SET last_open_at=excluded.last_open_at",
        params![uid, now()],
    )?;
    tx.commit()?;
    Ok(Json(response))
}
#[cfg(test)]
pub(super) async fn test_quote(
    state: &AppState,
    headers: &HeaderMap,
    task: &str,
    key: Option<&str>,
) -> Value {
    quote(
        State(state.clone()),
        Extension(RequestContext {
            request_id: Uuid::new_v4().to_string(),
            client_ip: "203.0.113.8".into(),
            admin_username: None,
        }),
        headers.clone(),
        Json(QuoteInput {
            task_id: task.into(),
            key_id: key.map(str::to_owned),
        }),
    )
    .await
    .unwrap()
    .0
}
pub(super) fn quote_intent(db: &Connection, uid: i64, body: &Value) -> Result<Value, ApiError> {
    let task = body["task_id"]
        .as_str()
        .ok_or_else(|| invalid("请重新打开红包"))?;
    let id = body["quote_id"]
        .as_str()
        .ok_or_else(|| invalid("请重新打开红包获取报价"))?;
    let q = read_quote(db, uid, id)?;
    if q.task_id != task {
        return Err(conflict("报价与红包不匹配，请重新打开"));
    }
    Ok(
        json!({"task_id":task,"quote_id":id,"reward_coins":q.coins,"currency":"COIN","policy_version":q.version}),
    )
}
/// Quotes with a fixed, server-decided amount (rain grand prize, 新人礼包 video packet): the
/// device tier policy does not re-price them.
fn fixed_task(task: &str) -> bool {
    task.starts_with("rain:") || task.starts_with("newbie:")
}
pub(super) fn attach(
    db: &Connection,
    uid: i64,
    q: &Quote,
    sid: &str,
    actual_device: &str,
) -> Result<Value, ApiError> {
    if q.session_id.is_some()
        || q.expires <= now()
        || (!fixed_task(&q.task_id) && q.version != policy(db)?.policy_version)
    {
        return Err(expired_quote());
    }
    let current_device = resolve_identity(db, actual_device)?;
    let quote_device = resolve_identity(db, &q.device)?;
    if q.device != "unknown" && quote_device != current_device {
        return Err(expired_quote());
    }
    if !fixed_task(&q.task_id) && q.device != "unknown" {
        let (tier, _, _, basis) = selection(
            db,
            &policy(db)?,
            &current_device,
            q.basis["key_id"].as_str().unwrap_or(""),
        )?;
        if tier != q.tier || basis["allocation_count_before"].as_i64() != Some(q.allocation) {
            return Err(expired_quote());
        }
    }
    if !fixed_task(&q.task_id) && q.device == "unknown" && current_device != "unknown" {
        let (completed, reserved) = counts(db, &current_device)?;
        let first: i64 = db.query_row(
            "SELECT first_seen FROM reward_device_profiles WHERE identity=?1",
            [&current_device],
            |r| r.get(0),
        )?;
        if completed + reserved > 0 || now() - first > policy(db)?.new_device.max_days * 86400 {
            return Err(expired_quote());
        }
    }
    let mut basis = q.basis.clone();
    basis["tier"] = json!(q.tier);
    basis["min_coins"] = json!(q.min);
    basis["max_coins"] = json!(q.max);
    basis["actual_device_identity"] = json!(current_device);
    db.execute(
        "UPDATE reward_quotes SET session_id=?1 WHERE id=?2 AND user_id=?3 AND session_id IS NULL",
        params![sid, q.id, uid],
    )?;
    Ok(basis)
}
async fn admin_config(State(state): State<AppState>) -> ApiResult<Policy> {
    Ok(Json(policy(&open(&state)?)?))
}
async fn update(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<Policy>,
) -> ApiResult<Policy> {
    for (min, max) in [
        (input.new_device.min_coins, input.new_device.max_coins),
        (input.active_device.min_coins, input.active_device.max_coins),
        (input.mature_device.min_coins, input.mature_device.max_coins),
        (
            input.unknown_device.min_coins,
            input.unknown_device.max_coins,
        ),
    ] {
        if min < 1 || max < min || max > 10000 {
            return Err(invalid("奖励金币范围应为 1 至 10000，最大值不低于最小值"));
        }
    }
    if !(1..=10000).contains(&input.fixed_coins)
        || !(30..=300).contains(&input.quote_ttl_seconds)
        || input.new_device.max_days < 1
        || input.new_device.max_completed < 1
        || input.active_device.max_days < input.new_device.max_days
        || input.active_device.max_completed < input.new_device.max_completed
    {
        return Err(invalid("请检查奖励天数、次数和报价有效期"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let old = policy(&tx)?;
    input.policy_version = old.policy_version + 1;
    tx.execute(
        "UPDATE reward_policy SET config_json=?1 WHERE id=1",
        [serde_json::to_string(&input)?],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "reward_policy",
        "reward-policy",
        "修改设备阶梯奖励",
        serde_json::to_value(old)?,
        serde_json::to_value(&input)?,
    )?;
    tx.commit()?;
    Ok(Json(input))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/rewards/quote", post(quote))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/reward-policy", get(admin_config).put(update))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session(db: &Connection, id: &str, uid: i64, identity: &str, completed: bool, started: i64) {
        db.execute("INSERT INTO reward_sessions(id,user_id,task_id,reward,started_at,completed_at,device_identity) VALUES(?1,?2,'welcome',100,?3,?4,?5)",params![id,uid,started,if completed{Some(now())}else{None},identity]).unwrap();
    }
    fn observe(db: &Connection, uid: i64, key: &str) -> String {
        let identity = observe_verified(db, uid, key, "shared-device").unwrap();
        user_admin::record_device(
            db,
            uid,
            &user_admin::DeviceObservation {
                device_id: "shared-device".into(),
                key_id: key.into(),
                report_id: "verified".into(),
                evidence_status: "signature_only".into(),
                platform: "android".into(),
                model: String::new(),
                os_version: String::new(),
            },
        )
        .unwrap();
        identity
    }
    #[test]
    fn verified_device_age_and_allocations_cross_accounts_and_include_boundary_reservations() {
        let (state, _, _, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let identity = observe(&db, 2, "old-key");
        let p = Policy::default();
        assert_eq!(
            selection(&db, &p, &identity, "old-key").unwrap().0,
            "new_device"
        );
        for i in 0..4 {
            session(
                &db,
                &format!("done{i}"),
                if i % 2 == 0 { 2 } else { 3 },
                &identity,
                true,
                now() - 10,
            );
        }
        assert_eq!(
            selection(&db, &p, &identity, "old-key").unwrap().0,
            "new_device"
        );
        let at = now();
        session(&db, "boundary", 3, &identity, false, at - 1800);
        assert_eq!(counts_at(&db, &identity, at).unwrap(), (4, 1));
        assert_eq!(counts_at(&db, &identity, at + 1).unwrap(), (4, 0));
        db.execute(
            "UPDATE reward_sessions SET started_at=?1 WHERE id='boundary'",
            [now()],
        )
        .unwrap();
        assert_eq!(
            selection(&db, &p, &identity, "old-key").unwrap().0,
            "active_device"
        );
        db.execute(
            "UPDATE reward_device_profiles SET first_seen=?1 WHERE identity=?2",
            params![now() - 31 * 86400, identity],
        )
        .unwrap();
        assert_eq!(
            selection(&db, &p, &identity, "old-key").unwrap().0,
            "mature_device"
        );
        let mut disabled = p;
        disabled.enabled = false;
        disabled.fixed_coins = 43;
        let s = selection(&db, &disabled, &identity, "old-key").unwrap();
        assert_eq!((s.0.as_str(), s.1, s.2), ("fixed", 43, 43));
    }
    #[test]
    fn key_only_upgrade_preserves_age_and_cross_account_history_without_duplicate_devices() {
        let (state, _, _, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let first = now() - 40 * 86400;
        for uid in [2, 3] {
            db.execute("INSERT INTO account_devices VALUES(?1,'legacy-key','android','','','legacy-key','old-report','signature_only',?2,?2)",params![uid,first]).unwrap();
        }
        initialize(&db).unwrap();
        session(
            &db,
            "legacy-completed",
            3,
            "key:legacy-key",
            true,
            now() - 50,
        );
        let identity = observe_verified(&db, 2, "legacy-key", "shared-device").unwrap();
        assert_eq!(counts(&db, &identity).unwrap().0, 1);
        assert_eq!(resolve_identity(&db, "key:legacy-key").unwrap(), identity);
        assert_eq!(trusted_identity(&db, 3, None).unwrap().0, identity);
        assert_eq!(
            trusted_identity(&db, 2, Some("unverified-attacker-key"))
                .unwrap_err()
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM account_devices WHERE device_id='shared-device'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(
            db.query_row(
                "SELECT first_seen FROM reward_device_profiles WHERE identity=?1",
                [identity],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            first
        );
    }
    #[tokio::test]
    async fn quote_reuses_price_and_pending_session_invalidates_other_high_tier_quotes() {
        let (state, _, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let identity = observe(&db, 2, "device-key");
        for i in 0..4 {
            session(&db, &format!("past{i}"), 3, &identity, true, now() - 10);
        }
        let first = test_quote(&state, &headers, "welcome", Some("device-key")).await;
        let again = test_quote(&state, &headers, "welcome", None).await;
        assert_eq!(first["quote_id"], again["quote_id"]);
        let second = test_quote(&state, &headers, "lucky", None).await;
        assert_eq!(first["reward"], second["reward"]);
        let q = read_quote(&db, 2, first["quote_id"].as_str().unwrap()).unwrap();
        attach(&db, 2, &q, "pending", &identity).unwrap();
        session(&db, "pending", 2, &identity, false, now());
        let stale = read_quote(&db, 2, second["quote_id"].as_str().unwrap()).unwrap();
        assert_eq!(
            attach(&db, 2, &stale, "extra", &identity).unwrap_err().0,
            StatusCode::GONE
        );
        let fresh = test_quote(&state, &headers, "lucky", None).await;
        assert!(fresh["reward"].as_i64().unwrap() <= 80);
        assert_eq!(trusted_identity(&db, 2, None).unwrap().0, identity);
    }
    #[tokio::test]
    async fn initial_unknown_quote_keeps_price_but_cannot_be_hoarded_after_device_binding() {
        let (state, _, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let a = test_quote(&state, &headers, "welcome", None).await;
        let b = test_quote(&state, &headers, "lucky", None).await;
        let identity = observe(&db, 2, "first-proof-key");
        let qa = read_quote(&db, 2, a["quote_id"].as_str().unwrap()).unwrap();
        assert_eq!(qa.tier, "unknown_device");
        assert!((10..=20).contains(&qa.coins));
        attach(&db, 2, &qa, "first-session", &identity).unwrap();
        session(&db, "first-session", 2, &identity, false, now());
        let qb = read_quote(&db, 2, b["quote_id"].as_str().unwrap()).unwrap();
        assert!(attach(&db, 2, &qb, "hoarded-session", &identity).is_err());
        let known = test_quote(&state, &headers, "lucky", None).await;
        assert!(known["reward"].as_i64().unwrap() >= 100);
        assert!(!known.to_string().contains("device_identity"));
    }
    #[tokio::test]
    async fn unknown_quote_cannot_select_high_price_for_old_device_with_zero_completions() {
        let (state, _, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let q = test_quote(&state, &headers, "welcome", None).await;
        let identity = observe(&db, 2, "older-key");
        db.execute(
            "UPDATE reward_device_profiles SET first_seen=?1 WHERE identity=?2",
            params![now() - 31 * 86400, identity],
        )
        .unwrap();
        let q = read_quote(&db, 2, q["quote_id"].as_str().unwrap()).unwrap();
        assert_eq!(counts(&db, &identity).unwrap(), (0, 0));
        assert_eq!(
            attach(&db, 2, &q, "old-price-choice", &identity)
                .unwrap_err()
                .0,
            StatusCode::GONE
        );
        let error = quote(
            State(state.clone()),
            Extension(RequestContext {
                request_id: "quote-investigation".into(),
                client_ip: "203.0.113.8".into(),
                admin_username: None,
            }),
            headers,
            Json(QuoteInput {
                task_id: "welcome".into(),
                key_id: Some("forged-key".into()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert!(!error.1.contains("key"));
        assert_eq!(db.query_row("SELECT COUNT(*) FROM user_behavior WHERE kind='reward_quote' AND status='rejected'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
    #[tokio::test]
    async fn begun_session_keeps_locked_coins_after_policy_changes_and_completion_retry() {
        let (state, context, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let quote = test_quote(&state, &headers, "welcome", None).await;
        let locked = quote["reward"].as_i64().unwrap();
        let started = reward_start(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RewardInput {
                task_id: "welcome".into(),
                quote_id: quote["quote_id"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        let sid = started["id"].as_str().unwrap();
        let mut changed = policy(&db).unwrap();
        changed.policy_version += 1;
        changed.unknown_device.min_coins = 500;
        changed.unknown_device.max_coins = 500;
        db.execute(
            "UPDATE reward_policy SET config_json=?1",
            [serde_json::to_string(&changed).unwrap()],
        )
        .unwrap();
        db.execute(
            "UPDATE reward_sessions SET started_at=?1 WHERE id=?2",
            params![now() - 6, sid],
        )
        .unwrap();
        let paid = reward_complete(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Path(sid.into()),
        )
        .await
        .unwrap()
        .0;
        let retry = reward_complete(
            State(state.clone()),
            Extension(context),
            headers,
            Path(sid.into()),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(paid["reward"], locked);
        assert_eq!(retry["user"]["coins"], locked);
        assert_eq!(retry["already_completed"], true);
    }
}
