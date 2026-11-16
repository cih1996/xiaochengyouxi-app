use super::*;
use axum::extract::rejection::JsonRejection;

#[derive(Clone, Default)]
pub(super) struct Config {
    token: Option<String>,
    sender: Option<String>,
}
impl Config {
    pub(super) fn from_env() -> Self {
        Self {
            token: env_value("ORANGE_MAIL_TENCENT_CALLBACK_TOKEN"),
            sender: env_value("ORANGE_MAIL_USERNAME"),
        }
    }
    fn authorize(&self, supplied: Option<&str>) -> Result<&str, ApiError> {
        let (Some(token), Some(sender)) = (&self.token, &self.sender) else {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "腾讯邮件回调尚未配置".into(),
                None,
            ));
        };
        if !equal_secret(token, supplied.unwrap_or("")) {
            return Err(ApiError(
                StatusCode::UNAUTHORIZED,
                "邮件回调认证失败".into(),
                None,
            ));
        }
        Ok(sender)
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS mail_events(id INTEGER PRIMARY KEY,payload_hash TEXT NOT NULL UNIQUE,event TEXT NOT NULL,email TEXT NOT NULL,from_email TEXT NOT NULL,event_at INTEGER NOT NULL,received_at INTEGER NOT NULL,raw_json TEXT NOT NULL);
      CREATE INDEX IF NOT EXISTS idx_mail_events_recent ON mail_events(id DESC);
      CREATE INDEX IF NOT EXISTS idx_mail_events_filter ON mail_events(event,email COLLATE NOCASE,id DESC);")
}
#[derive(Deserialize, Default)]
struct CallbackQuery {
    token: Option<String>,
}
async fn verify(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> ApiResult<Value> {
    state.config.mail_events.authorize(query.token.as_deref())?;
    Ok(Json(json!({"ok":true})))
}
fn required_text<'a>(payload: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    payload[key]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid(&format!("邮件回调缺少有效 {key}")))
}
async fn receive(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
    payload: Result<Json<Value>, JsonRejection>,
) -> ApiResult<Value> {
    let sender = state.config.mail_events.authorize(query.token.as_deref())?;
    let Json(payload) = payload.map_err(|error| invalid(&error.body_text()))?;
    let event = required_text(&payload, "event")?;
    let email = required_text(&payload, "email")?;
    let from = required_text(&payload, "from")?;
    if !from.eq_ignore_ascii_case(sender) {
        return Err(invalid("邮件回调发信地址不匹配"));
    }
    let stamp = payload["timestamp"]
        .as_i64()
        .filter(|stamp| *stamp > 0 && chrono::DateTime::from_timestamp(*stamp, 0).is_some())
        .ok_or_else(|| invalid("邮件回调 timestamp 应为有效的正整数秒时间戳"))?;
    // serde_json 的对象映射按键排序；保留数组顺序与全部扩展字段。
    let raw = serde_json::to_string(&payload)?;
    let hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
    open(&state)?.execute(
        "INSERT INTO mail_events(payload_hash,event,email,from_email,event_at,received_at,raw_json) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(payload_hash) DO NOTHING",
        params![hash,event,email,from,stamp,now(),raw],
    )?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize, Default)]
struct EventsQuery {
    limit: Option<u32>,
    event: Option<String>,
    email: Option<String>,
}
async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<Value> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let db = open(&state)?;
    let mut statement = db.prepare("SELECT id,event,email,from_email,event_at,received_at,raw_json FROM mail_events WHERE (?1 IS NULL OR ?1='' OR event=?1) AND (?2 IS NULL OR ?2='' OR email=?2 COLLATE NOCASE) ORDER BY id DESC LIMIT ?3")?;
    let items = statement
        .query_map(params![query.event, query.email, limit], |row| {
            let raw: String = row.get(6)?;
            let payload: Value = serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(json!({"id":row.get::<_,i64>(0)?,"event":row.get::<_,String>(1)?,"email":row.get::<_,String>(2)?,"from":row.get::<_,String>(3)?,"timestamp":iso(row.get(4)?),"received_at":iso(row.get(5)?),"subject":payload["subject"],"reason":payload["reason"],"bulk_id":payload["bulkId"],"message_id":payload["messageId"],"bounce_type":payload["bounceType"],"raw_json":raw}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({"items":items,"limit":limit})))
}
async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    response
}
pub(super) fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/mail/tencent/callback", get(verify).post(receive))
        .route_layer(middleware::from_fn(no_store))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/mail/events", get(events))
        .route_layer(middleware::from_fn(no_store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::FromRequest;
    fn fixture() -> AppState {
        let (mut state, _, _, _) = rain::tests::fixture(false, false);
        state.config.mail_events = Config {
            token: Some("test-private-mail-callback-token".into()),
            sender: Some("admin@example.test".into()),
        };
        state
    }
    fn query() -> Query<CallbackQuery> {
        Query(CallbackQuery {
            token: Some("test-private-mail-callback-token".into()),
        })
    }
    fn payload(event: &str, stamp: i64) -> Value {
        json!({"event":event,"email":"recipient@example.test","from":"ADMIN@example.test","timestamp":stamp,"subject":"邮件主题","messageId":"message-1","X-Tencentcloudses-Cb-Custom":"custom-value","extra":{"z":2,"a":1}})
    }
    #[tokio::test]
    async fn authentication_and_validation_fail_without_writing_or_leaking_secrets() {
        let mut state = fixture();
        assert_eq!(
            verify(State(state.clone()), query()).await.unwrap().0,
            json!({"ok":true})
        );
        assert_eq!(
            verify(State(state.clone()), Query(CallbackQuery::default()))
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        let wrong = Query(CallbackQuery {
            token: Some("wrong".into()),
        });
        assert_eq!(
            receive(
                State(state.clone()),
                wrong,
                Ok(Json(payload("delivered", now())))
            )
            .await
            .unwrap_err()
            .0,
            StatusCode::UNAUTHORIZED
        );
        for invalid_payload in [
            json!([]),
            json!({"event":"open","email":"x","from":"other@example.test","timestamp":now()}),
            json!({"event":"open","email":"x","from":"admin@example.test","timestamp":0}),
            json!({"event":"open","email":"x","from":"admin@example.test","timestamp":i64::MAX}),
        ] {
            assert_eq!(
                receive(State(state.clone()), query(), Ok(Json(invalid_payload)))
                    .await
                    .unwrap_err()
                    .0,
                StatusCode::BAD_REQUEST
            );
        }
        let malformed = Json::<Value>::from_request(
            Request::builder()
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from("{invalid-json"))
                .unwrap(),
            &(),
        )
        .await;
        assert_eq!(
            receive(State(state.clone()), query(), malformed)
                .await
                .unwrap_err()
                .0,
            StatusCode::BAD_REQUEST
        );
        let db = open(&state).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM mail_events", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        state.config.mail_events = Config::default();
        let error = verify(State(state), query()).await.unwrap_err();
        assert_eq!(error.0, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!error.1.contains("test-private-mail-callback-token"));
    }
    #[tokio::test]
    async fn canonical_retries_deduplicate_but_separate_opens_and_future_events_remain() {
        let state = fixture();
        let first = payload("open", now());
        let original = first.to_string();
        let reordered: Value = serde_json::from_str(&format!("{{\"timestamp\":{},\"event\":\"open\",\"from\":\"ADMIN@example.test\",\"email\":\"recipient@example.test\",\"subject\":\"邮件主题\",\"messageId\":\"message-1\",\"X-Tencentcloudses-Cb-Custom\":\"custom-value\",\"extra\":{{\"a\":1,\"z\":2}}}}",first["timestamp"])).unwrap();
        for value in [
            first.clone(),
            reordered,
            payload("open", first["timestamp"].as_i64().unwrap() + 1),
            payload("future_event", now()),
        ] {
            assert_eq!(
                receive(State(state.clone()), query(), Ok(Json(value)))
                    .await
                    .unwrap()
                    .0,
                json!({"ok":true})
            );
        }
        let db = open(&state).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM mail_events", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            db.query_row(
                "SELECT raw_json FROM mail_events ORDER BY id LIMIT 1",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            original
        );
        let result = events(
            State(state.clone()),
            Query(EventsQuery {
                limit: Some(1000),
                event: Some("open".into()),
                email: Some("RECIPIENT@example.test".into()),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["limit"], 100);
        assert_eq!(result["items"].as_array().unwrap().len(), 2);
        assert!(
            result["items"][0]["raw_json"]
                .as_str()
                .unwrap()
                .contains("custom-value")
        );
    }
    #[tokio::test]
    async fn mail_notifications_never_change_money_and_never_record_credentials_in_audit() {
        let state = fixture();
        let db = open(&state).unwrap();
        let before: (i64,i64,i64) = db.query_row("SELECT (SELECT SUM(coins) FROM users),(SELECT COUNT(*) FROM ledger),(SELECT COUNT(*) FROM admin_audit)", [], |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        for event in [
            "deferred",
            "delivered",
            "dropped",
            "open",
            "click",
            "bounce",
            "spamreport",
            "unsubscribe",
        ] {
            let _ = receive(
                State(state.clone()),
                query(),
                Ok(Json(payload(event, now()))),
            )
            .await
            .unwrap();
        }
        let after = db.query_row("SELECT (SELECT SUM(coins) FROM users),(SELECT COUNT(*) FROM ledger),(SELECT COUNT(*) FROM admin_audit)", [], |row|Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?))).unwrap();
        assert_eq!(before, after);
        let raw: String = db
            .query_row(
                "SELECT GROUP_CONCAT(raw_json,'') FROM mail_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!raw.contains("test-private-mail-callback-token"));
        assert!(!raw.contains("ORANGE_MAIL_PASSWORD"));
    }
}
