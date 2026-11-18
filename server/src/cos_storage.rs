//! Tencent COS for release files: admin-configured bucket (private), server-signed URLs.
//!
//! Downloads keep their public paths (/download/<file>.apk). nginx asks
//! `/api/cos/download/<file>` first: when COS is enabled and the object exists we answer 302 to
//! a short-lived signed URL on the custom download domain (COS refuses APKs on its default
//! domain); otherwise 404 and nginx serves the local copy, so nothing breaks while unconfigured.
use super::*;
use ring::hmac;

const SIGN_SECONDS: i64 = 7200;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct CosConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub secret_id: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default = "guangzhou")]
    pub region: String,
    /// Object key prefix for release files, e.g. "releases/".
    #[serde(default = "releases")]
    pub prefix: String,
    /// Custom domain bound to the bucket (required for APK downloads).
    #[serde(default)]
    pub download_domain: String,
    #[serde(default = "https")]
    pub download_scheme: String,
}
fn guangzhou() -> String {
    "ap-guangzhou".into()
}
fn releases() -> String {
    "releases/".into()
}
fn https() -> String {
    "https".into()
}
impl Default for CosConfig {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("defaults")
    }
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS cos_config(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,secret_key TEXT NOT NULL DEFAULT '',revision INTEGER NOT NULL DEFAULT 1,updated_at INTEGER NOT NULL);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO cos_config(id,json,updated_at) VALUES(1,?1,?2)",
        params![serde_json::to_string(&CosConfig::default())?, now()],
    )?;
    Ok(())
}

fn load(db: &Connection) -> Result<(CosConfig, String), ApiError> {
    let (raw, key): (String, String) =
        db.query_row("SELECT json,secret_key FROM cos_config WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok((serde_json::from_str(&raw)?, key))
}

fn ready(c: &CosConfig, key: &str) -> bool {
    !c.secret_id.is_empty() && !key.is_empty() && !c.bucket.is_empty() && !c.region.is_empty()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hmac_sha1_hex(key: &[u8], data: &str) -> String {
    hex(hmac::sign(&hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key), data.as_bytes()).as_ref())
}
fn sha1_hex(data: &str) -> String {
    hex(ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, data.as_bytes()).as_ref())
}
/// RFC 3986 encoding with COS's reserved set (-_.~); '/' is encoded in signatures.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// COS v5 request signature as a query string (signs the method, path and Host header).
pub(super) fn sign_query(secret_id: &str, secret_key: &str, method: &str, path: &str, host: &str, start: i64, end: i64) -> String {
    let key_time = format!("{start};{end}");
    let http_string = format!("{}\n{}\n\nhost={}\n", method.to_ascii_lowercase(), path, encode(host));
    let string_to_sign = format!("sha1\n{key_time}\n{}\n", sha1_hex(&http_string));
    let sign_key = hmac_sha1_hex(secret_key.as_bytes(), &key_time);
    let signature = hmac_sha1_hex(sign_key.as_bytes(), &string_to_sign);
    format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={key_time}&q-key-time={key_time}&q-header-list=host&q-url-param-list=&q-signature={signature}",
        encode(secret_id)
    )
}

fn default_host(c: &CosConfig) -> String {
    format!("{}.cos.{}.myqcloud.com", c.bucket, c.region)
}
fn object_path(c: &CosConfig, file: &str) -> String {
    let prefix = c.prefix.trim_matches('/');
    if prefix.is_empty() { format!("/{file}") } else { format!("/{prefix}/{file}") }
}
fn signed_url(c: &CosConfig, key: &str, method: &str, host: &str, scheme: &str, seconds: i64, path: &str) -> String {
    let start = now() - 60;
    let q = sign_query(&c.secret_id, key, method, path, host, start, start + 60 + seconds);
    format!("{scheme}://{host}{path}?{q}")
}

fn file_ok(file: &str) -> bool {
    file.len() <= 120
        && file.ends_with(".apk")
        && file.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        && !file.starts_with('.')
}

async fn head(c: &CosConfig, key: &str, path: &str) -> Result<Option<(u64, String)>, String> {
    let host = default_host(c);
    let url = signed_url(c, key, "HEAD", &host, "https", 300, path);
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?
        .head(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match resp.status().as_u16() {
        200 => {
            let size = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
            let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).unwrap_or("").trim_matches('"').to_string();
            Ok(Some((size, etag)))
        }
        404 => Ok(None),
        code => Err(format!("HTTP {code}")),
    }
}

/// nginx sub-request: 302 to COS when possible, 404 to fall back to the local file.
async fn download(State(state): State<AppState>, Path(file): Path<String>) -> Response {
    let not_here = || (StatusCode::NOT_FOUND, "").into_response();
    if !file_ok(&file) {
        return not_here();
    }
    let Ok(db) = open(&state) else { return not_here() };
    let Ok((c, key)) = load(&db) else { return not_here() };
    // The general entry always means "the current release": follow the published apk_url's
    // file name so it can never lag behind when only the versioned file was uploaded.
    let file = if file == "orange-rewards.apk" {
        current_release_file(&db).unwrap_or(file)
    } else {
        file
    };
    drop(db);
    if !c.enabled || !ready(&c, &key) || c.download_domain.is_empty() {
        return not_here();
    }
    let path = object_path(&c, &file);
    match head(&c, &key, &path).await {
        Ok(Some(_)) => {
            let url = signed_url(&c, &key, "GET", &c.download_domain, &c.download_scheme, SIGN_SECONDS, &path);
            (StatusCode::FOUND, [("Location", url), ("Cache-Control", "no-store".to_string())]).into_response()
        }
        Ok(None) => not_here(),
        Err(error) => {
            eprintln!("COS 检查 {file} 失败，改用本地文件：{error}");
            not_here()
        }
    }
}

fn current_release_file(db: &Connection) -> Option<String> {
    let raw: String = db
        .query_row("SELECT config_json FROM app_release WHERE id=1", [], |r| r.get(0))
        .ok()?;
    let url = serde_json::from_str::<Value>(&raw).ok()?["apk_url"].as_str()?.to_string();
    let name = url.rsplit('/').next()?.split('?').next()?.to_string();
    file_ok(&name).then_some(name)
}

// ---------- admin ----------

fn admin_json(db: &Connection) -> Result<Value, ApiError> {
    let (c, key) = load(db)?;
    let (revision, updated): (i64, i64) =
        db.query_row("SELECT revision,updated_at FROM cos_config WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let mut v = serde_json::to_value(&c)?;
    v["secret_key_set"] = json!(!key.is_empty());
    v["ready"] = json!(ready(&c, &key));
    v["default_host"] = json!(if c.bucket.is_empty() { String::new() } else { default_host(&c) });
    v["revision"] = json!(revision);
    v["updated_at"] = json!(iso(updated));
    Ok(v)
}

#[derive(Deserialize)]
struct AdminInput {
    #[serde(flatten)]
    config: CosConfig,
    #[serde(default)]
    secret_key: String,
}

fn validate(input: &mut AdminInput) -> Result<(), ApiError> {
    let c = &mut input.config;
    for f in [&mut c.secret_id, &mut c.bucket, &mut c.region, &mut c.prefix, &mut c.download_domain] {
        *f = f.trim().to_owned();
    }
    input.secret_key = input.secret_key.trim().to_owned();
    c.download_domain = c.download_domain.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_owned();
    let simple = |s: &str| s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' || ch == '_');
    if !simple(&c.bucket) || !simple(&c.region) || !simple(&c.download_domain) || c.secret_id.len() > 64 || input.secret_key.len() > 64 {
        return Err(invalid("COS 配置格式不正确"));
    }
    if !c.prefix.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_./".contains(ch)) || c.prefix.contains("..") {
        return Err(invalid("目录只能包含字母、数字、-、_ 和 /"));
    }
    if !["https", "http"].contains(&c.download_scheme.as_str()) {
        return Err(invalid("下载协议应为 https 或 http"));
    }
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
    let (_, stored) = load(&tx)?;
    let key = if input.secret_key.is_empty() { stored } else { input.secret_key.clone() };
    if input.config.enabled && (!ready(&input.config, &key) || input.config.download_domain.is_empty()) {
        return Err(invalid("启用前请填写 SecretId、SecretKey、存储桶、地域和下载域名"));
    }
    let before = admin_json(&tx)?;
    tx.execute(
        "UPDATE cos_config SET json=?1,secret_key=?2,revision=revision+1,updated_at=?3 WHERE id=1",
        params![serde_json::to_string(&input.config)?, key, now()],
    )?;
    let mut after = admin_json(&tx)?;
    after["secret_key_changed"] = json!(!input.secret_key.is_empty());
    user_admin::action_audit(&tx, &context, None, "cos_config_update", "cos-config", "修改 COS 配置", before, after.clone())?;
    tx.commit()?;
    Ok(Json(after))
}

/// Checks credentials and the download domain for one file (default: the general download entry).
#[derive(Deserialize)]
struct TestInput {
    #[serde(default)]
    file: String,
}
async fn admin_test(State(state): State<AppState>, Json(input): Json<TestInput>) -> ApiResult<Value> {
    let (c, key) = load(&open(&state)?)?;
    if !ready(&c, &key) {
        return Err(invalid("请先填写并保存 COS 配置"));
    }
    let file = if input.file.is_empty() { "orange-rewards.apk".to_string() } else { input.file };
    if !file_ok(&file) {
        return Err(invalid("文件名应为 .apk"));
    }
    let path = object_path(&c, &file);
    let object = match head(&c, &key, &path).await {
        Ok(Some((size, etag))) => json!({"ok":true,"exists":true,"size":size,"etag":etag}),
        Ok(None) => json!({"ok":true,"exists":false}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    // Try the download domain the way a user would (range request for the first bytes).
    let domain = if c.download_domain.is_empty() {
        json!({"ok":false,"error":"未填写下载域名"})
    } else {
        let url = signed_url(&c, &key, "GET", &c.download_domain, &c.download_scheme, 300, &path);
        match reqwest::Client::builder().timeout(std::time::Duration::from_secs(6)).build() {
            Err(e) => json!({"ok":false,"error":e.to_string()}),
            Ok(client) => match client.get(url).header("Range", "bytes=0-1").send().await {
                Err(e) => json!({"ok":false,"error":e.to_string()}),
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();
                    let code = body.split("<Code>").nth(1).and_then(|s| s.split("</Code>").next()).unwrap_or("").to_string();
                    json!({"ok":status == 200 || status == 206,"status":status,"code":code})
                }
            },
        }
    };
    Ok(Json(json!({"file":file,"object":object,"download":domain})))
}

/// Pre-signed PUT for the admin's browser upload (default domain; uploads are allowed there).
#[derive(Deserialize)]
struct UploadInput {
    file: String,
}
async fn admin_upload_url(State(state): State<AppState>, Json(input): Json<UploadInput>) -> ApiResult<Value> {
    let (c, key) = load(&open(&state)?)?;
    if !ready(&c, &key) {
        return Err(invalid("请先填写并保存 COS 配置"));
    }
    if !file_ok(&input.file) {
        return Err(invalid("文件名只能包含字母、数字、-、_、.，且以 .apk 结尾"));
    }
    let path = object_path(&c, &input.file);
    let url = signed_url(&c, &key, "PUT", &default_host(&c), "https", 1800, &path);
    Ok(Json(json!({"url":url,"key":path.trim_start_matches('/')})))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/cos/download/{file}", get(download))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/cos-config", get(admin_get).put(admin_put))
        .route("/cos-config/test", post(admin_test))
        .route("/cos/upload-url", post(admin_upload_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_reference_implementation() {
        // Expected value from an independent Python (hashlib/hmac) implementation of the COS v5
        // algorithm used by the official SDK (method, path, host header, fixed key time).
        let q = sign_query("AKIDEXAMPLE", "SECRETEXAMPLE", "GET", "/releases/a.apk", "dl.example.com", 1700000000, 1700007200);
        assert_eq!(q, include_str!("../tests/fixtures/cos-expected.txt").trim());
    }

    #[test]
    fn only_plain_apk_names_are_served() {
        assert!(file_ok("xiaochengyouxi-0.3.22-25.apk"));
        assert!(file_ok("orange-rewards.apk"));
        assert!(!file_ok("../x.apk"));
        assert!(!file_ok("a/b.apk"));
        assert!(!file_ok("x.zip"));
        let mut c = CosConfig::default();
        assert_eq!(object_path(&c, "a.apk"), "/releases/a.apk");
        c.prefix = "".into();
        assert_eq!(object_path(&c, "a.apk"), "/a.apk");
    }
}
