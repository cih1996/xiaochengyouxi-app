use super::*;

#[derive(Clone, Serialize, Deserialize)]
struct RequestConfig {
    enabled: bool,
    packet_enabled: bool,
    product_enabled: bool,
    cooldown_seconds: i64,
    daily_limit: i64,
}
impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            packet_enabled: true,
            product_enabled: true,
            cooldown_seconds: 20,
            daily_limit: 20,
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS hall_request_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS hall_request_usage(user_id INTEGER PRIMARY KEY,day TEXT NOT NULL,count INTEGER NOT NULL,last_at_ms INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO hall_request_config(id,config_json) VALUES(1,?1)",
        [serde_json::to_string(&RequestConfig::default())?],
    )?;
    Ok(())
}
fn config(db: &Connection) -> Result<RequestConfig, ApiError> {
    let raw: String = db.query_row(
        "SELECT config_json FROM hall_request_config WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}
fn usage(db: &Connection, uid: i64, c: &RequestConfig) -> Result<(i64, i64), ApiError> {
    let row: Option<(String, i64, i64)> = db
        .query_row(
            "SELECT day,count,last_at_ms FROM hall_request_usage WHERE user_id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (used, remaining) = if let Some((date, count, last)) = row {
        (
            (if date == day() { count } else { 0 }),
            ((last + c.cooldown_seconds * 1000 - rain::now_ms() + 999) / 1000).max(0),
        )
    } else {
        (0, 0)
    };
    Ok((remaining, (c.daily_limit - used).max(0)))
}
async fn public_config(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let c = config(&db)?;
    let mut result = serde_json::to_value(&c)?;
    let (wait, left) = if headers.contains_key("authorization") {
        usage(&db, auth(&db, &headers)?, &c)?
    } else {
        (0, c.daily_limit)
    };
    result["remaining_seconds"] = json!(wait);
    result["remaining_today"] = json!(left);
    Ok(Json(result))
}
#[derive(Deserialize)]
struct RequestInput {
    kind: String,
}
async fn request(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<RequestInput>,
) -> Result<Response, ApiError> {
    if !["packet", "product"].contains(&input.kind.as_str()) {
        return Err(invalid("请选择红包或好物"));
    }
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled
        || (input.kind == "packet" && !c.packet_enabled)
        || (input.kind == "product" && !c.product_enabled)
    {
        return Err(conflict("这个入口暂时休息啦"));
    }
    let (wait, left) = usage(&tx, uid, &c)?;
    if wait > 0 {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"稍等片刻，再来看看","retry_after_seconds":wait})),
        )
            .into_response());
    }
    if left == 0 {
        return Err(conflict("今天的请求次数已用完，明天再来吧"));
    }
    let at = rain::now_ms();
    tx.execute("INSERT INTO hall_request_usage(user_id,day,count,last_at_ms) VALUES(?1,?2,1,?3) ON CONFLICT(user_id) DO UPDATE SET count=CASE WHEN hall_request_usage.day=excluded.day THEN hall_request_usage.count+1 ELSE 1 END,day=excluded.day,last_at_ms=excluded.last_at_ms",params![uid,day(),at])?;
    let id = Uuid::new_v4().to_string();
    let mut event = json!({"id":id,"created_at":iso(now())});
    if input.kind == "packet" {
        event["kind"] = json!("red_packet");
        event["text"] = json!("给你准备了一份小惊喜，点开看看吧。");
        event["task"] = serde_json::to_value(task_payload(
            &tx,
            uid,
            (Uuid::new_v4().as_u128() % TASKS.len() as u128) as usize,
        )?)?;
    } else {
        let product =
            product_admin::random_active(&tx)?.ok_or_else(|| conflict("暂时没有在售好物"))?;
        event["kind"] = json!("product");
        event["text"] = json!("这件好物，帮你省一点。");
        event["product"] = product;
    }
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "hall_request",
        "success",
        &id,
        None,
        json!({"requested_kind":input.kind}),
    )?;
    tx.commit()?;
    Ok(
        Json(json!({"event":event,"cooldown_seconds":c.cooldown_seconds,"remaining_today":left-1}))
            .into_response(),
    )
}
async fn admin_config(State(state): State<AppState>) -> ApiResult<RequestConfig> {
    Ok(Json(config(&open(&state)?)?))
}
async fn update_config(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<RequestConfig>,
) -> ApiResult<RequestConfig> {
    if !(5..=3600).contains(&input.cooldown_seconds) || !(1..=200).contains(&input.daily_limit) {
        return Err(invalid(
            "请求间隔应为 5 至 3600 秒，每日次数应为 1 至 200 次",
        ));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = config(&tx)?;
    tx.execute(
        "UPDATE hall_request_config SET config_json=?1 WHERE id=1",
        [serde_json::to_string(&input)?],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "hall_request_config",
        "hall-request",
        "修改主动请求设置",
        serde_json::to_value(before)?,
        serde_json::to_value(&input)?,
    )?;
    tx.commit()?;
    Ok(Json(input))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/hall/request-config", get(public_config))
        .route("/hall/request", post(request))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/hall-request-config", get(admin_config).put(update_config))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn empty_product_library_does_not_emit_fake_products_or_consume_request_budget() {
        let (state, context, headers, uid) = rain::tests::fixture(false, false);
        let result = request(
            State(state.clone()),
            Extension(context),
            headers,
            Json(RequestInput {
                kind: "product".into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(result.0, StatusCode::CONFLICT);
        assert_eq!(result.1, "暂时没有在售好物");
        assert_eq!(
            open(&state)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM hall_request_usage WHERE user_id=?1",
                    [uid],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn requests_obey_switch_cooldown_and_daily_budget_without_persisting_messages() {
        let (state, context, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let mut c = config(&db).unwrap();
        c.daily_limit = 1;
        c.cooldown_seconds = 5;
        db.execute(
            "UPDATE hall_request_config SET config_json=?1",
            [serde_json::to_string(&c).unwrap()],
        )
        .unwrap();
        let first = request(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RequestInput {
                kind: "packet".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(first.into_body(), 102400)
            .await
            .unwrap();
        let event: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(event["event"]["kind"], "red_packet");
        assert_eq!(event["remaining_today"], 0);
        let cooldown = request(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(RequestInput {
                kind: "product".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(cooldown.status(), StatusCode::TOO_MANY_REQUESTS);
        db.execute(
            "UPDATE hall_request_usage SET last_at_ms=?1 WHERE user_id=2",
            [rain::now_ms() - 6000],
        )
        .unwrap();
        assert!(
            request(
                State(state.clone()),
                Extension(context.clone()),
                headers.clone(),
                Json(RequestInput {
                    kind: "packet".into()
                })
            )
            .await
            .is_err()
        );
        c.enabled = false;
        db.execute(
            "UPDATE hall_request_config SET config_json=?1",
            [serde_json::to_string(&c).unwrap()],
        )
        .unwrap();
        assert!(
            request(
                State(state),
                Extension(context),
                headers,
                Json(RequestInput {
                    kind: "product".into()
                })
            )
            .await
            .is_err()
        );
        assert_eq!(
            db.query_row(
                "SELECT count FROM hall_request_usage WHERE user_id=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
