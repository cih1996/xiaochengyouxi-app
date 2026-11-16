use super::*;

const ENTRY_PATH: &str = "/mailbox/";
#[derive(Clone, Default)]
pub(super) struct Config {
    secret: Option<String>,
    username: Option<String>,
    password: Option<String>,
}
impl Config {
    pub(super) fn from_env() -> Self {
        Self {
            secret: env_value("ORANGE_MAIL_SSO_SECRET"),
            username: env_value("ORANGE_MAIL_USERNAME"),
            password: env_value("ORANGE_MAIL_PASSWORD"),
        }
    }
    fn problem(&self) -> Option<String> {
        let missing = [
            ("ORANGE_MAIL_SSO_SECRET", &self.secret),
            ("ORANGE_MAIL_USERNAME", &self.username),
            ("ORANGE_MAIL_PASSWORD", &self.password),
        ]
        .into_iter()
        .filter_map(|(name, value)| if value.is_none() { Some(name) } else { None })
        .collect::<Vec<_>>();
        if missing.is_empty() {
            None
        } else {
            Some(format!("邮件服务尚未配置：缺少 {}", missing.join("、")))
        }
    }
    fn credentials(&self) -> Result<(&str, &str, &str), ApiError> {
        if let Some(error) = self.problem() {
            return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, error, None));
        }
        Ok((
            self.secret.as_deref().unwrap(),
            self.username.as_deref().unwrap(),
            self.password.as_deref().unwrap(),
        ))
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS mail_sso_tickets(ticket_hash TEXT PRIMARY KEY,admin_username TEXT NOT NULL,admin_session_hash TEXT NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,consumed_at INTEGER);
      CREATE INDEX IF NOT EXISTS idx_mail_ticket_expiry ON mail_sso_tickets(expires_at);")
}
fn denied(message: &str) -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, message.into(), None)
}
async fn status(State(state): State<AppState>) -> ApiResult<Value> {
    let error = state.config.mail.problem();
    let mut value = json!({"enabled":error.is_none(),"entry_path":ENTRY_PATH});
    if let Some(error) = error {
        value["error"] = json!(error);
    }
    Ok(Json(value))
}
async fn create_session(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let account =
        admin_account::session(&state, &tx, &headers)?.ok_or_else(|| denied("请先登录管理后台"))?;
    state.config.mail.credentials()?;
    let token = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| denied("请先登录管理后台"))?;
    let stamp = now();
    let remaining = account.expires_at - stamp;
    if remaining <= 0 {
        return Err(denied("后台登录已过期，请重新登录"));
    }
    let ticket = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let expires = (stamp + 60).min(account.expires_at);
    tx.execute("DELETE FROM mail_sso_tickets WHERE expires_at<=?1", [stamp])?;
    tx.execute("INSERT INTO mail_sso_tickets(ticket_hash,admin_username,admin_session_hash,created_at,expires_at) VALUES(?1,?2,?3,?4,?5)",params![admin_account::token_hash(&ticket),account.username,admin_account::token_hash(token),stamp,expires])?;
    let mut actor = context;
    actor.admin_username = Some(account.username.clone());
    user_admin::action_audit(
        &tx,
        &actor,
        None,
        "mail_session_create",
        "mail-sso",
        "进入后台邮箱",
        json!({}),
        json!({"username":account.username,"expires_at":iso(expires)}),
    )?;
    tx.commit()?;
    let cookie = format!(
        "orange_mail_admin={token}; Path=/mailbox/; Secure; HttpOnly; SameSite=Lax; Max-Age={}",
        remaining.min(12 * 3600)
    );
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?,
    );
    Ok((
        response_headers,
        Json(json!({"ticket":ticket,"entry_path":ENTRY_PATH})),
    ))
}
#[derive(Deserialize)]
struct RedeemInput {
    ticket: String,
}
async fn redeem(
    State(state): State<AppState>,
    Extension(mut context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<RedeemInput>,
) -> ApiResult<Value> {
    let (secret, username, password) = state.config.mail.credentials()?;
    let supplied = headers
        .get("x-mail-sso-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !equal_secret(secret, supplied) {
        return Err(denied("邮件内部认证失败"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let admin:Option<String>=tx.query_row("UPDATE mail_sso_tickets SET consumed_at=?1 WHERE ticket_hash=?2 AND consumed_at IS NULL AND expires_at>?1 AND EXISTS(SELECT 1 FROM admin_sessions s JOIN admin_account a ON a.id=s.admin_id WHERE s.token_hash=mail_sso_tickets.admin_session_hash AND s.expires_at>?1 AND a.username=mail_sso_tickets.admin_username) RETURNING admin_username",params![now(),admin_account::token_hash(&input.ticket)],|r|r.get(0)).optional()?;
    let admin = admin.ok_or_else(|| denied("邮箱登录链接已失效，请从后台重新进入"))?;
    context.admin_username = Some(admin.clone());
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "mail_session_redeem",
        "mail-sso",
        "兑换一次性邮箱登录",
        json!({}),
        json!({"username":admin}),
    )?;
    tx.commit()?;
    Ok(Json(json!({"username":username,"password":password})))
}
async fn access(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|part| part.trim().strip_prefix("orange_mail_admin="))
        .filter(|v| !v.is_empty())
        .ok_or_else(|| denied("请先登录管理后台"))?;
    let mut session_headers = HeaderMap::new();
    session_headers.insert(
        "x-admin-token",
        HeaderValue::from_str(token).map_err(|_| denied("后台登录无效"))?,
    );
    admin_account::session(&state, &open(&state)?, &session_headers)?
        .ok_or_else(|| denied("请先登录管理后台"))?;
    Ok(StatusCode::NO_CONTENT)
}
async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    response
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/mail/status", get(status))
        .route("/mail/session", post(create_session))
        .route_layer(middleware::from_fn(no_store))
}
pub(super) fn internal_routes() -> Router<AppState> {
    Router::new()
        .route("/mail-sso/redeem", post(redeem))
        .route("/mail-sso/access", get(access))
        .route_layer(middleware::from_fn(no_store))
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn fixture() -> (AppState, RequestContext, HeaderMap) {
        let (mut state, mut context, _, _) = rain::tests::fixture(false, false);
        state.config.admin_password = Some("MailAdmin123!".into());
        state.config.mail = Config {
            secret: Some("mail-internal-test-secret".into()),
            username: Some("service-mail@example.test".into()),
            password: Some("private-mail-password".into()),
        };
        admin_account::initialize(&state, &open(&state).unwrap()).unwrap();
        let input = serde_json::from_value::<admin_account::LoginInput>(
            json!({"username":"admin","password":"MailAdmin123!"}),
        )
        .unwrap();
        let login = admin_account::login(
            State(state.clone()),
            Extension(context.clone()),
            Json(input),
        )
        .await
        .unwrap()
        .0;
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-admin-token",
            login["token"].as_str().unwrap().parse().unwrap(),
        );
        context.admin_username = Some("admin".into());
        (state, context, headers)
    }
    fn internal_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-mail-sso-secret",
            HeaderValue::from_static("mail-internal-test-secret"),
        );
        headers
    }
    fn cookie_headers(headers: &HeaderMap) -> HeaderMap {
        let cookie = headers[axum::http::header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::COOKIE, cookie.parse().unwrap());
        headers
    }
    #[tokio::test]
    async fn ticket_redeems_once_with_internal_secret_and_no_public_credential_leak() {
        let (state, context, headers) = fixture().await;
        let (cookie, created) = create_session(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
        )
        .await
        .unwrap();
        let raw = created.0;
        let ticket = raw["ticket"].as_str().unwrap();
        assert_eq!(raw["entry_path"], "/mailbox/");
        assert_eq!(
            access(State(state.clone()), cookie_headers(&cookie))
                .await
                .unwrap(),
            StatusCode::NO_CONTENT
        );
        let db = open(&state).unwrap();
        let stored: String = db
            .query_row("SELECT ticket_hash FROM mail_sso_tickets", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stored, admin_account::token_hash(ticket));
        assert_ne!(stored, ticket);
        let wrong = redeem(
            State(state.clone()),
            Extension(context.clone()),
            HeaderMap::new(),
            Json(RedeemInput {
                ticket: ticket.into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
        let redeemed = redeem(
            State(state.clone()),
            Extension(context.clone()),
            internal_headers(),
            Json(RedeemInput {
                ticket: ticket.into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(redeemed["username"], "service-mail@example.test");
        assert_eq!(redeemed["password"], "private-mail-password");
        let repeat = redeem(
            State(state.clone()),
            Extension(context.clone()),
            internal_headers(),
            Json(RedeemInput {
                ticket: ticket.into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(repeat.0, StatusCode::UNAUTHORIZED);
        let public = status(State(state.clone())).await.unwrap().0;
        let audit = db
            .prepare("SELECT actor||filters_json||before_json||after_json||reason FROM admin_audit")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("");
        for secret in [
            ticket,
            headers["x-admin-token"].to_str().unwrap(),
            "private-mail-password",
            "mail-internal-test-secret",
        ] {
            assert!(!audit.contains(secret));
            assert!(!public.to_string().contains(secret));
        }
        assert!(!raw.to_string().contains("private-mail-password"));
        assert!(!raw.to_string().contains("service-mail@example.test"));
        assert_eq!(
            db.query_row(
                "SELECT actor FROM admin_audit WHERE action='mail_session_redeem'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "admin"
        );
    }
    #[tokio::test]
    async fn expired_ticket_and_revoked_original_session_block_redeem_and_mail_cookie() {
        let (state, context, headers) = fixture().await;
        let db = open(&state).unwrap();
        db.execute("UPDATE admin_sessions SET expires_at=?1", [now() + 30])
            .unwrap();
        let (cookie, created) = create_session(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
        )
        .await
        .unwrap();
        let cookie_text = cookie[axum::http::header::SET_COOKIE].to_str().unwrap();
        for attribute in ["Path=/mailbox/", "Secure", "HttpOnly", "SameSite=Lax"] {
            assert!(cookie_text.contains(attribute));
        }
        let age = cookie_text
            .split("Max-Age=")
            .nth(1)
            .unwrap()
            .parse::<i64>()
            .unwrap();
        assert!((1..=30).contains(&age));
        let ticket = created.0["ticket"].as_str().unwrap().to_owned();
        db.execute("UPDATE mail_sso_tickets SET expires_at=?1", [now() - 1])
            .unwrap();
        assert_eq!(
            redeem(
                State(state.clone()),
                Extension(context.clone()),
                internal_headers(),
                Json(RedeemInput { ticket })
            )
            .await
            .unwrap_err()
            .0,
            StatusCode::UNAUTHORIZED
        );
        let (_, fresh) = create_session(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
        )
        .await
        .unwrap();
        db.execute("DELETE FROM admin_sessions", []).unwrap();
        assert_eq!(
            access(State(state.clone()), cookie_headers(&cookie))
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            redeem(
                State(state.clone()),
                Extension(context.clone()),
                internal_headers(),
                Json(RedeemInput {
                    ticket: fresh.0["ticket"].as_str().unwrap().into()
                })
            )
            .await
            .unwrap_err()
            .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            create_session(State(state), Extension(context), headers)
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    #[tokio::test]
    async fn unconfigured_service_and_missing_login_are_explicit_and_mail_cookie_does_not_auth_admin_api()
     {
        let (mut state, context, headers) = fixture().await;
        assert_eq!(
            create_session(
                State(state.clone()),
                Extension(context.clone()),
                HeaderMap::new()
            )
            .await
            .unwrap_err()
            .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            access(State(state.clone()), headers.clone())
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        let (cookie, _) = create_session(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
        )
        .await
        .unwrap();
        assert!(
            admin_account::session(&state, &open(&state).unwrap(), &cookie_headers(&cookie))
                .is_err()
        );
        state.config.mail = Config::default();
        let status = status(State(state.clone())).await.unwrap().0;
        assert_eq!(status["enabled"], false);
        assert!(
            status["error"]
                .as_str()
                .unwrap()
                .contains("ORANGE_MAIL_SSO_SECRET")
        );
        assert_eq!(
            create_session(State(state), Extension(context), headers)
                .await
                .unwrap_err()
                .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
