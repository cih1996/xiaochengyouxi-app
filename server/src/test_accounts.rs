//! Test-distribution accounts: admins issue API tokens ("接口令牌"), and a token holder creates
//! ready-to-use accounts with phone + password in one call. A new account is bound to the
//! platform invite code and released by an admin ("人工放行") so test devices are not
//! restricted by risk control; risk events are still recorded and withdrawals stay under review.
//! Only a SHA-256 digest of each token is stored; the token itself is shown once at creation.
use super::*;

const DAILY_LIMIT: i64 = 500;
pub(super) const ACTOR_PREFIX: &str = "接口令牌:";

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS api_tokens(
            id TEXT PRIMARY KEY,name TEXT NOT NULL,token_hash TEXT NOT NULL UNIQUE,prefix TEXT NOT NULL,
            scope TEXT NOT NULL,created_at INTEGER NOT NULL,created_by TEXT NOT NULL,
            last_used_at INTEGER,use_count INTEGER NOT NULL DEFAULT 0,revoked_at INTEGER);
         CREATE TABLE IF NOT EXISTS api_token_accounts(
            id INTEGER PRIMARY KEY AUTOINCREMENT,token_id TEXT NOT NULL,user_id INTEGER NOT NULL,
            created INTEGER NOT NULL,created_at INTEGER NOT NULL,client_ip TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS api_token_accounts_token ON api_token_accounts(token_id,created_at);",
    )?;
    Ok(())
}

fn digest(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// Resolves the bearer token (`Authorization: Bearer …` or `X-Api-Token`) to an active token.
fn authorize(db: &Connection, headers: &HeaderMap) -> Result<(String, String), ApiError> {
    let supplied = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-api-token").and_then(|v| v.to_str().ok()))
        .unwrap_or("")
        .trim();
    let unauthorized = || ApiError(StatusCode::UNAUTHORIZED, "接口令牌无效或已停用".into(), None);
    if !supplied.starts_with("oat_") || supplied.len() > 100 {
        return Err(unauthorized());
    }
    db.query_row(
        "SELECT id,name FROM api_tokens WHERE token_hash=?1 AND revoked_at IS NULL AND scope='test_accounts'",
        [digest(supplied)],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(unauthorized)
}

#[derive(Deserialize)]
struct CreateInput {
    phone: String,
    password: String,
    #[serde(default)]
    nickname: Option<String>,
}

async fn create_account(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<CreateInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    let mut db = open(&state)?;
    let (token_id, token_name) = authorize(&db, &headers)?;
    let hash = password_hash(&input.password)?;
    let tx = db.transaction()?;
    let today: i64 = tx.query_row(
        "SELECT COUNT(*) FROM api_token_accounts WHERE token_id=?1 AND created=1 AND created_at>=?2",
        params![token_id, today_start()],
        |r| r.get(0),
    )?;
    let existing: Option<i64> = tx
        .query_row("SELECT id FROM users WHERE phone=?1", [&input.phone], |r| r.get(0))
        .optional()?;
    if existing.is_none() && today >= DAILY_LIMIT {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            format!("该令牌今日已创建 {DAILY_LIMIT} 个账号，请明天再试"),
            None,
        ));
    }
    let platform = invitations::platform_code(&tx)?;
    let actor = format!("{ACTOR_PREFIX}{token_name}");
    let (uid, created) = match existing {
        // Existing accounts are only whitelisted: the password is never changed through a token.
        Some(id) => (id, false),
        None => {
            let nickname = input
                .nickname
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(20).collect::<String>())
                .unwrap_or_else(|| format!("测试{}", &input.phone[7..]));
            tx.execute(
                "INSERT INTO users(phone,password_hash,nickname,coins,created_at,invite_code,invitation_bound_at,inviter_code) VALUES(?1,?2,?3,0,?4,?5,?4,?6)",
                params![input.phone, hash, nickname, now(), invitations::new_code(&tx)?, platform],
            )?;
            (tx.last_insert_rowid(), true)
        }
    };
    // An admin restriction ("restricted"/"review") set by a person is never overridden here.
    let manual_block: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM account_restrictions WHERE user_id=?1 AND status!='active' AND updated_by NOT IN ('DeviceTrust','BehaviorAudit'))",
        [uid],
        |r| r.get(0),
    )?;
    if manual_block {
        return Err(conflict("该账号已被后台人工限制，请在后台处理后再试"));
    }
    tx.execute(
        "INSERT INTO account_restrictions(user_id,status,reason,updated_at,updated_by,trace_id) VALUES(?1,'active',?2,?3,?4,?5)
         ON CONFLICT(user_id) DO UPDATE SET status='active',reason=excluded.reason,updated_at=excluded.updated_at,updated_by=excluded.updated_by,trace_id=excluded.trace_id",
        params![uid, "测试白名单（接口创建）", now(), actor, Uuid::new_v4().to_string()],
    )?;
    tx.execute(
        "INSERT INTO api_token_accounts(token_id,user_id,created,created_at,client_ip) VALUES(?1,?2,?3,?4,?5)",
        params![token_id, uid, created as i64, now(), context.client_ip],
    )?;
    tx.execute(
        "UPDATE api_tokens SET last_used_at=?1,use_count=use_count+1 WHERE id=?2",
        params![now(), token_id],
    )?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        Some(&input.phone),
        if created { "register" } else { "whitelist" },
        "success",
        &uid.to_string(),
        None,
        json!({"method":"api_token","token":token_name}),
    )?;
    let invite_code: String =
        tx.query_row("SELECT invite_code FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    tx.commit()?;
    Ok(Json(json!({
        "user_id": uid,
        "phone": input.phone,
        "created": created,
        "whitelisted": true,
        "inviter_code": platform,
        "invite_code": invite_code,
        "message": if created { "账号已创建并加入白名单" } else { "手机号已注册：已加入白名单，密码未修改" },
    })))
}

fn token_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": r.get::<_, String>(0)?,
        "name": r.get::<_, String>(1)?,
        "prefix": r.get::<_, String>(2)?,
        "created_at": iso(r.get(3)?),
        "created_by": r.get::<_, String>(4)?,
        "last_used_at": r.get::<_, Option<i64>>(5)?.map(iso),
        "use_count": r.get::<_, i64>(6)?,
        "accounts": r.get::<_, i64>(7)?,
        "revoked_at": r.get::<_, Option<i64>>(8)?.map(iso),
    }))
}

fn list_value(state: &AppState, db: &Connection) -> Result<Value, ApiError> {
    let mut stmt = db.prepare(
        "SELECT t.id,t.name,t.prefix,t.created_at,t.created_by,t.last_used_at,t.use_count,
                (SELECT COUNT(*) FROM api_token_accounts a WHERE a.token_id=t.id AND a.created=1),t.revoked_at
         FROM api_tokens t ORDER BY t.revoked_at IS NOT NULL,t.created_at DESC",
    )?;
    let tokens = stmt.query_map([], token_row)?.collect::<Result<Vec<_>, _>>()?;
    let mut recent = db.prepare(
        "SELECT a.created_at,u.id,u.phone,u.nickname,a.created,t.name FROM api_token_accounts a
         JOIN users u ON u.id=a.user_id JOIN api_tokens t ON t.id=a.token_id ORDER BY a.id DESC LIMIT 50",
    )?;
    let accounts = recent
        .query_map([], |r| {
            Ok(json!({"at":iso(r.get(0)?),"user_id":r.get::<_,i64>(1)?,"phone":r.get::<_,String>(2)?,"nickname":r.get::<_,String>(3)?,"created":r.get::<_,i64>(4)?==1,"token":r.get::<_,String>(5)?}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "tokens": tokens,
        "recent_accounts": accounts,
        "endpoint": format!("{}/api/integration/test-accounts", ad_mediation::callback_base(state)),
        "daily_limit": DAILY_LIMIT,
        "platform_code": invitations::platform_code(db)?,
    }))
}

async fn admin_list(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    Ok(Json(list_value(&state, &db)?))
}

#[derive(Deserialize)]
struct NewToken {
    name: String,
}

async fn admin_create(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<NewToken>,
) -> ApiResult<Value> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 30 {
        return Err(invalid("令牌名称需为 1–30 个字"));
    }
    let token = format!("oat_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let id = Uuid::new_v4().to_string();
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO api_tokens(id,name,token_hash,prefix,scope,created_at,created_by) VALUES(?1,?2,?3,?4,'test_accounts',?5,?6)",
        params![id, name, digest(&token), &token[..10], now(), context.admin_username.as_deref().unwrap_or("local-preview")],
    )?;
    user_admin::action_audit(&tx, &context, None, "api_token_create", &id, &format!("创建接口令牌：{name}"), json!({}), json!({"name":name,"scope":"test_accounts"}))?;
    let mut result = list_value(&state, &tx)?;
    tx.commit()?;
    result["created_token"] = json!({"id": id, "name": name, "token": token});
    Ok(Json(result))
}

async fn admin_revoke(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let name: Option<String> = tx
        .query_row("SELECT name FROM api_tokens WHERE id=?1 AND revoked_at IS NULL", [&id], |r| r.get(0))
        .optional()?;
    let Some(name) = name else {
        return Err(ApiError(StatusCode::NOT_FOUND, "令牌不存在或已停用".into(), None));
    };
    tx.execute("UPDATE api_tokens SET revoked_at=?1 WHERE id=?2", params![now(), id])?;
    user_admin::action_audit(&tx, &context, None, "api_token_revoke", &id, &format!("停用接口令牌：{name}"), json!({"name":name}), json!({"revoked":true}))?;
    let result = list_value(&state, &tx)?;
    tx.commit()?;
    Ok(Json(result))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/integration/test-accounts", post(create_account))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api-tokens", get(admin_list).post(admin_create))
        .route("/api-tokens/{id}/revoke", post(admin_revoke))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE api_tokens(id TEXT PRIMARY KEY,name TEXT NOT NULL,token_hash TEXT NOT NULL UNIQUE,prefix TEXT NOT NULL,scope TEXT NOT NULL,created_at INTEGER NOT NULL,created_by TEXT NOT NULL,last_used_at INTEGER,use_count INTEGER NOT NULL DEFAULT 0,revoked_at INTEGER);",
        )
        .unwrap();
        db
    }

    fn headers(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        h
    }

    #[test]
    fn only_active_tokens_authorize_and_only_digests_are_stored() {
        let db = db();
        let token = "oat_abc123";
        db.execute("INSERT INTO api_tokens VALUES('t1','测试分发',?1,'oat_abc123','test_accounts',0,'admin',NULL,0,NULL)", [digest(token)]).unwrap();
        assert_eq!(authorize(&db, &headers(token)).unwrap().1, "测试分发");
        assert!(authorize(&db, &headers("oat_wrong")).is_err());
        assert!(authorize(&db, &HeaderMap::new()).is_err());
        let mut x = HeaderMap::new();
        x.insert("x-api-token", token.parse().unwrap());
        assert!(authorize(&db, &x).is_ok());
        let stored: String = db.query_row("SELECT token_hash FROM api_tokens", [], |r| r.get(0)).unwrap();
        assert_ne!(stored, token);
        db.execute("UPDATE api_tokens SET revoked_at=1", []).unwrap();
        assert!(authorize(&db, &headers(token)).is_err());
    }
}
