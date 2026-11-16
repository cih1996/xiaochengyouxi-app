use super::*;

#[derive(Clone, Serialize, Deserialize)]
struct Release {
    enabled: bool,
    latest_version_code: i64,
    latest_version_name: String,
    min_supported_version_code: i64,
    title: String,
    release_notes: String,
    apk_url: String,
    apk_sha256: String,
    apk_size_bytes: i64,
}
impl Default for Release {
    fn default() -> Self {
        Self {
            enabled: false,
            latest_version_code: 0,
            latest_version_name: String::new(),
            min_supported_version_code: 0,
            title: "发现新版本".into(),
            release_notes: String::new(),
            apk_url: String::new(),
            apk_sha256: String::new(),
            apk_size_bytes: 0,
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS app_release(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,revision INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS app_runtime(id INTEGER PRIMARY KEY CHECK(id=1),hall_notice TEXT NOT NULL,revision INTEGER NOT NULL,updated_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO app_release(id,config_json,revision,updated_at) VALUES(1,?1,1,?2)",
        params![serde_json::to_string(&Release::default())?, now()],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO app_runtime(id,hall_notice,revision,updated_at) VALUES(1,'',1,?1)",
        [now()],
    )?;
    Ok(())
}
fn release(db: &Connection) -> Result<(Release, i64), ApiError> {
    let (raw, revision): (String, i64) = db.query_row(
        "SELECT config_json,revision FROM app_release WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((serde_json::from_str(&raw)?, revision))
}
fn release_json(value: &Release, revision: i64) -> Result<Value, ApiError> {
    let mut response = serde_json::to_value(value)?;
    response["revision"] = json!(revision);
    Ok(response)
}
fn normalize_release(value: &mut Release) -> Result<(), ApiError> {
    value.latest_version_name = value.latest_version_name.trim().into();
    value.apk_url = value.apk_url.trim().into();
    value.apk_sha256 = value.apk_sha256.trim().to_ascii_lowercase();
    if value.latest_version_code < 0
        || value.min_supported_version_code < 0
        || value.min_supported_version_code > value.latest_version_code
        || value.apk_size_bytes < 0
    {
        return Err(invalid("请检查版本号、最低支持版本和安装包大小"));
    }
    if value.enabled {
        if value.latest_version_code == 0 || value.latest_version_name.is_empty() {
            return Err(invalid("启用发布需要正版本号和版本名称"));
        }
        if !reqwest::Url::parse(&value.apk_url)
            .ok()
            .is_some_and(|url| url.scheme() == "https" && url.host_str().is_some())
        {
            return Err(invalid("安装包下载地址需为有效 HTTPS 地址"));
        }
        if value.apk_sha256.len() != 64 || !value.apk_sha256.bytes().all(|v| v.is_ascii_hexdigit())
        {
            return Err(invalid("请填写安装包的 64 位 SHA256"));
        }
        if value.apk_size_bytes == 0 {
            return Err(invalid("启用发布需要安装包的实际大小"));
        }
    }
    Ok(())
}
#[derive(Deserialize)]
struct UpdateQuery {
    version_code: i64,
}
async fn check_update(
    State(state): State<AppState>,
    Query(query): Query<UpdateQuery>,
) -> ApiResult<Value> {
    if query.version_code < 0 {
        return Err(invalid("客户端版本号不能为负数"));
    }
    let (config, revision) = release(&open(&state)?)?;
    let available = config.enabled && query.version_code < config.latest_version_code;
    let force = available && query.version_code < config.min_supported_version_code;
    let mut response = release_json(&config, revision)?;
    response["update_available"] = json!(available);
    response["force_update"] = json!(force);
    Ok(Json(response))
}
async fn admin_release(State(state): State<AppState>) -> ApiResult<Value> {
    let (config, revision) = release(&open(&state)?)?;
    Ok(Json(release_json(&config, revision)?))
}
async fn save_release(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<Release>,
) -> ApiResult<Value> {
    normalize_release(&mut input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let (old, revision) = release(&tx)?;
    let after = release_json(&input, revision + 1)?;
    tx.execute(
        "UPDATE app_release SET config_json=?1,revision=?2,updated_at=?3 WHERE id=1",
        params![serde_json::to_string(&input)?, revision + 1, now()],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "app_release_update",
        "app-release",
        "修改应用发布配置",
        release_json(&old, revision)?,
        after.clone(),
    )?;
    tx.commit()?;
    Ok(Json(after))
}
fn runtime_json(db: &Connection) -> Result<Value, ApiError> {
    Ok(db.query_row("SELECT revision,hall_notice,updated_at FROM app_runtime WHERE id=1",[],|r|Ok(json!({"revision":r.get::<_,i64>(0)?,"hall_notice":r.get::<_,String>(1)?,"updated_at":iso(r.get(2)?)})))?)
}
async fn runtime(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(runtime_json(&open(&state)?)?))
}
#[derive(Deserialize)]
struct RuntimeInput {
    hall_notice: String,
}
async fn save_runtime(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<RuntimeInput>,
) -> ApiResult<Value> {
    if input.hall_notice.chars().count() > 200 {
        return Err(invalid("大厅公告最多 200 字"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = runtime_json(&tx)?;
    tx.execute(
        "UPDATE app_runtime SET hall_notice=?1,revision=revision+1,updated_at=?2 WHERE id=1",
        params![input.hall_notice, now()],
    )?;
    let after = runtime_json(&tx)?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "app_runtime_update",
        "app-runtime",
        "修改大厅公告",
        before,
        after.clone(),
    )?;
    tx.commit()?;
    Ok(Json(after))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/update", get(check_update))
        .route("/app/runtime", get(runtime))
        .route_layer(middleware::from_fn(no_store))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/app-release", get(admin_release).put(save_release))
        .route("/app-runtime", get(runtime).put(save_runtime))
        .route_layer(middleware::from_fn(no_store))
}
async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Release {
        Release {
            enabled: true,
            latest_version_code: 9,
            latest_version_name: "0.3.6".into(),
            min_supported_version_code: 7,
            title: "新版本来了".into(),
            release_notes: "体验更新".into(),
            apk_url: "https://orange.example/download/app.apk".into(),
            apk_sha256: "a".repeat(64),
            apk_size_bytes: 123456,
        }
    }
    #[tokio::test]
    async fn numeric_version_comparison_force_boundary_and_disabled_release() {
        let (state, mut context, _, _) = rain::tests::fixture(false, false);
        context.admin_username = Some("版本运营".into());
        let _ = save_release(
            State(state.clone()),
            Extension(context.clone()),
            Json(valid()),
        )
        .await
        .unwrap();
        for (current, available, force) in [
            (0, true, true),
            (6, true, true),
            (7, true, false),
            (8, true, false),
            (9, false, false),
            (10, false, false),
        ] {
            let result = check_update(
                State(state.clone()),
                Query(UpdateQuery {
                    version_code: current,
                }),
            )
            .await
            .unwrap()
            .0;
            assert_eq!(result["update_available"], available);
            assert_eq!(result["force_update"], force);
        }
        let mut config = valid();
        config.enabled = false;
        let _ = save_release(
            State(state.clone()),
            Extension(context.clone()),
            Json(config),
        )
        .await
        .unwrap();
        let result = check_update(State(state.clone()), Query(UpdateQuery { version_code: 1 }))
            .await
            .unwrap()
            .0;
        assert_eq!(result["update_available"], false);
        assert_eq!(result["force_update"], false);
        let mut config = valid();
        config.min_supported_version_code = 0;
        let _ = save_release(State(state.clone()), Extension(context), Json(config))
            .await
            .unwrap();
        assert_eq!(
            check_update(State(state), Query(UpdateQuery { version_code: 0 }))
                .await
                .unwrap()
                .0["force_update"],
            false
        );
    }
    #[test]
    fn enabled_release_requires_complete_apk_metadata_and_valid_version_bounds() {
        assert!(normalize_release(&mut Release::default()).is_ok());
        assert!(normalize_release(&mut valid()).is_ok());
        for mutate in 0..8 {
            let mut config = valid();
            match mutate {
                0 => config.latest_version_code = 0,
                1 => config.latest_version_name = " ".into(),
                2 => config.min_supported_version_code = 10,
                3 => config.apk_url = "http://example.com/app.apk".into(),
                4 => config.apk_url = "https://".into(),
                5 => config.apk_sha256 = "z".repeat(64),
                6 => config.apk_sha256 = "a".repeat(63),
                _ => config.apk_size_bytes = 0,
            }
            assert!(normalize_release(&mut config).is_err(), "case {mutate}");
        }
        let mut config = valid();
        config.apk_sha256 = "ABCDEF12".repeat(8);
        normalize_release(&mut config).unwrap();
        assert_eq!(config.apk_sha256, "abcdef12".repeat(8));
    }
    #[tokio::test]
    async fn release_and_runtime_survive_initialization_and_audit_actual_actor() {
        let (state, mut context, _, _) = rain::tests::fixture(false, false);
        context.admin_username = Some("发布管理员".into());
        let db = open(&state).unwrap();
        let draft = admin_release(State(state.clone())).await.unwrap().0;
        assert_eq!(draft["enabled"], false);
        let release = save_release(
            State(state.clone()),
            Extension(context.clone()),
            Json(valid()),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(release["revision"], draft["revision"].as_i64().unwrap() + 1);
        let announcement = "喜".repeat(200);
        let runtime = save_runtime(
            State(state.clone()),
            Extension(context.clone()),
            Json(RuntimeInput {
                hall_notice: announcement.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(runtime["hall_notice"], announcement);
        assert!(
            chrono::DateTime::parse_from_rfc3339(runtime["updated_at"].as_str().unwrap()).is_ok()
        );
        initialize(&db).unwrap();
        assert_eq!(
            admin_release(State(state.clone())).await.unwrap().0,
            release
        );
        assert_eq!(
            super::runtime(State(state.clone())).await.unwrap().0,
            runtime
        );
        assert!(
            save_runtime(
                State(state.clone()),
                Extension(context.clone()),
                Json(RuntimeInput {
                    hall_notice: "喜".repeat(201)
                })
            )
            .await
            .is_err()
        );
        assert_eq!(
            super::runtime(State(state.clone())).await.unwrap().0,
            runtime
        );
        let cleared = save_runtime(
            State(state.clone()),
            Extension(context),
            Json(RuntimeInput {
                hall_notice: String::new(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(cleared["hall_notice"], "");
        assert_eq!(
            cleared["revision"],
            runtime["revision"].as_i64().unwrap() + 1
        );
        let actors=db.prepare("SELECT actor FROM admin_audit WHERE action IN ('app_release_update','app_runtime_update')").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(actors.len(), 3);
        assert!(actors.iter().all(|v| v == "发布管理员"));
    }
}
