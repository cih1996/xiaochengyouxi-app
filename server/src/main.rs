use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json, Router,
    extract::{Extension, OriginalUri, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use chrono::{NaiveDate, SecondsFormat, Utc};
use rand_core::OsRng;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, convert::Infallible, env, fs, path::PathBuf, time::Duration};
use tokio::time::Instant;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use uuid::Uuid;
mod ad_mediation;
mod admin_account;
mod app_config;
mod behavior_audit;
mod cos_storage;
mod business_guard;
mod campaign;
mod device_bootstrap;
mod hall_requests;
mod home_layout;
mod invitation_graph;
mod invitations;
mod mail_admin;
mod mail_events;
mod product_admin;
mod rain;
mod reward_policy;
mod risk;
mod human_check;
mod newbie;
mod sms_channel;
mod trace_lookup;
mod user_admin;
mod user_stats;
mod withdraw_rules;
use std::net::SocketAddr;
use user_admin::{RequestContext, client_ip};
const COINS_PER_YUAN: i64 = 1000;

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    config: RuntimeConfig,
}
#[derive(Clone)]
struct RuntimeConfig {
    production: bool,
    ad_mode: String,
    csj_reward_secret: Option<String>,
    admin_username: String,
    admin_password: Option<String>,
    mail: mail_admin::Config,
    mail_events: mail_events::Config,
    app_download_url: Option<String>,
    privacy_url: Option<String>,
    invite_base_url: String,
    device_trust: Option<risk::Config>,
    review_login_phone: Option<String>,
    review_login_code: Option<String>,
}
fn env_value(key: &str) -> Option<String> {
    env::var(key).ok().filter(|s| !s.trim().is_empty())
}
impl RuntimeConfig {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let environment = env_value("ORANGE_ENV").unwrap_or_else(|| "local".into());
        if !["local", "production"].contains(&environment.as_str()) {
            return Err("ORANGE_ENV 应为 local 或 production".into());
        }
        let result = Self {
            production: environment == "production",
            ad_mode: env_value("ORANGE_AD_MODE").unwrap_or_else(|| "mock".into()),
            csj_reward_secret: env_value("CSJ_REWARD_SECRET"),
            admin_username: env_value("ORANGE_ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
            admin_password: env_value("ORANGE_ADMIN_PASSWORD"),
            mail: mail_admin::Config::from_env(),
            mail_events: mail_events::Config::from_env(),
            app_download_url: env_value("ORANGE_APP_DOWNLOAD_URL"),
            privacy_url: env_value("ORANGE_PRIVACY_URL").or_else(|| Some("/privacy".into())),
            invite_base_url: env_value("ORANGE_INVITE_BASE_URL")
                .unwrap_or_else(|| "https://orange.lingzidev.com".into()),
            device_trust: risk::Config::from_env()?,
            review_login_phone: env_value("ORANGE_REVIEW_LOGIN_PHONE"),
            review_login_code: env_value("ORANGE_REVIEW_LOGIN_CODE"),
        };
        if !["mock", "csj"].contains(&result.ad_mode.as_str()) {
            return Err("ORANGE_AD_MODE 应为 mock 或 csj".into());
        }
        if result.production && result.ad_mode != "csj" {
            return Err("production 环境需要设置 ORANGE_AD_MODE=csj".into());
        }
        if result.ad_mode == "csj" && result.csj_reward_secret.is_none() {
            return Err("csj 广告模式需要设置 CSJ_REWARD_SECRET".into());
        }
        Ok(result)
    }
}
fn equal_secret(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[derive(Debug)]
struct ApiError(StatusCode, String, Option<&'static str>);
type ApiResult<T> = Result<Json<T>, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.0.is_server_error() {
            eprintln!("{}", self.1);
        }
        let mut body = json!({"error":self.1});
        if self.0 == StatusCode::FORBIDDEN {
            body["code"] = json!("INVITE_REQUIRED");
        }
        if self.0 == StatusCode::GONE {
            body["code"] = json!("QUOTE_EXPIRED");
        }
        if let Some(code) = self.2 {
            body["code"] = json!(code);
        }
        (self.0, Json(body)).into_response()
    }
}
impl From<rusqlite::Error> for ApiError {
    fn from(error: rusqlite::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None)
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None)
    }
}
fn invalid(message: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.into(), None)
}
fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "请先登录".into(), None)
}
fn conflict(message: &str) -> ApiError {
    ApiError(StatusCode::CONFLICT, message.into(), None)
}
fn now() -> i64 {
    Utc::now().timestamp()
}
fn iso(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .unwrap()
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
fn day() -> String {
    (Utc::now() + chrono::Duration::hours(8))
        .format("%Y-%m-%d")
        .to_string()
}
fn today_start() -> i64 {
    (Utc::now().timestamp() + 8 * 3600) / 86400 * 86400 - 8 * 3600
}
fn open(state: &AppState) -> Result<Connection, ApiError> {
    Ok(Connection::open(&state.db_path)?)
}
async fn public_config(State(state): State<AppState>) -> ApiResult<Value> {
    let config = &state.config;
    let mut result = json!({"app_name":"小橙有喜","operator_name":"广西奇易电子商务有限公司","android_package":"com.lingzidev.xiaochengyouxi","environment":if config.production {"production"} else {"local"},"admin_auth_required":admin_account::required(&state,&open(&state)?)?,"ad_mode":config.ad_mode,"sms_mode":"mock","payment_mode":"mock","invite_required":true,"invite_sms_required":false});
    result["device_trust_enabled"] = json!(config.device_trust.is_some());
    result["ads"] = ad_mediation::public_networks(&open(&state)?)?;
    result["sms_mode"] = json!(if sms_channel::active(&open(&state)?)? { "tencent" } else { "mock" });
    result["auth"] = json!({"register_mode":register_mode(&open(&state)?)?,"login_default":"password","login_methods":["password","sms"]});
    if let Some(url) = &config.app_download_url {
        result["app_download_url"] = json!(url);
    }
    if let Some(url) = &config.privacy_url {
        result["privacy_url"] = json!(url);
    }
    Ok(Json(result))
}

#[derive(Serialize)]
struct User {
    id: i64,
    phone: String,
    nickname: String,
    coins: i64,
    total_earned: i64,
    today_earned: i64,
    cash_yuan: f64,
    withdraw_min_yuan: f64,
    checked_in_today: bool,
    invite_code: String,
    invitation_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    inviter_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
}

fn user(db: &Connection, id: i64) -> Result<User, ApiError> {
    let mut result = db
        .query_row(
            "SELECT id,phone,nickname,coins,invite_code,invitation_bound_at,inviter_code,username FROM users WHERE id=?1",
            [id],
            |r| {
                Ok(User {
                    id: r.get(0)?,
                    // Username accounts have no phone; hide the internal placeholder.
                    phone: r.get::<_, String>(1).map(|p| if p.starts_with("u:") { String::new() } else { p })?,
                    nickname: r.get(2)?,
                    coins: r.get(3)?,
                    total_earned: 0,
                    today_earned: 0,
                    cash_yuan: 0.0,
                    withdraw_min_yuan: 0.0,
                    checked_in_today:false,
                    invite_code:r.get(4)?,
                    invitation_required:r.get::<_,Option<i64>>(5)?.is_none(),
                    inviter_code:r.get(6)?,
                    username:r.get(7)?,
                })
            },
        )
        .optional()?
        .ok_or_else(unauthorized)?;
    result.total_earned = db.query_row("SELECT COALESCE(SUM(coins),0) FROM ledger WHERE user_id=?1 AND coins>0 AND source NOT LIKE 'withdraw-refund:%'", [id], |r| r.get(0))?;
    result.today_earned = db.query_row("SELECT COALESCE(SUM(coins),0) FROM ledger WHERE user_id=?1 AND coins>0 AND source NOT LIKE 'withdraw-refund:%' AND created_at>=?2", params![id,today_start()], |r| r.get(0))?;
    result.cash_yuan = result.coins as f64 / COINS_PER_YUAN as f64;
    result.withdraw_min_yuan = withdraw_rules::minimum(db)?;
    result.checked_in_today = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM ledger WHERE user_id=?1 AND source=?2)",
        params![id, format!("check-in:{}:{id}", day())],
        |r| r.get(0),
    )?;
    Ok(result)
}

fn token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)
}
fn account_auth(db: &Connection, headers: &HeaderMap) -> Result<i64, ApiError> {
    db.query_row(
        "SELECT user_id FROM sessions WHERE token=?1 AND expires_at>?2",
        params![token(headers)?, now()],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(unauthorized)
}
fn auth(db: &Connection, headers: &HeaderMap) -> Result<i64, ApiError> {
    let uid = account_auth(db, headers)?;
    invitations::require_bound(db, uid)?;
    Ok(uid)
}
fn password_hash(password: &str) -> Result<String, ApiError> {
    if password.len() < 8 {
        return Err(invalid("密码至少需要 8 位"));
    }
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|x| x.to_string())
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))
}
/// The fixed app-store review account (ORANGE_REVIEW_LOGIN_PHONE). Reviewers use rooted or
/// debug-enabled test devices, so automatic risk restrictions and the quiet ad block skip this
/// account; risk events are still recorded for the admin.
pub(crate) fn is_review_account(db: &Connection, uid: i64) -> Result<bool, ApiError> {
    let Some(review) = env_value("ORANGE_REVIEW_LOGIN_PHONE") else {
        return Ok(false);
    };
    let phone: Option<String> = db
        .query_row("SELECT phone FROM users WHERE id=?1", [uid], |r| r.get(0))
        .optional()?;
    Ok(phone.is_some_and(|p| p == review))
}
/// Accounts whose risk hits are approved automatically instead of restricted: the store review
/// account, and accounts an admin has explicitly set back to normal ("人工放行"). Returns the
/// note sent to DeviceTrust with the approval. Automatic sources never count as a manual release.
pub(crate) fn trusted_override(db: &Connection, uid: i64) -> Result<Option<String>, ApiError> {
    if is_review_account(db, uid)? {
        return Ok(Some("应用市场审核账号，自动放行".into()));
    }
    let manual: Option<(String, String)> = db
        .query_row(
            "SELECT updated_by,reason FROM account_restrictions WHERE user_id=?1 AND status='active'",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(manual
        .filter(|(by, _)| !["DeviceTrust", "BehaviorAudit"].contains(&by.as_str()))
        .map(|(by, reason)| format!("后台人工放行（{by}）：{reason}")))
}
fn validate_phone(phone: &str) -> Result<(), ApiError> {
    if phone.len() != 11 || !phone.starts_with('1') || !phone.bytes().all(|c| c.is_ascii_digit()) {
        return Err(invalid("请输入 11 位手机号"));
    }
    Ok(())
}
/// Usernames are case-insensitive, start with a letter and can never look like a phone number.
fn normalize_username(value: &str) -> Result<String, ApiError> {
    let name = value.trim().to_ascii_lowercase();
    let valid = (4..=20).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !valid {
        return Err(invalid("用户名为 4 至 20 位字母、数字或下划线，并以字母开头"));
    }
    Ok(name)
}
/// Registration method chosen in the admin settings: "phone" (phone + SMS code) or "username".
fn register_mode(db: &Connection) -> Result<&'static str, ApiError> {
    Ok(if setting(db, "register_mode")? >= 0.5 { "username" } else { "phone" })
}
fn new_session(db: &Connection, user_id: i64) -> Result<Value, ApiError> {
    let token = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO sessions(token,user_id,expires_at) VALUES (?1,?2,?3)",
        params![token, user_id, now() + 30 * 86400],
    )?;
    Ok(json!({"token":token,"user":user(db,user_id)?}))
}
/// Checks an SMS code. Each code allows at most 5 wrong guesses, then it is void (a 6-digit code
/// cannot be brute-forced within its lifetime).
pub(crate) fn sms_code_ok(db: &Connection, phone: &str, purpose: &str, code: &str) -> Result<bool, ApiError> {
    let row: Option<(String, i64, i64)> = db
        .query_row(
            "SELECT code,expires_at,attempts FROM sms_codes WHERE phone=?1 AND purpose=?2",
            params![phone, purpose],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((expected, expires, attempts)) = row else {
        return Ok(false);
    };
    if expires <= now() || attempts >= 5 {
        return Ok(false);
    }
    if equal_secret(&expected, code.trim()) {
        return Ok(true);
    }
    db.execute(
        "UPDATE sms_codes SET attempts=attempts+1 WHERE phone=?1 AND purpose=?2",
        params![phone, purpose],
    )?;
    Ok(false)
}
fn verify_sms(db: &Connection, phone: &str, code: &str, purpose: &str) -> Result<(), ApiError> {
    if !sms_code_ok(db, phone, purpose, code)? {
        return Err(invalid("验证码错误或已过期，请重新获取"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct SmsInput {
    phone: String,
    purpose: String,
}
async fn sms(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<SmsInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    let purpose = match input.purpose.as_str() {
        "register" => "register",
        "login" => "login",
        "reset" | "reset-password" => "reset",
        _ => return Err(invalid("验证码用途应为 register 或 reset")),
    };
    // Real delivery once the admin turns the Tencent Cloud channel on. Without it, local
    // development keeps a fixed, returned code; production generates a random code that is
    // never returned or logged (so nothing changes in the app when the channel is switched on).
    let (channel, expires) = {
        let db = open(&state)?;
        let channel = sms_channel::active(&db)?;
        if channel || state.config.production {
            sms_channel::check_limits(&db, &input.phone, &context.client_ip)?;
        }
        (channel, sms_channel::expire_seconds(&db)?)
    };
    let mock = !channel && !state.config.production;
    let code = if mock {
        "123456".to_string()
    } else {
        use rand_core::RngCore;
        format!("{:06}", OsRng.next_u32() % 1_000_000)
    };
    if channel {
        sms_channel::send(&state, &input.phone, purpose, &code, &context.client_ip).await?;
    }
    let db = open(&state)?;
    db.execute("INSERT INTO sms_codes(phone,purpose,code,expires_at) VALUES(?1,?2,?3,?4) ON CONFLICT(phone,purpose) DO UPDATE SET code=excluded.code,expires_at=excluded.expires_at,attempts=0", params![input.phone,purpose,code,now()+expires])?;
    if mock {
        return Ok(Json(json!({"code":code,"expires_in":expires,"mode":"mock"})));
    }
    if !channel {
        sms_channel::record_unsent(&db, &input.phone, purpose, &context.client_ip)?;
        eprintln!("短信通道未开启：{purpose} 验证码已生成但未实际下发");
    }
    Ok(Json(json!({"expires_in":expires,"sent":true})))
}
#[derive(Deserialize)]
struct RegisterInput {
    #[serde(default)]
    phone: String,
    password: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    username: Option<String>,
    nickname: Option<String>,
}
async fn register(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<RegisterInput>,
) -> ApiResult<Value> {
    if register_mode(&open(&state)?)? == "username" {
        return register_username(&state, &context, &input);
    }
    validate_phone(&input.phone)?;
    let hash = password_hash(&input.password)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    verify_sms(&tx, &input.phone, &input.code, "register")?;
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE phone=?1)",
        [&input.phone],
        |r| r.get(0),
    )?;
    if exists {
        return Err(conflict("手机号已注册，请直接登录"));
    }
    let nickname = input
        .nickname
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("小橙{}", &input.phone[7..]));
    tx.execute(
        "INSERT INTO users(phone,password_hash,nickname,coins,created_at,invite_code) VALUES(?1,?2,?3,0,?4,?5)",
        params![input.phone, hash, nickname, now(),invitations::new_code(&tx)?],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "DELETE FROM sms_codes WHERE phone=?1 AND purpose='register'",
        [&input.phone],
    )?;
    invitations::consume_pending(&tx, id)?;
    user_admin::behavior(
        &tx,
        &context,
        Some(id),
        Some(&input.phone),
        "register",
        "success",
        &id.to_string(),
        None,
        json!({}),
    )?;
    let result = new_session(&tx, id)?;
    tx.commit()?;
    Ok(Json(result))
}
/// Username + password registration. The phone column keeps its NOT NULL/UNIQUE contract with a
/// "u:<username>" placeholder that can never match a real 11-digit number; it is hidden from clients.
fn register_username(
    state: &AppState,
    context: &RequestContext,
    input: &RegisterInput,
) -> ApiResult<Value> {
    let username = normalize_username(input.username.as_deref().unwrap_or(""))?;
    let hash = password_hash(&input.password)?;
    let placeholder = format!("u:{username}");
    let mut db = open(state)?;
    let tx = db.transaction()?;
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username=?1 OR phone=?2)",
        params![username, placeholder],
        |r| r.get(0),
    )?;
    if exists {
        return Err(conflict("用户名已被使用，请换一个"));
    }
    tx.execute(
        "INSERT INTO users(phone,password_hash,nickname,coins,created_at,invite_code,username) VALUES(?1,?2,?3,0,?4,?5,?6)",
        params![placeholder, hash, username, now(), invitations::new_code(&tx)?, username],
    )?;
    let id = tx.last_insert_rowid();
    user_admin::behavior(
        &tx,
        context,
        Some(id),
        None,
        "register",
        "success",
        &id.to_string(),
        None,
        json!({"method":"username"}),
    )?;
    let result = new_session(&tx, id)?;
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct LoginInput {
    phone: String,
    password: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    captcha_id: Option<String>,
    #[serde(default)]
    captcha_answer: Option<String>,
}

#[derive(Deserialize)]
struct CaptchaInput {
    phone: String,
}

fn login_guard_key(context: &RequestContext, phone: &str) -> String {
    format!("{}:{}", context.client_ip, phone)
}

fn captcha_required(db: &Connection, key: &str) -> Result<bool, ApiError> {
    Ok(db
        .query_row(
            "SELECT COALESCE(failures,0)>=5 FROM login_guard WHERE guard_key=?1",
            [key],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(false))
}

fn issue_login_captcha(db: &Connection, key: &str) -> Result<Value, ApiError> {
    let a = (Uuid::new_v4().as_u128() % 8 + 1) as i64;
    let b = (Uuid::new_v4().as_u128() % 8 + 1) as i64;
    let id = Uuid::new_v4().to_string();
    db.execute("UPDATE login_guard SET captcha_id=?1,captcha_answer=?2,captcha_expires_at=?3 WHERE guard_key=?4", params![id, a+b, now()+300, key])?;
    Ok(json!({"captcha_id":id,"question":format!("{} + {} = ?",a,b),"expires_in":300}))
}

fn verify_login_captcha(
    db: &Connection,
    key: &str,
    id: Option<&str>,
    answer: Option<&str>,
) -> Result<(), ApiError> {
    let Some(id) = id.filter(|v| !v.trim().is_empty()) else {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "请先完成安全验证".into(),
            Some("LOGIN_CAPTCHA_REQUIRED"),
        ));
    };
    let Some(answer) = answer else {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "请先完成安全验证".into(),
            Some("LOGIN_CAPTCHA_REQUIRED"),
        ));
    };
    let ok: bool = db.query_row("SELECT captcha_id=?1 AND captcha_answer=?2 AND captcha_expires_at>?3 FROM login_guard WHERE guard_key=?4",params![id,answer.trim(),now(),key],|r|r.get(0))?;
    if !ok {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "安全验证错误或已过期".into(),
            Some("LOGIN_CAPTCHA_REQUIRED"),
        ));
    }
    db.execute("UPDATE login_guard SET captcha_id=NULL,captcha_answer=NULL,captcha_expires_at=NULL WHERE guard_key=?1",[key])?;
    Ok(())
}

fn record_login_failure(db: &Connection, key: &str) -> Result<bool, ApiError> {
    db.execute("INSERT INTO login_guard(guard_key,failures,last_failed_at) VALUES(?1,1,?2) ON CONFLICT(guard_key) DO UPDATE SET failures=CASE WHEN login_guard.last_failed_at<?2-900 THEN 1 ELSE login_guard.failures+1 END,last_failed_at=?2",params![key,now()])?;
    captcha_required(db, key)
}

fn clear_login_failures(db: &Connection, key: &str) -> Result<(), ApiError> {
    db.execute("DELETE FROM login_guard WHERE guard_key=?1", [key])?;
    Ok(())
}

async fn login_captcha(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<CaptchaInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    let db = open(&state)?;
    let key = login_guard_key(&context, &input.phone);
    if !captcha_required(&db, &key)? {
        return Ok(Json(json!({"required":false})));
    }
    Ok(Json(
        json!({"required":true,"captcha":issue_login_captcha(&db,&key)?}),
    ))
}

fn review_login_enabled(state: &AppState, input: &LoginInput) -> bool {
    let Some(phone) = state.config.review_login_phone.as_deref() else {
        return false;
    };
    let Some(code) = state.config.review_login_code.as_deref() else {
        return false;
    };
    if !equal_secret(phone, &input.phone) {
        return false;
    }
    input
        .code
        .as_deref()
        .is_some_and(|value| equal_secret(code, value))
        || equal_secret(code, &input.password)
}

fn review_login(
    state: &AppState,
    context: &RequestContext,
    input: &LoginInput,
) -> ApiResult<Value> {
    let mut db = open(state)?;
    let tx = db.transaction()?;
    let id: i64 = if let Some(id) = tx
        .query_row("SELECT id FROM users WHERE phone=?1", [&input.phone], |r| {
            r.get(0)
        })
        .optional()?
    {
        tx.execute(
            "UPDATE users SET invitation_bound_at=COALESCE(invitation_bound_at,?1),inviter_code=COALESCE(inviter_code,'REVIEW') WHERE id=?2",
            params![now(), id],
        )?;
        id
    } else {
        let hash = password_hash(&Uuid::new_v4().to_string())?;
        let invite_code = invitations::new_code(&tx)?;
        tx.execute(
            "INSERT INTO users(phone,password_hash,nickname,coins,created_at,invite_code,invitation_bound_at,inviter_code) VALUES(?1,?2,'审核体验用户',0,?3,?4,?3,'REVIEW')",
            params![input.phone, hash, now(), invite_code],
        )?;
        tx.last_insert_rowid()
    };
    user_admin::behavior(
        &tx,
        context,
        Some(id),
        Some(&input.phone),
        "review_login",
        "success",
        &id.to_string(),
        None,
        json!({"source":"server_configured_review_account"}),
    )?;
    let result = new_session(&tx, id)?;
    tx.commit()?;
    Ok(Json(result))
}
async fn login(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<LoginInput>,
) -> ApiResult<Value> {
    if review_login_enabled(&state, &input) {
        return review_login(&state, &context, &input);
    }
    let mut db = open(&state)?;
    let guard_key = login_guard_key(&context, &input.phone);
    if captcha_required(&db, &guard_key)? {
        verify_login_captcha(
            &db,
            &guard_key,
            input.captcha_id.as_deref(),
            input.captcha_answer.as_deref(),
        )?;
    }
    let code_login = input
        .code
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    // The "phone" field carries the account: an 11-digit phone number or a username.
    let account = input.phone.trim().to_string();
    let by_phone = validate_phone(&account).is_ok();
    if code_login.is_some() && !by_phone {
        return Err(invalid("验证码登录请输入 11 位手机号"));
    }
    let credentials: Option<(i64, String)> = if by_phone {
        db.query_row(
            "SELECT id,password_hash FROM users WHERE phone=?1",
            [&account],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    } else {
        db.query_row(
            "SELECT id,password_hash FROM users WHERE username=?1",
            [account.to_ascii_lowercase()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    };
    // SMS login with an unregistered phone signs the user up (same as phone registration),
    // as long as the code is right. The random password can be set later via 找回密码.
    let credentials = match (credentials, code_login) {
        (None, Some(code)) if by_phone => {
            let code_ok = sms_code_ok(&db, &account, "login", code)?;
            if !code_ok {
                let needs_captcha = record_login_failure(&db, &guard_key)?;
                return Err(if needs_captcha {
                    ApiError(
                        StatusCode::TOO_MANY_REQUESTS,
                        "尝试次数过多，请完成安全验证".into(),
                        Some("LOGIN_CAPTCHA_REQUIRED"),
                    )
                } else {
                    ApiError(StatusCode::UNAUTHORIZED, "手机号或验证码错误".into(), None)
                });
            }
            use rand_core::RngCore;
            let mut seed = [0u8; 24];
            OsRng.fill_bytes(&mut seed);
            let hash = password_hash(&seed.iter().map(|b| format!("{b:02x}")).collect::<String>())?;
            let tx = db.transaction()?;
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM users WHERE phone=?1)",
                [&account],
                |r| r.get(0),
            )?;
            if exists {
                return Err(conflict("请重试登录"));
            }
            tx.execute(
                "INSERT INTO users(phone,password_hash,nickname,coins,created_at,invite_code) VALUES(?1,?2,?3,0,?4,?5)",
                params![account, hash, format!("小橙{}", &account[7..]), now(), invitations::new_code(&tx)?],
            )?;
            let id = tx.last_insert_rowid();
            user_admin::behavior(
                &tx,
                &context,
                Some(id),
                Some(&account),
                "register",
                "success",
                &id.to_string(),
                None,
                json!({"via":"sms_login"}),
            )?;
            tx.commit()?;
            Some((id, hash))
        }
        (found, _) => found,
    };
    let Some((id, hash)) = credentials else {
        let needs_captcha = record_login_failure(&db, &guard_key)?;
        user_admin::behavior(
            &db,
            &context,
            None,
            Some(&input.phone),
            "login",
            "failed",
            "",
            None,
            json!({"reason":"invalid_credentials"}),
        )?;
        return Err(if needs_captcha {
            ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "尝试次数过多，请完成安全验证".into(),
                Some("LOGIN_CAPTCHA_REQUIRED"),
            )
        } else {
            ApiError(StatusCode::UNAUTHORIZED, "账号或密码错误".into(), None)
        });
    };
    let valid = if let Some(code) = code_login {
        sms_code_ok(&db, &input.phone, "login", code)?
    } else {
        let parsed = PasswordHash::new(&hash)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
        Argon2::default()
            .verify_password(input.password.as_bytes(), &parsed)
            .is_ok()
    };
    if !valid {
        let needs_captcha = record_login_failure(&db, &guard_key)?;
        user_admin::behavior(
            &db,
            &context,
            Some(id),
            Some(&input.phone),
            "login",
            "failed",
            "",
            None,
            json!({"reason":"invalid_credentials"}),
        )?;
        return Err(if needs_captcha {
            ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "尝试次数过多，请完成安全验证".into(),
                Some("LOGIN_CAPTCHA_REQUIRED"),
            )
        } else {
            ApiError(
                StatusCode::UNAUTHORIZED,
                if code_login.is_some() { "手机号或验证码错误" } else { "账号或密码错误" }.into(),
                None,
            )
        });
    }
    clear_login_failures(&db, &guard_key)?;
    if code_login.is_some() {
        db.execute(
            "DELETE FROM sms_codes WHERE phone=?1 AND purpose='login'",
            [&input.phone],
        )?;
    }
    let tx = db.transaction()?;
    invitations::consume_pending(&tx, id)?;
    user_admin::behavior(
        &tx,
        &context,
        Some(id),
        Some(&input.phone),
        "login",
        "success",
        &id.to_string(),
        None,
        json!({}),
    )?;
    let result = new_session(&tx, id)?;
    tx.commit()?;
    Ok(Json(result))
}
#[derive(Deserialize)]
struct ResetInput {
    phone: String,
    code: String,
    password: String,
}
async fn reset_password(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<ResetInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    let hash = password_hash(&input.password)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    verify_sms(&tx, &input.phone, &input.code, "reset")?;
    let id: Option<i64> = tx
        .query_row("SELECT id FROM users WHERE phone=?1", [&input.phone], |r| {
            r.get(0)
        })
        .optional()?;
    let Some(id) = id else {
        return Err(invalid("手机号尚未注册"));
    };
    tx.execute(
        "UPDATE users SET password_hash=?1 WHERE id=?2",
        params![hash, id],
    )?;
    tx.execute("DELETE FROM sessions WHERE user_id=?1", [id])?;
    user_admin::behavior(
        &tx,
        &context,
        Some(id),
        Some(&input.phone),
        "password_reset",
        "success",
        &id.to_string(),
        None,
        json!({}),
    )?;
    tx.execute(
        "DELETE FROM sms_codes WHERE phone=?1 AND purpose='reset'",
        [input.phone],
    )?;
    tx.commit()?;
    Ok(Json(json!({"ok":true})))
}
async fn logout(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let tx = db.transaction()?;
    tx.execute("DELETE FROM sessions WHERE token=?1", [token(&headers)?])?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "logout",
        "success",
        &uid.to_string(),
        None,
        json!({}),
    )?;
    tx.commit()?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct AccountDeletionInput {
    reason: Option<String>,
}

async fn delete_account(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<AccountDeletionInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let (old_phone, nickname): (String, String) =
        db.query_row("SELECT phone,nickname FROM users WHERE id=?1", [uid], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    let existing: Option<(String, String, i64)> = db
        .query_row(
            "SELECT id,status,created_at FROM account_deletion_requests WHERE user_id=?1 ORDER BY created_at DESC LIMIT 1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((id, status, created_at)) = existing {
        if status == "pending" {
            return Ok(Json(json!({
                "status":"pending",
                "request_id":id,
                "created_at":iso(created_at),
                "message":"注销申请已提交，等待客服审核"
            })));
        }
    }
    let request_id = Uuid::new_v4().simple().to_string();
    let reason = input
        .reason
        .unwrap_or_default()
        .trim()
        .chars()
        .take(200)
        .collect::<String>();
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO account_deletion_requests(id,user_id,phone_snapshot,nickname_snapshot,status,reason,created_at) VALUES(?1,?2,?3,?4,'pending',?5,?6)",
        params![request_id, uid, old_phone, nickname, reason, now()],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        Some(uid),
        "account_delete_request",
        &request_id,
        "用户提交注销申请",
        json!({"phone":old_phone,"nickname":nickname}),
        json!({"status":"pending","reason":reason}),
    )?;
    tx.commit()?;
    Ok(Json(json!({
        "status":"pending",
        "request_id":request_id,
        "created_at":iso(now()),
        "message":"注销申请已提交，等待客服审核"
    })))
}

/// Lets the user withdraw their own request while it is still waiting for review.
async fn cancel_account_deletion(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let tx = db.transaction()?;
    let pending: Option<String> = tx
        .query_row(
            "SELECT id FROM account_deletion_requests WHERE user_id=?1 AND status='pending' ORDER BY created_at DESC LIMIT 1",
            [uid],
            |r| r.get(0),
        )
        .optional()?;
    let Some(id) = pending else {
        return Err(conflict("没有审核中的注销申请"));
    };
    tx.execute(
        "UPDATE account_deletion_requests SET status='cancelled',reviewed_at=?1,reviewer='user',review_note='用户撤回' WHERE id=?2 AND status='pending'",
        params![now(), id],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        Some(uid),
        "account_delete_cancel",
        &id,
        "用户撤回注销申请",
        json!({"status":"pending"}),
        json!({"status":"cancelled"}),
    )?;
    tx.commit()?;
    Ok(Json(json!({"status":"cancelled","request_id":id,"message":"已撤回注销申请"})))
}

async fn account_deletion_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let row: Option<(String, String, String, i64, Option<i64>, Option<String>)> = db
        .query_row(
            "SELECT id,status,reason,created_at,reviewed_at,review_note FROM account_deletion_requests WHERE user_id=?1 ORDER BY created_at DESC LIMIT 1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let Some((id, status, reason, created_at, reviewed_at, note)) = row else {
        return Ok(Json(json!({"status":"none"})));
    };
    Ok(Json(json!({
        "status":status,
        "request_id":id,
        "reason":reason,
        "created_at":iso(created_at),
        "reviewed_at":reviewed_at.map(iso),
        "review_note":note
    })))
}

#[derive(Deserialize)]
struct AccountDeletionListQuery {
    status: Option<String>,
}

async fn admin_account_deletions(
    State(state): State<AppState>,
    Query(query): Query<AccountDeletionListQuery>,
) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    let status = query
        .status
        .as_deref()
        .filter(|value| matches!(*value, "pending" | "approved" | "rejected"));
    let sql = if status.is_some() {
        "SELECT r.id,r.user_id,r.phone_snapshot,r.nickname_snapshot,r.status,r.reason,r.created_at,r.reviewed_at,r.reviewer,r.review_note FROM account_deletion_requests r WHERE r.status=?1 ORDER BY r.created_at DESC LIMIT 200"
    } else {
        "SELECT r.id,r.user_id,r.phone_snapshot,r.nickname_snapshot,r.status,r.reason,r.created_at,r.reviewed_at,r.reviewer,r.review_note FROM account_deletion_requests r ORDER BY r.created_at DESC LIMIT 200"
    };
    let mut stmt = db.prepare(sql)?;
    let rows = if let Some(status) = status {
        stmt.query_map([status], deletion_row)?
    } else {
        stmt.query_map([], deletion_row)?
    };
    Ok(Json(rows.collect::<Result<Vec<_>, _>>()?))
}

fn deletion_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id":row.get::<_,String>(0)?,
        "user_id":row.get::<_,i64>(1)?,
        "phone":row.get::<_,String>(2)?,
        "nickname":row.get::<_,String>(3)?,
        "status":row.get::<_,String>(4)?,
        "reason":row.get::<_,String>(5)?,
        "created_at":iso(row.get::<_,i64>(6)?),
        "reviewed_at":row.get::<_,Option<i64>>(7)?.map(iso),
        "reviewer":row.get::<_,Option<String>>(8)?,
        "review_note":row.get::<_,Option<String>>(9)?
    }))
}

#[derive(Deserialize)]
struct AccountDeletionReviewInput {
    decision: String,
    note: Option<String>,
}

async fn review_account_deletion(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<String>,
    Json(input): Json<AccountDeletionReviewInput>,
) -> ApiResult<Value> {
    if input.decision != "approve" && input.decision != "reject" {
        return Err(invalid("审核结果应为 approve 或 reject"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let record: Option<(i64, String, String, String)> = tx
        .query_row(
            "SELECT user_id,status,phone_snapshot,nickname_snapshot FROM account_deletion_requests WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((uid, status, phone, nickname)) = record else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "注销申请不存在".into(),
            None,
        ));
    };
    if status != "pending" {
        return Err(conflict("这笔注销申请已经处理"));
    }
    let note = input
        .note
        .unwrap_or_default()
        .trim()
        .chars()
        .take(300)
        .collect::<String>();
    let reviewer = context
        .admin_username
        .clone()
        .unwrap_or_else(|| "客服".into());
    let next = if input.decision == "approve" {
        "approved"
    } else {
        "rejected"
    };
    if next == "approved" {
        tx.execute("DELETE FROM sessions WHERE user_id=?1", [uid])?;
        tx.execute("DELETE FROM sms_codes WHERE phone=?1", [&phone])?;
        tx.execute(
            "DELETE FROM invitation_prebindings WHERE phone=?1",
            [&phone],
        )?;
        let anonymized_phone = format!("deleted-{}-{}", uid, now());
        let hash = password_hash(&Uuid::new_v4().to_string())?;
        tx.execute("UPDATE users SET phone=?1,password_hash=?2,nickname='已注销用户',inviter_id=NULL,inviter_code=NULL,invitation_bound_at=NULL WHERE id=?3", params![anonymized_phone, hash, uid])?;
    }
    tx.execute(
        "UPDATE account_deletion_requests SET status=?1,reviewed_at=?2,reviewer=?3,review_note=?4 WHERE id=?5",
        params![next, now(), reviewer, note, id],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        Some(uid),
        "account_delete_review",
        &id,
        if next == "approved" {
            "客服通过注销申请"
        } else {
            "客服驳回注销申请"
        },
        json!({"status":"pending","phone":phone,"nickname":nickname}),
        json!({"status":next,"note":note,"reviewer":reviewer}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"id":id,"status":next,"message":if next=="approved" {"注销已完成"} else {"注销申请已驳回"}}),
    ))
}
async fn me(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> ApiResult<User> {
    let db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let result = user(&db, uid)?;
    user_admin::behavior(
        &db,
        &context,
        Some(uid),
        None,
        "app_open",
        "success",
        "",
        None,
        json!({}),
    )?;
    Ok(Json(result))
}

fn setting(db: &Connection, key: &str) -> Result<f64, ApiError> {
    Ok(
        db.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
            r.get(0)
        })?,
    )
}
#[derive(Serialize)]
struct Task {
    id: String,
    title: String,
    subtitle: String,
    reward: i64,
    reward_min: i64,
    reward_max: i64,
    kind: String,
    claimed_today: i64,
}
const TASKS: [(&str, &str, &str, &str); 3] = [
    (
        "welcome",
        "见面红包",
        "一份小心意，开心收下吧",
        "rewarded_video",
    ),
    (
        "afternoon",
        "午后小惊喜",
        "歇一会儿，把好运装进口袋",
        "rewarded_video",
    ),
    (
        "lucky",
        "好运加倍",
        "今日份的好运，正在派送",
        "rewarded_video",
    ),
];
fn task_info(
    id: &str,
) -> Result<(&'static str, &'static str, &'static str, &'static str), ApiError> {
    TASKS
        .iter()
        .find(|t| t.0 == id)
        .copied()
        .ok_or_else(|| invalid("任务不存在"))
}
async fn tasks(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Vec<Task>> {
    let db = open(&state)?;
    let id = if headers.contains_key("authorization") {
        auth(&db, &headers)?
    } else {
        0
    };
    let mut tasks = Vec::new();
    for index in 0..TASKS.len() {
        tasks.push(task_payload(&db, id, index)?);
    }
    Ok(Json(tasks))
}
fn task_payload(db: &Connection, user_id: i64, index: usize) -> Result<Task, ApiError> {
    let (min, max) = reward_policy::preview(db, user_id)?;
    let (task_id, title, subtitle, kind) = TASKS[index];
    let count = db.query_row(
        "SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND task_id=?2 AND completed_at>=?3",
        params![user_id, task_id, today_start()],
        |r| r.get(0),
    )?;
    Ok(Task {
        id: task_id.into(),
        title: title.into(),
        subtitle: subtitle.into(),
        reward: min,
        reward_min: min,
        reward_max: max,
        kind: kind.into(),
        claimed_today: count,
    })
}
#[derive(Deserialize)]
struct RewardInput {
    task_id: String,
    quote_id: String,
}
async fn reward_start(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<RewardInput>,
) -> ApiResult<Value> {
    let title = if input.task_id.starts_with("rain:") {
        "红包雨大奖"
    } else {
        task_info(&input.task_id)?.1
    };
    let uid = {
        let db = open(&state)?;
        let uid = auth(&db, &headers)?;
        risk::check_account(&db, uid)?;
        let quote = reward_policy::read_quote(&db, uid, &input.quote_id)?;
        if quote.task_id != input.task_id {
            return Err(invalid("报价与红包不匹配"));
        }
        if let Some(sid) = quote.session_id {
            let (reward, provider): (i64, String) = db.query_row(
                "SELECT reward,provider FROM reward_sessions WHERE id=?1 AND user_id=?2",
                params![sid, uid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            business_guard::dispatch(state.clone(), sid.clone());
            return Ok(Json(reward_payload(&state, &db, &sid, uid, title, reward, &provider)?));
        }
        uid
    };
    let actual = json!({"task_id":input.task_id,"quote_id":input.quote_id});
    business_guard::ensure_permit(&state, &headers, uid, "reward", actual.clone()).await?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    risk::check_account(&tx, uid)?;
    let quote = reward_policy::read_quote(&tx, uid, &input.quote_id)?;
    if quote.task_id != input.task_id {
        return Err(invalid("报价与红包不匹配"));
    }
    if let Some(sid) = &quote.session_id {
        let (reward, provider): (i64, String) = tx.query_row(
            "SELECT reward,provider FROM reward_sessions WHERE id=?1 AND user_id=?2",
            params![sid, uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        return Ok(Json(reward_payload(&state, &tx, sid, uid, title, reward, &provider)?));
    }
    let grand = rain::grand_task(&state, &tx, uid, &input.task_id)?;
    if let Some(existing) = grand.as_ref().and_then(|v| v.session_id.as_deref()) {
        let (reward, provider): (i64, String) = tx.query_row(
            "SELECT reward,provider FROM reward_sessions WHERE id=?1 AND user_id=?2",
            params![existing, uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        return Ok(Json(reward_payload(
            &state, &tx, existing, uid, title, reward, &provider,
        )?));
    }
    let event = business_guard::validate_local(&state, &tx, &headers, uid, "reward", actual)?;
    let claimed: i64 = tx.query_row(
        "SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND completed_at>=?2",
        params![uid, today_start()],
        |r| r.get(0),
    )?;
    if claimed >= setting(&tx, "daily_reward_limit")? as i64 {
        return Err(invalid("今日红包已领完，明天再来吧"));
    }
    let sid = business_guard::operation(&tx, &event)?.unwrap_or_else(|| Uuid::new_v4().to_string());
    let device = business_guard::event_device(&tx, &event)?;
    let basis = reward_policy::attach(&tx, uid, &quote, &sid, &device)?;
    if grand.as_ref().is_some_and(|v| v.coins != quote.coins) {
        return Err(invalid("红包奖励状态不匹配"));
    }
    tx.execute("INSERT INTO reward_sessions(id,user_id,task_id,reward,started_at,provider,quote_id,device_identity,reward_basis_json,guard_event_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![sid,uid,input.task_id,quote.coins,now(),state.config.ad_mode,input.quote_id,device,basis.to_string(),event])?;
    risk::consume(&tx, &event, &sid)?;
    if let Some(grand) = grand {
        rain::attach_session(&tx, uid, &grand.packet_id, &sid)?;
    }
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "reward_start",
        "success",
        &sid,
        Some(quote.coins as f64 / 1000.0),
        json!({"provider":state.config.ad_mode,"task_id":input.task_id,"quote_id":input.quote_id}),
    )?;
    tx.commit()?;
    Ok(Json(reward_payload(
        &state,
        &db,
        &sid,
        uid,
        title,
        quote.coins,
        &state.config.ad_mode,
    )?))
}
/// Real-ad sessions carry the server's ad plan: the ordered networks the app may try.
/// An empty plan (switched off, or a risk decision) looks to the app like ordinary "no fill".
fn reward_payload(
    state: &AppState,
    db: &Connection,
    id: &str,
    uid: i64,
    title: &str,
    reward: i64,
    provider: &str,
) -> Result<Value, ApiError> {
    if provider != "csj" {
        return Ok(
            json!({"id":id,"operation_id":id,"reward":reward,"duration_seconds":5,"title":title,"mode":"mock"}),
        );
    }
    let plan = ad_mediation::plan(state, db, uid, "rewarded", id)?;
    let mut payload = json!({"id":id,"operation_id":id,"reward":reward,"title":title,"mode":"csj","user_id":uid.to_string(),"media_extra":json!({"reward_session_id":id}).to_string(),"plan":plan});
    // Clients before 0.3.21 only know 穿山甲 and read these two fields. They keep their old
    // behaviour (an empty code would make them show a visible load error); 0.3.21+ ignore
    // these and follow "plan", so the quiet risk block applies once users update.
    if let Some((app_id, code_id)) = ad_mediation::legacy_csj(db)? {
        payload["ad_app_id"] = json!(app_id);
        payload["ad_code_id"] = json!(code_id);
    }
    Ok(payload)
}
async fn reward_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let row:Option<(i64,String,i64,Option<i64>,Option<i64>)>=db.query_row("SELECT reward,provider,started_at,verified_at,completed_at FROM reward_sessions WHERE id=?1 AND user_id=?2",params![id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    let Some((reward, provider, started, verified, completed)) = row else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "红包记录不存在".into(),
            None,
        ));
    };
    Ok(Json(
        json!({"id":id,"status":if completed.is_some(){"completed"}else if verified.is_some()||(provider=="mock"&&now()-started>=5){"verified"}else{"pending"},"reward":reward,"verified_at":verified.map(iso),"completed_at":completed.map(iso),"expires_at":iso(started+1800),"expired":now()>started+1800}),
    ))
}
#[derive(Deserialize, Serialize)]
struct ClientRewardEvent {
    event: String,
    valid: Option<bool>,
    reward_type: Option<i64>,
    is_server_verify: Option<bool>,
    error_code: Option<i64>,
    error_message: Option<String>,
}
async fn reward_client_event(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut input): Json<ClientRewardEvent>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    if ![
        "reward_arrived",
        "reward_verify",
        "ad_closed",
        "video_error",
    ]
    .contains(&input.event.as_str())
        || input
            .error_message
            .as_ref()
            .is_some_and(|v| v.chars().count() > 500)
    {
        return Err(invalid("广告诊断参数不正确"));
    }
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM reward_sessions WHERE id=?1 AND user_id=?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "红包记录不存在".into(),
            None,
        ));
    }
    if let Some(message) = &mut input.error_message {
        *message = message.replace(token(&headers)?, "[redacted]");
    }
    let mut detail = serde_json::to_value(&input)?;
    detail["client_reported"] = json!(true);
    user_admin::behavior(
        &db,
        &context,
        Some(uid),
        None,
        &format!("ad_client_{}", input.event),
        "reported",
        &id,
        None,
        detail,
    )?;
    Ok(Json(json!({"ok":true})))
}
async fn reward_complete(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    let session:Option<(String,i64,i64,Option<i64>,String,Option<i64>)>=tx.query_row("SELECT task_id,reward,started_at,completed_at,provider,verified_at FROM reward_sessions WHERE id=?1 AND user_id=?2",params![id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
    let Some((task_id, reward, started_at, completed_at, provider, verified_at)) = session else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "红包记录不存在".into(),
            None,
        ));
    };
    if provider != state.config.ad_mode {
        return Err(invalid("奖励通道已更新，请重新领取"));
    }
    if completed_at.is_some() {
        business_guard::dispatch(state.clone(), id.clone());
        return Ok(Json(
            json!({"operation_id":id,"reward":reward,"user":user(&tx,uid)?,"already_completed":true}),
        ));
    }
    let quoted: bool = tx.query_row(
        "SELECT quote_id IS NOT NULL FROM reward_sessions WHERE id=?1",
        [&id],
        |r| r.get(0),
    )?;
    if quoted && now() - started_at > 1800 {
        return Err(invalid("红包已过期，请重新领取"));
    }
    risk::authorize_completed_reward(&state, &tx, uid, &id)?;
    if provider == "csj" {
        if now() - started_at > 1800 {
            return Err(invalid("红包已过期，请重新领取"));
        }
        if verified_at.is_none() {
            return Err(ApiError(
                StatusCode::CONFLICT,
                "广告奖励尚未通过平台确认".into(),
                Some("REWARD_PENDING"),
            ));
        }
    } else if now() - started_at < 5 {
        return Err(invalid("请完成体验后领取奖励"));
    }
    let claimed: i64 = tx.query_row(
        "SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND completed_at>=?2",
        params![uid, today_start()],
        |r| r.get(0),
    )?;
    if claimed >= setting(&tx, "daily_reward_limit")? as i64 {
        return Err(invalid("今日红包已领完，明天再来吧"));
    }
    tx.execute(
        "UPDATE reward_sessions SET completed_at=?1 WHERE id=?2",
        params![now(), id],
    )?;
    let packet = rain::completion_packet(&tx, uid, &task_id, &id)?;
    let title = if packet.is_some() {
        "红包雨大奖"
    } else {
        task_info(&task_id)?.1
    };
    let source = if let Some(pid) = &packet {
        format!("rain:{pid}")
    } else {
        format!("reward:{id}")
    };
    credit(&tx, uid, reward, title, &source)?;
    if let Some(pid) = &packet {
        rain::finish_grand(&tx, uid, pid, &id, reward)?;
    }
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        if packet.is_some() {
            "rain_large_claim"
        } else {
            "reward_complete"
        },
        "success",
        &id,
        Some(reward as f64 / 1000.0),
        json!({"rain_packet_id":packet}),
    )?;
    business_guard::record_receipt(&tx, uid, &id)?;
    tx.commit()?;
    business_guard::dispatch(state.clone(), id.clone());
    Ok(Json(
        json!({"operation_id":id,"reward":reward,"user":user(&db,uid)?}),
    ))
}
fn callback_rejected(reason: &'static str) -> ad_mediation::Decision {
    eprintln!("穿山甲回调未通过：{reason}");
    ad_mediation::Decision {
        valid: false,
        reason,
        session: String::new(),
        uid: None,
    }
}
fn verify_csj_callback(
    state: &AppState,
    query: &HashMap<String, String>,
) -> Result<ad_mediation::Decision, ApiError> {
    if state.config.ad_mode != "csj" {
        return Ok(callback_rejected("当前广告通道未开启"));
    }
    let db = open(state)?;
    let Some(secret) = ad_mediation::secret(state, &db, "csj")? else {
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "缺少 CSJ_REWARD_SECRET 配置".into(),
            None,
        ));
    };
    let get = |key: &str| query.get(key).map(String::as_str).unwrap_or("");
    let trans_id = get("trans_id");
    if trans_id.is_empty() || get("user_id").is_empty() {
        return Ok(callback_rejected("缺少奖励回调参数"));
    }
    let expected = format!(
        "{:x}",
        Sha256::digest(format!("{secret}:{trans_id}").as_bytes())
    );
    if !equal_secret(&expected, get("sign")) {
        return Ok(callback_rejected("签名不匹配"));
    }
    // Coins come from the server-locked (random) quote on the reward session, not from the
    // platform's reward name/amount, so those are not compared — signature, session ownership,
    // expiry and a unique trans_id are what make the callback trustworthy.
    drop(db);
    let decision = ad_mediation::accept_reward(state, "csj", trans_id, get("user_id"), get("extra"))?;
    if !decision.valid {
        eprintln!("穿山甲回调未通过：{}", decision.reason);
    }
    Ok(decision)
}
async fn csj_callback(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Value> {
    let decision = verify_csj_callback(&state, &query)?;
    let get = |key: &str| query.get(key).map(String::as_str).unwrap_or("");
    ad_mediation::record_callback(
        &state,
        &context,
        "csj",
        get("trans_id"),
        get("extra"),
        &decision,
    )?;
    Ok(Json(json!({"isValid":decision.valid})))
}
pub(crate) fn credit(
    db: &Connection,
    uid: i64,
    coins: i64,
    title: &str,
    source: &str,
) -> Result<(), ApiError> {
    db.execute(
        "INSERT INTO ledger(user_id,title,coins,source,created_at) VALUES(?1,?2,?3,?4,?5)",
        params![uid, title, coins, source, now()],
    )?;
    db.execute(
        "UPDATE users SET coins=coins+?1 WHERE id=?2",
        params![coins, uid],
    )?;
    Ok(())
}
const CHECKIN_REWARDS: [i64; 7] = [50, 70, 90, 110, 130, 150, 200];

fn checkin_offset(start: &str, current: &str) -> Result<i64, ApiError> {
    let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    let current = NaiveDate::parse_from_str(current, "%Y-%m-%d")
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    Ok((current - start).num_days())
}

fn ensure_checkin_cycle(db: &Connection, uid: i64) -> Result<(String, i64, i64, i64), ApiError> {
    let today = day();
    let existing: Option<(String, i64, i64)> = db
        .query_row(
            "SELECT cycle_start_day,checked_mask,makeup_mask FROM checkin_progress WHERE user_id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (start, checked, makeup) = existing.clone().unwrap_or_else(|| (today.clone(), 0, 0));
    let offset = checkin_offset(&start, &today)?;
    if offset < 0 || offset >= 7 {
        db.execute(
            "INSERT INTO checkin_progress(user_id,cycle_start_day,checked_mask,makeup_mask) VALUES(?1,?2,0,0) ON CONFLICT(user_id) DO UPDATE SET cycle_start_day=excluded.cycle_start_day,checked_mask=0,makeup_mask=0",
            params![uid, today],
        )?;
        return Ok((today, 0, 0, 0));
    }
    if existing.is_none() {
        db.execute(
            "INSERT INTO checkin_progress(user_id,cycle_start_day,checked_mask,makeup_mask) VALUES(?1,?2,0,0)",
            params![uid, start],
        )?;
    }
    Ok((start, offset, checked, makeup))
}

fn checkin_status_json(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let (start, offset, checked, makeup) = ensure_checkin_cycle(db, uid)?;
    let mut days = Vec::with_capacity(7);
    for index in 0..7_i64 {
        let bit = 1_i64 << index;
        let status = if checked & bit != 0 {
            "checked"
        } else if makeup & bit != 0 {
            "makeup"
        } else if index < offset {
            "missed"
        } else if index == offset {
            "today"
        } else {
            "upcoming"
        };
        days.push(json!({"day":index+1,"reward":CHECKIN_REWARDS[index as usize],"status":status,"can_makeup":status=="missed"}));
    }
    Ok(
        json!({"cycle_start_day":start,"current_day":offset+1,"streak":(0..7).take_while(|i| checked & (1_i64<<i) != 0).count(),"makeup_available":days.iter().filter(|item|item["status"]=="missed").count(),"days":days}),
    )
}

async fn checkin_status(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    Ok(Json(checkin_status_json(&db, uid)?))
}

async fn check_in(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    let uid = {
        let db = open(&state)?;
        let uid = auth(&db, &headers)?;
        let (_, offset, checked, _) = ensure_checkin_cycle(&db, uid)?;
        if checked & (1_i64 << offset) != 0 {
            business_guard::dispatch(state.clone(), format!("check-in:{}:{uid}", day()));
            return Err(conflict("今天已经签到啦"));
        }
        uid
    };
    business_guard::ensure_permit(&state, &headers, uid, "check_in", json!({})).await?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let source = format!("check-in:{}:{uid}", day());
    let (cycle_start, offset, checked, _) = ensure_checkin_cycle(&tx, uid)?;
    if checked & (1_i64 << offset) != 0 {
        return Err(conflict("今天已经签到啦"));
    }
    let event = business_guard::validate_local(&state, &tx, &headers, uid, "check_in", json!({}))?;
    let reward = CHECKIN_REWARDS[offset as usize];
    credit(&tx, uid, reward, "每日签到", &source)?;
    tx.execute(
        "UPDATE checkin_progress SET checked_mask=checked_mask | ?1 WHERE user_id=?2",
        params![1_i64 << offset, uid],
    )?;
    risk::consume(&tx, &event, &source)?;
    business_guard::record_receipt(&tx, uid, &source)?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "check_in",
        "success",
        &source,
        Some(reward as f64 / 1000.0),
        json!({"cycle_start_day":cycle_start,"day":offset+1}),
    )?;
    tx.commit()?;
    business_guard::dispatch(state.clone(), source.clone());
    Ok(Json(
        json!({"operation_id":source,"reward":reward,"day":offset+1,"user":user(&db,uid)?}),
    ))
}
#[derive(Deserialize)]
struct MakeupInput {
    day: i64,
}
async fn check_in_makeup(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<MakeupInput>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    if !(1..=7).contains(&input.day) {
        return Err(invalid("补签日期无效"));
    }
    let (_, current, checked, makeup) = ensure_checkin_cycle(&db, uid)?;
    let index = input.day - 1;
    let bit = 1_i64 << index;
    if index >= current {
        return Err(invalid("只能补签已错过的日期"));
    }
    if checked & bit != 0 || makeup & bit != 0 {
        return Err(conflict("这一天已经签到啦"));
    }
    business_guard::ensure_permit(
        &state,
        &headers,
        uid,
        "check_in",
        json!({"makeup_day":input.day}),
    )
    .await?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let event = business_guard::validate_local(
        &state,
        &tx,
        &headers,
        uid,
        "check_in",
        json!({"makeup_day":input.day}),
    )?;
    let source = format!("check-in-makeup:{}:{}:{}", day(), input.day, uid);
    let reward = CHECKIN_REWARDS[index as usize];
    credit(&tx, uid, reward, "连续签到补签", &source)?;
    tx.execute(
        "UPDATE checkin_progress SET makeup_mask=makeup_mask | ?1 WHERE user_id=?2",
        params![bit, uid],
    )?;
    risk::consume(&tx, &event, &source)?;
    business_guard::record_receipt(&tx, uid, &source)?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "check_in_makeup",
        "success",
        &source,
        Some(reward as f64 / 1000.0),
        json!({"day":input.day}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"operation_id":source,"reward":reward,"day":input.day,"user":user(&db,uid)?}),
    ))
}
async fn ledger(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let mut stmt = db.prepare(
        "SELECT id,title,coins,created_at FROM ledger WHERE user_id=?1 ORDER BY id DESC LIMIT 200",
    )?;
    let rows=stmt.query_map([uid],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"coins":r.get::<_,i64>(2)?,"created_at":iso(r.get(3)?)})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(rows))
}
async fn products(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    if headers.contains_key("authorization") {
        auth(&db, &headers)?;
    }
    Ok(Json(product_admin::active(&db)?))
}
#[derive(Deserialize)]
struct WithdrawInput {
    amount_yuan: f64,
    channel: String,
    account: String,
}
async fn withdraw(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(mut input): Json<WithdrawInput>,
) -> ApiResult<Value> {
    input.channel = business_guard::payment_channel(&input.channel)?;
    input.account = input.account.trim().to_owned();
    let actual =
        json!({"amount_yuan":input.amount_yuan,"channel":input.channel,"account":input.account});
    let uid = {
        let db = open(&state)?;
        let uid = auth(&db, &headers)?;
        if let Some(result) = withdraw_retry(&state, &db, &headers, uid, &input)? {
            return Ok(Json(result));
        }
        if !input.amount_yuan.is_finite() {
            return Err(invalid("请选择提现金额"));
        }
        withdraw_rules::check(&db, uid, input.amount_yuan)?;
        if input.account.is_empty() {
            return Err(invalid("请填写收款账号"));
        }
        if ((input.amount_yuan * 100.0).round() - input.amount_yuan * 100.0).abs() > 0.001 {
            return Err(invalid("提现金额最多保留两位小数"));
        }
        uid
    };
    business_guard::ensure_permit(&state, &headers, uid, "withdraw", actual.clone()).await?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    if let Some(result) = withdraw_retry(&state, &tx, &headers, uid, &input)? {
        return Ok(Json(result));
    }
    let event = business_guard::validate_local(&state, &tx, &headers, uid, "withdraw", actual)?;
    // Re-check inside the transaction so two concurrent requests cannot both use a daily tier.
    withdraw_rules::check(&tx, uid, input.amount_yuan)?;
    let coins = (input.amount_yuan * 1000.0).round() as i64;
    let balance: i64 = tx.query_row("SELECT coins FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    if coins > balance {
        return Err(invalid("可提现余额不足"));
    }
    let id = business_guard::operation(&tx, &event)?.unwrap_or_else(|| Uuid::new_v4().to_string());
    tx.execute("INSERT INTO withdrawals(id,user_id,amount_yuan,coins,channel,account,status,created_at) VALUES(?1,?2,?3,?4,?5,?6,'模拟申请',?7)",params![id,uid,input.amount_yuan,coins,input.channel,input.account,now()])?;
    credit(&tx, uid, -coins, "提现申请", &format!("withdraw:{id}"))?;
    risk::consume(&tx, &event, &id)?;
    business_guard::record_receipt(&tx, uid, &id)?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "withdrawal",
        "success",
        &id,
        Some(input.amount_yuan),
        json!({"channel":input.channel}),
    )?;
    tx.commit()?;
    business_guard::dispatch(state.clone(), id.clone());
    Ok(Json(
        json!({"id":id,"operation_id":id,"amount_yuan":input.amount_yuan,"channel":input.channel,"account":input.account,"status":"审核中","created_at":iso(now()),"mode":"mock","user":user(&db,uid)?}),
    ))
}
fn withdraw_retry(
    state: &AppState,
    db: &Connection,
    headers: &HeaderMap,
    uid: i64,
    input: &WithdrawInput,
) -> Result<Option<Value>, ApiError> {
    let Some(reference) = user_admin::retry_reference(state, db, headers, uid, "withdraw")? else {
        return Ok(None);
    };
    let existing=db.query_row("SELECT id,amount_yuan,channel,account,status,created_at FROM withdrawals WHERE id=?1 AND user_id=?2",params![reference,uid],|r|Ok(json!({"id":r.get::<_,String>(0)?,"amount_yuan":r.get::<_,f64>(1)?,"channel":r.get::<_,String>(2)?,"account":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"created_at":iso(r.get(5)?)}))).optional()?.ok_or_else(||conflict("该请求已处理，请刷新后重试"))?;
    if existing["amount_yuan"] != json!(input.amount_yuan)
        || business_guard::payment_channel(existing["channel"].as_str().unwrap())? != input.channel
        || existing["account"].as_str().unwrap().trim() != input.account
    {
        return Err(conflict("该请求已处理，请刷新后重试"));
    }
    let mut result = existing;
    result["operation_id"] = json!(reference);
    result["mode"] = json!("mock");
    result["user"] = serde_json::to_value(user(db, uid)?)?;
    business_guard::dispatch(state.clone(), reference);
    Ok(Some(result))
}
fn withdrawal_rows(db: &Connection, uid: Option<i64>) -> Result<Vec<Value>, ApiError> {
    let mut stmt=db.prepare("SELECT w.id,w.user_id,u.phone,w.amount_yuan,w.channel,w.account,w.status,w.created_at FROM withdrawals w JOIN users u ON u.id=w.user_id WHERE (?1 IS NULL OR w.user_id=?1) ORDER BY w.created_at DESC LIMIT 200")?;
    Ok(stmt.query_map([uid],|r|Ok(json!({"id":r.get::<_,String>(0)?,"user_id":r.get::<_,i64>(1)?,"phone":r.get::<_,String>(2)?,"amount_yuan":r.get::<_,f64>(3)?,"channel":r.get::<_,String>(4)?,"account":r.get::<_,String>(5)?,"status":r.get::<_,String>(6)?,"created_at":iso(r.get(7)?)})))?.collect::<Result<Vec<_>,_>>()?)
}
/// The stored statuses carry the payment mode ("模拟…" while payouts are manual); users only
/// see what the request means for them. `status` is replaced so older apps show it too.
fn user_withdrawal_view(mut row: Value, coins: i64) -> Value {
    let raw = row["status"].as_str().unwrap_or("").to_string();
    let (state, text) = if raw.ends_with("已完成") || raw == "approved" {
        ("done", "已到账")
    } else if raw.ends_with("已退回") || raw == "rejected" {
        ("returned", "已退回")
    } else {
        ("pending", "审核中")
    };
    let account = row["account"].as_str().unwrap_or("").to_string();
    let chars: Vec<char> = account.chars().collect();
    let masked = if chars.len() <= 4 {
        account.clone()
    } else if account.contains('@') {
        let (name, domain) = account.split_once('@').unwrap();
        let head: String = name.chars().take(2).collect();
        format!("{head}***@{domain}")
    } else {
        let head: String = chars[..3.min(chars.len())].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{head}****{tail}")
    };
    row["status"] = json!(text);
    row["state"] = json!(state);
    row["coins"] = json!(coins);
    row["account"] = json!(masked);
    row["channel_text"] = json!(match row["channel"].as_str().unwrap_or("") {
        "alipay" => "支付宝",
        "wechat" => "微信",
        other => other,
    });
    row["note"] = json!(match state {
        "done" => "已打款到你的收款账户",
        "returned" => "未通过审核，金币已退回账户",
        _ => "预计 1–3 个工作日内处理",
    });
    if let Some(obj) = row.as_object_mut() {
        obj.remove("phone");
        obj.remove("user_id");
    }
    row
}

async fn withdrawals(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let mut out = Vec::new();
    for row in withdrawal_rows(&db, Some(uid))? {
        let coins: i64 = db
            .query_row(
                "SELECT coins FROM withdrawals WHERE id=?1",
                [row["id"].as_str().unwrap_or("")],
                |r| r.get(0),
            )
            .unwrap_or(0);
        out.push(user_withdrawal_view(row, coins));
    }
    Ok(Json(out))
}
#[derive(Clone, Deserialize, Serialize)]
struct HallStrategy {
    enabled: bool,
    interval_seconds: u64,
    video_weight: u32,
    product_weight: u32,
    #[serde(default = "default_native_ad_weight")]
    native_ad_weight: u32,
    welcome_text: String,
    packet_text: String,
    product_text: String,
}
impl Default for HallStrategy {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 20,
            video_weight: 70,
            product_weight: 30,
            native_ad_weight: default_native_ad_weight(),
            welcome_text: "欢迎回来，今天也有小惊喜。".into(),
            packet_text: "给你送来一份红包，点开看看吧。".into(),
            product_text: "这件好物值得看看，还能省一点。".into(),
        }
    }
}
fn default_native_ad_weight() -> u32 {
    15
}
fn read_hall_strategy(db: &Connection) -> Result<HallStrategy, ApiError> {
    let config: String = db.query_row("SELECT config FROM hall_strategy WHERE id=1", [], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&config)?)
}
async fn hall_strategy(State(state): State<AppState>) -> ApiResult<HallStrategy> {
    Ok(Json(read_hall_strategy(&open(&state)?)?))
}
async fn update_hall_strategy(
    State(state): State<AppState>,
    Json(input): Json<HallStrategy>,
) -> ApiResult<HallStrategy> {
    if input.interval_seconds == 0 {
        return Err(invalid("推送间隔至少为 1 秒"));
    }
    open(&state)?.execute(
        "UPDATE hall_strategy SET config=?1 WHERE id=1",
        [serde_json::to_string(&input)?],
    )?;
    Ok(Json(input))
}
struct HallSchedule {
    started: Instant,
    welcomed: bool,
    first_packet: bool,
    first_product: bool,
    first_native: bool,
    packet_count: u8,
    native_count: u8,
    last_packet_at: i64,
    last_native_at: i64,
    last_dispatch_at: i64,
    unhandled_dispatches: u8,
    native_capable: bool,
    last_dispatch: Instant,
}
fn hall_item(
    state: &AppState,
    headers: &HeaderMap,
    schedule: &mut HallSchedule,
) -> Result<Option<Value>, ApiError> {
    let db = open(state)?;
    let uid = auth(&db, headers)?;
    let strategy = read_hall_strategy(&db)?;
    if !strategy.enabled {
        return Ok(None);
    }
    // "Did the user do anything since the last push?" — behaviour audit events, or any
    // server-recorded action (opening a packet, starting a video, check-in, rain, product
    // clicks, hall requests), so a broken audit upload can never freeze the hall.
    let mut interacted = true;
    if schedule.last_dispatch_at > 0 {
        let since = schedule.last_dispatch_at;
        interacted = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM behavior_events WHERE user_id=?1 AND server_received_at>=?2 AND (type=5 OR action IN (1,2,3,4,5,6)))
              OR EXISTS(SELECT 1 FROM hall_activity WHERE user_id=?1 AND last_open_at>=?2)
              OR EXISTS(SELECT 1 FROM product_clicks WHERE user_id=?1 AND created_at>=?2)
              OR EXISTS(SELECT 1 FROM user_behavior WHERE user_id=?1 AND created_at>=?2 AND kind IN ('reward_start','reward_complete','check_in','rain_join','hall_request'))",
            params![uid, since],
            |r| r.get(0),
        )?;
    }
    // Idle user (two pushes in a row went unanswered): keep the hall alive at a quarter of the
    // pace instead of stopping. Counted per push, not per stream tick.
    let slow_mode = !interacted && schedule.unhandled_dispatches >= 2;
    let elapsed = schedule.started.elapsed().as_secs();
    let since_dispatch = schedule.last_dispatch.elapsed().as_secs();
    let interval = strategy.interval_seconds.max(5);
    let product = product_admin::random_active(&db)?;
    let product_weight = if product.is_some() {
        strategy.product_weight
    } else {
        0
    };
    // Red packets: next one once the previous was opened (quote) or 3 minutes passed; at most 8
    // per connection; none once today's claim limit is used up.
    let packet_ok = strategy.video_weight > 0 && schedule.packet_count < 8 && {
        let claimed: i64 = db.query_row(
            "SELECT COUNT(*) FROM reward_sessions WHERE user_id=?1 AND completed_at>=?2",
            params![uid, today_start()],
            |r| r.get(0),
        )?;
        let opened: bool = schedule.last_packet_at == 0
            || db.query_row(
                "SELECT EXISTS(SELECT 1 FROM hall_activity WHERE user_id=?1 AND last_open_at>=?2)",
                params![uid, schedule.last_packet_at],
                |r| r.get(0),
            )?;
        claimed < setting(&db, "daily_reward_limit")? as i64
            && (opened || now() - schedule.last_packet_at >= 180)
    };
    // Feed ads: weighted like the rest, at least 2 minutes apart, at most 5 per connection.
    let native_ok = schedule.native_capable
        && strategy.native_ad_weight > 0
        && schedule.native_count < 5
        && (schedule.last_native_at == 0 || now() - schedule.last_native_at >= 120);
    let kind = if !schedule.welcomed {
        schedule.welcomed = true;
        "text"
    } else if !schedule.first_packet && packet_ok && elapsed >= 1 {
        schedule.first_packet = true;
        "red_packet"
    } else if !schedule.first_product && product_weight > 0 && elapsed >= 6 {
        schedule.first_product = true;
        "product"
    } else if !schedule.first_native && native_ok && elapsed >= 10 {
        schedule.first_native = true;
        "native_ad"
    } else if elapsed >= 10 && since_dispatch >= if slow_mode { interval * 4 } else { interval } {
        let packet_weight = if packet_ok { u64::from(strategy.video_weight) } else { 0 };
        let native_weight = if native_ok { u64::from(strategy.native_ad_weight) } else { 0 };
        let weight = packet_weight + u64::from(product_weight) + native_weight;
        if weight == 0 {
            return Ok(None);
        }
        let selected = (Uuid::new_v4().as_u128() % u128::from(weight)) as u64;
        if selected < packet_weight {
            "red_packet"
        } else if selected < packet_weight + native_weight {
            "native_ad"
        } else {
            "product"
        }
    } else {
        return Ok(None);
    };
    if schedule.last_dispatch_at > 0 {
        schedule.unhandled_dispatches = if interacted {
            0
        } else {
            schedule.unhandled_dispatches.saturating_add(1)
        };
    }
    schedule.last_dispatch = Instant::now();
    schedule.last_dispatch_at = now();
    let text = match kind {
        "red_packet" => strategy.packet_text,
        "product" => strategy.product_text,
        "native_ad" => "为你精选".into(),
        _ => strategy.welcome_text,
    };
    let mut item =
        json!({"id":Uuid::new_v4().to_string(),"kind":kind,"text":text,"created_at":iso(now())});
    if kind == "red_packet" {
        schedule.packet_count = schedule.packet_count.saturating_add(1);
        schedule.last_packet_at = now();
        let index = (Uuid::new_v4().as_u128() % TASKS.len() as u128) as usize;
        item["task"] = serde_json::to_value(task_payload(&db, uid, index)?)?;
    } else if kind == "product" {
        item["product"] = product.expect("positive product weight has an active product");
    } else if kind == "native_ad" {
        schedule.native_count = schedule.native_count.saturating_add(1);
        schedule.last_native_at = now();
        let plan = ad_mediation::plan(state, &db, uid, "feed", "")?;
        if plan.is_empty() {
            return Ok(None);
        }
        // Clients before 0.3.21 render 穿山甲 only and read ad_code_id.
        if let Some(code) = ad_mediation::config(&db)?
            .networks
            .get("csj")
            .filter(|n| n.enabled && !n.feed.is_empty())
        {
            item["ad_code_id"] = json!(code.feed);
        }
        item["ads"] = json!(plan);
    }
    Ok(Some(item))
}
#[derive(Deserialize)]
struct HallQuery {
    native_ads: Option<u8>,
}
async fn hall_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HallQuery>,
) -> Result<impl IntoResponse, ApiError> {
    {
        let db = open(&state)?;
        auth(&db, &headers)?;
        read_hall_strategy(&db)?;
    }
    let native_capable = state.config.ad_mode == "csj" && query.native_ads == Some(1);
    let stream = async_stream::stream! {
        let mut ticker=tokio::time::interval(Duration::from_secs(1));
        let mut schedule=HallSchedule {started:Instant::now(),welcomed:false,first_packet:false,first_product:false,first_native:false,packet_count:0,native_count:0,last_packet_at:0,last_native_at:0,last_dispatch_at:0,unhandled_dispatches:0,native_capable,last_dispatch:Instant::now()};
        loop {
            ticker.tick().await;
            match hall_item(&state,&headers,&mut schedule) {
                Ok(Some(item))=>yield Ok::<Event,Infallible>(Event::default().data(item.to_string())),
                Ok(None)=>{},
                Err(error)=>{
                    if error.0.is_server_error() {eprintln!("大厅推送：{}",error.1);}
                    yield Ok(Event::default().event("error").data(json!({"error":error.1,"status":error.0.as_u16()}).to_string()));
                    break;
                }
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
async fn admin_overview(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let users: i64 = db.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    let today_users: i64 = db.query_row(
        "SELECT COUNT(*) FROM users WHERE created_at>=?1",
        [today_start()],
        |r| r.get(0),
    )?;
    let rewards: i64 = db.query_row(
        "SELECT COUNT(*) FROM reward_sessions WHERE completed_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let today_rewards: i64 = db.query_row(
        "SELECT COUNT(*) FROM reward_sessions WHERE completed_at>=?1",
        [today_start()],
        |r| r.get(0),
    )?;
    let coins:i64=db.query_row("SELECT COALESCE(SUM(coins),0) FROM ledger WHERE coins>0 AND source NOT LIKE 'withdraw-refund:%'",[],|r|r.get(0))?;
    let today_coins:i64=db.query_row("SELECT COALESCE(SUM(coins),0) FROM ledger WHERE coins>0 AND source NOT LIKE 'withdraw-refund:%' AND created_at>=?1",[today_start()],|r|r.get(0))?;
    let withdrawal_count: i64 =
        db.query_row("SELECT COUNT(*) FROM withdrawals", [], |r| r.get(0))?;
    let withdrawal_amount: f64 = db.query_row(
        "SELECT COALESCE(SUM(amount_yuan),0) FROM withdrawals",
        [],
        |r| r.get(0),
    )?;
    let pending: i64 = db.query_row(
        "SELECT COUNT(*) FROM withdrawals WHERE status='模拟申请'",
        [],
        |r| r.get(0),
    )?;
    let review: i64 = db.query_row(
        "SELECT COUNT(*) FROM account_restrictions WHERE status='review'",
        [],
        |r| r.get(0),
    )?;
    let restricted: i64 = db.query_row(
        "SELECT COUNT(*) FROM account_restrictions WHERE status='restricted'",
        [],
        |r| r.get(0),
    )?;
    let errors: i64 = db.query_row(
        "SELECT COUNT(*) FROM risk_events WHERE status='assessment_error' AND updated_at>=?1",
        [now() - 86400],
        |r| r.get(0),
    )?;
    Ok(Json(
        json!({"users":users,"total_users":users,"today_users":today_users,"rewards":rewards,"today_rewards":today_rewards,"coins_issued":coins,"today_coins":today_coins,"withdrawal_count":withdrawal_count,"withdrawal_amount":withdrawal_amount,"pending_withdrawals":pending,"risk_review_users":review,"risk_restricted_users":restricted,"risk_assessment_errors_24h":errors,"mode":state.config.ad_mode,"ad_platforms":[{"name":"百度联盟","status":"待配置"},{"name":"快手联盟","status":"待配置"},{"name":"腾讯优量汇","status":"待配置"},{"name":"穿山甲","status":if state.config.ad_mode=="csj" {"已配置"} else {"待配置"}}]}),
    ))
}
async fn admin_users(State(state): State<AppState>) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    let mut stmt = db.prepare(
        "SELECT id,phone,nickname,coins,created_at FROM users ORDER BY id DESC LIMIT 200",
    )?;
    Ok(Json(stmt.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"phone":r.get::<_,String>(1)?,"nickname":r.get::<_,String>(2)?,"coins":r.get::<_,i64>(3)?,"created_at":iso(r.get(4)?)})))?.collect::<Result<Vec<_>,_>>()?))
}
async fn admin_rewards(State(state): State<AppState>) -> ApiResult<Vec<Value>> {
    let db = open(&state)?;
    let mut stmt=db.prepare("SELECT r.id,r.user_id,u.phone,r.task_id,r.reward,r.started_at,r.completed_at FROM reward_sessions r JOIN users u ON u.id=r.user_id ORDER BY r.started_at DESC LIMIT 200")?;
    Ok(Json(stmt.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"user_id":r.get::<_,i64>(1)?,"phone":r.get::<_,String>(2)?,"task_id":r.get::<_,String>(3)?,"reward":r.get::<_,i64>(4)?,"started_at":iso(r.get(5)?),"completed_at":r.get::<_,Option<i64>>(6)?.map(iso),"status":if r.get::<_,Option<i64>>(6)?.is_some(){"已到账"}else{"体验中"}})))?.collect::<Result<Vec<_>,_>>()?))
}
async fn admin_withdrawals(State(state): State<AppState>) -> ApiResult<Vec<Value>> {
    Ok(Json(withdrawal_rows(&open(&state)?, None)?))
}
#[derive(Deserialize)]
struct WithdrawalStatusInput {
    status: String,
    reason: Option<String>,
}
async fn update_withdrawal(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<String>,
    Json(input): Json<WithdrawalStatusInput>,
) -> ApiResult<Value> {
    if input.status != "approved" && input.status != "rejected" {
        return Err(invalid("状态应为 approved 或 rejected"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let record: Option<(i64, i64, String)> = tx
        .query_row(
            "SELECT user_id,coins,status FROM withdrawals WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((uid, coins, status)) = record else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "提现记录不存在".into(),
            None,
        ));
    };
    if status != "模拟申请" {
        return Err(conflict("这笔申请已经处理"));
    }
    let next = if input.status == "approved" {
        "模拟已完成"
    } else {
        "模拟已退回"
    };
    tx.execute(
        "UPDATE withdrawals SET status=?1 WHERE id=?2",
        params![next, id],
    )?;
    if input.status == "rejected" {
        credit(
            &tx,
            uid,
            coins,
            "提现退回",
            &format!("withdraw-refund:{id}"),
        )?;
    }
    user_admin::action_audit(
        &tx,
        &context,
        Some(uid),
        "admin_review",
        &id,
        input
            .reason
            .as_deref()
            .filter(|reason| !reason.trim().is_empty())
            .unwrap_or(if input.status == "approved" {
                "管理员通过模拟提现"
            } else {
                "管理员驳回模拟提现"
            }),
        json!({"status":status}),
        json!({"status":next,"coins":coins}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"id":id,"status":next,"mode":"mock","user":user(&db,uid)?}),
    ))
}
async fn admin_settings(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(settings_json(&open(&state)?, &state.config)?))
}
fn settings_json(db: &Connection, config: &RuntimeConfig) -> Result<Value, ApiError> {
    let (name, amount): (String, i64) = db.query_row(
        "SELECT name,amount FROM csj_reward_config WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(
        json!({"coins_per_yuan":COINS_PER_YUAN,"min_withdraw_yuan":setting(db,"min_withdraw_yuan")?,"daily_reward_limit":setting(db,"daily_reward_limit")? as i64,"reward_amount":setting(db,"reward_amount")? as i64,"csj_expected_reward_name":name,"csj_expected_reward_amount":amount,"sms_mode":"mock","ad_mode":config.ad_mode,"payment_mode":"mock","register_mode":register_mode(db)?}),
    )
}
#[derive(Deserialize)]
struct SettingsInput {
    min_withdraw_yuan: f64,
    daily_reward_limit: i64,
    reward_amount: i64,
    csj_expected_reward_name: Option<String>,
    csj_expected_reward_amount: Option<i64>,
    #[serde(default)]
    register_mode: Option<String>,
}
async fn update_settings(
    State(state): State<AppState>,
    Json(input): Json<SettingsInput>,
) -> ApiResult<Value> {
    if !input.min_withdraw_yuan.is_finite()
        || input.min_withdraw_yuan <= 0.0
        || input.daily_reward_limit <= 0
        || input.reward_amount <= 0
    {
        return Err(invalid("提现门槛、每日次数和奖励金币应大于 0"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    if input
        .csj_expected_reward_name
        .as_deref()
        .is_some_and(|v| v.trim().is_empty() || v.chars().count() > 80)
        || input.csj_expected_reward_amount.is_some_and(|v| v <= 0)
    {
        return Err(invalid("请填写广告平台奖励名称与正数奖励数量"));
    }
    if let Some(name) = input.csj_expected_reward_name {
        tx.execute("UPDATE csj_reward_config SET name=?1 WHERE id=1", [name])?;
    }
    if let Some(amount) = input.csj_expected_reward_amount {
        tx.execute(
            "UPDATE csj_reward_config SET amount=?1 WHERE id=1",
            [amount],
        )?;
    }
    if let Some(mode) = input.register_mode.as_deref() {
        let value = match mode {
            "phone" => 0.0,
            "username" => 1.0,
            _ => return Err(invalid("注册方式应为 phone 或 username")),
        };
        tx.execute("UPDATE settings SET value=?1 WHERE key='register_mode'", params![value])?;
    }
    for (key, value) in [
        ("min_withdraw_yuan", input.min_withdraw_yuan),
        ("daily_reward_limit", input.daily_reward_limit as f64),
        ("reward_amount", input.reward_amount as f64),
    ] {
        tx.execute(
            "UPDATE settings SET value=?1 WHERE key=?2",
            params![value, key],
        )?;
    }
    tx.commit()?;
    Ok(Json(settings_json(&db, &state.config)?))
}
async fn api_not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "接口不存在".into(), None)
}
async fn json_error_responses(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if content_type.contains("json") {
        return response;
    }
    let status = response.status();
    let text = match axum::body::to_bytes(response.into_body(), 1024 * 1024).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
        Err(error) => error.to_string(),
    };
    (
        status,
        Json(json!({"error":if text.is_empty(){"请求失败"}else{&text}})),
    )
        .into_response()
}

fn init(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    let db = Connection::open(&state.db_path)?;
    db.execute_batch("PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS csj_reward_config(id INTEGER PRIMARY KEY CHECK(id=1),name TEXT NOT NULL,amount INTEGER NOT NULL);
        INSERT OR IGNORE INTO csj_reward_config(id,name,amount) VALUES(1,'金币',100);
        CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY,phone TEXT NOT NULL UNIQUE,password_hash TEXT NOT NULL,nickname TEXT NOT NULL,coins INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY,user_id INTEGER NOT NULL,expires_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS account_deletion_requests(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,phone_snapshot TEXT NOT NULL,nickname_snapshot TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'pending',reason TEXT NOT NULL DEFAULT '',created_at INTEGER NOT NULL,reviewed_at INTEGER,reviewer TEXT,review_note TEXT);
        CREATE INDEX IF NOT EXISTS idx_account_deletion_user ON account_deletion_requests(user_id,created_at);
        CREATE INDEX IF NOT EXISTS idx_account_deletion_status ON account_deletion_requests(status,created_at);
        CREATE TABLE IF NOT EXISTS sms_codes(phone TEXT NOT NULL,purpose TEXT NOT NULL,code TEXT NOT NULL,expires_at INTEGER NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(phone,purpose));
        CREATE TABLE IF NOT EXISTS login_guard(guard_key TEXT PRIMARY KEY,failures INTEGER NOT NULL DEFAULT 0,last_failed_at INTEGER NOT NULL DEFAULT 0,captcha_id TEXT,captcha_answer TEXT,captcha_expires_at INTEGER);
        CREATE TABLE IF NOT EXISTS checkin_progress(user_id INTEGER PRIMARY KEY,cycle_start_day TEXT NOT NULL,checked_mask INTEGER NOT NULL DEFAULT 0,makeup_mask INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS hall_activity(user_id INTEGER PRIMARY KEY,last_open_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS reward_sessions(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,task_id TEXT NOT NULL,reward INTEGER NOT NULL,started_at INTEGER NOT NULL,completed_at INTEGER,provider TEXT NOT NULL DEFAULT 'mock',verified_at INTEGER,csj_trans_id TEXT);
        CREATE TABLE IF NOT EXISTS ledger(id INTEGER PRIMARY KEY,user_id INTEGER NOT NULL,title TEXT NOT NULL,coins INTEGER NOT NULL,source TEXT NOT NULL UNIQUE,created_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS withdrawals(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,amount_yuan REAL NOT NULL,coins INTEGER NOT NULL,channel TEXT NOT NULL,account TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS hall_strategy(id INTEGER PRIMARY KEY CHECK(id=1),config TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value REAL NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_ledger_user ON ledger(user_id,id);
        CREATE INDEX IF NOT EXISTS idx_rewards_user ON reward_sessions(user_id,completed_at);
        INSERT OR IGNORE INTO settings(key,value) VALUES('min_withdraw_yuan',1),('daily_reward_limit',20),('reward_amount',100);")?;
    let columns = db
        .prepare("PRAGMA table_info(reward_sessions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    for (name, definition) in [
        ("provider", "TEXT NOT NULL DEFAULT 'mock'"),
        ("verified_at", "INTEGER"),
        ("csj_trans_id", "TEXT"),
    ] {
        if !columns.iter().any(|column| column == name) {
            db.execute_batch(&format!(
                "ALTER TABLE reward_sessions ADD COLUMN {name} {definition}"
            ))?;
        }
    }
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS idx_rewards_csj_trans ON reward_sessions(csj_trans_id) WHERE csj_trans_id IS NOT NULL;")?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE phone='13800138000')",
        [],
        |r| r.get(0),
    )?;
    if !state.config.production && !exists {
        let hash = password_hash("Orange123!").map_err(|e| e.1)?;
        db.execute("INSERT INTO users(phone,password_hash,nickname,coins,created_at) VALUES('13800138000',?1,'小橙同学',2680,?2)",params![hash,now()])?;
        let uid = db.last_insert_rowid();
        db.execute("INSERT INTO ledger(user_id,title,coins,source,created_at) VALUES(?1,'初见好礼',2680,'demo-welcome',?2)",params![uid,now()])?;
    }
    db.execute(
        "INSERT OR IGNORE INTO hall_strategy(id,config) VALUES(1,?1)",
        [serde_json::to_string(&HallStrategy::default())?],
    )?;
    invitations::initialize(&db).map_err(|e| e.1)?;
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username) WHERE username IS NOT NULL; INSERT OR IGNORE INTO settings(key,value) VALUES('register_mode',0);")?;
    user_admin::initialize(&db)?;
    admin_account::initialize(state, &db).map_err(|e| e.1)?;
    mail_admin::initialize(&db)?;
    mail_events::initialize(&db)?;
    product_admin::initialize(&db)?;
    app_config::initialize(&db).map_err(|e| e.1)?;
    home_layout::initialize(&db).map_err(|e| e.1)?;
    withdraw_rules::initialize(&db).map_err(|e| e.1)?;
    risk::init(&db)?;
    rain::initialize(&db).map_err(|e| e.1)?;
    hall_requests::initialize(&db).map_err(|e| e.1)?;
    reward_policy::initialize(&db).map_err(|e| e.1)?;
    ad_mediation::initialize(&db).map_err(|e| e.1)?;
    sms_channel::initialize(&db).map_err(|e| e.1)?;
    cos_storage::initialize(&db).map_err(|e| e.1)?;
    business_guard::initialize(&db).map_err(|e| e.1)?;
    campaign::initialize(&db).map_err(|e| e.1)?;
    newbie::initialize(&db).map_err(|e| e.1)?;
    human_check::initialize(&db).map_err(|e| e.1)?;
    let has_attempts: bool = db
        .query_row("SELECT COUNT(*) FROM pragma_table_info('sms_codes') WHERE name='attempts'", [], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)
        .unwrap_or(true);
    if !has_attempts {
        db.execute_batch("ALTER TABLE sms_codes ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0")
            .map_err(|e| e.to_string())?;
    }
    behavior_audit::initialize(&db)
        .map_err(|e| format!("behavior audit initialization failed: {}", e.1))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = env::var_os("ORANGE_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("data/orange.sqlite3"));
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let state = AppState {
        db_path,
        config: RuntimeConfig::from_env()?,
    };
    init(&state)?;
    let admin = Router::new()
        .merge(admin_account::routes())
        .merge(mail_admin::admin_routes())
        .merge(mail_events::admin_routes())
        .merge(product_admin::admin_routes())
        .merge(app_config::admin_routes())
        .merge(home_layout::admin_routes())
        .merge(withdraw_rules::admin_routes())
        .merge(ad_mediation::admin_routes())
        .merge(sms_channel::admin_routes())
        .merge(trace_lookup::admin_routes())
        .merge(cos_storage::admin_routes())
        .merge(reward_policy::admin_routes())
        .merge(business_guard::admin_routes())
        .merge(campaign::admin_routes())
        .merge(newbie::admin_routes())
        .merge(human_check::admin_routes())
        .merge(rain::admin_routes())
        .merge(hall_requests::admin_routes())
        .merge(user_admin::routes())
        .merge(invitation_graph::admin_routes())
        .merge(behavior_audit::admin_routes())
        .merge(user_stats::admin_routes())
        .route("/overview", get(admin_overview))
        .route("/users", get(admin_users))
        .route("/account-deletions", get(admin_account_deletions))
        .route(
            "/account-deletions/{id}/review",
            post(review_account_deletion),
        )
        .route("/rewards", get(admin_rewards))
        .route("/withdrawals", get(admin_withdrawals))
        .route("/withdrawals/{id}", axum::routing::put(update_withdrawal))
        .route("/settings", get(admin_settings).put(update_settings))
        .route(
            "/hall-strategy",
            get(hall_strategy).put(update_hall_strategy),
        )
        .route(
            "/invitations",
            get(invitations::admin_overview).put(invitations::update_platform_code),
        )
        .fallback(api_not_found)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_account::authenticate,
        ));
    let api = Router::new()
        .merge(mail_admin::internal_routes())
        .merge(product_admin::routes())
        .merge(app_config::routes())
        .merge(home_layout::routes())
        .merge(withdraw_rules::routes())
        .merge(ad_mediation::routes())
        .merge(cos_storage::routes())
        .merge(reward_policy::routes())
        .merge(rain::routes())
        .merge(hall_requests::routes())
        .merge(risk::routes())
        .merge(campaign::routes())
        .merge(newbie::routes())
        .merge(device_bootstrap::routes())
        .merge(behavior_audit::public_routes())
        .route("/health", get(|| async { Json(json!({"ok":true})) }))
        .route("/config", get(public_config))
        .route("/auth/sms", post(sms))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/login/captcha", post(login_captcha))
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/logout", post(logout))
        .route("/account/delete", post(delete_account))
        .route("/account/deletion-status", get(account_deletion_status))
        .route("/account/deletion/cancel", post(cancel_account_deletion))
        .route("/me", get(me))
        .route("/tasks", get(tasks))
        .route("/check-in", post(check_in))
        .route("/check-in/status", get(checkin_status))
        .route("/check-in/makeup", post(check_in_makeup))
        .route("/rewards/start", post(reward_start))
        .route("/rewards/{id}/complete", post(reward_complete))
        .route("/rewards/{id}/status", get(reward_status))
        .route("/rewards/{id}/client-event", post(reward_client_event))
        .route("/ledger", get(ledger))
        .route("/products", get(products))
        .route("/invite/prebind", post(invitations::prebind))
        .route("/invite/sms", post(invitations::send_code))
        .route("/invite/bind", post(invitations::bind))
        .route("/invite/summary", get(invitations::summary))
        .route("/invite/qr/{code}", get(invitations::qr))
        .route("/hall/stream", get(hall_stream))
        .route("/withdrawals", get(withdrawals).post(withdraw))
        .route("/admin/login", post(admin_account::login))
        .nest("/admin", admin)
        .fallback(api_not_found)
        .layer(middleware::from_fn(json_error_responses));
    let web = env::var_os("ORANGE_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("../web/dist"));
    let origins=env_value("ORANGE_CORS_ORIGINS").unwrap_or_else(||"http://localhost:5173,http://127.0.0.1:5173,http://localhost:38180,http://127.0.0.1:38180".into())
        .split(',').map(|s|s.trim().parse::<HeaderValue>()).collect::<Result<Vec<_>,_>>()?;
    let app = Router::new()
        .merge(mail_events::public_routes())
        .merge(product_admin::media_routes())
        .nest("/api", api)
        .route("/csj/callback", get(csj_callback))
        .route_service("/admin", ServeFile::new(web.join("index.html")))
        .route_service("/admin/", ServeFile::new(web.join("index.html")))
        .route_service("/privacy", ServeFile::new(web.join("privacy.html")))
        .route_service("/terms", ServeFile::new(web.join("terms.html")))
        .route_service("/invite", ServeFile::new(web.join("index.html")))
        .route_service("/i/{code}", ServeFile::new(web.join("index.html")))
        .fallback_service(ServeDir::new(&web))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::HeaderName::from_static("x-admin-token"),
                    axum::http::header::HeaderName::from_static("x-risk-event"),
                    axum::http::header::HeaderName::from_static("x-product-api-key"),
                ]),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            user_admin::request_context,
        ))
        .with_state(state.clone());
    let addr = env::var("ORANGE_BIND").unwrap_or_else(|_| "127.0.0.1:38180".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("小橙有喜服务启动 http://{}", listener.local_addr()?);
    println!("SQLite {}", state.db_path.display());
    println!(
        "环境：{}，广告通道：{}，短信：后台配置，提现：mock",
        if state.config.production {
            "production"
        } else {
            "local"
        },
        state.config.ad_mode
    );
    if !state.config.production {
        println!("本地演示账号 13800138000 / Orange123!");
    }
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
