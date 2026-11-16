use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Intent {
    pub method: String,
    pub path: String,
    pub body: Value,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Permit {
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "eventId")]
    pub event_id: String,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    #[serde(rename = "requestHash")]
    pub request_hash: String,
    pub token: String,
    pub mode: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    let columns = db
        .prepare("PRAGMA table_info(risk_events)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    for (name, definition) in [
        ("intent_json", "TEXT NOT NULL DEFAULT '{}'"),
        ("request_hash", "TEXT NOT NULL DEFAULT ''"),
        ("planned_operation", "TEXT NOT NULL DEFAULT ''"),
        ("permit_json", "TEXT NOT NULL DEFAULT '{}'"),
        ("permit_expires_at", "INTEGER"),
        ("authorized_at", "INTEGER"),
        ("device_identity", "TEXT NOT NULL DEFAULT ''"),
        ("binding_version", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if !columns.iter().any(|v| v == name) {
            db.execute_batch(&format!(
                "ALTER TABLE risk_events ADD COLUMN {name} {definition}"
            ))?;
        }
    }
    db.execute_batch("CREATE TABLE IF NOT EXISTS risk_receipts(event_id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,operation_id TEXT NOT NULL,receipt_id TEXT NOT NULL UNIQUE,request_hash TEXT NOT NULL,reference TEXT NOT NULL,execution_status TEXT NOT NULL DEFAULT 'succeeded',delivery_status TEXT NOT NULL DEFAULT 'pending',error TEXT NOT NULL DEFAULT '',attempts INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);")?;
    let columns = db
        .prepare("PRAGMA table_info(account_restrictions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !columns.iter().any(|v| v == "risk_event_id") {
        db.execute_batch("ALTER TABLE account_restrictions ADD COLUMN risk_event_id TEXT NOT NULL DEFAULT '';UPDATE account_restrictions SET risk_event_id=trace_id WHERE updated_by='DeviceTrust' AND EXISTS(SELECT 1 FROM risk_events WHERE id=account_restrictions.trace_id);")?;
    }
    Ok(())
}
pub(super) fn payment_channel(value: &str) -> Result<String, ApiError> {
    match value {
        "支付宝" | "alipay" => Ok("alipay".into()),
        "微信" | "wechat" => Ok("wechat".into()),
        _ => Err(invalid("请选择支付宝或微信")),
    }
}
pub(super) fn normalize(
    state: &AppState,
    db: &Connection,
    uid: i64,
    kind: &str,
    intent: &Intent,
) -> Result<Intent, ApiError> {
    let path = match kind {
        "login" => "/api/auth/login/report",
        "reward" => "/api/rewards/start",
        "check_in" => "/api/check-in",
        "withdraw" => "/api/withdrawals",
        "rain" => "/api/rain/join",
        "behavior" => "/api/behavior/session",
        _ => return Err(invalid("请求类型错误")),
    };
    if intent.method.to_uppercase() != "POST" || intent.path != path || !intent.body.is_object() {
        return Err(invalid("业务请求与验证场景不匹配"));
    }
    let recipient = risk::account_id(state, uid);
    let body = match kind {
        "reward" => {
            let mut value = reward_policy::quote_intent(db, uid, &intent.body)?;
            value["recipient"] = json!(recipient);
            value
        }
        "check_in" => {
            json!({"date":day(),"reward_coins":50,"currency":"COIN","recipient":recipient})
        }
        "withdraw" => {
            let amount = intent.body["amount_yuan"]
                .as_f64()
                .ok_or_else(|| invalid("请填写提现金额"))?;
            if !amount.is_finite()
                || amount <= 0.0
                || ((amount * 100.0).round() - amount * 100.0).abs() > 0.001
            {
                return Err(invalid("提现金额最多保留两位小数"));
            }
            let account = intent.body["account"]
                .as_str()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .ok_or_else(|| invalid("请填写收款账号"))?;
            json!({"amount_fen":(amount*100.0).round()as i64,"debit_coins":(amount*1000.0).round()as i64,"currency":"CNY","channel":payment_channel(intent.body["channel"].as_str().unwrap_or(""))?,"account":account,"payer":recipient})
        }
        "rain" => rain::binding_summary(db, &intent.body)?,
        "behavior" => behavior_audit::binding_summary(&intent.body)?,
        _ => json!({"action":"authenticated_login_report","account":recipient}),
    };
    Ok(Intent {
        method: "POST".into(),
        path: path.into(),
        body,
    })
}
pub(super) fn actual_intent(kind: &str, body: Value) -> Intent {
    Intent {
        method: "POST".into(),
        path: match kind {
            "reward" => "/api/rewards/start",
            "check_in" => "/api/check-in",
            "withdraw" => "/api/withdrawals",
            "rain" => "/api/rain/join",
            "behavior" => "/api/behavior/session",
            _ => "/api/auth/login/report",
        }
        .into(),
        body,
    }
}
#[derive(Serialize)]
struct Envelope<'a> {
    v: &'a str,
    method: &'a str,
    path: &'a str,
    account: &'a str,
    #[serde(rename = "bodyHash")]
    body_hash: String,
}
pub(super) fn request_hash(account: &str, intent: &Intent) -> Result<String, ApiError> {
    let body = serde_json::to_vec(&intent.body)?;
    let envelope = Envelope {
        v: "dtp-request-1",
        method: &intent.method,
        path: &intent.path,
        account,
        body_hash: format!("{:x}", Sha256::digest(body)),
    };
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&envelope)?)
    ))
}
pub(super) fn planned_operation(uid: i64, kind: &str, event: &str, normalized: &Intent) -> String {
    match kind {
        "check_in" => format!("check-in:{}:{uid}", day()),
        "rain" => format!(
            "rain:{}:{uid}",
            normalized.body["round_id"].as_str().unwrap()
        ),
        "login" => format!("login:{event}"),
        _ => Uuid::new_v4().to_string(),
    }
}
fn unavailable(reference: &str) -> ApiError {
    ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        format!("服务暂不可用，请稍后再试。参考号：{reference}"),
        None,
    )
}
fn validate_permit(
    config: &risk::Config,
    event: &str,
    operation: &str,
    hash: &str,
    permit: &Permit,
    expiry_required: bool,
) -> Result<i64, ApiError> {
    let expected = format!(
        "{:x}",
        Sha256::digest(format!("{}:{event}", config.app_id).as_bytes())
    );
    let expires = chrono::DateTime::parse_from_rfc3339(&permit.expires_at)
        .map_err(|_| unavailable(event))?
        .timestamp();
    if permit.app_id != config.app_id
        || permit.event_id != expected
        || permit.operation_id != operation
        || permit.request_hash != hash
        || permit.mode != "enforce"
        || permit.token.is_empty()
        || (expiry_required && expires <= now())
    {
        return Err(unavailable(event));
    }
    Ok(expires)
}
pub(super) async fn ensure_permit(
    state: &AppState,
    headers: &HeaderMap,
    uid: i64,
    kind: &str,
    body: Value,
) -> Result<(), ApiError> {
    let Some(config) = &state.config.device_trust else {
        return Ok(());
    };
    let id = headers
        .get("x-risk-event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError(StatusCode::LOCKED, "请在最新版 APP 中完成。".into(), None))?
        .to_owned();
    let (hash, operation, stored_permit) = {
        let db = open(state)?;
        risk::check_account(&db, uid)?;
        let actual = normalize(state, &db, uid, kind, &actual_intent(kind, body))?;
        let hash = request_hash(&risk::account_id(state, uid), &actual)?;
        let row:Option<(String,String,String,String,i64)>=db.query_row("SELECT request_hash,planned_operation,permit_json,status,binding_version FROM risk_events WHERE id=?1 AND user_id=?2 AND kind=?3",params![id,uid,kind],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        let Some((stored, operation, permit, status, version)) = row else {
            return Err(unavailable(&id));
        };
        if status != "reported" || version != 1 || stored != hash {
            return Err(unavailable(&id));
        }
        (hash, operation, permit)
    };
    if stored_permit != "{}" {
        let permit: Permit = serde_json::from_str(&stored_permit)?;
        validate_permit(config, &id, &operation, &hash, &permit, true)?;
        return Ok(());
    }
    let result = config
        .call(
            "/v1/decisions/claim",
            json!({"appId":config.app_id,"eventId":id,"operationId":operation,"requestHash":hash}),
        )
        .await;
    let result = match result {
        Ok(value) => value,
        Err(error) => {
            record_error(state, &id, &error)?;
            return Err(unavailable(&id));
        }
    };
    let permit: Permit = serde_json::from_value(result).map_err(|_| unavailable(&id))?;
    let expiry = match validate_permit(config, &id, &operation, &hash, &permit, true) {
        Ok(value) => value,
        Err(error) => {
            record_error(
                state,
                &id,
                "DeviceTrust execution permit binding/mode/expiry mismatch",
            )?;
            return Err(error);
        }
    };
    open(state)?.execute("UPDATE risk_events SET permit_json=?1,permit_expires_at=?2,authorized_at=?3 WHERE id=?4 AND user_id=?5 AND request_hash=?6",params![serde_json::to_string(&permit)?,expiry,now(),id,uid,hash])?;
    Ok(())
}
fn record_error(state: &AppState, id: &str, error: &str) -> Result<(), ApiError> {
    open(state)?.execute(
        "UPDATE risk_events SET error=?1,updated_at=?2 WHERE id=?3",
        params![error, now(), id],
    )?;
    eprintln!("DeviceTrust guard reference={id}: {error}");
    Ok(())
}
pub(super) fn validate_local(
    state: &AppState,
    db: &Connection,
    headers: &HeaderMap,
    uid: i64,
    kind: &str,
    body: Value,
) -> Result<Option<String>, ApiError> {
    risk::check_account(db, uid)?;
    let Some(config) = &state.config.device_trust else {
        return Ok(None);
    };
    let id = headers
        .get("x-risk-event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError(StatusCode::LOCKED, "请在最新版 APP 中完成。".into(), None))?;
    let intent = normalize(state, db, uid, kind, &actual_intent(kind, body))?;
    let hash = request_hash(&risk::account_id(state, uid), &intent)?;
    let row:Option<(String,String,String,String,String)>=db.query_row("SELECT request_hash,planned_operation,permit_json,status,key_id FROM risk_events WHERE id=?1 AND user_id=?2 AND kind=?3 AND binding_version=1",params![id,uid,kind],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    let Some((stored, operation, permit, status, key)) = row else {
        return Err(unavailable(id));
    };
    if stored != hash || status != "reported" || key.is_empty() {
        return Err(unavailable(id));
    }
    validate_permit(
        config,
        id,
        &operation,
        &hash,
        &serde_json::from_str(&permit).map_err(|_| unavailable(id))?,
        true,
    )?;
    Ok(Some(id.into()))
}
pub(super) fn operation(db: &Connection, id: &Option<String>) -> Result<Option<String>, ApiError> {
    id.as_ref()
        .map(|id| {
            db.query_row(
                "SELECT planned_operation FROM risk_events WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
        .transpose()
}
pub(super) fn event_device(db: &Connection, id: &Option<String>) -> Result<String, ApiError> {
    if let Some(id) = id {
        Ok(db.query_row(
            "SELECT device_identity FROM risk_events WHERE id=?1",
            [id],
            |r| r.get(0),
        )?)
    } else {
        Ok("unknown".into())
    }
}
pub(super) fn validate_reserved(
    state: &AppState,
    db: &Connection,
    uid: i64,
    sid: &str,
) -> Result<(), ApiError> {
    let Some(config) = &state.config.device_trust else {
        return Ok(());
    };
    let row:Option<(String,String,String,i64)>=db.query_row("SELECT id,request_hash,permit_json,authorized_at FROM risk_events WHERE user_id=?1 AND operation_ref=?2 AND kind='reward' AND binding_version=1 AND status='reported' AND key_id!=''",params![uid,sid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((event, hash, permit, authorized)) = row else {
        return Err(unavailable(sid));
    };
    let permit: Permit = serde_json::from_str(&permit)?;
    let expires = validate_permit(config, &event, sid, &hash, &permit, false)?;
    if authorized >= expires {
        return Err(unavailable(sid));
    }
    Ok(())
}
pub(super) async fn resolve_approved(
    state: &AppState,
    event: &str,
    reason: &str,
) -> Result<(), ApiError> {
    let Some(config) = &state.config.device_trust else {
        return Ok(());
    };
    let action: Option<String> = open(state)?
        .query_row("SELECT action FROM risk_events WHERE id=?1", [event], |r| {
            r.get(0)
        })
        .optional()?;
    if !action
        .as_deref()
        .is_some_and(|v| ["verify", "hold"].contains(&v))
    {
        return Ok(());
    }
    let id = format!("orange-approval:{event}");
    let result=config.call("/v1/decisions/resolve",json!({"appId":config.app_id,"eventId":event,"resolutionId":id,"status":"approved","note":reason})).await.map_err(|e|ApiError(StatusCode::BAD_GATEWAY,e, None))?;
    let expected = format!(
        "{:x}",
        Sha256::digest(format!("{}:{event}", config.app_id).as_bytes())
    );
    if result["eventId"] != expected
        || result["resolutionId"] != id
        || result["status"] != "approved"
    {
        return Err(ApiError(
            StatusCode::BAD_GATEWAY,
            "DeviceTrust复核结果绑定不一致".into(),
            None,
        ));
    }
    Ok(())
}
pub(super) fn record_receipt(db: &Connection, uid: i64, reference: &str) -> Result<(), ApiError> {
    let event:Option<(String,String)>=db.query_row("SELECT id,request_hash FROM risk_events WHERE user_id=?1 AND operation_ref=?2 AND binding_version=1",params![uid,reference],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((event, hash)) = event {
        db.execute("INSERT OR IGNORE INTO risk_receipts(event_id,user_id,operation_id,receipt_id,request_hash,reference,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?3,?6,?6)",params![event,uid,reference,format!("orange-execution:{event}"),hash,now()])?;
    }
    Ok(())
}
pub(super) fn dispatch(state: AppState, reference: String) {
    if state.config.device_trust.is_none() {
        return;
    }
    #[cfg(test)]
    {
        if state
            .config
            .device_trust
            .as_ref()
            .is_some_and(|c| c.is_test())
        {
            return;
        }
    }
    tokio::spawn(async move {
        if let Err(error) = deliver_reference(&state, &reference).await {
            eprintln!("DeviceTrust receipt reference={reference}: {}", error.1);
        }
    });
}
async fn deliver_reference(state: &AppState, reference: &str) -> Result<(), ApiError> {
    let Some(config) = &state.config.device_trust else {
        return Ok(());
    };
    let row = {
        let db = open(state)?;
        db.query_row("SELECT r.event_id,r.operation_id,r.receipt_id,r.request_hash,r.execution_status,e.permit_json FROM risk_receipts r JOIN risk_events e ON e.id=r.event_id WHERE r.reference=?1 AND r.delivery_status!='sent'",[reference],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?))).optional()?
    };
    let Some((event, operation, receipt, hash, status, permit)) = row else {
        return Ok(());
    };
    let permit: Permit = serde_json::from_str(&permit)?;
    validate_permit(config, &event, &operation, &hash, &permit, false)?;
    let result=config.call("/v1/executions",json!({"appId":config.app_id,"eventId":event,"operationId":operation,"token":permit.token,"receiptId":receipt,"status":status,"reference":reference})).await;
    let error = match result {
        Ok(value) => {
            if value["appId"] != config.app_id
                || value["eventId"] != permit.event_id
                || value["operationId"] != operation
                || value["receiptId"] != receipt
                || value["status"] != status
                || value["reference"] != reference
            {
                Some("DeviceTrust execution receipt binding mismatch".into())
            } else {
                None
            }
        }
        Err(error) => Some(error),
    };
    open(state)?.execute("UPDATE risk_receipts SET delivery_status=?1,error=?2,attempts=attempts+1,updated_at=?3 WHERE event_id=?4",params![if error.is_some(){"failed"}else{"sent"},error.as_deref().unwrap_or(""),now(),event])?;
    if let Some(error) = error {
        return Err(ApiError(StatusCode::BAD_GATEWAY, error, None));
    }
    Ok(())
}
async fn receipt(State(state): State<AppState>, Path(event): Path<String>) -> ApiResult<Value> {
    let db = open(&state)?;
    let record=db.query_row("SELECT event_id,user_id,operation_id,receipt_id,reference,execution_status,delivery_status,error,attempts,created_at,updated_at FROM risk_receipts WHERE event_id=?1",[event],|r|Ok(json!({"event_id":r.get::<_,String>(0)?,"user_id":r.get::<_,i64>(1)?,"operation_id":r.get::<_,String>(2)?,"receipt_id":r.get::<_,String>(3)?,"reference":r.get::<_,String>(4)?,"execution_status":r.get::<_,String>(5)?,"status":r.get::<_,String>(6)?,"error":r.get::<_,String>(7)?,"attempts":r.get::<_,i64>(8)?,"created_at":iso(r.get(9)?),"updated_at":iso(r.get(10)?)}))).optional()?.ok_or_else(||ApiError(StatusCode::NOT_FOUND,"尚无已提交业务回执".into(), None))?;
    Ok(Json(record))
}
async fn retry(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(event): Path<String>,
) -> ApiResult<Value> {
    let (reference, uid) = {
        let db = open(&state)?;
        db.query_row(
            "SELECT reference,user_id FROM risk_receipts WHERE event_id=?1",
            [&event],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "尚无已提交业务回执".into(), None))?
    };
    let result = deliver_reference(&state, &reference).await;
    user_admin::action_audit(
        &open(&state)?,
        &context,
        Some(uid),
        "risk_receipt_retry",
        &event,
        "补报已提交业务回执",
        json!({}),
        json!({"succeeded":result.is_ok()}),
    )?;
    result?;
    receipt(State(state), Path(event)).await
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/risk-receipts/{id}", get(receipt))
        .route("/risk-receipts/{id}/retry", post(retry))
}

#[cfg(test)]
pub(super) fn test_event(state: &AppState, id: &str, uid: i64, kind: &str, body: Value) {
    let db = open(state).unwrap();
    let intent = normalize(state, &db, uid, kind, &actual_intent(kind, body)).unwrap();
    let hash = request_hash(&risk::account_id(state, uid), &intent).unwrap();
    let operation = planned_operation(uid, kind, id, &intent);
    let config = state.config.device_trust.as_ref().unwrap();
    let permit = json!({"appId":config.app_id,"eventId":format!("{:x}",Sha256::digest(format!("{}:{id}",config.app_id))),"operationId":operation,"requestHash":hash,"token":"private-test-permit","mode":"enforce","receivedAt":iso(now()),"expiresAt":iso(now()+600)});
    let identity =
        reward_policy::observe_verified(&db, uid, "verified-test-key", "verified-test-device")
            .unwrap();
    user_admin::record_device(
        &db,
        uid,
        &user_admin::DeviceObservation {
            device_id: "verified-test-device".into(),
            key_id: "verified-test-key".into(),
            evidence_status: "signature_only".into(),
            platform: "android".into(),
            model: String::new(),
            os_version: String::new(),
            report_id: id.into(),
        },
    )
    .unwrap();
    db.execute("INSERT OR REPLACE INTO risk_events(id,user_id,kind,client_ip,context_json,key_id,status,created_at,updated_at,intent_json,request_hash,planned_operation,permit_json,permit_expires_at,authorized_at,device_identity,binding_version) VALUES(?1,?2,?3,'203.0.113.8','{}','verified-test-key','reported',?4,?4,?5,?6,?7,?8,?9,?4,?10,1)",params![id,uid,kind,now(),serde_json::to_string(&intent).unwrap(),hash,operation,permit.to_string(),now()+600,identity]).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_hash_matches_protocol_and_binds_body_path_method_account() {
        let intent = Intent {
            method: "POST".into(),
            path: "/api/example".into(),
            body: json!({"recipient":"user-2","amount":100}),
        };
        assert_eq!(
            request_hash("user-2", &intent).unwrap(),
            "eaa787e26a10f2f1c0b67c9d84caa99008164d7eb9e11954d1a21b476293e76b"
        );
        let original = request_hash("user-2", &intent).unwrap();
        for mutated in [
            Intent {
                body: json!({"recipient":"user-2","amount":101}),
                ..intent.clone()
            },
            Intent {
                path: "/api/other".into(),
                ..intent.clone()
            },
            Intent {
                method: "GET".into(),
                ..intent.clone()
            },
        ] {
            assert_ne!(original, request_hash("user-2", &mutated).unwrap());
        }
        assert_ne!(original, request_hash("user-3", &intent).unwrap());
    }
    #[test]
    fn permit_requires_enforce_and_every_binding_while_receipt_recovery_accepts_old_expiry() {
        let config = risk::Config::test_config();
        let p = Permit {
            app_id: config.app_id.clone(),
            event_id: format!("{:x}", Sha256::digest(b"test-app:event")),
            operation_id: "operation".into(),
            request_hash: "hash".into(),
            token: "private-only".into(),
            mode: "enforce".into(),
            expires_at: iso(now() + 60),
        };
        assert!(validate_permit(&config, "event", "operation", "hash", &p, true).is_ok());
        for (field, value) in [
            ("appId", "other"),
            ("eventId", "other"),
            ("operationId", "other"),
            ("requestHash", "other"),
            ("token", ""),
            ("mode", "shadow"),
        ] {
            let mut value_json = serde_json::to_value(&p).unwrap();
            value_json[field] = json!(value);
            let changed: Permit = serde_json::from_value(value_json).unwrap();
            assert!(
                validate_permit(&config, "event", "operation", "hash", &changed, true).is_err(),
                "{field}"
            );
        }
        let old = Permit {
            expires_at: iso(now() - 60),
            ..p
        };
        assert!(validate_permit(&config, "event", "operation", "hash", &old, true).is_err());
        assert!(validate_permit(&config, "event", "operation", "hash", &old, false).is_ok());
    }
    #[tokio::test]
    async fn guard_rejects_body_owner_and_kind_changes_and_rolls_back_failed_consumption() {
        let (state, _, mut headers, _) = rain::tests::fixture(true, false);
        let mut db = open(&state).unwrap();
        let body = json!({"amount_yuan":1.25,"channel":"alipay","account":"recipient"});
        test_event(&state, "withdraw-guard", 2, "withdraw", body.clone());
        headers.insert("x-risk-event", HeaderValue::from_static("withdraw-guard"));
        assert_eq!(
            validate_local(&state, &db, &headers, 2, "withdraw", body.clone()).unwrap(),
            Some("withdraw-guard".into())
        );
        for bad in [
            json!({"amount_yuan":2.25,"channel":"alipay","account":"recipient"}),
            json!({"amount_yuan":1.25,"channel":"alipay","account":"other"}),
            json!({"amount_yuan":1.25,"channel":"wechat","account":"recipient"}),
        ] {
            assert!(validate_local(&state, &db, &headers, 2, "withdraw", bad).is_err());
        }
        assert!(validate_local(&state, &db, &headers, 3, "withdraw", body).is_err());
        assert!(validate_local(&state, &db, &headers, 2, "check_in", json!({})).is_err());
        {
            let tx = db.transaction().unwrap();
            credit(&tx, 2, 50, "must rollback", "failed-test-operation").unwrap();
            assert!(risk::consume(&tx, &Some("withdraw-guard".into()), "wrong-operation").is_err());
        }
        assert_eq!(user(&db, 2).unwrap().coins, 0);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM ledger WHERE source='failed-test-operation'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        let reference = operation(&db, &Some("withdraw-guard".into()))
            .unwrap()
            .unwrap();
        {
            let tx = db.transaction().unwrap();
            risk::consume(&tx, &Some("withdraw-guard".into()), &reference).unwrap();
            record_receipt(&tx, 2, &reference).unwrap();
            tx.commit().unwrap();
        }
        record_receipt(&db, 2, &reference).unwrap();
        let result = receipt(State(state), Path("withdraw-guard".into()))
            .await
            .unwrap()
            .0;
        assert_eq!(result["status"], "pending");
        assert!(!result.to_string().contains("private-test-permit"));
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM risk_receipts WHERE event_id='withdraw-guard'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
