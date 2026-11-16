use super::*;
use axum::extract::{ConnectInfo, Extension};
use std::net::SocketAddr;

#[derive(Clone)]
pub struct Config {
    pub(super) app_id: String,
    app_key: String,
    pub(super) base_url: String,
    pub(super) pins: Vec<String>,
    client: reqwest::Client,
}
impl Config {
    #[cfg(test)]
    pub(super) fn is_test(&self) -> bool {
        self.app_id == "test-app" && self.base_url == "https://invalid.invalid"
    }
    #[cfg(test)]
    pub(crate) fn test_config() -> Self {
        Self {
            app_id: "test-app".into(),
            app_key: "test-private-key".into(),
            base_url: "https://invalid.invalid".into(),
            pins: vec!["test-pin".into()],
            client: reqwest::Client::new(),
        }
    }
    pub fn from_env() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Some(app_id) = env_value("ORANGE_DEVICE_TRUST_APP_ID") else {
            if env_value("ORANGE_DEVICE_TRUST_APP_KEY").is_some() {
                return Err("DeviceTrust APP_ID 缺失".into());
            }
            return Ok(None);
        };
        if env_value("ORANGE_DEVICE_TRUST_MODE").is_some_and(|mode| mode != "enforce") {
            return Err("业务执行授权需要 ORANGE_DEVICE_TRUST_MODE=enforce".into());
        }
        let app_key = env_value("ORANGE_DEVICE_TRUST_APP_KEY").ok_or("DeviceTrust APP_KEY 缺失")?;
        let base_url = env_value("ORANGE_DEVICE_TRUST_URL").ok_or("DeviceTrust URL 缺失")?;
        if !base_url.starts_with("https://") {
            return Err("DeviceTrust URL 必须使用 HTTPS".into());
        }
        let pins = env_value("ORANGE_DEVICE_TRUST_PINS")
            .ok_or("DeviceTrust PINS 缺失")?
            .split(',')
            .map(|v| v.trim().to_owned())
            .collect();
        Ok(Some(Self {
            app_id,
            app_key,
            base_url: base_url.trim_end_matches('/').into(),
            pins,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        }))
    }
    pub(super) async fn call(&self, path: &str, value: Value) -> Result<Value, String> {
        let private_token = value
            .get("token")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.app_key)
            .json(&value)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let mut body = response.text().await.map_err(|e| e.to_string())?;
        body = body.replace(&self.app_key, "[redacted]");
        if let Some(token) = private_token {
            if !token.is_empty() {
                body = body.replace(&token, "[redacted]");
            }
        }
        if !status.is_success() {
            return Err(format!("DeviceTrust {path}: HTTP {status}: {body}"));
        }
        serde_json::from_str(&body).map_err(|e| format!("DeviceTrust {path}: {e}: {body}"))
    }
}

pub fn init(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS risk_events(
        id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,kind TEXT NOT NULL,client_ip TEXT NOT NULL,
        context_json TEXT NOT NULL,key_id TEXT NOT NULL DEFAULT '',report_id TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL,decision TEXT NOT NULL DEFAULT '',action TEXT NOT NULL DEFAULT '',
        score INTEGER,detail_json TEXT NOT NULL DEFAULT '{}',error TEXT NOT NULL DEFAULT '',
        operation_ref TEXT UNIQUE,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_risk_user_time ON risk_events(user_id,created_at);
        CREATE INDEX IF NOT EXISTS idx_risk_key ON risk_events(key_id,user_id);")
}

fn public_error(reference: &str) -> ApiError {
    ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        format!("服务暂不可用，请稍后再试。参考号：{reference}"),
        None,
    )
}
fn restriction_error(reference: &str) -> ApiError {
    ApiError(
        StatusCode::LOCKED,
        format!("当前账户暂不能参与此活动。参考号：{reference}"),
        None,
    )
}
pub(super) fn account_id(state: &AppState, uid: i64) -> String {
    format!(
        "orange-{}-user-{uid}",
        if state.config.production {
            "prod"
        } else {
            "local"
        }
    )
}
fn failed(
    state: &AppState,
    id: &str,
    error: &str,
    context: &user_admin::RequestContext,
) -> Result<(), ApiError> {
    let mut db = open(state)?;
    let tx = db.transaction()?;
    tx.execute("UPDATE risk_events SET status='assessment_error',error=?1,updated_at=?2 WHERE id=?3 AND status!='reported'", params![error,now(),id])?;
    let uid: i64 = tx.query_row("SELECT user_id FROM risk_events WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    user_admin::behavior(
        &tx,
        context,
        Some(uid),
        None,
        "risk_assessment",
        "failed",
        id,
        None,
        json!({"error":error,"reference":id}),
    )?;
    tx.commit()?;
    eprintln!("DeviceTrust reference={id}: {error}");
    Ok(())
}

#[derive(Deserialize)]
struct Prepare {
    kind: String,
    #[serde(default)]
    key_id: String,
    intent: business_guard::Intent,
}
async fn prepare(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(request_context): Extension<user_admin::RequestContext>,
    headers: HeaderMap,
    Json(input): Json<Prepare>,
) -> ApiResult<Value> {
    if ![
        "login", "reward", "check_in", "withdraw", "rain", "behavior",
    ]
    .contains(&input.kind.as_str())
    {
        return Err(invalid("请求类型错误"));
    }
    if input.key_id.len() > 128 {
        return Err(invalid("请求参数错误"));
    }
    let id = Uuid::new_v4().to_string();
    let config = state
        .config
        .device_trust
        .as_ref()
        .ok_or_else(|| public_error(&id))?;
    let (uid, ip, context, event_type) = {
        let db = open(&state)?;
        let uid = account_auth(&db, &headers)?;
        let intent = business_guard::normalize(&state, &db, uid, &input.kind, &input.intent)?;
        let request_hash = business_guard::request_hash(&account_id(&state, uid), &intent)?;
        let operation = business_guard::planned_operation(uid, &input.kind, &id, &intent);
        let ip = client_ip(&headers, peer);
        let created: i64 =
            db.query_row("SELECT created_at FROM users WHERE id=?1", [uid], |r| {
                r.get(0)
            })?;
        let event_type = match input.kind.as_str() {
            "check_in" | "rain" => "reward",
            "withdraw" => "withdraw",
            "reward" => "reward",
            "behavior" => "behavior",
            _ => "login",
        };
        let mut context = json!({"accountCreatedAt": iso(created)});
        context["requestHash"] = json!(request_hash);
        if !["login", "behavior"].contains(&input.kind.as_str()) {
            context["campaignId"] = json!(format!("orange-{}", input.kind));
            context["recipientId"] = json!(account_id(&state, uid));
        }
        db.execute("INSERT INTO risk_events(id,user_id,kind,client_ip,context_json,key_id,status,created_at,updated_at,intent_json,request_hash,planned_operation,binding_version) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,?7,?8,?9,?10,1)",
            params![id,uid,input.kind,ip,context.to_string(),input.key_id,now(),serde_json::to_string(&intent)?,request_hash,operation])?;
        (uid, ip, context, event_type)
    };
    let result = config.call("/v1/challenges",json!({"appId":config.app_id,"eventId":id,"accountId":account_id(&state,uid),"type":event_type,"context":context,"clientIp":ip,"keyId":input.key_id})).await;
    match result {
        Ok(challenge) => Ok(Json(
            json!({"event_id":id,"challenge":challenge,"sdk":{"app_id":config.app_id,"base_url":config.base_url,"spki_pins":config.pins}}),
        )),
        Err(error) => {
            failed(&state, &id, &error, &request_context)?;
            Err(public_error(&id))
        }
    }
}

#[derive(Deserialize)]
struct Complete {
    event_id: String,
    report_id: String,
}
async fn complete(
    State(state): State<AppState>,
    Extension(request_context): Extension<user_admin::RequestContext>,
    headers: HeaderMap,
    Json(input): Json<Complete>,
) -> ApiResult<Value> {
    let config = state
        .config
        .device_trust
        .as_ref()
        .ok_or_else(|| public_error(&input.event_id))?;
    let (uid, kind, ip, context, prior_report, status) = {
        let db = open(&state)?;
        let uid = account_auth(&db, &headers)?;
        let row: Option<(String,String,String,String,String,i64)> = db.query_row(
            "SELECT kind,client_ip,context_json,report_id,status,created_at FROM risk_events WHERE id=?1 AND user_id=?2",
            params![input.event_id,uid], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
        let Some((kind, ip, context, report, status, created)) = row else {
            return Err(invalid("记录不存在"));
        };
        if status != "reported" && now() - created > 600 {
            return Err(public_error(&input.event_id));
        }
        if !report.is_empty() && report != input.report_id {
            return Err(conflict("请求已处理"));
        }
        (
            uid,
            kind,
            ip,
            serde_json::from_str::<Value>(&context)?,
            report,
            status,
        )
    };
    if status == "reported" && prior_report == input.report_id {
        return public_status(&state, uid, &input.event_id);
    }
    let event_type = match kind.as_str() {
        "check_in" | "rain" => "reward",
        "withdraw" => "withdraw",
        "reward" => "reward",
        "behavior" => "behavior",
        _ => "login",
    };
    let result = config
        .call(
            "/v1/events",
            json!({"appId":config.app_id,"eventId":input.event_id,"reportId":input.report_id,
        "accountId":account_id(&state,uid),"type":event_type,"clientIp":ip,"context":context}),
        )
        .await;
    let event = match result {
        Ok(event) => event,
        Err(error) => {
            failed(&state, &input.event_id, &error, &request_context)?;
            return Err(public_error(&input.event_id));
        }
    };
    let key = event
        .pointer("/reportProof/keyId")
        .and_then(Value::as_str)
        .unwrap_or("");
    let proof = event
        .pointer("/reportProof/level")
        .and_then(Value::as_str)
        .unwrap_or("");
    if key.is_empty()
        || !["signature_only", "tee", "strongbox"].contains(&proof)
        || event["eventId"] != input.event_id
        || event["accountId"] != account_id(&state, uid)
        || event.pointer("/context/requestHash") != context.get("requestHash")
    {
        failed(
            &state,
            &input.event_id,
            "DeviceTrust response missing verified event binding",
            &request_context,
        )?;
        return Err(public_error(&input.event_id));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let device_id = event["deviceId"].as_str().unwrap_or("");
    let device_identity = reward_policy::observe_verified(&tx, uid, key, device_id)?;
    // Re-check inside the same transaction: equal retries retain their original decision.
    let prior_status: String = tx.query_row(
        "SELECT status FROM risk_events WHERE id=?1",
        [&input.event_id],
        |r| r.get(0),
    )?;
    if prior_status != "reported" {
        tx.execute("UPDATE risk_events SET report_id=?1,key_id=?2,status='reported',decision=?3,action=?4,score=?5,detail_json=?6,error='',updated_at=?7,device_identity=?9 WHERE id=?8",
            params![input.report_id,key,event["decision"].as_str().unwrap_or(""),event["action"].as_str().unwrap_or(""),event["score"].as_i64(),event.to_string(),now(),input.event_id,device_identity])?;
        let action = event["action"].as_str().unwrap_or("observe");
        if ["verify", "hold"].contains(&action) {
            let status = if action == "hold" {
                "restricted"
            } else {
                "review"
            };
            tx.execute("INSERT INTO account_restrictions(user_id,status,reason,updated_at,updated_by,trace_id,risk_event_id) VALUES(?1,?2,?3,?4,'DeviceTrust',?5,?5)
                ON CONFLICT(user_id) DO UPDATE SET status=CASE WHEN account_restrictions.status='restricted' THEN 'restricted' ELSE excluded.status END,reason=excluded.reason,updated_at=excluded.updated_at,updated_by=excluded.updated_by,trace_id=excluded.trace_id,risk_event_id=excluded.risk_event_id
                WHERE account_restrictions.status='active' OR account_restrictions.updated_by='DeviceTrust'",
                params![uid,status,format!("DeviceTrust {action}: {}",event["reasons"]),now(),input.event_id])?;
        }
    }
    user_admin::record_device(
        &tx,
        uid,
        &user_admin::DeviceObservation {
            device_id: if device_id.is_empty() {
                key.into()
            } else {
                device_id.into()
            },
            platform: "Android".into(),
            model: String::new(),
            os_version: String::new(),
            key_id: key.into(),
            report_id: input.report_id.clone(),
            evidence_status: proof.into(),
        },
    )?;
    tx.commit()?;
    public_status(&state, uid, &input.event_id)
}
fn public_status(state: &AppState, uid: i64, id: &str) -> ApiResult<Value> {
    let db = open(state)?;
    let observational: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM risk_events WHERE id=?1 AND user_id=?2 AND kind='behavior' AND status='reported')",params![id,uid],|r|r.get(0))?;
    if observational {
        return Ok(Json(json!({"status":"recorded","reference":id})));
    }
    let status: Option<String> = db
        .query_row(
            "SELECT status FROM account_restrictions WHERE user_id=?1",
            [uid],
            |r| r.get(0),
        )
        .optional()?;
    Ok(Json(
        json!({"status":match status.as_deref(){Some("restricted")=>"restricted",Some("review")=>"review",_=>"recorded"},"reference":id}),
    ))
}

#[derive(Deserialize)]
struct Failure {
    #[serde(default)]
    event_id: String,
    #[serde(default)]
    kind: String,
    stage: String,
    code: String,
    #[serde(default)]
    detail: String,
}
async fn failure(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(request_context): Extension<user_admin::RequestContext>,
    headers: HeaderMap,
    Json(input): Json<Failure>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    if input.detail.len() > 2048
        || input.stage.len() > 32
        || input.code.len() > 80
        || !input
            .stage
            .chars()
            .chain(input.code.chars())
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err(invalid("请求参数错误"));
    }
    let id = if input.event_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.event_id
    };
    if !["login", "reward", "check_in", "withdraw", "rain"].contains(&input.kind.as_str())
        && input.kind != ""
    {
        return Err(invalid("请求参数错误"));
    }
    let owner: Option<i64> = db
        .query_row("SELECT user_id FROM risk_events WHERE id=?1", [&id], |r| {
            r.get(0)
        })
        .optional()?;
    if owner.is_some_and(|owner| owner != uid) {
        return Err(invalid("记录不存在"));
    }
    let mut diagnostic = input.detail.clone();
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        if !token.is_empty() {
            diagnostic = diagnostic.replace(token, "[redacted]");
        }
    }
    db.execute("INSERT OR IGNORE INTO risk_events(id,user_id,kind,client_ip,context_json,status,created_at,updated_at) VALUES(?1,?2,?3,?4,'{}','pending',?5,?5)",params![id,uid,input.kind,client_ip(&headers,peer),now()])?;
    db.execute("UPDATE risk_events SET status='assessment_error',error=?1,updated_at=?2 WHERE id=?3 AND user_id=?4 AND status!='reported' AND error=''",
        params![format!("SDK {}: {} {}",input.stage,input.code,diagnostic),now(),id,uid])?;
    user_admin::behavior(
        &db,
        &request_context,
        Some(uid),
        None,
        "risk_sdk_failure",
        "failed",
        &id,
        None,
        json!({"stage":input.stage,"code":input.code,"detail":diagnostic,"reference":id}),
    )?;
    Ok(Json(json!({"reference":id})))
}

pub fn check_account(db: &Connection, uid: i64) -> Result<(), ApiError> {
    let row: Option<(String, String)> = db
        .query_row(
            "SELECT status,trace_id FROM account_restrictions WHERE user_id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((status, trace)) = row {
        if status != "active" {
            return Err(restriction_error(&trace));
        }
    }
    Ok(())
}
pub fn consume(db: &Connection, id: &Option<String>, reference: &str) -> Result<(), ApiError> {
    if let Some(id) = id {
        let updated=db.execute("UPDATE risk_events SET operation_ref=?1,updated_at=?2 WHERE id=?3 AND planned_operation=?1 AND binding_version=1 AND permit_expires_at>?2 AND (operation_ref IS NULL OR operation_ref=?1)",params![reference,now(),id])?;
        if updated == 0 {
            return Err(public_error(id));
        }
    }
    Ok(())
}
pub fn authorize_completed_reward(
    state: &AppState,
    db: &Connection,
    uid: i64,
    sid: &str,
) -> Result<(), ApiError> {
    check_account(db, uid)?;
    let quote: Option<String> = db.query_row(
        "SELECT quote_id FROM reward_sessions WHERE id=?1 AND user_id=?2",
        params![sid, uid],
        |r| r.get(0),
    )?;
    if state.config.device_trust.is_some() && quote.is_some() {
        business_guard::validate_reserved(state, db, uid, sid)?;
    }

    Ok(())
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/risk/prepare", post(prepare))
        .route("/risk/complete", post(complete))
        .route("/risk/failure", post(failure))
}
