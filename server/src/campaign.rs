use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Config {
    pub enabled: bool,
    pub title: String,
    #[serde(alias = "copy")]
    pub subtitle: String,
    #[serde(default = "default_entry_label")]
    pub entry_label: String,
    #[serde(default)]
    pub hero_image: Option<String>,
    #[serde(default)]
    pub starts_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(alias = "daily_limit")]
    pub daily_spin_limit: i64,
    pub settle_delay_seconds: i64,
    #[serde(default, alias = "arrival_threshold")]
    pub arrival_threshold_coins: i64,
    pub login_opportunities: i64,
    pub video_opportunities_per_day: i64,
    pub invite_opportunities_per_invitee: i64,
    #[serde(alias = "reward_tiers")]
    pub prizes: Vec<i64>,
    #[serde(default)]
    pub revision: i64,
}
fn default_entry_label() -> String {
    "开始抽奖".into()
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "每日幸运转盘".into(),
            subtitle: "完成登录、邀请或激励任务获得机会".into(),
            entry_label: default_entry_label(),
            hero_image: None,
            starts_at: None,
            expires_at: None,
            daily_spin_limit: 3,
            settle_delay_seconds: 60,
            arrival_threshold_coins: 0,
            login_opportunities: 1,
            video_opportunities_per_day: 3,
            invite_opportunities_per_invitee: 1,
            prizes: vec![5, 10, 20, 50, 100],
            revision: 1,
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS campaign_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS campaign_opportunities(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,source TEXT NOT NULL,source_ref TEXT NOT NULL,day TEXT NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,used_at INTEGER);
 CREATE UNIQUE INDEX IF NOT EXISTS idx_campaign_opp_source ON campaign_opportunities(user_id,source,source_ref);
 CREATE INDEX IF NOT EXISTS idx_campaign_opp_user_day ON campaign_opportunities(user_id,day,used_at);
 CREATE TABLE IF NOT EXISTS campaign_spins(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,opportunity_id TEXT NOT NULL UNIQUE,prize_coins INTEGER NOT NULL,created_at INTEGER NOT NULL,status TEXT NOT NULL DEFAULT 'pending',settled_at INTEGER);
 CREATE TABLE IF NOT EXISTS campaign_audit(id TEXT PRIMARY KEY,user_id INTEGER,action TEXT NOT NULL,related_id TEXT NOT NULL,created_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO campaign_config VALUES(1,?1,?2)",
        params![serde_json::to_string(&Config::default())?, now()],
    )?;
    Ok(())
}
fn config(db: &Connection) -> Result<Config, ApiError> {
    Ok(serde_json::from_str(&db.query_row::<String, _, _>(
        "SELECT config_json FROM campaign_config WHERE id=1",
        [],
        |r| r.get(0),
    )?)?)
}
fn ensure_opportunity(
    tx: &Connection,
    uid: i64,
    source: &str,
    source_ref: &str,
    day_key: &str,
    expires: i64,
) -> Result<(), ApiError> {
    tx.execute("INSERT OR IGNORE INTO campaign_opportunities(id,user_id,source,source_ref,day,created_at,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![Uuid::new_v4().to_string(),uid,source,source_ref,day_key,now(),expires])?;
    Ok(())
}
fn sync(tx: &Connection, uid: i64, c: &Config) -> Result<(), ApiError> {
    let d = day();
    let expiry = today_start() + 86400;
    if c.login_opportunities > 0 {
        ensure_opportunity(tx, uid, "login", &format!("login:{d}:{uid}"), &d, expiry)?;
    }
    let invites: i64 = tx.query_row(
        "SELECT COUNT(*) FROM users WHERE inviter_id=?1 AND invitation_bound_at>=?2",
        params![uid, today_start()],
        |r| r.get(0),
    )?;
    let existing:i64=tx.query_row("SELECT COUNT(*) FROM campaign_opportunities WHERE user_id=?1 AND source='invite' AND day=?2",params![uid,d],|r|r.get(0))?;
    for i in existing..invites * c.invite_opportunities_per_invitee {
        ensure_opportunity(tx, uid, "invite", &format!("invite:{d}:{i}"), &d, expiry)?;
    }
    let mut stmt=tx.prepare("SELECT id FROM reward_sessions WHERE user_id=?1 AND completed_at>=?2 ORDER BY completed_at")?;
    let ids = stmt
        .query_map(params![uid, today_start()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids.into_iter().take(c.video_opportunities_per_day as usize) {
        ensure_opportunity(tx, uid, "video", &id, &d, expiry)?;
    }
    Ok(())
}
fn opportunity_json(row: &rusqlite::Row<'_>) -> Result<Value, rusqlite::Error> {
    Ok(
        json!({"id":row.get::<_,String>(0)?,"source":row.get::<_,String>(1)?,"source_ref":row.get::<_,String>(2)?,"created_at":iso(row.get::<_,i64>(3)?),"expires_at":iso(row.get::<_,i64>(4)?),"used":row.get::<_,Option<i64>>(5)?.is_some()}),
    )
}
async fn runtime(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    sync(&tx, uid, &c)?;
    let items = {
        let mut stmt=tx.prepare("SELECT id,source,source_ref,created_at,expires_at,used_at FROM campaign_opportunities WHERE user_id=?1 AND day=?2 ORDER BY created_at")?;
        stmt.query_map(params![uid, day()], opportunity_json)?
            .collect::<Result<Vec<_>, _>>()?
    };
    let pending=tx.prepare("SELECT id,prize_coins,created_at,status,settled_at FROM campaign_spins WHERE user_id=?1 ORDER BY created_at DESC LIMIT 50")?.query_map([uid],|r|Ok(json!({"id":r.get::<_,String>(0)?,"prize_coins":r.get::<_,i64>(1)?,"created_at":iso(r.get(2)?),"status":r.get::<_,String>(3)?,"settled_at":r.get::<_,Option<i64>>(4)?.map(iso)})))?.collect::<Result<Vec<_>,_>>()?;
    let remaining = items
        .iter()
        .filter(|item| !item["used"].as_bool().unwrap_or(true))
        .count();
    tx.commit()?;
    Ok(Json(
        json!({"enabled":c.enabled,"title":c.title,"subtitle":c.subtitle,"copy":c.subtitle,"entry_label":c.entry_label,"hero_image":c.hero_image,"starts_at":c.starts_at,"expires_at":c.expires_at,"daily_spin_limit":c.daily_spin_limit,"daily_limit":c.daily_spin_limit,"remaining_spins":remaining,"arrival_threshold":c.arrival_threshold_coins,"arrival_threshold_coins":c.arrival_threshold_coins,"settle_delay_seconds":c.settle_delay_seconds,"reward_tiers":c.prizes,"opportunities":items,"pending_rewards":pending,"revision":c.revision,"can_spin":c.enabled&&remaining>0 ,"spin_path":"/api/campaign/spin"}),
    ))
}
#[derive(Deserialize)]
struct SpinInput {
    opportunity_id: String,
}
async fn spin(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<SpinInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("活动暂未开启"));
    }
    sync(&tx, uid, &c)?;
    let row:Option<(String,i64,Option<i64>)>=tx.query_row("SELECT source,expires_at,used_at FROM campaign_opportunities WHERE id=?1 AND user_id=?2",params![input.opportunity_id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let Some((source, expires, used_at)) = row else {
        return Err(invalid("抽奖机会不存在"));
    };
    let source_enabled = match source.as_str() {
        "login" => c.login_opportunities > 0,
        "video" => c.video_opportunities_per_day > 0,
        "invite" => c.invite_opportunities_per_invitee > 0,
        _ => false,
    };
    if !source_enabled {
        return Err(conflict("该机会来源已暂停"));
    }
    if used_at.is_some() {
        return Err(conflict("抽奖机会已使用"));
    }
    if expires <= now() {
        return Err(conflict("抽奖机会已过期"));
    }
    let index = (Uuid::new_v4().as_u128() % c.prizes.len() as u128) as usize;
    let prize = c.prizes[index];
    let sid = Uuid::new_v4().to_string();
    tx.execute(
        "UPDATE campaign_opportunities SET used_at=?1 WHERE id=?2 AND used_at IS NULL",
        params![now(), input.opportunity_id],
    )?;
    tx.execute("INSERT INTO campaign_spins(id,user_id,opportunity_id,prize_coins,created_at) VALUES(?1,?2,?3,?4,?5)",params![sid,uid,input.opportunity_id,prize,now()])?;
    tx.execute(
        "INSERT INTO campaign_audit VALUES(?1,?2,'spin',?3,?4)",
        params![Uuid::new_v4().to_string(), uid, sid, now()],
    )?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "campaign_spin",
        "success",
        &sid,
        None,
        json!({"source":source,"prize_coins":prize}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"id":sid,"status":"pending","prize_coins":prize,"settle_after":iso(now()+c.settle_delay_seconds),"message":"奖励待结算，请稍后查看"}),
    ))
}
async fn settle(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    let row:Option<(i64,i64,String,Option<i64>)>=tx.query_row("SELECT prize_coins,created_at,status,settled_at FROM campaign_spins WHERE id=?1 AND user_id=?2",params![id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((coins, created, status, settled)) = row else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "活动奖励不存在".into(),
            None,
        ));
    };
    if status == "settled" {
        return Ok(Json(
            json!({"id":id,"status":"settled","prize_coins":coins,"settled_at":settled.map(iso),"user":user(&tx,uid)?}),
        ));
    }
    let c = config(&tx)?;
    if now() - created < c.settle_delay_seconds {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "奖励仍在结算门槛内，请稍后再试".into(),
            Some("REWARD_PENDING"),
        ));
    }
    let balance: i64 = tx.query_row("SELECT coins FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    if balance < c.arrival_threshold_coins {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "奖励待达标，满足活动门槛后可领取".into(),
            Some("THRESHOLD_NOT_MET"),
        ));
    }
    let source = format!("campaign:{id}");
    super::credit(&tx, uid, coins, "活动奖励", &source)?;
    tx.execute(
        "UPDATE campaign_spins SET status='settled',settled_at=?1 WHERE id=?2 AND status='pending'",
        params![now(), id],
    )?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "campaign_settle",
        "success",
        &id,
        Some(coins as f64 / 1000.0),
        json!({"source":source}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"id":id,"status":"settled","prize_coins":coins,"settled_at":iso(now()),"user":user(&db,uid)?}),
    ))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/campaign", get(runtime))
        .route("/campaign/spin", post(spin))
        .route("/campaign/rewards/{id}/settle", post(settle))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn spin_is_single_use_and_pending_until_threshold() {
        let (state, context, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        db.execute("INSERT INTO campaign_config(id,config_json,updated_at) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET config_json=excluded.config_json",params![serde_json::to_string(&Config{settle_delay_seconds:3600,..Config::default()}).unwrap(),now()]).unwrap();
        let runtime = runtime(State(state.clone()), headers.clone())
            .await
            .unwrap()
            .0;
        let opp = runtime["opportunities"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let result = spin(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(SpinInput {
                opportunity_id: opp.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["status"], "pending");
        assert!(
            spin(
                State(state),
                Extension(context),
                headers,
                Json(SpinInput {
                    opportunity_id: opp
                })
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn coin_arrival_threshold_does_not_credit_balance_early() {
        let (state, context, headers, _at) = rain::tests::fixture(false, false);
        let uid = 2;
        let db = open(&state).unwrap();
        let cfg = Config {
            settle_delay_seconds: 0,
            arrival_threshold_coins: 999999,
            ..Config::default()
        };
        db.execute(
            "UPDATE campaign_config SET config_json=?1",
            [serde_json::to_string(&cfg).unwrap()],
        )
        .unwrap();
        let runtime = runtime(State(state.clone()), headers.clone())
            .await
            .unwrap()
            .0;
        let opp = runtime["opportunities"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let spin_result = spin(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(SpinInput {
                opportunity_id: opp,
            }),
        )
        .await
        .unwrap()
        .0;
        let before: i64 = db
            .query_row("SELECT coins FROM users WHERE id=?1", [uid], |r| r.get(0))
            .unwrap();
        let pending = settle(
            State(state.clone()),
            Extension(context),
            headers,
            Path(spin_result["id"].as_str().unwrap().to_owned()),
        )
        .await
        .unwrap_err();
        assert_eq!(pending.2, Some("THRESHOLD_NOT_MET"));
        let after: i64 = db
            .query_row("SELECT coins FROM users WHERE id=?1", [uid], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
    }
}
