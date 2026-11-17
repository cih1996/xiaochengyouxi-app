use super::*;
use axum::extract::{ConnectInfo, Extension};
use std::net::{IpAddr, SocketAddr};

#[derive(Clone)]
pub(crate) struct RequestContext {
    pub request_id: String,
    pub client_ip: String,
    pub admin_username: Option<String>,
}

pub(crate) fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    trusted_ip(headers, peer, env_value("ORANGE_PROXY_TOKEN").as_deref())
}
fn trusted_ip(headers: &HeaderMap, peer: SocketAddr, secret: Option<&str>) -> String {
    let authorized = peer.ip().is_loopback()
        && secret.is_some_and(|expected| {
            headers
                .get("x-orange-proxy-token")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|actual| equal_secret(expected, actual))
        });
    if authorized {
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<IpAddr>().ok())
        {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
}
pub(super) async fn request_context(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let context = RequestContext {
        request_id: Uuid::new_v4().to_string(),
        client_ip: client_ip(request.headers(), peer),
        admin_username: None,
    };
    let path = request.uri().path().to_owned();
    let bearer = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned);
    request.extensions_mut().insert(context.clone());
    let response = next.run(request).await;
    if (response.status().is_client_error() || response.status().is_server_error())
        && path.starts_with("/api/")
        && !path.starts_with("/api/admin/")
        && path != "/api/auth/login"
    {
        let db = open(&state)?;
        let uid = if let Some(token) = bearer {
            db.query_row(
                "SELECT user_id FROM sessions WHERE token=?1",
                [token],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
        } else {
            None
        };
        let kind = if path.contains("/rewards/") {
            "reward_request"
        } else if path == "/api/check-in" {
            "check_in"
        } else if path == "/api/withdrawals" {
            "withdrawal"
        } else {
            "request"
        };
        behavior(
            &db,
            &context,
            uid,
            None,
            kind,
            "failed",
            "",
            None,
            json!({"path":path,"http_status":response.status().as_u16()}),
        )?;
    }
    Ok(response)
}
pub(super) fn initialize(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS investigation_meta(key TEXT PRIMARY KEY,value INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS user_behavior(id TEXT PRIMARY KEY,user_id INTEGER,subject_hash TEXT,kind TEXT NOT NULL,status TEXT NOT NULL,related_id TEXT NOT NULL DEFAULT '',amount_yuan REAL,client_ip TEXT NOT NULL,request_id TEXT NOT NULL,detail_json TEXT NOT NULL,created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_behavior_user_time ON user_behavior(user_id,created_at);
        CREATE INDEX IF NOT EXISTS idx_behavior_subject ON user_behavior(subject_hash,created_at);
        CREATE TABLE IF NOT EXISTS callback_events(id TEXT PRIMARY KEY,user_id INTEGER,reward_session_id TEXT NOT NULL,trans_id TEXT NOT NULL,status TEXT NOT NULL,reason_code TEXT NOT NULL,client_ip TEXT NOT NULL,request_id TEXT NOT NULL,created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_callback_user_time ON callback_events(user_id,created_at);
        CREATE TABLE IF NOT EXISTS admin_audit(id TEXT PRIMARY KEY,user_id INTEGER,actor TEXT NOT NULL,action TEXT NOT NULL,related_id TEXT NOT NULL DEFAULT '',client_ip TEXT NOT NULL,request_id TEXT NOT NULL,reason TEXT NOT NULL DEFAULT '',filters_json TEXT NOT NULL DEFAULT '{}',before_json TEXT NOT NULL DEFAULT '{}',after_json TEXT NOT NULL DEFAULT '{}',http_status INTEGER,created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_admin_audit_user_time ON admin_audit(user_id,created_at);
        CREATE TABLE IF NOT EXISTS account_restrictions(user_id INTEGER PRIMARY KEY,status TEXT NOT NULL DEFAULT 'active',reason TEXT NOT NULL DEFAULT '',updated_at INTEGER NOT NULL,updated_by TEXT NOT NULL,trace_id TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS account_devices(user_id INTEGER NOT NULL,device_id TEXT NOT NULL,platform TEXT NOT NULL,model TEXT NOT NULL,os_version TEXT NOT NULL,key_id TEXT NOT NULL,report_id TEXT NOT NULL,evidence_status TEXT NOT NULL,first_seen INTEGER NOT NULL,last_seen INTEGER NOT NULL,PRIMARY KEY(user_id,device_id));
        CREATE INDEX IF NOT EXISTS idx_devices_id ON account_devices(device_id);")?;
    db.execute(
        "INSERT OR IGNORE INTO investigation_meta(key,value) VALUES('collection_since',?1)",
        [now()],
    )?;
    Ok(())
}
pub(crate) struct DeviceObservation {
    pub device_id: String,
    pub platform: String,
    pub model: String,
    pub os_version: String,
    pub key_id: String,
    pub report_id: String,
    pub evidence_status: String,
}
pub(crate) fn record_device(
    db: &Connection,
    uid: i64,
    device: &DeviceObservation,
) -> Result<(), ApiError> {
    db.execute("INSERT INTO account_devices(user_id,device_id,platform,model,os_version,key_id,report_id,evidence_status,first_seen,last_seen) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)
        ON CONFLICT(user_id,device_id) DO UPDATE SET platform=excluded.platform,model=excluded.model,os_version=excluded.os_version,key_id=excluded.key_id,report_id=excluded.report_id,evidence_status=excluded.evidence_status,last_seen=excluded.last_seen",
        params![uid,device.device_id,device.platform,device.model,device.os_version,device.key_id,device.report_id,device.evidence_status,now()])?;
    Ok(())
}
pub(super) fn subject_hash(phone: &str) -> String {
    format!("{:x}", Sha256::digest(phone.as_bytes()))
}
pub(super) fn retry_reference(
    state: &AppState,
    db: &Connection,
    headers: &HeaderMap,
    uid: i64,
    kind: &str,
) -> Result<Option<String>, ApiError> {
    risk::check_account(db, uid)?;
    if state.config.device_trust.is_none() {
        return Ok(None);
    }
    let Some(id) = headers
        .get("x-risk-event")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };
    Ok(db.query_row("SELECT operation_ref FROM risk_events WHERE id=?1 AND user_id=?2 AND kind=?3 AND status='reported' AND key_id!='' AND operation_ref IS NOT NULL",params![id,uid,kind],|r|r.get(0)).optional()?)
}
pub(super) fn behavior(
    db: &Connection,
    context: &RequestContext,
    uid: Option<i64>,
    phone: Option<&str>,
    kind: &str,
    status: &str,
    related: &str,
    amount: Option<f64>,
    detail: Value,
) -> Result<(), ApiError> {
    db.execute("INSERT INTO user_behavior(id,user_id,subject_hash,kind,status,related_id,amount_yuan,client_ip,request_id,detail_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![Uuid::new_v4().to_string(),uid,phone.map(subject_hash),kind,status,related,amount,context.client_ip,context.request_id,detail.to_string(),now()])?;
    Ok(())
}
pub(super) fn action_audit(
    db: &Connection,
    context: &RequestContext,
    uid: Option<i64>,
    action: &str,
    related: &str,
    reason: &str,
    before: Value,
    after: Value,
) -> Result<String, ApiError> {
    let id = Uuid::new_v4().to_string();
    db.execute("INSERT INTO admin_audit(id,user_id,actor,action,related_id,client_ip,request_id,reason,before_json,after_json,http_status,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,200,?11)",
        params![id,uid,context.admin_username.as_deref().unwrap_or("local-preview"),action,related,context.client_ip,context.request_id,reason,before.to_string(),after.to_string(),now()])?;
    Ok(id)
}
pub(super) fn access_audit(
    db: &Connection,
    context: &RequestContext,
    method: &str,
    uri: &axum::http::Uri,
) -> Result<String, ApiError> {
    let mut filters = json!({"method":method,"path":uri.path()});
    if let Ok(Query(query)) = Query::<HashMap<String, String>>::try_from_uri(uri) {
        for (key, value) in query {
            if [
                "q",
                "page",
                "page_size",
                "from",
                "to",
                "kind",
                "status",
                "min_amount",
                "max_amount",
                "related",
                "section",
                "invitation_status",
                "account_status",
            ]
            .contains(&key.as_str())
            {
                filters[&key] = json!(value.chars().take(200).collect::<String>());
            }
        }
    }
    let parts: Vec<&str> = uri.path().split('/').collect();
    let uid = parts
        .iter()
        .position(|v| *v == "users")
        .and_then(|index| parts.get(index + 1))
        .and_then(|id| id.parse::<i64>().ok());
    let id = Uuid::new_v4().to_string();
    db.execute("INSERT INTO admin_audit(id,user_id,actor,action,client_ip,request_id,filters_json,created_at) VALUES(?1,?2,?3,'admin_access',?4,?5,?6,?7)",params![id,uid,context.admin_username.as_deref().unwrap_or("local-preview"),context.client_ip,context.request_id,filters.to_string(),now()])?;
    Ok(id)
}
pub(super) fn finish_access(db: &Connection, id: &str, status: u16) -> Result<(), ApiError> {
    db.execute(
        "UPDATE admin_audit SET http_status=?1 WHERE id=?2",
        params![status, id],
    )?;
    Ok(())
}
pub(super) fn callback_audit(
    db: &Connection,
    context: &RequestContext,
    query: &HashMap<String, String>,
    valid: bool,
    reason: &str,
) -> Result<(), ApiError> {
    let sid = query
        .get("extra")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| value["reward_session_id"].as_str().map(str::to_owned))
        .filter(|value| Uuid::parse_str(value).is_ok())
        .unwrap_or_default();
    let uid: Option<i64> = db
        .query_row(
            "SELECT user_id FROM reward_sessions WHERE id=?1",
            [&sid],
            |r| r.get(0),
        )
        .optional()?;
    let trans = query
        .get("trans_id")
        .map(|v| v.chars().take(160).collect::<String>())
        .unwrap_or_default();
    db.execute("INSERT INTO callback_events(id,user_id,reward_session_id,trans_id,status,reason_code,client_ip,request_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![Uuid::new_v4().to_string(),uid,sid,trans,if valid{"accepted"}else{"rejected"},reason,context.client_ip,context.request_id,now()])?;
    Ok(())
}

#[derive(Deserialize, Default)]
pub(super) struct Filters {
    pub q: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub related: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub section: Option<String>,
    pub invitation_status: Option<String>,
    pub account_status: Option<String>,
}
fn timestamp(input: &str, end: bool) -> Result<i64, ApiError> {
    if let Ok(value) = input.parse::<i64>() {
        return Ok(value);
    }
    if let Ok(value) = chrono::DateTime::parse_from_rfc3339(input) {
        return Ok(value.timestamp());
    }
    if let Ok(value) = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Ok(
            value.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() - 8 * 3600
                + if end { 86399 } else { 0 },
        );
    }
    Err(invalid("日期应为 YYYY-MM-DD 或带时区的 ISO 时间"))
}
fn pagination(f: &Filters) -> Result<(i64, i64), ApiError> {
    let page = f.page.unwrap_or(1);
    let size = f.page_size.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&size) {
        return Err(invalid("页码应大于 0，每页 1 至 100 条"));
    }
    Ok((page, size))
}
fn amount_filters(f: &Filters) -> Result<(), ApiError> {
    if [f.min_amount, f.max_amount]
        .into_iter()
        .flatten()
        .any(|v| !v.is_finite() || v < 0.0)
    {
        return Err(invalid("金额范围应为非负数字"));
    }
    if f.min_amount
        .zip(f.max_amount)
        .is_some_and(|(min, max)| min > max)
    {
        return Err(invalid("最低金额不能大于最高金额"));
    }
    Ok(())
}
fn availability(state: &AppState, db: &Connection) -> Result<Value, ApiError> {
    let since: i64 = db.query_row(
        "SELECT value FROM investigation_meta WHERE key='collection_since'",
        [],
        |r| r.get(0),
    )?;
    Ok(
        json!({"behavior_since":iso(since),"callbacks_since":iso(since),"audit_since":iso(since),"device_trust_configured":state.config.device_trust.is_some(),"notes":["交易及邀请按现有业务记录回溯。","登录、请求IP、失败回调与管理员操作从采集启用后记录；此前明细未采集。","历史已验证回调只证明业务验证状态，不代表保留了当时完整请求。"]}),
    )
}
fn restriction(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    Ok(db.query_row("SELECT status,reason,updated_at,updated_by,trace_id FROM account_restrictions WHERE user_id=?1",[uid],|r|Ok(json!({"user_id":uid,"status":r.get::<_,String>(0)?,"reason":r.get::<_,String>(1)?,"updated_at":iso(r.get(2)?),"updated_by":r.get::<_,String>(3)?,"trace_id":r.get::<_,String>(4)?}))).optional()?.unwrap_or_else(||json!({"user_id":uid,"status":"active","reason":"","updated_at":Value::Null,"updated_by":"","trace_id":""})))
}
fn ensure_user(db: &Connection, uid: i64) -> Result<(), ApiError> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1)",
        [uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(ApiError(StatusCode::NOT_FOUND, "用户不存在".into(), None));
    }
    Ok(())
}
pub(super) async fn detail(
    State(state): State<AppState>,
    Path(uid): Path<i64>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    ensure_user(&db, uid)?;
    let mut profile = serde_json::to_value(user(&db, uid)?)?;
    let created: i64 = db.query_row("SELECT created_at FROM users WHERE id=?1", [uid], |r| {
        r.get(0)
    })?;
    profile["created_at"] = if created > 0 {
        json!(iso(created))
    } else {
        Value::Null
    };
    let last:Option<i64>=db.query_row("SELECT MAX(created_at) FROM user_behavior WHERE user_id=?1 AND kind='login' AND status='success'",[uid],|r|r.get(0))?;
    profile["last_login_at"] = json!(last.map(iso));
    let count =
        |sql: &str| -> Result<i64, ApiError> { Ok(db.query_row(sql, [uid], |r| r.get(0))?) };
    let summary = json!({"ledger_count":count("SELECT COUNT(*) FROM ledger WHERE user_id=?1")?,"ad_sessions":count("SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1")?,"ad_verified":count("SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND verified_at IS NOT NULL")?,"ad_completed":count("SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND completed_at IS NOT NULL")?,"withdrawal_count":count("SELECT COUNT(*) FROM withdrawals WHERE user_id=?1")?,"withdrawal_total_yuan":db.query_row("SELECT COALESCE(SUM(amount_yuan),0) FROM withdrawals WHERE user_id=?1",[uid],|r|r.get::<_,f64>(0))?,"withdrawal_pending_yuan":db.query_row("SELECT COALESCE(SUM(amount_yuan),0) FROM withdrawals WHERE user_id=?1 AND status='模拟申请'",[uid],|r|r.get::<_,f64>(0))?,"invite_registered":count("SELECT COUNT(*) FROM users WHERE inviter_id=?1 AND invitation_bound_at IS NOT NULL")?,"invite_pending":count("SELECT COUNT(*) FROM invitation_prebindings WHERE inviter_id=?1 AND consumed_at IS NULL")?,"device_count":count("SELECT COUNT(*) FROM account_devices WHERE user_id=?1")?,"risk_count":count("SELECT COUNT(*) FROM risk_events WHERE user_id=?1")?});
    let inviter=db.query_row("SELECT p.id,p.phone,p.nickname,u.inviter_code,u.invitation_bound_at FROM users u LEFT JOIN users p ON p.id=u.inviter_id WHERE u.id=?1 AND u.invitation_bound_at IS NOT NULL",[uid],|r|Ok(json!({"id":r.get::<_,Option<i64>>(0)?,"phone":r.get::<_,Option<String>>(1)?,"nickname":r.get::<_,Option<String>>(2)?,"invite_code":r.get::<_,String>(3)?,"bound_at":iso(r.get(4)?)}))).optional()?;
    Ok(Json(
        json!({"user":profile,"summary":summary,"inviter":inviter,"restriction":restriction(&db,uid)?,"availability":availability(&state,&db)?}),
    ))
}
fn parameter(
    values: &mut Vec<rusqlite::types::Value>,
    value: impl Into<rusqlite::types::Value>,
) -> String {
    values.push(value.into());
    format!("?{}", values.len())
}
pub(super) async fn search(
    State(state): State<AppState>,
    Query(f): Query<Filters>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let (page, size) = pagination(&f)?;
    amount_filters(&f)?;
    let mut values = Vec::new();
    let mut clauses = vec!["1=1".to_owned()];
    if let Some(q) = f.q.filter(|q| !q.trim().is_empty()) {
        let p = parameter(&mut values, format!("%{}%", q.trim()));
        clauses.push(format!("(CAST(u.id AS TEXT) LIKE {p} OR u.phone LIKE {p} OR u.nickname LIKE {p} OR u.invite_code LIKE {p} OR u.inviter_code LIKE {p})"));
    }
    if let Some(status) = f.invitation_status {
        match status.as_str() {
            "bound" => clauses.push("u.invitation_bound_at IS NOT NULL".into()),
            "unbound" => clauses.push("u.invitation_bound_at IS NULL".into()),
            "" => {}
            _ => return Err(invalid("邀请状态应为 bound 或 unbound")),
        }
    }
    if let Some(status) = f.account_status.filter(|v| !v.is_empty()) {
        let p = parameter(&mut values, status);
        clauses.push(format!("COALESCE(a.status,'active')={p}"));
    }
    for (value, operator) in [(f.from.as_deref(), ">="), (f.to.as_deref(), "<=")] {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            let p = parameter(&mut values, timestamp(value, operator == "<=")?);
            clauses.push(format!("u.created_at{operator}{p}"));
        }
    }
    for (amount, operator) in [(f.min_amount, ">="), (f.max_amount, "<=")] {
        if let Some(amount) = amount {
            let p = parameter(&mut values, amount);
            clauses.push(format!("u.coins/1000.0{operator}{p}"));
        }
    }
    let filter = clauses.join(" AND ");
    let source =
        format!("users u LEFT JOIN account_restrictions a ON a.user_id=u.id WHERE {filter}");
    let total: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM {source}"),
        rusqlite::params_from_iter(values.iter()),
        |r| r.get(0),
    )?;
    let limit = parameter(&mut values, size);
    let offset = parameter(&mut values, (page - 1) * size);
    let mut stmt=db.prepare(&format!("SELECT u.id,u.phone,u.nickname,u.coins,u.created_at,u.invite_code,u.inviter_code,u.invitation_bound_at,COALESCE(a.status,'active'),(SELECT MAX(created_at) FROM user_behavior WHERE user_id=u.id AND kind='login' AND status='success') FROM {source} ORDER BY u.created_at DESC,u.id DESC LIMIT {limit} OFFSET {offset}"))?;
    let mut items=stmt.query_map(rusqlite::params_from_iter(values.iter()),|r|Ok(json!({"id":r.get::<_,i64>(0)?,"phone":r.get::<_,String>(1)?,"nickname":r.get::<_,String>(2)?,"coins":r.get::<_,i64>(3)?,"cash_yuan":r.get::<_,i64>(3)? as f64/1000.0,"created_at":Some(r.get::<_,i64>(4)?).filter(|v|*v>0).map(iso),"invite_code":r.get::<_,String>(5)?,"inviter_code":r.get::<_,Option<String>>(6)?,"invitation_required":r.get::<_,Option<i64>>(7)?.is_none(),"account_status":r.get::<_,String>(8)?,"last_login_at":r.get::<_,Option<i64>>(9)?.map(iso)})))?.collect::<Result<Vec<_>,_>>()?;
    for row in &mut items {
        let uid = row["id"].as_i64().unwrap();
        row["risk_count"] = json!(db.query_row(
            "SELECT COUNT(*) FROM risk_events WHERE user_id=?1",
            [uid],
            |r| r.get::<_, i64>(0)
        )?);
        let latest:Option<(i64,String,Option<i64>)>=db.query_row("SELECT created_at,decision,score FROM risk_events WHERE user_id=?1 ORDER BY created_at DESC,id DESC LIMIT 1",[uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        row["latest_risk_at"] = json!(latest.as_ref().map(|v| iso(v.0)));
        row["last_risk_decision"] = json!(latest.as_ref().map(|v| &v.1));
        row["last_risk_score"] = json!(latest.and_then(|v| v.2));
    }
    Ok(Json(
        json!({"items":items,"total":total,"page":page,"page_size":size}),
    ))
}
fn base(section: &str) -> Result<String, ApiError> {
    let ledger = "SELECT CAST(id AS TEXT) id,user_id,CASE WHEN source LIKE 'rain:%' THEN 'rain_reward' WHEN source LIKE 'reward:%' THEN 'reward' WHEN source LIKE 'check-in:%' THEN 'check_in' WHEN source LIKE 'withdraw-refund:%' THEN 'withdrawal_refund' WHEN source LIKE 'withdraw:%' THEN 'withdrawal' ELSE 'other' END kind,CASE WHEN coins>=0 THEN 'credited' ELSE 'debited' END status,created_at,coins/1000.0 amount_yuan,source related_id,json_object('title',title,'coins',coins,'source',source) extra FROM ledger WHERE user_id=?1";
    let ads = "SELECT id,user_id,'rewarded_video' kind,CASE WHEN completed_at IS NOT NULL THEN 'completed' WHEN verified_at IS NOT NULL THEN 'verified' ELSE 'started' END status,started_at created_at,reward/1000.0 amount_yuan,id related_id,json_object('task_id',task_id,'provider',provider,'reward',reward,'started_at',strftime('%Y-%m-%dT%H:%M:%SZ',started_at,'unixepoch'),'verified_at',strftime('%Y-%m-%dT%H:%M:%SZ',verified_at,'unixepoch'),'completed_at',strftime('%Y-%m-%dT%H:%M:%SZ',completed_at,'unixepoch'),'csj_trans_id',csj_trans_id,'quote_id',quote_id,'reward_tier',json_extract(reward_basis_json,'$.tier'),'policy_version',json_extract(reward_basis_json,'$.policy_version'),'device_first_seen',json_extract(reward_basis_json,'$.first_seen'),'device_age_days',json_extract(reward_basis_json,'$.age_days'),'device_completed_before',json_extract(reward_basis_json,'$.completed_before'),'reward_min',json_extract(reward_basis_json,'$.min_coins'),'reward_max',json_extract(reward_basis_json,'$.max_coins'),'reward_basis',json(reward_basis_json)) extra FROM reward_sessions WHERE user_id=?1";
    let callbacks = "SELECT id,user_id,'csj_callback' kind,status,created_at,NULL amount_yuan,reward_session_id related_id,json_object('reward_session_id',reward_session_id,'trans_id',trans_id,'reason_code',reason_code,'client_ip',client_ip,'request_id',request_id,'source','captured') extra FROM callback_events WHERE user_id=?1 UNION ALL SELECT 'verified:'||id,user_id,'csj_callback','accepted',verified_at,NULL,id,json_object('reward_session_id',id,'trans_id',csj_trans_id,'reason_code','verified_state','client_ip',NULL,'source','business') FROM reward_sessions r WHERE user_id=?1 AND verified_at IS NOT NULL AND NOT EXISTS(SELECT 1 FROM callback_events c WHERE c.reward_session_id=r.id AND c.status='accepted')";
    let withdrawals = "SELECT w.id,w.user_id,'withdrawal' kind,w.status,w.created_at,w.amount_yuan,w.id related_id,json_object('channel',w.channel,'account',w.account,'coins',w.coins,'reviewed_at',(SELECT strftime('%Y-%m-%dT%H:%M:%SZ',created_at,'unixepoch') FROM admin_audit a WHERE a.action='admin_review' AND a.related_id=w.id ORDER BY created_at DESC LIMIT 1),'reviewed_by',(SELECT actor FROM admin_audit a WHERE a.action='admin_review' AND a.related_id=w.id ORDER BY created_at DESC LIMIT 1)) extra FROM withdrawals w WHERE w.user_id=?1";
    let audit = "SELECT id,user_id,action kind,CASE WHEN http_status IS NULL THEN 'requested' WHEN http_status<400 THEN 'success' ELSE 'failed' END status,created_at,NULL amount_yuan,related_id,json_object('actor',actor,'action',action,'client_ip',client_ip,'request_id',request_id,'reason',reason,'filters',json(filters_json),'before',json(before_json),'after',json(after_json),'http_status',http_status) extra FROM admin_audit WHERE user_id=?1";
    let risk = "SELECT id,user_id,kind,status,created_at,NULL amount_yuan,COALESCE(operation_ref,id) related_id,json_object('decision',decision,'action',action,'score',score,'key_id',key_id,'report_id',report_id,'client_ip',client_ip,'context',json(context_json),'detail',json(detail_json),'error',error,'updated_at',strftime('%Y-%m-%dT%H:%M:%SZ',updated_at,'unixepoch'),'request_hash',request_hash,'operation_id',planned_operation,'receipt_status',(SELECT delivery_status FROM risk_receipts rr WHERE rr.event_id=risk_events.id),'receipt_error',(SELECT error FROM risk_receipts rr WHERE rr.event_id=risk_events.id),'receipt_updated_at',(SELECT strftime('%Y-%m-%dT%H:%M:%SZ',updated_at,'unixepoch') FROM risk_receipts rr WHERE rr.event_id=risk_events.id)) extra FROM risk_events WHERE user_id=?1";
    let devices = "SELECT device_id id,user_id,'device' kind,evidence_status status,last_seen created_at,NULL amount_yuan,device_id related_id,json_object('device_id',device_id,'platform',platform,'model',model,'os_version',os_version,'first_seen',strftime('%Y-%m-%dT%H:%M:%SZ',first_seen,'unixepoch'),'last_seen',strftime('%Y-%m-%dT%H:%M:%SZ',last_seen,'unixepoch'),'key_id',key_id,'report_id',report_id,'evidence_status',evidence_status,'account_count',(SELECT COUNT(*) FROM account_devices d WHERE d.device_id=a.device_id)) extra FROM account_devices a WHERE user_id=?1";
    let invitations = "SELECT 'upstream:'||u.id id,u.id user_id,'invitation' kind,'bound' status,u.invitation_bound_at created_at,NULL amount_yuan,u.inviter_code related_id,json_object('direction','upstream','related_user_id',p.id,'phone',p.phone,'nickname',p.nickname,'invite_code',u.inviter_code,'bound_at',strftime('%Y-%m-%dT%H:%M:%SZ',u.invitation_bound_at,'unixepoch')) extra FROM users u LEFT JOIN users p ON p.id=u.inviter_id WHERE u.id=?1 AND u.invitation_bound_at IS NOT NULL UNION ALL SELECT 'downstream:'||id,?1,'invitation','bound',invitation_bound_at,NULL,invite_code,json_object('direction','downstream','related_user_id',id,'phone',phone,'nickname',nickname,'invite_code',invite_code,'bound_at',strftime('%Y-%m-%dT%H:%M:%SZ',invitation_bound_at,'unixepoch')) FROM users WHERE inviter_id=?1 AND invitation_bound_at IS NOT NULL UNION ALL SELECT 'pending:'||phone,?1,'invitation','pending',created_at,NULL,invite_code,json_object('direction','pending','related_user_id',(SELECT id FROM users WHERE users.phone=p.phone),'phone',phone,'nickname',NULL,'invite_code',invite_code,'bound_at',NULL) FROM invitation_prebindings p WHERE inviter_id=?1 AND consumed_at IS NULL";
    match section {
        "ledger" => Ok(ledger.into()),
        "ads" => Ok(ads.into()),
        "callbacks" => Ok(callbacks.into()),
        "withdrawals" => Ok(withdrawals.into()),
        "audit" => Ok(audit.into()),
        "risk" => Ok(risk.into()),
        "devices" => Ok(devices.into()),
        "invitations" => Ok(invitations.into()),
        "timeline" => Ok(format!(
            "SELECT id,user_id,kind,status,created_at,amount_yuan,related_id,json_set(json(extra),'$.source','business') extra FROM ({ledger}) UNION ALL SELECT id,user_id,'reward_start',status,created_at,NULL,related_id,json_set(json(extra),'$.title','广告会话创建','$.source','business') FROM ({ads}) UNION ALL SELECT id,user_id,kind,status,created_at,NULL,related_id,json_set(json(extra),'$.title','激励回调','$.source',json_extract(extra,'$.source')) FROM ({callbacks}) UNION ALL SELECT id,user_id,kind,status,created_at,amount_yuan,related_id,json_set(json(extra),'$.title','提现申请','$.source','business') FROM ({withdrawals}) UNION ALL SELECT id,user_id,'invitation',status,created_at,NULL,related_id,json_set(json(extra),'$.title','邀请关系','$.source','business') FROM ({invitations}) UNION ALL SELECT id,user_id,action,CASE WHEN http_status IS NULL THEN 'requested' WHEN http_status<400 THEN 'success' ELSE 'failed' END,created_at,NULL,related_id,json_object('title',CASE action WHEN 'admin_review' THEN '提现审核' ELSE '账号状态变更' END,'client_ip',client_ip,'request_id',request_id,'source','captured','detail',json(after_json)) FROM admin_audit WHERE user_id=?1 AND action!='admin_access' UNION ALL SELECT id,user_id,kind,status,created_at,amount_yuan,related_id,json_object('title',kind,'client_ip',client_ip,'request_id',request_id,'source','captured','detail',json(detail_json)) FROM user_behavior WHERE user_id=?1 OR subject_hash=?2 UNION ALL SELECT id,user_id,'risk_assessment',status,updated_at,NULL,COALESCE(operation_ref,id),json_object('title','设备风险评估','client_ip',client_ip,'source','device_trust','detail',json(detail_json),'sdk_kind',kind,'decision',decision,'action',action,'score',score,'operation_ref',operation_ref,'updated_at',strftime('%Y-%m-%dT%H:%M:%SZ',updated_at,'unixepoch')) FROM risk_events WHERE user_id=?1"
        )),
        _ => Err(invalid("未知调查记录类型")),
    }
}
pub(super) async fn records(
    State(state): State<AppState>,
    Path(uid): Path<i64>,
    Query(f): Query<Filters>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    ensure_user(&db, uid)?;
    user(&db, uid)?;
    let section = f.section.as_deref().unwrap_or("timeline");
    let sql = base(section)?;
    let mut values = vec![rusqlite::types::Value::Integer(uid)];
    if section == "timeline" {
        let phone: String =
            db.query_row("SELECT phone FROM users WHERE id=?1", [uid], |r| r.get(0))?;
        values.push(subject_hash(&phone).into());
    }
    let mut clauses = vec!["1=1".to_owned()];
    amount_filters(&f)?;
    let (page, size) = pagination(&f)?;
    for (value, operator) in [(f.from.as_deref(), ">="), (f.to.as_deref(), "<=")] {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            let p = parameter(&mut values, timestamp(value, operator == "<=")?);
            clauses.push(format!("created_at{operator}{p}"));
        }
    }
    for (value, column) in [(f.kind.as_deref(), "kind"), (f.status.as_deref(), "status")] {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            let p = parameter(&mut values, value.to_owned());
            clauses.push(format!("{column}={p}"));
        }
    }
    for (amount, operator) in [(f.min_amount, ">="), (f.max_amount, "<=")] {
        if let Some(amount) = amount {
            let p = parameter(&mut values, amount);
            clauses.push(format!("ABS(amount_yuan){operator}{p}"));
        }
    }
    if let Some(related) = f.related.as_ref().filter(|v| !v.is_empty()) {
        let p = parameter(&mut values, format!("%{related}%"));
        clauses.push(format!(
            "(related_id LIKE {p} OR id LIKE {p} OR extra LIKE {p})"
        ));
    }
    let filter = clauses.join(" AND ");
    let total: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM ({sql}) WHERE {filter}"),
        rusqlite::params_from_iter(values.iter()),
        |r| r.get(0),
    )?;
    let limit = parameter(&mut values, size);
    let offset = parameter(&mut values, (page - 1) * size);
    let mut stmt=db.prepare(&format!("SELECT id,user_id,kind,status,created_at,amount_yuan,related_id,extra FROM ({sql}) WHERE {filter} ORDER BY created_at DESC,id DESC LIMIT {limit} OFFSET {offset}"))?;
    let raw = stmt
        .query_map(rusqlite::params_from_iter(values.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<f64>>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut items = Vec::new();
    for (id, user_id, kind, status, created, amount, related, extra) in raw {
        let mut row: Value = serde_json::from_str(&extra)?;
        row["id"] = json!(id);
        row["user_id"] = json!(user_id);
        row["kind"] = json!(kind);
        row["status"] = json!(status);
        row["created_at"] = if created > 0 {
            json!(iso(created))
        } else {
            Value::Null
        };
        row["amount_yuan"] = json!(amount);
        row["related_id"] = json!(related);
        if section == "devices" {
            let device = row["device_id"].as_str().unwrap();
            let mut related=db.prepare("SELECT u.id,u.phone,u.nickname FROM account_devices d JOIN users u ON u.id=d.user_id WHERE d.device_id=?1 ORDER BY u.id")?;
            row["related_users"]=json!(related.query_map([device],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"phone":r.get::<_,String>(1)?,"nickname":r.get::<_,String>(2)?})))?.collect::<Result<Vec<_>,_>>()?);
        }
        items.push(row);
    }
    let note = match section {
        "callbacks" => "采集启用前仅保留业务验证状态；未采集的请求与失败原因无法回溯。",
        "devices" => "仅列出真实已验证报告关联的设备；历史未采集。",
        "timeline" => "业务记录与采集后的行为共同展示；历史登录和请求IP未采集。",
        _ => "",
    };
    Ok(Json(
        json!({"items":items,"total":total,"page":page,"page_size":size,"availability":availability(&state,&db)?,"note":note}),
    ))
}
#[derive(Deserialize)]
pub(super) struct RestrictionInput {
    status: String,
    reason: String,
}
pub(super) async fn change_restriction(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(uid): Path<i64>,
    Json(input): Json<RestrictionInput>,
) -> ApiResult<Value> {
    if !["active", "review", "restricted"].contains(&input.status.as_str())
        || input.reason.trim().is_empty()
    {
        return Err(invalid("请选择账号状态并填写操作理由"));
    }
    let (before, source) = {
        let db = open(&state)?;
        ensure_user(&db, uid)?;
        let before = restriction(&db, uid)?;
        let source: Option<String> = db
            .query_row(
                "SELECT risk_event_id FROM account_restrictions WHERE user_id=?1",
                [uid],
                |r| r.get(0),
            )
            .optional()?;
        (before, source.unwrap_or_default())
    };
    if input.status == "active" && !source.is_empty() {
        if let Err(error) = business_guard::resolve_approved(&state, &source, &input.reason).await {
            action_audit(
                &open(&state)?,
                &context,
                Some(uid),
                "restriction_resolve_failed",
                &source,
                &input.reason,
                before,
                json!({"error":error.1}),
            )?;
            return Err(error);
        }
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let current = restriction(&tx, uid)?;
    if current["trace_id"] != before["trace_id"] {
        return Err(conflict("账号状态已更新，请刷新后重试"));
    }
    let trace = Uuid::new_v4().to_string();
    tx.execute("INSERT INTO account_restrictions(user_id,status,reason,updated_at,updated_by,trace_id) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(user_id) DO UPDATE SET status=excluded.status,reason=excluded.reason,updated_at=excluded.updated_at,updated_by=excluded.updated_by,trace_id=excluded.trace_id",params![uid,input.status,input.reason,now(),context.admin_username.as_deref().unwrap_or("local-preview"),trace])?;
    let after = restriction(&tx, uid)?;
    action_audit(
        &tx,
        &context,
        Some(uid),
        "restriction_change",
        &trace,
        &input.reason,
        before,
        after.clone(),
    )?;
    tx.commit()?;
    Ok(Json(after))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/search", get(search))
        .route("/users/{id}/detail", get(detail))
        .route("/users/{id}/records", get(records))
        .route(
            "/users/{id}/restriction",
            axum::routing::put(change_restriction),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(sdk: bool) -> (AppState, RequestContext, HeaderMap) {
        let state = AppState {
            db_path: std::env::temp_dir()
                .join(format!("orange-investigation-{}.sqlite3", Uuid::new_v4())),
            config: RuntimeConfig {
                production: false,
                ad_mode: "mock".into(),
                csj_reward_secret: None,
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
        for (uid, phone, code, token) in [
            (2, "13900139000", "AB7C8D", "test-token"),
            (3, "13900139001", "EF4G5H", "other-token"),
        ] {
            db.execute("INSERT INTO users(id,phone,password_hash,nickname,coins,created_at,invite_code,inviter_code,invitation_bound_at) VALUES(?1,?2,'unused','调查测试',5000,?3,?4,'JZ8F6A',?3)",params![uid,phone,now(),code]).unwrap();
            db.execute(
                "INSERT INTO sessions(token,user_id,expires_at) VALUES(?1,?2,?3)",
                params![token, uid, now() + 3600],
            )
            .unwrap();
            db.execute("INSERT INTO ledger(user_id,title,coins,source,created_at) VALUES(?1,'既有余额',5000,?2,?3)",params![uid,format!("fixture:{uid}"),now()]).unwrap();
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer test-token"),
        );
        (
            state,
            RequestContext {
                request_id: Uuid::new_v4().to_string(),
                client_ip: "203.0.113.8".into(),
                admin_username: None,
            },
            headers,
        )
    }
    #[test]
    fn forwarded_ip_needs_loopback_and_private_proxy_authentication() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.4"));
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.99"));
        let local = "127.0.0.1:4567".parse().unwrap();
        let remote = "203.0.113.9:4567".parse().unwrap();
        assert_eq!(
            trusted_ip(&headers, local, Some("private-token")),
            "127.0.0.1"
        );
        headers.insert(
            "x-orange-proxy-token",
            HeaderValue::from_static("wrong-token"),
        );
        assert_eq!(
            trusted_ip(&headers, local, Some("private-token")),
            "127.0.0.1"
        );
        headers.insert(
            "x-orange-proxy-token",
            HeaderValue::from_static("private-token"),
        );
        assert_eq!(
            trusted_ip(&headers, remote, Some("private-token")),
            "203.0.113.9"
        );
        assert_eq!(
            trusted_ip(&headers, local, Some("private-token")),
            "198.51.100.4"
        );
    }
    #[tokio::test]
    async fn filters_pagination_and_safe_audit_fields_are_real_data() {
        let (state, context, _) = fixture(false);
        let db = open(&state).unwrap();
        credit(&db, 2, 2000, "红包", "reward:amount-target").unwrap();
        credit(&db, 2, -1000, "提现", "withdraw:amount-out").unwrap();
        let result = records(
            State(state.clone()),
            Path(2),
            Query(Filters {
                section: Some("ledger".into()),
                min_amount: Some(1.0),
                max_amount: Some(3.0),
                status: Some("credited".into()),
                page_size: Some(1),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["total"], 1);
        assert_eq!(result["items"][0]["coins"], 2000);
        let found = search(
            State(state.clone()),
            Query(Filters {
                q: Some("' OR 1=1 --".into()),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(found["total"], 0);
        for section in [
            "timeline",
            "ads",
            "callbacks",
            "withdrawals",
            "risk",
            "devices",
            "audit",
            "invitations",
        ] {
            let _ = records(
                State(state.clone()),
                Path(2),
                Query(Filters {
                    section: Some(section.into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        }
        let uri:axum::http::Uri="/api/admin/users/2/records?section=ledger&q=139&token=never-store&sign=never-sign&password=never-password".parse().unwrap();
        let audit = access_audit(&db, &context, "GET", &uri).unwrap();
        let raw: String = db
            .query_row(
                "SELECT filters_json FROM admin_audit WHERE id=?1",
                [audit],
                |r| r.get(0),
            )
            .unwrap();
        assert!(raw.contains("139"));
        assert!(!raw.contains("never-store"));
        assert!(!raw.contains("never-sign"));
        assert!(!raw.contains("never-password"));
        assert_eq!(
            timestamp("2026-11-11", false).unwrap(),
            chrono::DateTime::parse_from_rfc3339("2026-11-10T16:00:00Z")
                .unwrap()
                .timestamp()
        );
        assert_eq!(
            timestamp("2026-11-11", true).unwrap() - timestamp("2026-11-11", false).unwrap(),
            86399
        );
    }
    #[tokio::test]
    async fn restriction_and_admin_audit_commit_together_without_public_reason() {
        let (state, context, _) = fixture(false);
        let db = open(&state).unwrap();
        assert!(
            change_restriction(
                State(state.clone()),
                Extension(context.clone()),
                Path(2),
                Json(RestrictionInput {
                    status: "restricted".into(),
                    reason: "".into()
                })
            )
            .await
            .is_err()
        );
        let _ = change_restriction(
            State(state.clone()),
            Extension(context),
            Path(2),
            Json(RestrictionInput {
                status: "review".into(),
                reason: "内部证据不公开".into(),
            }),
        )
        .await
        .unwrap();
        let error = risk::check_account(&db, 2).unwrap_err();
        assert_eq!(error.0, StatusCode::LOCKED);
        assert!(!error.1.contains("内部证据"));
        db.execute("INSERT INTO risk_events(id,user_id,kind,client_ip,context_json,status,created_at,updated_at) VALUES('recent-error',2,'login','203.0.113.8','{}','assessment_error',?1,?2)",params![now()-172800,now()]).unwrap();
        let overview = admin_overview(State(state.clone())).await.unwrap().0;
        assert_eq!(overview["risk_review_users"], 1);
        assert_eq!(overview["risk_assessment_errors_24h"], 1);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM admin_audit WHERE user_id=2 AND action='restriction_change'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT coins FROM users WHERE id=2", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            5000
        );
    }
    #[tokio::test]
    async fn sdk_reward_requires_matching_event_and_retry_preserves_money() {
        let (state, context, mut headers) = fixture(true);
        let db = open(&state).unwrap();
        let quote = reward_policy::test_quote(&state, &headers, "welcome", None).await;
        let qid = quote["quote_id"].as_str().unwrap().to_owned();
        let body = json!({"task_id":"welcome","quote_id":qid});
        assert_eq!(
            reward_start(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(RewardInput {
                    task_id: "welcome".into(),
                    quote_id: qid.clone()
                })
            )
            .await
            .unwrap_err()
            .0,
            StatusCode::LOCKED
        );
        business_guard::test_event(&state, "other-user", 3, "check_in", json!({}));
        headers.insert("x-risk-event", HeaderValue::from_static("other-user"));
        assert!(
            business_guard::validate_local(&state, &db, &headers, 2, "reward", body.clone())
                .is_err()
        );
        business_guard::test_event(&state, "wrong-kind", 2, "check_in", json!({}));
        headers.insert("x-risk-event", HeaderValue::from_static("wrong-kind"));
        assert!(
            business_guard::validate_local(&state, &db, &headers, 2, "reward", body.clone())
                .is_err()
        );
        business_guard::test_event(&state, "reward-proof", 2, "reward", body);
        headers.insert("x-risk-event", HeaderValue::from_static("reward-proof"));
        let first = reward_start(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RewardInput {
                task_id: "welcome".into(),
                quote_id: qid.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        db.execute(
            "UPDATE risk_events SET created_at=?1 WHERE id='reward-proof'",
            [now() - 900],
        )
        .unwrap();
        let retry = reward_start(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RewardInput {
                task_id: "welcome".into(),
                quote_id: qid.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(first["id"], retry["id"]);
        assert!(
            reward_start(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(RewardInput {
                    task_id: "lucky".into(),
                    quote_id: qid
                })
            )
            .await
            .is_err()
        );
        let sid = first["id"].as_str().unwrap();
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
        let again = reward_complete(
            State(state.clone()),
            Extension(context),
            headers,
            Path(sid.into()),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(paid["user"]["coins"], again["user"]["coins"]);
        assert_eq!(again["already_completed"], true);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM ledger WHERE source=?1",
                [format!("reward:{sid}")],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    #[tokio::test]
    async fn sdk_withdrawal_retry_never_debits_twice_and_restrictions_apply() {
        let (state, context, mut headers) = fixture(true);
        let db = open(&state).unwrap();
        db.execute(
            "UPDATE withdraw_rules SET json=?1 WHERE id=1",
            [r#"{"tiers":[{"id":"t1","amount_yuan":1.0,"daily_limit":0}]}"#],
        )
        .unwrap();
        business_guard::test_event(
            &state,
            "withdraw-proof",
            2,
            "withdraw",
            json!({"amount_yuan":1.0,"channel":"alipay","account":"receiver"}),
        );
        headers.insert("x-risk-event", HeaderValue::from_static("withdraw-proof"));
        let input = || {
            Json(WithdrawInput {
                amount_yuan: 1.0,
                channel: "alipay".into(),
                account: "receiver".into(),
            })
        };
        let first = withdraw(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            input(),
        )
        .await
        .unwrap()
        .0;
        db.execute(
            "UPDATE risk_events SET created_at=?1 WHERE id='withdraw-proof'",
            [now() - 900],
        )
        .unwrap();
        let retry = withdraw(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            input(),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(first["id"], retry["id"]);
        assert_eq!(retry["user"]["coins"], 4000);
        assert!(
            withdraw(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(WithdrawInput {
                    amount_yuan: 2.0,
                    channel: "alipay".into(),
                    account: "receiver".into()
                })
            )
            .await
            .is_err()
        );
        let _ = change_restriction(
            State(state.clone()),
            Extension(context.clone()),
            Path(2),
            Json(RestrictionInput {
                status: "restricted".into(),
                reason: "待核查".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            withdraw(State(state), Extension(context), headers, input())
                .await
                .unwrap_err()
                .0,
            StatusCode::LOCKED
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM withdrawals WHERE user_id=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
