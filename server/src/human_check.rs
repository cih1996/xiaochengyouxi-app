//! Human verification (Tencent Cloud Captcha / 天御验证码) in front of anything that sends an SMS
//! from a public web page. The browser shows the slider/click challenge and gets a one-time ticket;
//! only the server checks it (DescribeCaptchaResult, TC3-signed with the SMS channel's API key).
//! Tickets are single use here as well, so a captured ticket cannot be replayed.
use super::*;

const HOST: &str = "captcha.tencentcloudapi.com";

#[derive(Clone, Serialize, Deserialize, Default)]
pub(super) struct CaptchaConfig {
    pub enabled: bool,
    /// CaptchaAppId from the Tencent Cloud captcha console.
    pub app_id: String,
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS captcha_config(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,app_secret_key TEXT NOT NULL DEFAULT '',updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS captcha_tickets(ticket_hash TEXT PRIMARY KEY,used_at INTEGER NOT NULL);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO captcha_config(id,json,updated_at) VALUES(1,?1,?2)",
        params![serde_json::to_string(&CaptchaConfig::default())?, now()],
    )?;
    Ok(())
}

fn load(db: &Connection) -> Result<(CaptchaConfig, String), ApiError> {
    let (raw, key): (String, String) = db.query_row(
        "SELECT json,app_secret_key FROM captcha_config WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((serde_json::from_str(&raw)?, key))
}

/// CaptchaAppId for the page when verification is fully set up, else None (the page must then
/// not offer SMS at all).
pub(super) fn public_app_id(db: &Connection) -> Result<Option<String>, ApiError> {
    let (cfg, key) = load(db)?;
    Ok((cfg.enabled && !cfg.app_id.is_empty() && !key.is_empty() && sms_channel::cloud_keys(db)?.is_some())
        .then_some(cfg.app_id))
}

fn failed() -> ApiError {
    ApiError(StatusCode::FORBIDDEN, "安全验证未通过，请重新验证".into(), Some("CAPTCHA_FAILED"))
}

pub(super) async fn verify(state: &AppState, ticket: &str, randstr: &str, ip: &str) -> Result<(), ApiError> {
    if ticket.is_empty() || ticket.len() > 4096 || randstr.is_empty() || randstr.len() > 128 {
        return Err(failed());
    }
    let (cfg, app_key, (secret_id, secret_key)) = {
        let db = open(state)?;
        let (cfg, key) = load(&db)?;
        let Some(keys) = sms_channel::cloud_keys(&db)? else {
            return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "安全验证暂不可用".into(), None));
        };
        // Single use: reserve the ticket before asking Tencent.
        let hash: String = Sha256::digest(ticket.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        if db.execute("INSERT OR IGNORE INTO captcha_tickets(ticket_hash,used_at) VALUES(?1,?2)", params![hash, now()])? == 0 {
            return Err(failed());
        }
        db.execute("DELETE FROM captcha_tickets WHERE used_at<?1", [now() - 86400])?;
        (cfg, key, keys)
    };
    let app_id: u64 = cfg.app_id.parse().map_err(|_| failed())?;
    let payload = json!({
        "CaptchaType": 9,
        "Ticket": ticket,
        "UserIp": ip,
        "Randstr": randstr,
        "CaptchaAppId": app_id,
        "AppSecretKey": app_key,
    })
    .to_string();
    let timestamp = now();
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .post(format!("https://{HOST}/"))
        .header(
            "Authorization",
            sms_channel::authorization_for("captcha", HOST, &secret_id, &secret_key, timestamp, &payload),
        )
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Host", HOST)
        .header("X-TC-Action", "DescribeCaptchaResult")
        .header("X-TC-Version", "2019-07-22")
        .header("X-TC-Timestamp", timestamp.to_string())
        .body(payload)
        .send()
        .await;
    let body: Value = match response {
        Ok(r) => r.json().await.unwrap_or(Value::Null),
        Err(e) => {
            eprintln!("人机验证请求失败：{e}");
            return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "安全验证暂不可用，请稍后再试".into(), None));
        }
    };
    let r = &body["Response"];
    if r["CaptchaCode"].as_i64() == Some(1) {
        return Ok(());
    }
    eprintln!(
        "人机验证未通过：code={} msg={} err={}",
        r["CaptchaCode"],
        r["CaptchaMsg"],
        r["Error"]
    );
    Err(failed())
}

// ---------- admin ----------
async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let (cfg, key) = load(&db)?;
    Ok(Json(json!({
        "enabled": cfg.enabled,
        "app_id": cfg.app_id,
        "app_secret_key_set": !key.is_empty(),
        "cloud_keys_set": sms_channel::cloud_keys(&db)?.is_some(),
        "ready": public_app_id(&db)?.is_some(),
    })))
}

#[derive(Deserialize)]
struct AdminInput {
    enabled: bool,
    app_id: String,
    #[serde(default)]
    app_secret_key: String,
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<AdminInput>,
) -> ApiResult<Value> {
    let app_id = input.app_id.trim().to_string();
    if !app_id.is_empty() && (!app_id.bytes().all(|c| c.is_ascii_digit()) || app_id.len() > 20) {
        return Err(invalid("CaptchaAppId 应为数字"));
    }
    let key = input.app_secret_key.trim().to_string();
    if key.len() > 256 {
        return Err(invalid("AppSecretKey 过长"));
    }
    {
        let mut db = open(&state)?;
        let tx = db.transaction()?;
        let (before, old_key) = load(&tx)?;
        let next = CaptchaConfig { enabled: input.enabled, app_id };
        if next.enabled && (next.app_id.is_empty() || (key.is_empty() && old_key.is_empty())) {
            return Err(invalid("开启前请填写 CaptchaAppId 和 AppSecretKey"));
        }
        tx.execute(
            "UPDATE captcha_config SET json=?1,app_secret_key=CASE WHEN ?2='' THEN app_secret_key ELSE ?2 END,updated_at=?3 WHERE id=1",
            params![serde_json::to_string(&next)?, key, now()],
        )?;
        user_admin::action_audit(
            &tx,
            &context,
            None,
            "captcha_config",
            "captcha",
            "修改人机验证配置",
            serde_json::to_value(&before)?,
            json!({"enabled":next.enabled,"app_id":next.app_id,"app_secret_key_changed":!key.is_empty()}),
        )?;
        tx.commit()?;
    }
    admin_get(State(state)).await
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/captcha-config", get(admin_get).put(admin_put))
}
