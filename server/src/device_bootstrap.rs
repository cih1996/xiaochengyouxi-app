use super::*;

#[derive(Deserialize)]
struct DeviceInput {
    #[serde(rename = "appId")]
    app_id: String,
    #[serde(rename = "installationId")]
    installation_id: String,
    #[serde(rename = "keyId", default)]
    key_id: String,
}

async fn configuration(State(state): State<AppState>) -> ApiResult<Value> {
    let Some(config) = &state.config.device_trust else {
        return Ok(Json(json!({"enabled":false})));
    };
    let mut pins = config.pins.clone();
    if let Some(extra) = env_value("ORANGE_APP_API_SPKI_PINS") {
        pins.extend(
            extra
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_owned),
        );
    }
    pins.sort();
    pins.dedup();
    Ok(Json(
        json!({"enabled":true,"app_id":config.app_id,"business_url":"https://orange-api.lingzidev.com/api","platform_url":config.base_url,"spki_pins":pins}),
    ))
}

async fn prepare(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<DeviceInput>,
) -> ApiResult<Value> {
    let config = state.config.device_trust.as_ref().ok_or_else(|| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "设备服务暂不可用".into(),
            None,
        )
    })?;
    if input.app_id != config.app_id {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "应用登记信息不匹配".into(),
            None,
        ));
    }
    if !(16..=128).contains(&input.installation_id.len()) || input.key_id.len() > 128 {
        return Err(invalid("设备登记参数错误"));
    }
    let reference = Uuid::new_v4().to_string();
    {
        let db = open(&state)?;
        let recent:i64=db.query_row("SELECT COUNT(*) FROM user_behavior WHERE kind='device_bootstrap' AND client_ip=?1 AND created_at>=?2",params![context.client_ip,now()-60],|r|r.get(0))?;
        if recent >= 30 {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "设备登记请求频繁，请稍后再试".into(),
                None,
            ));
        }
        user_admin::behavior(
            &db,
            &context,
            None,
            None,
            "device_bootstrap",
            "requested",
            &reference,
            None,
            json!({"installation_hash":format!("{:x}",Sha256::digest(input.installation_id.as_bytes()))}),
        )?;
    }
    let guest = format!(
        "guest:{:x}",
        Sha256::digest(input.installation_id.as_bytes())
    );
    let intent = business_guard::Intent {
        method: "POST".into(),
        path: "/api/device/prepare".into(),
        body: json!({"appId":input.app_id,"installationId":input.installation_id,"keyId":input.key_id}),
    };
    let request_hash = business_guard::request_hash(&guest, &intent)?;
    let kind = if input.key_id.is_empty() {
        "device_init"
    } else {
        "device_refresh"
    };
    let challenge=config.call("/v1/challenges",json!({"appId":config.app_id,"eventId":format!("startup-{reference}"),"accountId":guest,"type":kind,"context":{"requestHash":request_hash},"clientIp":context.client_ip,"keyId":input.key_id})).await;
    match challenge {
        Ok(challenge) => Ok(Json(
            json!({"appId":config.app_id,"platformUrl":config.base_url,"challenge":challenge}),
        )),
        Err(error) => {
            user_admin::behavior(
                &open(&state)?,
                &context,
                None,
                None,
                "device_bootstrap_error",
                "failed",
                &reference,
                None,
                json!({"upstream_error":error}),
            )?;
            Err(ApiError(
                StatusCode::BAD_GATEWAY,
                format!("设备服务暂不可用。参考号：{reference}"),
                None,
            ))
        }
    }
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/device/config", get(configuration))
        .route("/device/prepare", post(prepare))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn bootstrap_validates_application_and_keeps_authenticated_money_separate() {
        let (mut state, context, _, _) = rain::tests::fixture(true, false);
        let db = open(&state).unwrap();
        let before: i64 = db
            .query_row("SELECT SUM(coins) FROM users", [], |r| r.get(0))
            .unwrap();
        let wrong = prepare(
            State(state.clone()),
            Extension(context.clone()),
            Json(DeviceInput {
                app_id: "other".into(),
                installation_id: "observation-long-enough".into(),
                key_id: String::new(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(wrong.0, StatusCode::FORBIDDEN);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new().route(
            "/v1/challenges",
            post(|Json(body): Json<Value>| async move {
                assert_eq!(body["type"], "device_init");
                assert!(body["accountId"].as_str().unwrap().starts_with("guest:"));
                assert_eq!(body["context"]["requestHash"].as_str().unwrap().len(), 64);
                Json(json!({"id":"test-challenge","keyId":"","hardwareRequired":false}))
            }),
        );
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        state.config.device_trust.as_mut().unwrap().base_url = format!("http://{address}");
        let result = prepare(
            State(state),
            Extension(context),
            Json(DeviceInput {
                app_id: "test-app".into(),
                installation_id: "observation-long-enough".into(),
                key_id: String::new(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["appId"], "test-app");
        assert_eq!(result["challenge"]["id"], "test-challenge");
        assert!(!result.to_string().contains("test-private-key"));
        assert_eq!(
            db.query_row("SELECT SUM(coins) FROM users", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            before
        );
        server.abort();
    }
}
