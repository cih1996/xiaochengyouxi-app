use super::*;
use axum::extract::{DefaultBodyLimit, FromRequest, Multipart, rejection::JsonRejection};
use base64::{Engine, engine::general_purpose::STANDARD};

const MAX_IMAGE: usize = 5 * 1024 * 1024;
pub(super) fn initialize(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS product_config(id INTEGER PRIMARY KEY CHECK(id=1),platform_share_bps INTEGER NOT NULL,default_valid_days INTEGER NOT NULL,revision INTEGER NOT NULL);
      INSERT OR IGNORE INTO product_config VALUES(1,2000,7,1);
      CREATE TABLE IF NOT EXISTS product_api_key(id INTEGER PRIMARY KEY CHECK(id=1),key TEXT NOT NULL,created_at INTEGER NOT NULL,rotated_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS product_images(id TEXT PRIMARY KEY,image TEXT NOT NULL UNIQUE,content_type TEXT NOT NULL,size_bytes INTEGER NOT NULL,created_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS product_display_config(id INTEGER PRIMARY KEY CHECK(id=1),hero_image TEXT,revision INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      INSERT OR IGNORE INTO product_display_config(id,hero_image,revision,updated_at) VALUES(1,NULL,1,0);
      CREATE TABLE IF NOT EXISTS products(id INTEGER PRIMARY KEY,title TEXT NOT NULL,url TEXT NOT NULL,price_fen INTEGER NOT NULL,commission_bps INTEGER NOT NULL,image TEXT NOT NULL,platform TEXT NOT NULL,category TEXT NOT NULL,external_id TEXT UNIQUE,starts_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,enabled INTEGER NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS product_clicks(id TEXT PRIMARY KEY,product_id INTEGER NOT NULL,user_id INTEGER,kind TEXT NOT NULL,session_hash TEXT NOT NULL,created_at INTEGER NOT NULL,request_id TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS product_impressions(id TEXT PRIMARY KEY,product_id INTEGER NOT NULL,user_id INTEGER,session_hash TEXT NOT NULL,created_at INTEGER NOT NULL,request_id TEXT NOT NULL);
      CREATE INDEX IF NOT EXISTS idx_products_active ON products(enabled,starts_at,expires_at);")
}
#[derive(Serialize, Deserialize)]
struct ProductConfig {
    platform_share_percent: f64,
    default_valid_days: i64,
    #[serde(default)]
    revision: i64,
}
fn config(db: &Connection) -> Result<ProductConfig, ApiError> {
    Ok(db.query_row(
        "SELECT platform_share_bps,default_valid_days,revision FROM product_config WHERE id=1",
        [],
        |r| {
            Ok(ProductConfig {
                platform_share_percent: r.get::<_, i64>(0)? as f64 / 100.0,
                default_valid_days: r.get(1)?,
                revision: r.get(2)?,
            })
        },
    )?)
}
pub(super) fn reward_coins(price_fen: i64, commission_bps: i64, platform_share_bps: i64) -> i64 {
    (i128::from(price_fen)
        * i128::from(commission_bps)
        * i128::from(10000 - platform_share_bps)
        * i128::from(COINS_PER_YUAN)
        / (100_i128 * 10000 * 10000)) as i64
}
fn decimal(raw: &str, name: &str) -> Result<i64, ApiError> {
    let value = raw.trim();
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts[0].is_empty()
        || !parts[0].bytes().all(|c| c.is_ascii_digit())
        || parts
            .get(1)
            .is_some_and(|v| v.is_empty() || v.len() > 2 || !v.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err(invalid(&format!("{name}请填写最多两位小数的非负数")));
    }
    let whole = parts[0]
        .parse::<i64>()
        .map_err(|_| invalid(&format!("{name}数值过大")))?;
    let fraction = parts
        .get(1)
        .map(|v| v.parse::<i64>().unwrap() * if v.len() == 1 { 10 } else { 1 })
        .unwrap_or(0);
    whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(fraction))
        .ok_or_else(|| invalid(&format!("{name}数值过大")))
}
fn media_dir(state: &AppState) -> PathBuf {
    state
        .db_path
        .parent()
        .expect("SQLite parent directory")
        .join("product-media")
}
fn image_type(bytes: &[u8]) -> Result<(&'static str, &'static str), ApiError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(("png", "image/png"))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok(("jpg", "image/jpeg"))
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok(("webp", "image/webp"))
    } else {
        Err(invalid("商品图片仅支持 PNG、JPEG 或 WebP"))
    }
}
#[derive(Serialize)]
struct StoredImage {
    image_id: String,
    image: String,
    content_type: String,
    size_bytes: i64,
}
fn store_image(
    state: &AppState,
    db: &Connection,
    context: &RequestContext,
    bytes: &[u8],
    integration: bool,
) -> Result<StoredImage, ApiError> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE {
        return Err(invalid("商品图片应大于0且不超过5MiB"));
    }
    let (extension, content_type) = image_type(bytes)?;
    let id = Uuid::new_v4().to_string();
    let filename = format!("{id}.{extension}");
    let dir = media_dir(state);
    fs::create_dir_all(&dir)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    fs::write(dir.join(&filename), bytes)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    let result = StoredImage {
        image_id: id,
        image: format!("/media/products/{filename}"),
        content_type: content_type.into(),
        size_bytes: bytes.len() as i64,
    };
    db.execute("INSERT INTO product_images(id,image,content_type,size_bytes,created_at) VALUES(?1,?2,?3,?4,?5)",params![result.image_id,result.image,result.content_type,result.size_bytes,now()])?;
    let mut actor = context.clone();
    if integration {
        actor.admin_username = Some("product-api".into());
    }
    user_admin::action_audit(
        db,
        &actor,
        None,
        "product_image_upload",
        &format!("product-image:{}", result.image_id),
        "上传商品图片",
        json!({}),
        serde_json::to_value(&result)?,
    )?;
    Ok(result)
}
fn registered_image(state: &AppState, db: &Connection, raw: &str) -> Result<String, ApiError> {
    let id = Uuid::parse_str(raw)
        .map_err(|_| invalid("image_id必须是本服务上传返回的UUID"))?
        .to_string();
    let image: Option<String> = db
        .query_row("SELECT image FROM product_images WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let image = image.ok_or_else(|| invalid("image_id不存在，请先上传图片"))?;
    let filename = image
        .strip_prefix("/media/products/")
        .ok_or_else(|| invalid("图片登记信息错误"))?;
    let metadata = fs::metadata(media_dir(state).join(filename)).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            invalid("图片文件不存在，请重新上传")
        } else {
            ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None)
        }
    })?;
    if !metadata.is_file() {
        return Err(invalid("图片文件不存在，请重新上传"));
    }
    Ok(image)
}
fn decode_image(raw: &str) -> Result<Vec<u8>, ApiError> {
    let encoded = if raw.starts_with("data:") {
        let (header, content) = raw
            .split_once(',')
            .ok_or_else(|| invalid("图片data URL格式错误"))?;
        let header = header.to_ascii_lowercase();
        if !header.starts_with("data:image/") || !header.ends_with(";base64") {
            return Err(invalid("图片data URL应使用data:image/...;base64格式"));
        }
        content
    } else {
        raw
    };
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| invalid(&format!("图片Base64无效：{e}")))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE {
        return Err(invalid("商品图片应大于0且不超过5MiB"));
    }
    image_type(&bytes)?;
    Ok(bytes)
}
async fn media(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Response, ApiError> {
    let (stem, ext) = filename
        .rsplit_once('.')
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "图片不存在".into(), None))?;
    if Uuid::parse_str(stem).is_err() || !["png", "jpg", "webp"].contains(&ext) {
        return Err(ApiError(StatusCode::NOT_FOUND, "图片不存在".into(), None));
    }
    let bytes = fs::read(media_dir(&state).join(&filename)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError(StatusCode::NOT_FOUND, "图片不存在".into(), None)
        } else {
            ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None)
        }
    })?;
    let (_, content_type) = image_type(&bytes)?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                axum::http::header::CACHE_CONTROL,
                "public, max-age=31536000, immutable",
            ),
        ],
        bytes,
    )
        .into_response())
}
fn product_row(row: &rusqlite::Row<'_>, share: i64) -> Result<Value, rusqlite::Error> {
    let price: i64 = row.get(3)?;
    let commission: i64 = row.get(4)?;
    let starts: i64 = row.get(9)?;
    let expires: i64 = row.get(10)?;
    let enabled: bool = row.get(11)?;
    let reward = reward_coins(price, commission, share);
    let at = now();
    Ok(
        json!({"id":row.get::<_,i64>(0)?,"title":row.get::<_,String>(1)?,"url":row.get::<_,String>(2)?,"price":price as f64/100.0,"price_text":format!("{}.{:02}",price/100,price%100),"price_fen":price,"commission_percent":commission as f64/100.0,"commission_bps":commission,"reward_coins":reward,"commission":reward as f64/COINS_PER_YUAN as f64,"image":row.get::<_,String>(5)?,"platform":row.get::<_,String>(6)?,"category":row.get::<_,String>(7)?,"external_id":row.get::<_,Option<String>>(8)?,"starts_at":iso(starts),"expires_at":iso(expires),"server_now":iso(at),"seconds_until_start":(starts-at).max(0),"seconds_until_expiry":(expires-at).max(0),"enabled":enabled,"revision":row.get::<_,i64>(12)?,"status":if !enabled {"disabled"}else if starts>at{"scheduled"}else if expires<=at{"expired"}else{"active"}}),
    )
}
fn share(db: &Connection) -> Result<i64, ApiError> {
    Ok(db.query_row(
        "SELECT platform_share_bps FROM product_config WHERE id=1",
        [],
        |r| r.get(0),
    )?)
}
fn one(db: &Connection, id: i64, active: bool) -> Result<Value, ApiError> {
    let share = share(db)?;
    db.query_row("SELECT id,title,url,price_fen,commission_bps,image,platform,category,external_id,starts_at,expires_at,enabled,revision FROM products WHERE id=?1 AND (?2=0 OR (enabled=1 AND starts_at<=?3 AND expires_at>?3))",params![id,active,now()],|r|product_row(r,share)).optional()?.ok_or_else(||ApiError(StatusCode::NOT_FOUND,"商品已下架或已过期".into(),None))
}
pub(super) fn active(db: &Connection) -> Result<Vec<Value>, ApiError> {
    let share = share(db)?;
    Ok(db.prepare("SELECT id,title,url,price_fen,commission_bps,image,platform,category,external_id,starts_at,expires_at,enabled,revision FROM products WHERE enabled=1 AND starts_at<=?1 AND expires_at>?1 ORDER BY id DESC")?.query_map([now()],|r|product_row(r,share))?.collect::<Result<Vec<_>,_>>()?)
}
pub(super) fn random_active(db: &Connection) -> Result<Option<Value>, ApiError> {
    let share = share(db)?;
    Ok(db.query_row("SELECT id,title,url,price_fen,commission_bps,image,platform,category,external_id,starts_at,expires_at,enabled,revision FROM products WHERE enabled=1 AND starts_at<=?1 AND expires_at>?1 ORDER BY RANDOM() LIMIT 1",[now()],|r|product_row(r,share)).optional()?)
}
async fn public_one(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    if headers.contains_key("authorization") {
        auth(&db, &headers)?;
    }
    Ok(Json(one(&db, id, true)?))
}
fn display_config(db: &Connection) -> Result<Value, ApiError> {
    let (image, revision): (Option<String>, i64) = db.query_row(
        "SELECT hero_image,revision FROM product_display_config WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(json!({"hero_image":image,"revision":revision}))
}
async fn public_runtime(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(display_config(&open(&state)?)?))
}
async fn admin_display(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(display_config(&open(&state)?)?))
}
#[derive(Deserialize)]
struct DisplayInput {
    image_id: Option<String>,
}
async fn update_display(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<DisplayInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = display_config(&tx)?;
    let image = input
        .image_id
        .as_deref()
        .map(|id| registered_image(&state, &tx, id))
        .transpose()?;
    tx.execute("UPDATE product_display_config SET hero_image=?1,revision=revision+1,updated_at=?2 WHERE id=1",params![image,now()])?;
    let after = display_config(&tx)?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "product_display_config",
        "product-display",
        "修改捡漏展示图",
        before,
        after.clone(),
    )?;
    tx.commit()?;
    Ok(Json(after))
}
fn optional_user(db: &Connection, headers: &HeaderMap) -> Result<Option<i64>, ApiError> {
    if headers.contains_key("authorization") {
        Ok(Some(account_auth(db, headers)?))
    } else {
        Ok(None)
    }
}
fn session_hash(context: &RequestContext, headers: &HeaderMap) -> String {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    format!(
        "{:x}",
        Sha256::digest(format!("{}:{}", context.client_ip, ua).as_bytes())
    )
}
#[derive(Deserialize)]
struct ClickInput {
    kind: String,
}
async fn click(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ClickInput>,
) -> ApiResult<Value> {
    if !["detail", "outbound"].contains(&input.kind.as_str()) {
        return Err(invalid("点击类型应为detail或outbound"));
    }
    let db = open(&state)?;
    one(&db, id, true)?;
    let uid = optional_user(&db, &headers)?;
    let request_id = context.request_id.clone();
    db.execute("INSERT INTO product_clicks(id,product_id,user_id,kind,session_hash,created_at,request_id) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![Uuid::new_v4().to_string(),id,uid,input.kind,session_hash(&context,&headers),now(),request_id])?;
    Ok(Json(json!({"ok":true,"product_id":id,"kind":input.kind})))
}
async fn impression(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(_body): Json<Value>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    one(&db, id, true)?;
    let uid = optional_user(&db, &headers)?;
    db.execute("INSERT INTO product_impressions(id,product_id,user_id,session_hash,created_at,request_id) VALUES(?1,?2,?3,?4,?5,?6)",params![Uuid::new_v4().to_string(),id,uid,session_hash(&context,&headers),now(),context.request_id])?;
    Ok(Json(json!({"ok":true,"product_id":id})))
}
#[derive(Deserialize, Default)]
struct AnalyticsQuery {
    days: Option<i64>,
    product_id: Option<i64>,
}
async fn analytics(
    State(state): State<AppState>,
    Query(q): Query<AnalyticsQuery>,
) -> ApiResult<Value> {
    let days = q.days.unwrap_or(7);
    if ![7, 30, 90].contains(&days) {
        return Err(invalid("days应为7、30或90"));
    }
    let db = open(&state)?;
    let since = now() - days * 86400;
    let product_id = q.product_id;
    let totals:Value=db.query_row("SELECT COUNT(*),COUNT(DISTINCT user_id),COALESCE(SUM(CASE WHEN kind='detail' THEN 1 ELSE 0 END),0),COALESCE(SUM(CASE WHEN kind='outbound' THEN 1 ELSE 0 END),0) FROM product_clicks WHERE created_at>=?1 AND (?2 IS NULL OR product_id=?2)",params![since,product_id],|r|Ok(json!({"clicks":r.get::<_,i64>(0)?,"unique_users":r.get::<_,i64>(1)?,"detail_opens":r.get::<_,i64>(2)?,"outbound_clicks":r.get::<_,i64>(3)?})))?;
    let detail = totals["detail_opens"].as_i64().unwrap_or(0);
    let outbound = totals["outbound_clicks"].as_i64().unwrap_or(0);
    let products=db.prepare("SELECT p.id,p.title,COALESCE(i.impressions,0),COALESCE(c.details,0),COALESCE(c.outbound,0),COALESCE(c.unique_users,0) FROM products p LEFT JOIN (SELECT product_id,COUNT(*) impressions FROM product_impressions WHERE created_at>=?1 GROUP BY product_id)i ON i.product_id=p.id LEFT JOIN (SELECT product_id,SUM(kind='detail') details,SUM(kind='outbound') outbound,COUNT(DISTINCT user_id) unique_users FROM product_clicks WHERE created_at>=?1 GROUP BY product_id)c ON c.product_id=p.id WHERE (?2 IS NULL OR p.id=?2) ORDER BY COALESCE(c.outbound,0) DESC,p.id DESC")?.query_map(params![since,product_id],|r|Ok(json!({"product_id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"impressions":r.get::<_,i64>(2)?,"details":r.get::<_,i64>(3)?,"outbound":r.get::<_,i64>(4)?,"unique_users":r.get::<_,i64>(5)?,"ctr":if r.get::<_,i64>(3)?==0{Value::Null}else{json!(r.get::<_,i64>(4)? as f64/r.get::<_,i64>(3)? as f64)}})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(
        json!({"days":days,"since":iso(since),"definition":"CTR=outbound_clicks/detail_opens；无目录曝光，不虚构曝光率","summary":totals,"summary_ctr":if detail==0{Value::Null}else{json!(outbound as f64/detail as f64)},"products":products,"impressions":{"count":db.query_row("SELECT COUNT(*) FROM product_impressions WHERE created_at>=?1 AND (?2 IS NULL OR product_id=?2)",params![since,product_id],|r|r.get::<_,i64>(0))?,"unique_users":db.query_row("SELECT COUNT(DISTINCT user_id) FROM product_impressions WHERE created_at>=?1 AND (?2 IS NULL OR product_id=?2)",params![since,product_id],|r|r.get::<_,i64>(0))?},"product_id":product_id}),
    ))
}
#[derive(Deserialize, Default)]
struct ListQuery {
    q: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}
async fn list(State(state): State<AppState>, Query(query): Query<ListQuery>) -> ApiResult<Value> {
    let db = open(&state)?;
    let page = query.page.unwrap_or(1).max(1);
    let size = query.page_size.unwrap_or(20).clamp(1, 100);
    let status = query.status.as_deref().unwrap_or("all");
    if !["all", "active", "scheduled", "expired", "disabled"].contains(&status) {
        return Err(invalid("商品状态筛选无效"));
    }
    let filter = "WHERE (?1='' OR title LIKE '%'||?1||'%' OR external_id LIKE '%'||?1||'%') AND (?2='all' OR (?2='active' AND enabled=1 AND starts_at<=?3 AND expires_at>?3) OR (?2='scheduled' AND enabled=1 AND starts_at>?3) OR (?2='expired' AND enabled=1 AND expires_at<=?3) OR (?2='disabled' AND enabled=0))";
    let q = query.q.as_deref().unwrap_or("");
    let total: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM products {filter}"),
        params![q, status, now()],
        |r| r.get(0),
    )?;
    let share = share(&db)?;
    let items=db.prepare(&format!("SELECT id,title,url,price_fen,commission_bps,image,platform,category,external_id,starts_at,expires_at,enabled,revision FROM products {filter} ORDER BY id DESC LIMIT ?4 OFFSET ?5"))?.query_map(params![q,status,now(),size,page.saturating_sub(1).saturating_mul(size)],|r|product_row(r,share))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(
        json!({"items":items,"total":total,"page":page,"page_size":size}),
    ))
}
#[derive(Default)]
struct Upload {
    fields: HashMap<String, String>,
    image: Option<(Vec<u8>, String)>,
    existing_image: Option<String>,
}
fn json_upload(payload: Value) -> Result<Upload, ApiError> {
    let fields = payload
        .as_object()
        .ok_or_else(|| invalid("商品JSON应为对象"))?;
    let mut result = Upload::default();
    for (name, value) in fields {
        match name.as_str() {
            "image_base64" => {
                let bytes = decode_image(
                    value
                        .as_str()
                        .ok_or_else(|| invalid("image_base64应为字符串"))?,
                )?;
                let (extension, _) = image_type(&bytes)?;
                result.image = Some((bytes, extension.into()));
            }
            "image_id" => {
                result.existing_image = Some(
                    value
                        .as_str()
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| invalid("image_id应为UUID字符串"))?
                        .to_owned(),
                )
            }
            "price" | "commission_percent" => {
                let value = match value {
                    Value::String(text) => text.clone(),
                    Value::Number(number) => number.to_string(),
                    _ => return Err(invalid(&format!("{name}应为十进制字符串或数字"))),
                };
                result.fields.insert(name.clone(), value);
            }
            "enabled" => {
                let enabled = value
                    .as_bool()
                    .ok_or_else(|| invalid("JSON enabled应为true或false"))?;
                result.fields.insert(name.clone(), enabled.to_string());
            }
            "title" | "url" | "platform" | "category" | "external_id" | "starts_at"
            | "expires_at" => {
                let text = value
                    .as_str()
                    .ok_or_else(|| invalid(&format!("{name}应为字符串")))?;
                if text.len() > 8192 {
                    return Err(invalid("商品字段内容过长"));
                }
                result.fields.insert(name.clone(), text.into());
            }
            _ => return Err(invalid(&format!("未知商品字段：{name}"))),
        }
    }
    if result.image.is_some() && result.existing_image.is_some() {
        return Err(invalid("image_base64与image_id只能选择一项"));
    }
    Ok(result)
}
fn json_error(error: JsonRejection) -> ApiError {
    ApiError(
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        },
        error.body_text(),
        None,
    )
}
async fn upload(mut multipart: Multipart) -> Result<Upload, ApiError> {
    let mut result = Upload::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| invalid(&e.to_string()))?
    {
        let name = field
            .name()
            .ok_or_else(|| invalid("上传字段缺少名称"))?
            .to_owned();
        if name == "image" {
            if result.image.is_some() {
                return Err(invalid("只能上传一张商品图片"));
            }
            let bytes = field.bytes().await.map_err(|e| invalid(&e.to_string()))?;
            if bytes.is_empty() || bytes.len() > MAX_IMAGE {
                return Err(invalid("商品图片应大于0且不超过5MB"));
            }
            let (extension, _) = image_type(&bytes)?;
            result.image = Some((bytes.to_vec(), extension.into()));
        } else {
            if ![
                "title",
                "url",
                "price",
                "commission_percent",
                "platform",
                "category",
                "external_id",
                "starts_at",
                "expires_at",
                "enabled",
            ]
            .contains(&name.as_str())
            {
                return Err(invalid(&format!("未知商品字段：{name}")));
            }
            let value = field.text().await.map_err(|e| invalid(&e.to_string()))?;
            if value.len() > 8192 {
                return Err(invalid("商品字段内容过长"));
            }
            if result.fields.insert(name, value).is_some() {
                return Err(invalid("商品字段不可重复"));
            }
        }
    }
    Ok(result)
}
fn text(
    fields: &HashMap<String, String>,
    old: &Value,
    key: &str,
    required: bool,
) -> Result<String, ApiError> {
    let value = fields
        .get(key)
        .map(String::as_str)
        .or_else(|| old[key].as_str())
        .unwrap_or("")
        .trim();
    if required && value.is_empty() {
        return Err(invalid(&format!("请填写商品 {key}")));
    }
    Ok(value.into())
}
fn date(
    fields: &HashMap<String, String>,
    old: &Value,
    key: &str,
    default: i64,
) -> Result<i64, ApiError> {
    let value = fields
        .get(key)
        .map(String::as_str)
        .or_else(|| old[key].as_str());
    match value {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|v| v.timestamp())
            .map_err(|_| invalid(&format!("{key}请填写RFC3339时间"))),
        None => Ok(default),
    }
}
fn save(
    state: &AppState,
    context: &RequestContext,
    upload: Upload,
    id: Option<i64>,
    integration: bool,
) -> Result<Value, ApiError> {
    let mut db = open(state)?;
    let tx = db.transaction()?;
    let external = upload
        .fields
        .get("external_id")
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_owned());
    let target = if id.is_some() {
        id
    } else if let Some(external) = &external {
        tx.query_row(
            "SELECT id FROM products WHERE external_id=?1",
            [external],
            |r| r.get(0),
        )
        .optional()?
    } else {
        None
    };
    let old = if let Some(id) = target {
        one(&tx, id, false)?
    } else {
        json!({})
    };
    let partial = id.is_some();
    let preserve_existing = target.is_some();
    if !partial {
        for key in ["title", "url", "price", "commission_percent"] {
            if !upload.fields.contains_key(key) {
                return Err(invalid(&format!("请填写商品 {key}")));
            }
        }
        if upload.image.is_none() && upload.existing_image.is_none() {
            return Err(invalid("请上传商品图片文件"));
        }
    }
    if partial
        && upload.fields.is_empty()
        && upload.image.is_none()
        && upload.existing_image.is_none()
    {
        return Err(invalid("请提交商品修改内容"));
    }
    let title = text(&upload.fields, &old, "title", true)?;
    if title.chars().count() > 200 {
        return Err(invalid("商品名称最多200字"));
    }
    let url = text(&upload.fields, &old, "url", true)?;
    let parsed = reqwest::Url::parse(&url).map_err(|_| invalid("商品链接请填写HTTP或HTTPS地址"))?;
    if !["http", "https"].contains(&parsed.scheme())
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(invalid("商品链接请填写HTTP或HTTPS地址"));
    }
    let price = match upload.fields.get("price") {
        Some(value) => decimal(value, "价格")?,
        None => old["price_fen"].as_i64().unwrap_or(0),
    };
    if price <= 0 || price > 1_000_000_000_000 {
        return Err(invalid("商品价格应大于0且不超过100亿元"));
    }
    let commission = match upload.fields.get("commission_percent") {
        Some(value) => decimal(value, "佣金比例")?,
        None => old["commission_bps"].as_i64().unwrap_or(0),
    };
    if commission > 10000 {
        return Err(invalid("佣金比例应为0至100%"));
    }
    let c = config(&tx)?;
    let base = if preserve_existing {
        old.clone()
    } else {
        json!({})
    };
    let starts = date(&upload.fields, &base, "starts_at", now())?;
    let expires = date(
        &upload.fields,
        &base,
        "expires_at",
        starts + c.default_valid_days * 86400,
    )?;
    if expires <= starts {
        return Err(invalid("商品过期时间应晚于开始时间"));
    }
    let enabled = match upload.fields.get("enabled").map(String::as_str) {
        Some("true") => true,
        Some("false") => false,
        Some(_) => return Err(invalid("上架状态请填写true或false")),
        None => {
            if preserve_existing {
                old["enabled"].as_bool().unwrap()
            } else {
                true
            }
        }
    };
    let platform = text(
        &upload.fields,
        if preserve_existing {
            &old
        } else {
            &Value::Null
        },
        "platform",
        false,
    )?;
    let category = text(
        &upload.fields,
        if preserve_existing {
            &old
        } else {
            &Value::Null
        },
        "category",
        false,
    )?;
    let external = if partial && !upload.fields.contains_key("external_id") {
        old["external_id"].as_str().map(str::to_owned)
    } else {
        external
    };
    if let Some(external) = &external {
        let conflict_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM products WHERE external_id=?1",
                [external],
                |r| r.get(0),
            )
            .optional()?;
        if conflict_id.is_some() && conflict_id != target {
            return Err(conflict("external_id已被另一件商品使用"));
        }
    }
    if upload.image.is_some() && upload.existing_image.is_some() {
        return Err(invalid("图片文件与image_id只能选择一项"));
    }
    let image = if let Some((bytes, _)) = upload.image {
        store_image(state, &tx, context, &bytes, integration)?.image
    } else if let Some(id) = upload.existing_image {
        registered_image(state, &tx, &id)?
    } else {
        old["image"].as_str().unwrap().into()
    };
    let result_id = if let Some(id) = target {
        tx.execute("UPDATE products SET title=?1,url=?2,price_fen=?3,commission_bps=?4,image=?5,platform=?6,category=?7,external_id=?8,starts_at=?9,expires_at=?10,enabled=?11,revision=revision+1,updated_at=?12 WHERE id=?13",params![title,url,price,commission,image,platform,category,external,starts,expires,enabled,now(),id])?;
        id
    } else {
        tx.execute("INSERT INTO products(title,url,price_fen,commission_bps,image,platform,category,external_id,starts_at,expires_at,enabled,revision,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,?12,?12)",params![title,url,price,commission,image,platform,category,external,starts,expires,enabled,now()])?;
        tx.last_insert_rowid()
    };
    let result = one(&tx, result_id, false)?;
    let mut actor = context.clone();
    if integration {
        actor.admin_username = Some("product-api".into());
    }
    user_admin::action_audit(
        &tx,
        &actor,
        None,
        if target.is_some() {
            "product_update"
        } else {
            "product_create"
        },
        &format!("product:{result_id}"),
        "保存商品",
        old,
        result.clone(),
    )?;
    tx.commit()?;
    Ok(result)
}
async fn create(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    multipart: Multipart,
) -> ApiResult<Value> {
    Ok(Json(save(
        &state,
        &context,
        upload(multipart).await?,
        None,
        false,
    )?))
}
async fn update(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> ApiResult<Value> {
    Ok(Json(save(
        &state,
        &context,
        upload(multipart).await?,
        Some(id),
        false,
    )?))
}
fn authorize_integration(db: &Connection, headers: &HeaderMap) -> Result<(), ApiError> {
    let stored: Option<String> = db
        .query_row("SELECT key FROM product_api_key WHERE id=1", [], |r| {
            r.get(0)
        })
        .optional()?;
    let supplied = headers
        .get("x-product-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if stored
        .as_deref()
        .is_none_or(|key| !equal_secret(key, supplied))
    {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "商品API密钥无效".into(),
            None,
        ));
    }
    Ok(())
}
async fn integration(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    request: Request,
) -> ApiResult<Value> {
    authorize_integration(&open(&state)?, &headers)?;
    let content_type = request
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let input = if content_type.eq_ignore_ascii_case("application/json") {
        let Json(value) = Json::<Value>::from_request(request, &state)
            .await
            .map_err(json_error)?;
        json_upload(value)?
    } else if content_type.eq_ignore_ascii_case("multipart/form-data") {
        let multipart = Multipart::from_request(request, &state)
            .await
            .map_err(|e| invalid(&e.to_string()))?;
        upload(multipart).await?
    } else {
        return Err(invalid(
            "请使用application/json或multipart/form-data上传商品",
        ));
    };
    Ok(Json(save(&state, &context, input, None, true)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageInput {
    image_base64: String,
}
async fn image_upload(
    state: &AppState,
    context: &RequestContext,
    request: Request,
    integration: bool,
) -> ApiResult<StoredImage> {
    let Json(input) = Json::<ImageInput>::from_request(request, state)
        .await
        .map_err(json_error)?;
    let bytes = decode_image(&input.image_base64)?;
    let mut db = open(state)?;
    let tx = db.transaction()?;
    let image = store_image(state, &tx, context, &bytes, integration)?;
    tx.commit()?;
    Ok(Json(image))
}
async fn integration_image(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    request: Request,
) -> ApiResult<StoredImage> {
    authorize_integration(&open(&state)?, &headers)?;
    image_upload(&state, &context, request, true).await
}
async fn admin_image(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    request: Request,
) -> ApiResult<StoredImage> {
    image_upload(&state, &context, request, false).await
}
async fn get_config(State(state): State<AppState>) -> ApiResult<ProductConfig> {
    Ok(Json(config(&open(&state)?)?))
}
async fn set_config(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<ProductConfig>,
) -> ApiResult<ProductConfig> {
    if !input.platform_share_percent.is_finite()
        || !(0.0..=100.0).contains(&input.platform_share_percent)
        || (input.platform_share_percent * 100.0 - (input.platform_share_percent * 100.0).round())
            .abs()
            > 0.000001
        || !(1..=365).contains(&input.default_valid_days)
    {
        return Err(invalid(
            "平台分成应为0至100%且最多两位小数，默认有效期为1至365天",
        ));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = config(&tx)?;
    tx.execute("UPDATE product_config SET platform_share_bps=?1,default_valid_days=?2,revision=revision+1 WHERE id=1",params![(input.platform_share_percent*100.0).round() as i64,input.default_valid_days])?;
    let after = config(&tx)?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "product_config_update",
        "product-config",
        "修改商品规则",
        serde_json::to_value(before)?,
        serde_json::to_value(&after)?,
    )?;
    tx.commit()?;
    Ok(Json(after))
}
fn key_value(db: &Connection) -> Result<Value, ApiError> {
    let row: Option<(String, i64, i64)> = db
        .query_row(
            "SELECT key,created_at,rotated_at FROM product_api_key WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((key, created, rotated)) => {
            json!({"enabled":true,"key":key,"created_at":iso(created),"rotated_at":iso(rotated),"header_name":"X-Product-Api-Key","endpoint":"/api/integration/products"})
        }
        None => {
            json!({"enabled":false,"key":null,"created_at":null,"rotated_at":null,"header_name":"X-Product-Api-Key","endpoint":"/api/integration/products"})
        }
    })
}
async fn key(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(key_value(&open(&state)?)?))
}
async fn rotate_key(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let key = format!(
        "product_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    tx.execute("INSERT INTO product_api_key(id,key,created_at,rotated_at) VALUES(1,?1,?2,?2) ON CONFLICT(id) DO UPDATE SET key=excluded.key,rotated_at=excluded.rotated_at",params![key,now()])?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "product_api_key_rotate",
        "product-api-key",
        "生成或轮换商品API密钥",
        json!({}),
        json!({"enabled":true}),
    )?;
    let result = key_value(&tx)?;
    tx.commit()?;
    Ok(Json(result))
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
        .route("/products", get(list).post(create))
        .route("/products/{id}", axum::routing::patch(update))
        .route("/products/config", get(get_config).put(set_config))
        .route("/products/api-key", get(key).post(rotate_key))
        .route("/products/images", post(admin_image))
        .route(
            "/products/display-config",
            get(admin_display).put(update_display),
        )
        .route("/products/analytics", get(analytics))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .route_layer(middleware::from_fn(no_store))
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/products/{id}", get(public_one))
        .route("/products/runtime", get(public_runtime))
        .route("/products/{id}/click", post(click))
        .route("/products/{id}/impression", post(impression))
        .route("/integration/products", post(integration))
        .route("/integration/product-images", post(integration_image))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .route_layer(middleware::from_fn(no_store))
}
pub(super) fn media_routes() -> Router<AppState> {
    Router::new().route("/media/products/{filename}", get(media))
}

#[cfg(test)]
mod tests {
    use super::*;
    const PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2,
        0, 0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 223, 192, 0, 0,
        4, 1, 1, 128, 197, 42, 24, 93, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    fn upload_data() -> Upload {
        Upload {
            fields: HashMap::from([
                ("title".into(), "真实商品".into()),
                ("url".into(), "https://example.test/item/1".into()),
                ("price".into(), "99.99".into()),
                ("commission_percent".into(), "50".into()),
                ("external_id".into(), "external-1".into()),
            ]),
            image: Some((PNG.to_vec(), "png".into())),
            existing_image: None,
        }
    }
    #[test]
    fn integer_money_uses_one_final_floor_without_decimal_rounding() {
        assert_eq!(reward_coins(10000, 5000, 2000), 40000);
        assert_eq!(reward_coins(9999, 5000, 2000), 39996);
        assert_eq!(reward_coins(1, 1, 2000), 0);
        assert_eq!(reward_coins(9999, 10000, 10000), 0);
        assert_eq!(decimal("99.99", "价格").unwrap(), 9999);
        assert_eq!(decimal("1.2", "比例").unwrap(), 120);
        for bad in [
            "NaN",
            "-1",
            "1.001",
            "1e3",
            "",
            "1.",
            ".5",
            "9223372036854775807",
        ] {
            assert!(decimal(bad, "价格").is_err());
        }
    }
    #[tokio::test]
    async fn only_valid_products_reach_public_details_and_hall_without_seeds() {
        let (state, context, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        assert!(active(&db).unwrap().is_empty());
        assert!(random_active(&db).unwrap().is_none());
        let saved = save(&state, &context, upload_data(), None, false).unwrap();
        let id = saved["id"].as_i64().unwrap();
        assert_eq!(saved["reward_coins"], 39996);
        assert_eq!(active(&db).unwrap().len(), 1);
        assert_eq!(
            one(&db, id, true).unwrap()["url"],
            "https://example.test/item/1"
        );
        let updated = save(&state, &context, upload_data(), None, true).unwrap();
        assert_eq!(updated["id"], id);
        assert_eq!(updated["revision"], 2);
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM products", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let saved_url = saved["image"].as_str().unwrap();
        let media_response = media(
            State(state.clone()),
            Path(saved_url.rsplit('/').next().unwrap().to_owned()),
        )
        .await
        .unwrap();
        assert_eq!(
            media_response.headers()[axum::http::header::CONTENT_TYPE],
            "image/png"
        );
        assert_eq!(
            media_response.headers()[axum::http::header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
        assert_eq!(
            axum::body::to_bytes(media_response.into_body(), 1024)
                .await
                .unwrap()
                .as_ref(),
            PNG
        );
        for sql in [
            "UPDATE products SET enabled=0",
            "UPDATE products SET enabled=1,starts_at=strftime('%s','now')+500,expires_at=strftime('%s','now')+600",
            "UPDATE products SET starts_at=1,expires_at=2",
        ] {
            db.execute(sql, []).unwrap();
            assert!(active(&db).unwrap().is_empty());
            assert!(random_active(&db).unwrap().is_none());
            assert_eq!(one(&db, id, true).unwrap_err().0, StatusCode::NOT_FOUND);
        }
        let mut strategy = HallStrategy::default();
        strategy.video_weight = 0;
        strategy.product_weight = 100;
        strategy.native_ad_weight = 0;
        db.execute(
            "UPDATE hall_strategy SET config=?1",
            [serde_json::to_string(&strategy).unwrap()],
        )
        .unwrap();
        let mut schedule = HallSchedule {
            started: Instant::now() - Duration::from_secs(30),
            welcomed: true,
            first_packet: false,
            first_product: false,
            first_native: false,
            packet_count: 0,
            native_sent: false,
            last_dispatch_at: 0,
            unhandled_dispatches: 0,
            native_capable: false,
            last_dispatch: Instant::now() - Duration::from_secs(30),
        };
        assert!(
            hall_item(&state, &headers, &mut schedule)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM ledger WHERE user_id=2", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    fn multipart(fields: &[(&str, &str)], image: Option<&[u8]>) -> Vec<u8> {
        let mut body = Vec::new();
        for (key, value) in fields {
            body.extend_from_slice(format!("--product-test\r\nContent-Disposition: form-data; name=\"{key}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        if let Some(image) = image {
            body.extend_from_slice(b"--product-test\r\nContent-Disposition: form-data; name=\"image\"; filename=\"fake-extension.txt\"\r\nContent-Type: text/plain\r\n\r\n");
            body.extend_from_slice(image);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(b"--product-test--\r\n");
        body
    }
    #[tokio::test]
    async fn http_upload_requires_its_own_key_rotates_and_serves_actual_image_bytes() {
        let (mut state, _, _, _) = rain::tests::fixture(false, false);
        state.config.admin_username = "商品管理员".into();
        state.config.admin_password = Some("ProductAdmin123!".into());
        admin_account::initialize(&state, &open(&state).unwrap()).unwrap();
        let admin = Router::new()
            .merge(admin_routes())
            .layer(middleware::from_fn_with_state(
                state.clone(),
                admin_account::authenticate,
            ));
        let api = Router::new()
            .merge(routes())
            .route("/products", get(products))
            .route("/admin/login", post(admin_account::login))
            .nest("/admin", admin);
        let app = Router::new()
            .merge(media_routes())
            .nest("/api", api)
            .layer(middleware::from_fn_with_state(
                state.clone(),
                user_admin::request_context,
            ))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let service = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert_eq!(
            client
                .get(format!("{base}/api/admin/products/api-key"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let login: Value = client
            .post(format!("{base}/api/admin/login"))
            .json(&json!({"username":"商品管理员","password":"ProductAdmin123!"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let admin_token = login["token"].as_str().unwrap();
        let api_key: Value = client
            .post(format!("{base}/api/admin/products/api-key"))
            .header("x-admin-token", admin_token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let key = api_key["key"].as_str().unwrap();
        let fields = [
            ("title", "HTTP实际图片"),
            ("url", "https://example.test/p/2"),
            ("price", "100"),
            ("commission_percent", "50"),
            ("external_id", "http-test"),
        ];
        let body = multipart(&fields, Some(PNG));
        for unauthorized_key in ["", admin_token] {
            assert_eq!(
                client
                    .post(format!("{base}/api/integration/products"))
                    .header("content-type", "multipart/form-data; boundary=product-test")
                    .header("x-product-api-key", unauthorized_key)
                    .body(body.clone())
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::UNAUTHORIZED
            );
        }
        let uploaded = client
            .post(format!("{base}/api/integration/products"))
            .header("content-type", "multipart/form-data; boundary=product-test")
            .header("x-product-api-key", key)
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(uploaded.status(), StatusCode::OK);
        let product: Value = uploaded.json().await.unwrap();
        assert_eq!(product["reward_coins"], 40000);
        let image = client
            .get(format!("{}{}", base, product["image"].as_str().unwrap()))
            .send()
            .await
            .unwrap();
        assert_eq!(
            image.headers()[axum::http::header::CONTENT_TYPE],
            "image/png"
        );
        assert_eq!(
            image.headers()[axum::http::header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
        assert_eq!(image.bytes().await.unwrap().as_ref(), PNG);
        let repeat: Value = client
            .post(format!("{base}/api/integration/products"))
            .header("content-type", "multipart/form-data; boundary=product-test")
            .header("x-product-api-key", key)
            .body(body.clone())
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(repeat["id"], product["id"]);
        assert_eq!(
            client
                .get(format!("{base}/api/admin/products"))
                .header("x-admin-token", key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let rule: Value = client
            .put(format!("{base}/api/admin/products/config"))
            .header("x-admin-token", admin_token)
            .json(&json!({"platform_share_percent":25,"default_valid_days":9}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(rule["revision"], 2);
        assert_eq!(
            config(&open(&state).unwrap()).unwrap().default_valid_days,
            9
        );
        let repriced: Value = client
            .get(format!("{base}/api/products/{}", product["id"]))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(repriced["reward_coins"], 37500);
        assert!(!repriced.to_string().contains(key));
        let bad = client
            .post(format!("{base}/api/integration/products"))
            .header("content-type", "multipart/form-data; boundary=product-test")
            .header("x-product-api-key", key)
            .body(multipart(&fields, Some(b"<html>fake png</html>")))
            .send()
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
        let mut too_large = PNG.to_vec();
        too_large.resize(MAX_IMAGE + 1, 0);
        assert_eq!(
            client
                .post(format!("{base}/api/integration/products"))
                .header("content-type", "multipart/form-data; boundary=product-test")
                .header("x-product-api-key", key)
                .body(multipart(&fields, Some(&too_large)))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
        let disabled = client
            .patch(format!("{base}/api/admin/products/{}", product["id"]))
            .header("x-admin-token", admin_token)
            .header("content-type", "multipart/form-data; boundary=product-test")
            .body(multipart(&[("enabled", "false")], None))
            .send()
            .await
            .unwrap();
        assert_eq!(disabled.status(), StatusCode::OK);
        assert_eq!(
            client
                .get(format!("{base}/api/products/{}", product["id"]))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        let all: Value = client
            .get(format!("{base}/api/products"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(all, json!([]));
        let rotated = client
            .post(format!("{base}/api/admin/products/api-key"))
            .header("x-admin-token", admin_token)
            .send()
            .await
            .unwrap();
        assert_eq!(rotated.status(), StatusCode::OK);
        assert_eq!(
            client
                .post(format!("{base}/api/integration/products"))
                .header("content-type", "multipart/form-data; boundary=product-test")
                .header("x-product-api-key", key)
                .body(body)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let db = open(&state).unwrap();
        let audit:String=db.query_row("SELECT GROUP_CONCAT(actor||before_json||after_json||filters_json,'') FROM admin_audit",[],|r|r.get(0)).unwrap();
        assert!(!audit.contains(key));
        assert!(!audit.contains(admin_token));
        assert!(!audit.contains("ProductAdmin123!"));
        assert!(audit.contains("商品管理员"));
        assert!(audit.contains("product-api"));
        service.abort();
    }
    #[test]
    fn each_product_keeps_its_own_expiry_and_upsert_cannot_revive_or_extend_it() {
        let (state, context, _, _) = rain::tests::fixture(false, false);
        let at = now();
        let mut a = upload_data();
        a.fields.insert("starts_at".into(), iso(at));
        a.fields.insert("expires_at".into(), iso(at + 3600));
        a.fields.insert("platform".into(), "淘宝".into());
        let a = save(&state, &context, a, None, false).unwrap();
        let aid = a["id"].as_i64().unwrap();
        let mut b = upload_data();
        b.fields.insert("external_id".into(), "external-2".into());
        b.fields.insert("starts_at".into(), iso(at));
        b.fields.insert("expires_at".into(), iso(at + 7200));
        let b = save(&state, &context, b, None, false).unwrap();
        let bid = b["id"].as_i64().unwrap();
        let db = open(&state).unwrap();
        assert_eq!(active(&db).unwrap().len(), 2);
        let change = Upload {
            fields: HashMap::from([("expires_at".into(), iso(at))]),
            ..Default::default()
        };
        // 将甲的开始时间移至过去，使过期边界恰好等于服务器当前秒。
        db.execute(
            "UPDATE products SET starts_at=?1 WHERE id=?2",
            params![at - 3600, aid],
        )
        .unwrap();
        let expired = save(&state, &context, change, Some(aid), false).unwrap();
        assert_eq!(expired["seconds_until_expiry"], 0);
        assert_eq!(one(&db, aid, true).unwrap_err().0, StatusCode::NOT_FOUND);
        assert_eq!(one(&db, bid, true).unwrap()["expires_at"], b["expires_at"]);
        db.execute("UPDATE products SET enabled=0 WHERE id=?1", [aid])
            .unwrap();
        db.execute(
            "UPDATE product_config SET default_valid_days=30 WHERE id=1",
            [],
        )
        .unwrap();
        let updated = save(&state, &context, upload_data(), None, true).unwrap();
        assert_eq!(updated["id"], aid);
        assert_eq!(updated["expires_at"], expired["expires_at"]);
        assert_eq!(updated["enabled"], false);
        assert_eq!(updated["platform"], "淘宝");
        assert_eq!(one(&db, bid, true).unwrap()["expires_at"], b["expires_at"]);
        let exact:Value=serde_json::from_str(r#"{"price":99.990000000000001,"commission_percent":50,"image_id":"12345678-1234-4234-8234-123456789abc"}"#).unwrap();
        let parsed = json_upload(exact).unwrap();
        assert_eq!(parsed.fields["price"], "99.990000000000001");
        assert!(decimal(&parsed.fields["price"], "价格").is_err());
    }
    #[tokio::test]
    async fn json_inline_and_registered_image_uploads_validate_bytes_ids_and_keep_secrets_private()
    {
        let (state, context, _, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        db.execute(
            "INSERT INTO product_api_key VALUES(1,'json-product-private-key',?1,?1)",
            [now()],
        )
        .unwrap();
        let money: (i64, i64) = db
            .query_row(
                "SELECT (SELECT SUM(coins) FROM users),(SELECT COUNT(*) FROM ledger)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let app = Router::new()
            .nest("/api", routes())
            .merge(media_routes())
            .layer(Extension(context))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let service = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let encoded = STANDARD.encode(PNG);
        let product = json!({"title":"JSON一步商品","url":"https://example.test/json","price":99.99,"commission_percent":"50","image_base64":encoded,"external_id":"json-inline","starts_at":iso(now()),"expires_at":iso(now()+3600)});
        assert_eq!(
            client
                .post(format!("{base}/api/integration/products"))
                .header("x-product-api-key", "wrong")
                .json(&product)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let response = client
            .post(format!("{base}/api/integration/products"))
            .header("x-product-api-key", "json-product-private-key")
            .json(&product)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let inline: Value = response.json().await.unwrap();
        assert_eq!(inline["reward_coins"], 39996);
        assert_eq!(
            client
                .post(format!("{base}/api/integration/product-images"))
                .json(&json!({"image_base64":encoded}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let response = client
            .post(format!("{base}/api/integration/product-images"))
            .header("x-product-api-key", "json-product-private-key")
            .json(&json!({"image_base64":format!("data:image/png;base64,{encoded}")}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let image: Value = response.json().await.unwrap();
        assert_eq!(image["size_bytes"], PNG.len());
        assert_eq!(image["content_type"], "image/png");
        assert!(Uuid::parse_str(image["image_id"].as_str().unwrap()).is_ok());
        let mut referenced = product.clone();
        referenced.as_object_mut().unwrap().remove("image_base64");
        referenced["image_id"] = image["image_id"].clone();
        referenced["external_id"] = json!("json-reference");
        let response = client
            .post(format!("{base}/api/integration/products"))
            .header("x-product-api-key", "json-product-private-key")
            .json(&referenced)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let referenced_product: Value = response.json().await.unwrap();
        assert_eq!(referenced_product["image"], image["image"]);
        let photo = client
            .get(format!("{}{}", base, image["image"].as_str().unwrap()))
            .send()
            .await
            .unwrap();
        assert_eq!(photo.bytes().await.unwrap().as_ref(), PNG);
        for payload in [
            json!({"image_base64":"not:base64"}),
            json!({"image_base64":STANDARD.encode(b"<html>not an image</html>")}),
            json!({"image_base64":"data:text/plain;base64,YQ=="}),
        ] {
            assert_eq!(
                client
                    .post(format!("{base}/api/integration/product-images"))
                    .header("x-product-api-key", "json-product-private-key")
                    .json(&payload)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        let mut missing = product.clone();
        missing.as_object_mut().unwrap().remove("image_base64");
        let mut conflict_image = product.clone();
        conflict_image["image_id"] = image["image_id"].clone();
        let mut foreign = missing.clone();
        foreign["image_id"] = json!("https://example.test/image.png");
        let mut unknown = missing.clone();
        unknown["image_id"] = json!(Uuid::new_v4().to_string());
        for bad in [missing, conflict_image, foreign, unknown] {
            assert_eq!(
                client
                    .post(format!("{base}/api/integration/products"))
                    .header("x-product-api-key", "json-product-private-key")
                    .json(&bad)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        let mut oversized = PNG.to_vec();
        oversized.resize(MAX_IMAGE + 1, 0);
        assert_eq!(
            client
                .post(format!("{base}/api/integration/product-images"))
                .header("x-product-api-key", "json-product-private-key")
                .json(&json!({"image_base64":STANDARD.encode(oversized)}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            client
                .post(format!("{base}/api/integration/product-images"))
                .header("x-product-api-key", "json-product-private-key")
                .json(&json!({"image_base64":"A".repeat(8*1024*1024)}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        let audit: String = db
            .query_row(
                "SELECT GROUP_CONCAT(before_json||after_json||filters_json,'') FROM admin_audit",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!audit.contains(&encoded));
        assert!(!audit.contains("json-product-private-key"));
        let after: (i64, i64) = db
            .query_row(
                "SELECT (SELECT SUM(coins) FROM users),(SELECT COUNT(*) FROM ledger)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(after, money);
        service.abort();
    }
    #[tokio::test]
    async fn click_and_impression_stats_are_real_and_expiry_is_enforced() {
        let (state, context, headers, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        let product = save(&state, &context, upload_data(), None, false).unwrap();
        let id = product["id"].as_i64().unwrap();
        let click_result = click(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Path(id),
            Json(ClickInput {
                kind: "detail".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(click_result.0["ok"], true);
        let click_result = click(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Path(id),
            Json(ClickInput {
                kind: "outbound".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(click_result.0["kind"], "outbound");
        let _ = impression(
            State(state.clone()),
            Extension(context.clone()),
            headers,
            Path(id),
            Json(json!({})),
        )
        .await
        .unwrap();
        let stats = analytics(
            State(state.clone()),
            Query(AnalyticsQuery {
                days: Some(7),
                product_id: None,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(stats["summary"]["detail_opens"], 1);
        assert_eq!(stats["summary"]["outbound_clicks"], 1);
        assert_eq!(stats["summary_ctr"], 1.0);
        assert_eq!(stats["impressions"]["count"], 1);
        assert_eq!(stats["products"][0]["details"], 1);
        db.execute(
            "UPDATE products SET expires_at=?1 WHERE id=?2",
            params![now() - 1, id],
        )
        .unwrap();
        let expired = click(
            State(state),
            Extension(context),
            HeaderMap::new(),
            Path(id),
            Json(ClickInput {
                kind: "detail".into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(expired.0, StatusCode::NOT_FOUND);
    }
    #[tokio::test]
    async fn empty_analytics_returns_zero_counts_and_null_ctr() {
        let (state, _, _, _) = rain::tests::fixture(false, false);
        let stats = analytics(
            State(state),
            Query(AnalyticsQuery {
                days: Some(7),
                product_id: None,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(stats["summary"]["clicks"], 0);
        assert_eq!(stats["summary"]["detail_opens"], 0);
        assert_eq!(stats["summary"]["outbound_clicks"], 0);
        assert!(stats["summary_ctr"].is_null());
        assert_eq!(stats["impressions"]["count"], 0);
    }
}
