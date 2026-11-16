use super::*;

const SESSION_SECONDS: i64 = 12 * 3600;

pub(super) fn initialize(state: &AppState, db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS admin_account(id INTEGER PRIMARY KEY CHECK(id=1),username TEXT NOT NULL UNIQUE,password_hash TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS admin_sessions(token_hash TEXT PRIMARY KEY,admin_id INTEGER NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL);
      CREATE INDEX IF NOT EXISTS idx_admin_session_expiry ON admin_sessions(expires_at);")?;
    if exists(db)? {
        return Ok(());
    }
    if let Some(password) = &state.config.admin_password {
        let username = username(&state.config.admin_username)?;
        let hash = password_hash(password)?;
        db.execute("INSERT OR IGNORE INTO admin_account(id,username,password_hash,created_at,updated_at) VALUES(1,?1,?2,?3,?3)",params![username,hash,now()])?;
    } else if state.config.production {
        return Err(invalid("首次建立后台账号需要 ORANGE_ADMIN_PASSWORD"));
    }
    Ok(())
}
fn exists(db: &Connection) -> Result<bool, ApiError> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM admin_account WHERE id=1)",
        [],
        |r| r.get(0),
    )?)
}
pub(super) fn required(state: &AppState, db: &Connection) -> Result<bool, ApiError> {
    Ok(exists(db)? || state.config.production || state.config.device_trust.is_some())
}
fn username(raw: &str) -> Result<String, ApiError> {
    let value = raw.trim();
    if !(2..=40).contains(&value.chars().count())
        || !value
            .chars()
            .all(|c| c.is_alphanumeric() || "_.-".contains(c))
    {
        return Err(invalid("账号需为 2 至 40 位文字、数字或 _ . -"));
    }
    Ok(value.into())
}
fn denied() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "请登录管理后台".into(), None)
}
fn incorrect() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "账号或密码错误".into(), None)
}
fn verify(hash: &str, password: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(error) => Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
            None,
        )),
    }
}
pub(super) fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
fn request_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .ok_or_else(denied)
}
pub(super) struct Session {
    pub username: String,
    pub expires_at: i64,
}
pub(super) fn session(
    state: &AppState,
    db: &Connection,
    headers: &HeaderMap,
) -> Result<Option<Session>, ApiError> {
    if !required(state, db)? {
        return Ok(None);
    }
    let token = request_token(headers)?;
    let value=db.query_row("SELECT a.username,s.expires_at FROM admin_sessions s JOIN admin_account a ON a.id=s.admin_id WHERE s.token_hash=?1 AND s.expires_at>?2",params![token_hash(token),now()],|r|Ok(Session{username:r.get(0)?,expires_at:r.get(1)?})).optional()?;
    value.map(Some).ok_or_else(denied)
}
pub(super) async fn authenticate(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let (context, audit) = {
        let db = open(&state)?;
        let session = session(&state, &db, request.headers())?;
        let mut context = request
            .extensions()
            .get::<RequestContext>()
            .expect("request context middleware")
            .clone();
        context.admin_username = Some(
            session
                .map(|s| s.username)
                .unwrap_or_else(|| "local-preview".into()),
        );
        let audit = user_admin::access_audit(&db, &context, request.method().as_str(), &uri)?;
        (context, audit)
    };
    request.extensions_mut().insert(context);
    let response = next.run(request).await;
    user_admin::finish_access(&open(&state)?, &audit, response.status().as_u16())?;
    Ok(response)
}
#[derive(Deserialize)]
pub(super) struct LoginInput {
    username: String,
    password: String,
}
pub(super) async fn login(
    State(state): State<AppState>,
    Extension(mut context): Extension<RequestContext>,
    Json(input): Json<LoginInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let row: Option<(String, String)> = tx
        .query_row(
            "SELECT username,password_hash FROM admin_account WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((username, hash)) = row else {
        return Err(incorrect());
    };
    if input.username.trim() != username || !verify(&hash, &input.password)? {
        return Err(incorrect());
    }
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let expires = now() + SESSION_SECONDS;
    tx.execute("DELETE FROM admin_sessions WHERE expires_at<=?1", [now()])?;
    tx.execute(
        "INSERT INTO admin_sessions(token_hash,admin_id,created_at,expires_at) VALUES(?1,1,?2,?3)",
        params![token_hash(&token), now(), expires],
    )?;
    context.admin_username = Some(username.clone());
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "admin_login",
        "admin-account",
        "后台登录",
        json!({}),
        json!({"username":username}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"token":token,"username":username,"expires_at":iso(expires)}),
    ))
}
async fn account(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let session = session(&state, &db, &headers)?;
    Ok(Json(match session {
        Some(s) => {
            json!({"username":s.username,"auth_required":true,"session_expires_at":iso(s.expires_at)})
        }
        None => json!({"username":"","auth_required":false,"session_expires_at":null}),
    }))
}
#[derive(Deserialize)]
struct ChangeInput {
    username: String,
    current_password: String,
    new_password: Option<String>,
}
async fn update(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<ChangeInput>,
) -> ApiResult<Value> {
    let next_username = username(&input.username)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    if session(&state, &tx, &headers)?.is_none() {
        return Err(invalid("请先在启动配置中设置后台账号"));
    }
    let (old_username, old_hash): (String, String) = tx.query_row(
        "SELECT username,password_hash FROM admin_account WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if !verify(&old_hash, &input.current_password)? {
        return Err(invalid("当前密码错误"));
    }
    if next_username == old_username && input.new_password.is_none() {
        return Err(invalid("请修改账号或设置新密码"));
    }
    let hash = match input.new_password.as_deref() {
        Some(value) => password_hash(value)?,
        None => old_hash,
    };
    tx.execute(
        "UPDATE admin_account SET username=?1,password_hash=?2,updated_at=?3 WHERE id=1",
        params![next_username, hash, now()],
    )?;
    tx.execute("DELETE FROM admin_sessions", [])?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "admin_account_change",
        "admin-account",
        "修改后台账号凭据",
        json!({"username":old_username}),
        json!({"username":next_username,"password_changed":input.new_password.is_some(),"sessions_revoked":true}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"username":next_username,"reauthenticate":true}),
    ))
}
async fn logout(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    if session(&state, &tx, &headers)?.is_some() {
        tx.execute(
            "DELETE FROM admin_sessions WHERE token_hash=?1",
            [token_hash(request_token(&headers)?)],
        )?;
    }
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "admin_logout",
        "admin-account",
        "退出后台",
        json!({}),
        json!({}),
    )?;
    tx.commit()?;
    Ok(Json(json!({"ok":true})))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/account", get(account).put(update))
        .route("/logout", post(logout))
}

#[cfg(test)]
mod tests {
    use super::*;
    const OLD_PASSWORD: &str = "InitialAdmin123!";
    const NEW_PASSWORD: &str = "ChangedAdmin456!";
    fn fixture() -> (AppState, RequestContext) {
        let (mut state, context, _, _) = rain::tests::fixture(false, false);
        state.config.admin_username = "operator".into();
        state.config.admin_password = Some(OLD_PASSWORD.into());
        initialize(&state, &open(&state).unwrap()).unwrap();
        (state, context)
    }
    async fn sign_in(
        state: &AppState,
        context: &RequestContext,
        name: &str,
        password: &str,
    ) -> Result<Value, ApiError> {
        login(
            State(state.clone()),
            Extension(context.clone()),
            Json(LoginInput {
                username: name.into(),
                password: password.into(),
            }),
        )
        .await
        .map(|v| v.0)
    }
    fn headers(value: &Value) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-admin-token",
            value["token"].as_str().unwrap().parse().unwrap(),
        );
        headers
    }
    #[tokio::test]
    async fn wrong_credentials_expiry_logout_and_old_static_token_are_rejected() {
        let (state, mut context) = fixture();
        let db = open(&state).unwrap();
        assert_eq!(
            sign_in(&state, &context, "wrong-user", OLD_PASSWORD)
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            sign_in(&state, &context, "operator", "wrong-password")
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        let first = sign_in(&state, &context, "operator", OLD_PASSWORD)
            .await
            .unwrap();
        let second = sign_in(&state, &context, "operator", OLD_PASSWORD)
            .await
            .unwrap();
        assert_ne!(first["token"], second["token"]);
        assert_eq!(
            session(&state, &db, &headers(&first))
                .unwrap()
                .unwrap()
                .username,
            "operator"
        );
        let stored: String = db
            .query_row(
                "SELECT token_hash FROM admin_sessions WHERE token_hash=?1",
                [token_hash(first["token"].as_str().unwrap())],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(stored, first["token"].as_str().unwrap());
        let mut legacy = HeaderMap::new();
        legacy.insert(
            "x-admin-token",
            HeaderValue::from_static("old-static-environment-token"),
        );
        assert!(session(&state, &db, &legacy).is_err());
        db.execute(
            "UPDATE admin_sessions SET expires_at=?1 WHERE token_hash=?2",
            params![now() - 1, stored],
        )
        .unwrap();
        assert!(session(&state, &db, &headers(&first)).is_err());
        context.admin_username = Some("operator".into());
        let _ = logout(State(state.clone()), Extension(context), headers(&second))
            .await
            .unwrap();
        assert!(session(&state, &db, &headers(&second)).is_err());
    }
    #[tokio::test]
    async fn credential_change_revokes_every_session_and_audits_safe_fields_under_actual_actor() {
        let (state, mut context) = fixture();
        let db = open(&state).unwrap();
        let first = sign_in(&state, &context, "operator", OLD_PASSWORD)
            .await
            .unwrap();
        let other = sign_in(&state, &context, "operator", OLD_PASSWORD)
            .await
            .unwrap();
        context.admin_username = Some("operator".into());
        let invalid = update(
            State(state.clone()),
            Extension(context.clone()),
            headers(&first),
            Json(ChangeInput {
                username: "new-operator".into(),
                current_password: "bad-password".into(),
                new_password: Some(NEW_PASSWORD.into()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(invalid.0, StatusCode::BAD_REQUEST);
        assert!(session(&state, &db, &headers(&other)).is_ok());
        let changed = update(
            State(state.clone()),
            Extension(context.clone()),
            headers(&first),
            Json(ChangeInput {
                username: "new-operator".into(),
                current_password: OLD_PASSWORD.into(),
                new_password: Some(NEW_PASSWORD.into()),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(changed["reauthenticate"], true);
        assert!(session(&state, &db, &headers(&first)).is_err());
        assert!(session(&state, &db, &headers(&other)).is_err());
        assert!(
            sign_in(&state, &context, "operator", OLD_PASSWORD)
                .await
                .is_err()
        );
        assert!(
            sign_in(&state, &context, "new-operator", OLD_PASSWORD)
                .await
                .is_err()
        );
        let fresh = sign_in(&state, &context, "new-operator", NEW_PASSWORD)
            .await
            .unwrap();
        let me = account(State(state.clone()), headers(&fresh))
            .await
            .unwrap()
            .0;
        assert_eq!(me["username"], "new-operator");
        let (actor,before,after):(String,String,String)=db.query_row("SELECT actor,before_json,after_json FROM admin_audit WHERE action='admin_account_change'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(actor, "operator");
        assert!(before.contains("operator"));
        assert!(after.contains("new-operator"));
        let raw = db
            .prepare("SELECT actor||filters_json||before_json||after_json||reason FROM admin_audit")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("");
        for secret in [
            OLD_PASSWORD,
            NEW_PASSWORD,
            first["token"].as_str().unwrap(),
            fresh["token"].as_str().unwrap(),
        ] {
            assert!(!raw.contains(secret));
        }
        assert!(
            db.query_row(
                "SELECT password_hash FROM admin_account WHERE id=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap()
            .starts_with("$argon2id$")
        );
        context.admin_username = Some("new-operator".into());
        let _ = update(
            State(state.clone()),
            Extension(context.clone()),
            headers(&fresh),
            Json(ChangeInput {
                username: "阿东".into(),
                current_password: NEW_PASSWORD.into(),
                new_password: None,
            }),
        )
        .await
        .unwrap();
        assert!(session(&state, &db, &headers(&fresh)).is_err());
        assert!(
            sign_in(&state, &context, "阿东", NEW_PASSWORD)
                .await
                .is_ok()
        );
        assert!(
            sign_in(&state, &context, "new-operator", NEW_PASSWORD)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn stored_credentials_survive_restart_and_removed_or_changed_bootstrap_environment() {
        let (mut state, mut context) = fixture();
        let initial = sign_in(&state, &context, "operator", OLD_PASSWORD)
            .await
            .unwrap();
        context.admin_username = Some("operator".into());
        let _ = update(
            State(state.clone()),
            Extension(context.clone()),
            headers(&initial),
            Json(ChangeInput {
                username: "persisted-user".into(),
                current_password: OLD_PASSWORD.into(),
                new_password: Some(NEW_PASSWORD.into()),
            }),
        )
        .await
        .unwrap();
        state.config.admin_username = "env-reset-user".into();
        state.config.admin_password = Some("env-reset-password".into());
        initialize(&state, &open(&state).unwrap()).unwrap();
        assert!(
            sign_in(&state, &context, "env-reset-user", "env-reset-password")
                .await
                .is_err()
        );
        assert!(
            sign_in(&state, &context, "persisted-user", NEW_PASSWORD)
                .await
                .is_ok()
        );
        state.config.admin_password = None;
        state.config.production = true;
        initialize(&state, &open(&state).unwrap()).unwrap();
        assert!(required(&state, &open(&state).unwrap()).unwrap());
        assert_eq!(
            public_config(State(state.clone())).await.unwrap().0["admin_auth_required"],
            true
        );
        assert!(session(&state, &open(&state).unwrap(), &HeaderMap::new()).is_err());
        assert!(
            sign_in(&state, &context, "persisted-user", NEW_PASSWORD)
                .await
                .is_ok()
        );
    }
    #[tokio::test]
    async fn only_unconfigured_local_demo_is_open_and_production_requires_bootstrap() {
        let (mut state, _, _, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        assert!(!required(&state, &db).unwrap());
        assert_eq!(
            account(State(state.clone()), HeaderMap::new())
                .await
                .unwrap()
                .0["auth_required"],
            false
        );
        state.config.production = true;
        assert!(initialize(&state, &db).is_err());
        assert!(session(&state, &db, &HeaderMap::new()).is_err());
        state.config.production = false;
        state.config.device_trust = Some(risk::Config::test_config());
        assert!(required(&state, &db).unwrap());
        assert!(session(&state, &db, &HeaderMap::new()).is_err());
    }
    #[tokio::test]
    async fn all_admin_routes_use_session_middleware_and_record_real_username() {
        let (state, _) = fixture();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let protected = routes()
            .route(
                "/probe",
                get(|Extension(context): Extension<RequestContext>| async move {
                    Json(json!({"actor":context.admin_username}))
                }),
            )
            .fallback(api_not_found)
            .layer(middleware::from_fn_with_state(state.clone(), authenticate));
        let app = Router::new()
            .route("/api/admin/login", post(login))
            .nest("/api/admin", protected)
            .layer(middleware::from_fn_with_state(
                state.clone(),
                user_admin::request_context,
            ))
            .with_state(state.clone());
        let service = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let client = reqwest::Client::new();
        for path in [
            "/api/admin/probe",
            "/api/admin/account",
            "/api/admin/not-real",
        ] {
            assert_eq!(
                client
                    .get(format!("{base}{path}"))
                    .header("x-admin-token", "legacy-static-token")
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::UNAUTHORIZED
            );
        }
        let result: Value = client
            .post(format!("{base}/api/admin/login"))
            .json(&json!({"username":"operator","password":OLD_PASSWORD}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let token = result["token"].as_str().unwrap();
        let probe: Value = client
            .get(format!("{base}/api/admin/probe"))
            .header("x-admin-token", token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(probe["actor"], "operator");
        assert_eq!(
            client
                .post(format!("{base}/api/admin/logout"))
                .header("x-admin-token", token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/api/admin/account"))
                .header("x-admin-token", token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let db = open(&state).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT actor FROM admin_audit WHERE action='admin_access' LIMIT 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "operator"
        );
        service.abort();
    }
}
