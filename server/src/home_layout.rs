//! Server-controlled home entries: an icon grid and one animated floating entry in the hall.
//! Each item opens either a built-in page or an H5 activity page hosted on our own domains.
use super::*;

pub(super) const NATIVE_TARGETS: &[&str] =
    &["checkin", "invite", "activity", "withdraw", "shop", "ledger", "me"];
pub(super) const ICONS: &[&str] = &["gift", "coin", "clock", "shop", "me", "hall", "notice", "shield", "check"];
const ANIMATIONS: &[&str] = &["float", "bounce", "shake", "none"];

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct HomeEntry {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub badge: String,
    /// Built-in line icon name, used when image_url is empty.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub image_url: String,
    /// "native" or "web".
    pub action: String,
    /// Native page key, or a web path ("/activity/x.html") / https URL on an allowed host.
    pub target: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub expires_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Floating {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub image_url: String,
    #[serde(default = "gift")]
    pub icon: String,
    #[serde(default = "web")]
    pub action: String,
    #[serde(default)]
    pub target: String,
    #[serde(default = "float")]
    pub animation: String,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub expires_at: String,
}
fn yes() -> bool {
    true
}
fn gift() -> String {
    "gift".into()
}
fn web() -> String {
    "web".into()
}
fn float() -> String {
    "float".into()
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct HomeLayout {
    pub entries: Vec<HomeEntry>,
    pub floating: Floating,
}
impl Default for HomeLayout {
    /// Empty by default: the hall already has check-in, tabs and "me"; entries are only for
    /// campaigns that operations add with their own name and artwork.
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            floating: Floating {
                enabled: false,
                title: String::new(),
                image_url: String::new(),
                icon: "gift".into(),
                action: "web".into(),
                target: String::new(),
                animation: "float".into(),
                starts_at: String::new(),
                expires_at: String::new(),
            },
        }
    }
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS home_layout(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,revision INTEGER NOT NULL,updated_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO home_layout(id,json,revision,updated_at) VALUES(1,?1,1,?2)",
        params![serde_json::to_string(&HomeLayout::default())?, now()],
    )?;
    Ok(())
}

fn allowed_hosts() -> Vec<String> {
    env_value("ORANGE_ACTIVITY_HOSTS")
        .unwrap_or_else(|| "orange.lingzidev.com,orange-api.lingzidev.com".into())
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn check_target(action: &str, target: &str) -> Result<(), ApiError> {
    match action {
        "native" if NATIVE_TARGETS.contains(&target) => Ok(()),
        "native" => Err(invalid("未知的内置页面")),
        "web" => check_url(target, "活动地址"),
        _ => Err(invalid("打开方式应为 native 或 web")),
    }
}

/// Relative paths are served from our web host; absolute URLs must be https on an allowed host.
fn check_url(value: &str, name: &str) -> Result<(), ApiError> {
    if value.len() > 300 || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid(&format!("{name}格式不正确")));
    }
    if value.starts_with('/') && !value.starts_with("//") {
        return Ok(());
    }
    let rest = value
        .strip_prefix("https://")
        .ok_or_else(|| invalid(&format!("{name}只能是 / 开头的站内路径或 https 地址")))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if host.contains('@') || !allowed_hosts().iter().any(|h| *h == host) {
        return Err(invalid(&format!("{name}的域名不在允许列表中")));
    }
    Ok(())
}

fn check_time(value: &str) -> Result<Option<i64>, ApiError> {
    if value.is_empty() {
        return Ok(None);
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| Some(t.timestamp()))
        .map_err(|_| invalid("时间格式应为 RFC3339，例如 2026-12-01T00:00:00+08:00"))
}

fn validate(input: &mut HomeLayout) -> Result<(), ApiError> {
    if input.entries.len() > 10 {
        return Err(invalid("首页入口最多 10 个"));
    }
    let mut seen = std::collections::HashSet::new();
    for e in &mut input.entries {
        e.id = e.id.trim().to_owned();
        e.title = e.title.trim().to_owned();
        e.badge = e.badge.trim().to_owned();
        if e.id.is_empty() || e.id.len() > 32 || !seen.insert(e.id.clone()) {
            return Err(invalid("入口标识不能为空且不能重复"));
        }
        if e.title.is_empty() || e.title.chars().count() > 6 {
            return Err(invalid("请填写入口名称，1 至 6 个字"));
        }
        if e.badge.chars().count() > 4 {
            return Err(invalid("角标最多 4 个字"));
        }
        if e.image_url.is_empty() && !ICONS.contains(&e.icon.as_str()) {
            return Err(invalid("请选择内置图标或填写图标图片"));
        }
        if !e.image_url.is_empty() {
            check_url(&e.image_url, "图标图片")?;
        }
        check_target(&e.action, &e.target)?;
        check_time(&e.starts_at)?;
        check_time(&e.expires_at)?;
    }
    let f = &mut input.floating;
    f.title = f.title.trim().to_owned();
    if f.title.chars().count() > 8 {
        return Err(invalid("悬浮入口名称最多 8 个字"));
    }
    if !ANIMATIONS.contains(&f.animation.as_str()) {
        return Err(invalid("动画应为 float、bounce、shake 或 none"));
    }
    if !f.image_url.is_empty() {
        check_url(&f.image_url, "悬浮图片")?;
    } else if !ICONS.contains(&f.icon.as_str()) {
        return Err(invalid("请选择内置图标或填写悬浮图片"));
    }
    if f.enabled && f.target.is_empty() {
        return Err(invalid("开启悬浮入口前请填写打开的页面或活动地址"));
    }
    if !f.target.is_empty() {
        check_target(&f.action, &f.target)?;
    }
    check_time(&f.starts_at)?;
    check_time(&f.expires_at)?;
    Ok(())
}

fn live(enabled: bool, starts: &str, expires: &str, at: i64) -> bool {
    enabled
        && check_time(starts).ok().flatten().is_none_or(|s| s <= at)
        && check_time(expires).ok().flatten().is_none_or(|e| e > at)
}

/// What the app shows right now: only enabled items inside their time window.
async fn public_home(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let (raw, revision): (String, i64) =
        db.query_row("SELECT json,revision FROM home_layout WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let layout: HomeLayout = serde_json::from_str(&raw)?;
    let at = Utc::now().timestamp();
    let entries: Vec<&HomeEntry> =
        layout.entries.iter().filter(|e| live(e.enabled, &e.starts_at, &e.expires_at, at)).collect();
    let f = &layout.floating;
    let floating = if live(f.enabled, &f.starts_at, &f.expires_at, at) { json!(f) } else { Value::Null };
    Ok(Json(json!({"revision":revision,"entries":entries,"floating":floating})))
}

fn layout_json(db: &Connection) -> Result<Value, ApiError> {
    let (raw, revision, updated): (String, i64, i64) = db.query_row(
        "SELECT json,revision,updated_at FROM home_layout WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut value: Value = serde_json::from_str(&raw)?;
    value["revision"] = json!(revision);
    value["updated_at"] = json!(iso(updated));
    value["native_targets"] = json!(NATIVE_TARGETS);
    value["icons"] = json!(ICONS);
    value["allowed_hosts"] = json!(allowed_hosts());
    Ok(value)
}
async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(layout_json(&open(&state)?)?))
}
async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<HomeLayout>,
) -> ApiResult<Value> {
    validate(&mut input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = layout_json(&tx)?;
    tx.execute(
        "UPDATE home_layout SET json=?1,revision=revision+1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&input)?, now()],
    )?;
    let after = layout_json(&tx)?;
    user_admin::action_audit(&tx, &context, None, "home_layout_update", "home-layout", "修改首页入口", before, after.clone())?;
    tx.commit()?;
    Ok(Json(after))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/app/home", get(public_home))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/home-layout", get(admin_get).put(admin_put))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_are_limited_to_own_hosts_and_paths() {
        assert!(check_url("/activity/demo.html", "x").is_ok());
        assert!(check_url("https://orange.lingzidev.com/a", "x").is_ok());
        assert!(check_url("//evil.com/a", "x").is_err());
        assert!(check_url("http://orange.lingzidev.com/a", "x").is_err());
        assert!(check_url("https://evil.com/a", "x").is_err());
        assert!(check_url("https://orange.lingzidev.com@evil.com/a", "x").is_err());
        assert!(check_target("native", "checkin").is_ok());
        assert!(check_target("native", "rm").is_err());
        let mut layout = HomeLayout::default();
        assert!(validate(&mut layout).is_ok());
        assert!(live(true, "", "2000-01-01T00:00:00Z", Utc::now().timestamp()) == false);
    }
}
