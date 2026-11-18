//! Verification SMS through Tencent Cloud SMS (SendSms 2021-01-11, TC3-HMAC-SHA256), set up in
//! the admin page. When the channel is off, production still generates a random code but cannot
//! deliver it, and local development keeps the fixed 123456. The app never needs to change: it
//! keeps calling /auth/sms and only autofills when the server returns a code (local mock only).
use super::*;
use ring::hmac;

const HOST: &str = "sms.tencentcloudapi.com";
const REGIONS: &[&str] = &["ap-guangzhou", "ap-beijing", "ap-nanjing"];

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct SmsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub secret_id: String,
    #[serde(default = "guangzhou")]
    pub region: String,
    #[serde(default)]
    pub sdk_app_id: String,
    #[serde(default)]
    pub sign_name: String,
    /// Default verification template; per-purpose templates override it when filled.
    #[serde(default)]
    pub template_id: String,
    #[serde(default)]
    pub register_template_id: String,
    #[serde(default)]
    pub login_template_id: String,
    #[serde(default)]
    pub reset_template_id: String,
    /// Template variables in order: "{code}" and "{minutes}" are replaced, e.g. "{code},{minutes}".
    #[serde(default = "code_only")]
    pub template_params: String,
    #[serde(default = "five")]
    pub expire_minutes: u32,
    #[serde(default = "sixty")]
    pub cooldown_seconds: u32,
    #[serde(default = "ten")]
    pub phone_daily_limit: u32,
    #[serde(default = "thirty")]
    pub ip_hourly_limit: u32,
}
fn guangzhou() -> String {
    "ap-guangzhou".into()
}
fn code_only() -> String {
    "{code}".into()
}
fn five() -> u32 {
    5
}
fn sixty() -> u32 {
    60
}
fn ten() -> u32 {
    10
}
fn thirty() -> u32 {
    30
}
impl Default for SmsConfig {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("defaults")
    }
}
impl SmsConfig {
    fn template(&self, purpose: &str) -> &str {
        let specific = match purpose {
            "register" => &self.register_template_id,
            "login" => &self.login_template_id,
            "reset" => &self.reset_template_id,
            _ => &self.template_id,
        };
        if specific.is_empty() { &self.template_id } else { specific }
    }
    fn params(&self, code: &str) -> Vec<String> {
        self.template_params
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| p.replace("{code}", code).replace("{minutes}", &self.expire_minutes.to_string()))
            .collect()
    }
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS sms_config(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,secret_key TEXT NOT NULL DEFAULT '',revision INTEGER NOT NULL DEFAULT 1,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS sms_log(id INTEGER PRIMARY KEY,created_at INTEGER NOT NULL,phone_hash TEXT NOT NULL,phone_masked TEXT NOT NULL,purpose TEXT NOT NULL,client_ip TEXT NOT NULL,status TEXT NOT NULL,code TEXT NOT NULL DEFAULT '',message TEXT NOT NULL DEFAULT '',request_id TEXT NOT NULL DEFAULT '',serial_no TEXT NOT NULL DEFAULT '');
         CREATE INDEX IF NOT EXISTS idx_sms_log_phone ON sms_log(phone_hash,created_at);
         CREATE INDEX IF NOT EXISTS idx_sms_log_ip ON sms_log(client_ip,created_at);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO sms_config(id,json,updated_at) VALUES(1,?1,?2)",
        params![serde_json::to_string(&SmsConfig::default())?, now()],
    )?;
    db.execute("DELETE FROM sms_log WHERE created_at<?1", [now() - 180 * 86400])?;
    Ok(())
}

fn load(db: &Connection) -> Result<(SmsConfig, String), ApiError> {
    let (raw, key): (String, String) =
        db.query_row("SELECT json,secret_key FROM sms_config WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok((serde_json::from_str(&raw)?, key))
}

/// Tencent Cloud API key pair saved with the SMS channel (also used for the captcha check).
pub(super) fn cloud_keys(db: &Connection) -> Result<Option<(String, String)>, ApiError> {
    let (cfg, key) = load(db)?;
    Ok((!cfg.secret_id.is_empty() && !key.is_empty()).then(|| (cfg.secret_id, key)))
}

/// True when real SMS delivery is switched on and complete.
pub(super) fn active(db: &Connection) -> Result<bool, ApiError> {
    let (cfg, key) = load(db)?;
    Ok(cfg.enabled && ready(&cfg, &key))
}
fn ready(cfg: &SmsConfig, key: &str) -> bool {
    !cfg.secret_id.is_empty()
        && !key.is_empty()
        && !cfg.sdk_app_id.is_empty()
        && !cfg.sign_name.is_empty()
        && !cfg.template_id.is_empty()
}
pub(super) fn expire_seconds(db: &Connection) -> Result<i64, ApiError> {
    Ok(i64::from(load(db)?.0.expire_minutes.clamp(1, 30)) * 60)
}

fn phone_hash(phone: &str) -> String {
    format!("{:x}", Sha256::digest(format!("orange-sms:{phone}").as_bytes()))
}
fn masked(phone: &str) -> String {
    if phone.len() == 11 { format!("{}****{}", &phone[..3], &phone[7..]) } else { "***".into() }
}

/// Frequency limits, checked before a code is generated. Messages are generic for users.
pub(super) fn check_limits(db: &Connection, phone: &str, ip: &str) -> Result<(), ApiError> {
    let (cfg, _) = load(db)?;
    let hash = phone_hash(phone);
    let last: Option<i64> = db
        .query_row(
            "SELECT MAX(created_at) FROM sms_log WHERE phone_hash=?1 AND status IN ('sent','unsent')",
            [&hash],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    if let Some(last) = last {
        let wait = i64::from(cfg.cooldown_seconds) - (now() - last);
        if wait > 0 {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                format!("验证码已发送，请 {wait} 秒后再试"),
                Some("SMS_COOLDOWN"),
            ));
        }
    }
    let phone_today: i64 = db.query_row(
        "SELECT COUNT(*) FROM sms_log WHERE phone_hash=?1 AND created_at>=?2 AND status IN ('sent','unsent')",
        params![hash, today_start()],
        |r| r.get(0),
    )?;
    if cfg.phone_daily_limit > 0 && phone_today >= i64::from(cfg.phone_daily_limit) {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "今日验证码次数已用完，请明天再试".into(), Some("SMS_LIMIT")));
    }
    let ip_hour: i64 = db.query_row(
        "SELECT COUNT(*) FROM sms_log WHERE client_ip=?1 AND created_at>=?2 AND status IN ('sent','unsent')",
        params![ip, now() - 3600],
        |r| r.get(0),
    )?;
    if cfg.ip_hourly_limit > 0 && ip_hour >= i64::from(cfg.ip_hourly_limit) {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "请求过于频繁，请稍后再试".into(), Some("SMS_LIMIT")));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record(
    db: &Connection,
    phone: &str,
    purpose: &str,
    ip: &str,
    status: &str,
    code: &str,
    message: &str,
    request_id: &str,
    serial: &str,
) -> Result<(), ApiError> {
    db.execute(
        "INSERT INTO sms_log(created_at,phone_hash,phone_masked,purpose,client_ip,status,code,message,request_id,serial_no) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![now(), phone_hash(phone), masked(phone), purpose, ip, status, code, message.chars().take(300).collect::<String>(), request_id, serial],
    )?;
    Ok(())
}

/// Production without a configured channel: the code exists but nobody receives it.
pub(super) fn record_unsent(db: &Connection, phone: &str, purpose: &str, ip: &str) -> Result<(), ApiError> {
    record(db, phone, purpose, ip, "unsent", "", "短信通道未开启", "", "")
}

fn hmac256(key: &[u8], data: &str) -> Vec<u8> {
    hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), data.as_bytes()).as_ref().to_vec()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// TC3-HMAC-SHA256 Authorization header for a JSON POST to sms.tencentcloudapi.com.
pub(super) fn authorization(secret_id: &str, secret_key: &str, timestamp: i64, payload: &str) -> String {
    authorization_for("sms", HOST, secret_id, secret_key, timestamp, payload)
}

/// TC3-HMAC-SHA256 for any Tencent Cloud API (service name + host), e.g. captcha.
pub(super) fn authorization_for(
    service: &str,
    host: &str,
    secret_id: &str,
    secret_key: &str,
    timestamp: i64,
    payload: &str,
) -> String {
    let date = chrono::DateTime::from_timestamp(timestamp, 0).unwrap().format("%Y-%m-%d").to_string();
    let canonical = format!(
        "POST\n/\n\ncontent-type:application/json; charset=utf-8\nhost:{host}\n\ncontent-type;host\n{}",
        hex(&Sha256::digest(payload.as_bytes()))
    );
    let scope = format!("{date}/{service}/tc3_request");
    let to_sign = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        hex(&Sha256::digest(canonical.as_bytes()))
    );
    let k_date = hmac256(format!("TC3{secret_key}").as_bytes(), &date);
    let k_service = hmac256(&k_date, service);
    let k_signing = hmac256(&k_service, "tc3_request");
    let signature = hex(&hmac256(&k_signing, &to_sign));
    format!("TC3-HMAC-SHA256 Credential={secret_id}/{scope}, SignedHeaders=content-type;host, Signature={signature}")
}

/// Send one verification code. Errors are logged in full for the admin and returned generically.
pub(super) async fn send(state: &AppState, phone: &str, purpose: &str, code: &str, ip: &str) -> Result<(), ApiError> {
    let (cfg, key) = load(&open(state)?)?;
    if !ready(&cfg, &key) {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "短信服务暂不可用，请稍后再试".into(), None));
    }
    let payload = json!({
        "PhoneNumberSet": [format!("+86{phone}")],
        "SmsSdkAppId": cfg.sdk_app_id,
        "SignName": cfg.sign_name,
        "TemplateId": cfg.template(purpose),
        "TemplateParamSet": cfg.params(code),
        "SessionContext": purpose,
    })
    .to_string();
    let timestamp = now();
    let result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .post(format!("https://{HOST}/"))
        .header("Authorization", authorization(&cfg.secret_id, &key, timestamp, &payload))
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Host", HOST)
        .header("X-TC-Action", "SendSms")
        .header("X-TC-Version", "2021-01-11")
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Region", &cfg.region)
        .body(payload)
        .send()
        .await;
    let (status, code_text, message, request_id, serial) = match result {
        Err(error) => ("failed", "network".to_string(), error.to_string(), String::new(), String::new()),
        Ok(response) => match response.json::<Value>().await {
            Err(error) => ("failed", "parse".to_string(), error.to_string(), String::new(), String::new()),
            Ok(body) => {
                let r = &body["Response"];
                let request_id = r["RequestId"].as_str().unwrap_or("").to_string();
                if let Some(err) = r.get("Error") {
                    ("failed", err["Code"].as_str().unwrap_or("").into(), err["Message"].as_str().unwrap_or("").into(), request_id, String::new())
                } else {
                    let s = &r["SendStatusSet"][0];
                    let c = s["Code"].as_str().unwrap_or("").to_string();
                    (
                        if c == "Ok" { "sent" } else { "failed" },
                        c,
                        s["Message"].as_str().unwrap_or("").into(),
                        request_id,
                        s["SerialNo"].as_str().unwrap_or("").into(),
                    )
                }
            }
        },
    };
    record(&open(state)?, phone, purpose, ip, status, &code_text, &message, &request_id, &serial)?;
    if status != "sent" {
        eprintln!("短信发送失败：{code_text} {message} RequestId={request_id}");
        let user_message = if code_text.starts_with("LimitExceeded") {
            "验证码发送过于频繁，请稍后再试"
        } else {
            "短信发送失败，请稍后再试"
        };
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, user_message.into(), Some("SMS_FAILED")));
    }
    Ok(())
}

// ---------- admin ----------

fn admin_json(db: &Connection) -> Result<Value, ApiError> {
    let (raw, key, revision, updated): (String, String, i64, i64) = db.query_row(
        "SELECT json,secret_key,revision,updated_at FROM sms_config WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let cfg: SmsConfig = serde_json::from_str(&raw)?;
    let mut value = serde_json::to_value(&cfg)?;
    value["secret_key_set"] = json!(!key.is_empty());
    value["ready"] = json!(ready(&cfg, &key));
    value["active"] = json!(cfg.enabled && ready(&cfg, &key));
    value["revision"] = json!(revision);
    value["updated_at"] = json!(iso(updated));
    value["regions"] = json!(REGIONS);
    let mut stats = serde_json::Map::new();
    for status in ["sent", "failed", "unsent"] {
        let n: i64 = db.query_row(
            "SELECT COUNT(*) FROM sms_log WHERE status=?1 AND created_at>=?2",
            params![status, today_start()],
            |r| r.get(0),
        )?;
        stats.insert(status.into(), json!(n));
    }
    value["today"] = Value::Object(stats);
    Ok(value)
}

#[derive(Deserialize)]
struct AdminInput {
    #[serde(flatten)]
    config: SmsConfig,
    /// A new SecretKey; empty keeps the stored one.
    #[serde(default)]
    secret_key: String,
}

fn validate(input: &mut AdminInput) -> Result<(), ApiError> {
    let c = &mut input.config;
    for field in [
        &mut c.secret_id,
        &mut c.sdk_app_id,
        &mut c.sign_name,
        &mut c.template_id,
        &mut c.register_template_id,
        &mut c.login_template_id,
        &mut c.reset_template_id,
        &mut c.template_params,
    ] {
        *field = field.trim().to_owned();
    }
    input.secret_key = input.secret_key.trim().to_owned();
    if !REGIONS.contains(&c.region.as_str()) {
        return Err(invalid("地域应为广州、北京或南京"));
    }
    if c.secret_id.len() > 64 || input.secret_key.len() > 64 {
        return Err(invalid("SecretId/SecretKey 长度不正确"));
    }
    if !c.sdk_app_id.is_empty() && !(c.sdk_app_id.len() <= 12 && c.sdk_app_id.chars().all(|ch| ch.is_ascii_digit())) {
        return Err(invalid("SdkAppId 应为数字，例如 1400006666"));
    }
    for t in [&c.template_id, &c.register_template_id, &c.login_template_id, &c.reset_template_id] {
        if !t.is_empty() && !(t.len() <= 12 && t.chars().all(|ch| ch.is_ascii_digit())) {
            return Err(invalid("模板 ID 应为数字"));
        }
    }
    let sign = c.sign_name.chars().count();
    if !c.sign_name.is_empty() && !(2..=12).contains(&sign) {
        return Err(invalid("短信签名应为 2 至 12 个字"));
    }
    if !c.template_params.contains("{code}") {
        return Err(invalid("模板参数必须包含 {code}"));
    }
    if c.template_params.split(',').count() > 5 {
        return Err(invalid("模板参数最多 5 个"));
    }
    c.expire_minutes = c.expire_minutes.clamp(1, 30);
    c.cooldown_seconds = c.cooldown_seconds.clamp(30, 600);
    c.phone_daily_limit = c.phone_daily_limit.min(100);
    c.ip_hourly_limit = c.ip_hourly_limit.min(1000);
    Ok(())
}

async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(admin_json(&open(&state)?)?))
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<AdminInput>,
) -> ApiResult<Value> {
    validate(&mut input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let (_, stored_key) = load(&tx)?;
    let key = if input.secret_key.is_empty() { stored_key } else { input.secret_key.clone() };
    if input.config.enabled && !ready(&input.config, &key) {
        return Err(invalid("开启前请填写 SecretId、SecretKey、SdkAppId、签名和默认模板 ID"));
    }
    let before = admin_json(&tx)?;
    tx.execute(
        "UPDATE sms_config SET json=?1,secret_key=?2,revision=revision+1,updated_at=?3 WHERE id=1",
        params![serde_json::to_string(&input.config)?, key, now()],
    )?;
    let mut after = admin_json(&tx)?;
    after["secret_key_changed"] = json!(!input.secret_key.is_empty());
    user_admin::action_audit(&tx, &context, None, "sms_config_update", "sms-config", "修改短信通道", before, after.clone())?;
    tx.commit()?;
    Ok(Json(after))
}

#[derive(Deserialize)]
struct TestInput {
    phone: String,
}

/// Sends a real code with the stored settings (works before the switch is turned on).
async fn admin_test(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<TestInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    use rand_core::RngCore;
    let code = format!("{:06}", OsRng.next_u32() % 1_000_000);
    let result = send(&state, &input.phone, "test", &code, &context.client_ip).await;
    let db = open(&state)?;
    user_admin::action_audit(&db, &context, None, "sms_test", "sms-config", "发送测试短信", json!({}), json!({"phone":masked(&input.phone),"ok":result.is_ok()}))?;
    let last: (String, String, String) = db.query_row(
        "SELECT status,code,message FROM sms_log ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    // The admin sees the platform's own code and message to fix configuration problems.
    Ok(Json(json!({"ok":result.is_ok(),"status":last.0,"code":last.1,"message":last.2})))
}

#[derive(Deserialize)]
struct LogQuery {
    page: Option<i64>,
    status: Option<String>,
}

async fn admin_logs(State(state): State<AppState>, Query(q): Query<LogQuery>) -> ApiResult<Value> {
    let db = open(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let status = q.status.filter(|s| !s.is_empty());
    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM sms_log WHERE (?1 IS NULL OR status=?1)",
        [&status],
        |r| r.get(0),
    )?;
    let mut stmt = db.prepare(
        "SELECT created_at,phone_masked,purpose,client_ip,status,code,message,request_id FROM sms_log WHERE (?1 IS NULL OR status=?1) ORDER BY id DESC LIMIT 50 OFFSET ?2",
    )?;
    let items = stmt
        .query_map(params![status, (page - 1) * 50], |r| {
            Ok(json!({"created_at":iso(r.get(0)?),"phone":r.get::<_,String>(1)?,"purpose":r.get::<_,String>(2)?,"client_ip":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"code":r.get::<_,String>(5)?,"message":r.get::<_,String>(6)?,"request_id":r.get::<_,String>(7)?}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({"items":items,"total":total,"page":page,"page_size":50})))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/sms-config", get(admin_get).put(admin_put))
        .route("/sms-config/test", post(admin_test))
        .route("/sms-logs", get(admin_logs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc3_signature_matches_reference_implementation() {
        // Expected value computed with an independent Python (hmac/hashlib) implementation
        // of the documented TC3-HMAC-SHA256 steps for the same inputs.
        let payload = r#"{"PhoneNumberSet":["+8618501234444"],"SmsSdkAppId":"1400006666","SignName":"腾讯云","TemplateId":"1110","TemplateParamSet":["4370"]}"#;
        let auth = authorization("AKIDEXAMPLE", "SECRETEXAMPLE", 1_700_000_000, payload);
        assert_eq!(auth, include_str!("../tests/fixtures/tc3-expected.txt").trim());
    }

    #[test]
    fn template_params_and_purpose_templates() {
        let mut cfg = SmsConfig::default();
        cfg.template_id = "100".into();
        cfg.reset_template_id = "200".into();
        cfg.template_params = "{code}, {minutes}".into();
        assert_eq!(cfg.params("123456"), vec!["123456", "5"]);
        assert_eq!(cfg.template("register"), "100");
        assert_eq!(cfg.template("reset"), "200");
        assert_eq!(masked("13800138000"), "138****8000");
    }
}
