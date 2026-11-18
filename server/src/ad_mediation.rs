//! Ad networks: per-network app/placement config, weighted routing, quiet risk blocking,
//! reward callbacks for every network, and the event log behind the admin statistics.
//!
//! The server decides which networks the app may try for each request ("plan"). When the
//! user or device is under a risk decision the plan is simply empty, which the app treats
//! exactly like "no fill": no special prompt, and coin amounts are never touched here.
//! Coins are only ever credited from a verified server callback (see `accept_reward`).
use super::*;
use std::collections::BTreeMap;

pub(super) const NETWORKS: &[(&str, &str)] =
    &[("csj", "穿山甲"), ("kuaishou", "快手"), ("baidu", "百度"), ("gdt", "优量汇")];
pub(super) const FORMATS: &[(&str, &str)] = &[("rewarded", "激励视频"), ("feed", "信息流")];
const CLIENT_EVENTS: &[&str] = &[
    "request", "fill", "no_fill", "show", "click", "close", "reward", "skip", "error", "timeout",
];
const SERVER_EVENTS: &[&str] = &["plan", "risk_block", "empty", "callback_ok", "callback_retry", "callback_reject"];
const KEEP_DAYS: i64 = 90;
const DAILY_EVENT_CAP: i64 = 3000;

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub(super) struct Network {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    /// Placement (code) id for rewarded video.
    #[serde(default)]
    pub rewarded: String,
    /// Placement (code) id for template feed cards in the hall.
    #[serde(default)]
    pub feed: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct FormatRule {
    /// "weighted": random pick by weight, the rest follow as fallbacks by weight.
    /// "ordered": always try in the listed order.
    #[serde(default = "weighted")]
    pub mode: String,
    #[serde(default)]
    pub weights: BTreeMap<String, u32>,
    #[serde(default)]
    pub order: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct AdConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Return no ads while the account or device is under a risk decision.
    #[serde(default = "yes")]
    pub risk_block: bool,
    /// How far back a DeviceTrust review/deny decision still blocks ads.
    #[serde(default = "day_hours")]
    pub risk_window_hours: u32,
    #[serde(default)]
    pub networks: BTreeMap<String, Network>,
    pub rewarded: FormatRule,
    pub feed: FormatRule,
}
fn yes() -> bool {
    true
}
fn day_hours() -> u32 {
    24
}
fn weighted() -> String {
    "weighted".into()
}

impl FormatRule {
    fn only_csj() -> Self {
        Self {
            mode: weighted(),
            weights: NETWORKS
                .iter()
                .map(|(n, _)| (n.to_string(), if *n == "csj" { 100 } else { 0 }))
                .collect(),
            order: NETWORKS.iter().map(|(n, _)| n.to_string()).collect(),
        }
    }
}

impl Default for AdConfig {
    fn default() -> Self {
        let mut networks: BTreeMap<String, Network> =
            NETWORKS.iter().map(|(n, _)| (n.to_string(), Network::default())).collect();
        networks.insert(
            "csj".into(),
            Network {
                enabled: true,
                app_id: "5931950".into(),
                rewarded: "993606639".into(),
                feed: "993606636".into(),
            },
        );
        Self {
            enabled: true,
            risk_block: true,
            risk_window_hours: 24,
            networks,
            rewarded: FormatRule::only_csj(),
            feed: FormatRule::only_csj(),
        }
    }
}

impl AdConfig {
    fn rule(&self, format: &str) -> &FormatRule {
        if format == "feed" { &self.feed } else { &self.rewarded }
    }
}
fn placement<'a>(network: &'a Network, format: &str) -> &'a str {
    if format == "feed" { &network.feed } else { &network.rewarded }
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS ad_config(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 1,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS ad_secrets(network TEXT PRIMARY KEY,secret TEXT NOT NULL,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS ad_events(id INTEGER PRIMARY KEY,created_at INTEGER NOT NULL,day TEXT NOT NULL,user_id INTEGER,network TEXT NOT NULL,format TEXT NOT NULL,placement TEXT NOT NULL DEFAULT '',event TEXT NOT NULL,source TEXT NOT NULL,session_id TEXT NOT NULL DEFAULT '',error_code TEXT NOT NULL DEFAULT '',message TEXT NOT NULL DEFAULT '',detail_json TEXT NOT NULL DEFAULT '{}');
         CREATE INDEX IF NOT EXISTS idx_ad_events_day ON ad_events(day,network,format,event);
         CREATE INDEX IF NOT EXISTS idx_ad_events_time ON ad_events(created_at);
         CREATE INDEX IF NOT EXISTS idx_ad_events_user ON ad_events(user_id,created_at);",
    )?;
    db.execute(
        "INSERT OR IGNORE INTO ad_config(id,json,updated_at) VALUES(1,?1,?2)",
        params![serde_json::to_string(&AdConfig::default())?, now()],
    )?;
    let columns = db
        .prepare("PRAGMA table_info(reward_sessions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !columns.iter().any(|c| c == "ad_network") {
        db.execute("ALTER TABLE reward_sessions ADD COLUMN ad_network TEXT", [])?;
    }
    db.execute(
        "DELETE FROM ad_events WHERE created_at<?1",
        [now() - KEEP_DAYS * 86400],
    )?;
    Ok(())
}

pub(super) fn config(db: &Connection) -> Result<AdConfig, ApiError> {
    let raw: String = db.query_row("SELECT json FROM ad_config WHERE id=1", [], |r| r.get(0))?;
    Ok(serde_json::from_str(&raw)?)
}

/// Callback signing key: the admin-entered one, or for 穿山甲 the deployment env fallback.
pub(super) fn secret(state: &AppState, db: &Connection, network: &str) -> Result<Option<String>, ApiError> {
    let stored: Option<String> = db
        .query_row("SELECT secret FROM ad_secrets WHERE network=?1", [network], |r| r.get(0))
        .optional()?;
    Ok(stored
        .filter(|s| !s.is_empty())
        .or_else(|| if network == "csj" { state.config.csj_reward_secret.clone() } else { None }))
}

/// Why this user should not get ads right now (admin-only wording), if any.
fn risk_reason(db: &Connection, uid: i64, cfg: &AdConfig) -> Result<Option<String>, ApiError> {
    let restriction: Option<String> = db
        .query_row("SELECT status FROM account_restrictions WHERE user_id=?1", [uid], |r| r.get(0))
        .optional()?;
    if let Some(status) = restriction.filter(|s| s != "active") {
        return Ok(Some(format!("account_{status}")));
    }
    let since = now() - i64::from(cfg.risk_window_hours.clamp(1, 24 * 30)) * 3600;
    let hit: Option<(String, String, String)> = db
        .query_row(
            "SELECT id,decision,action FROM risk_events WHERE user_id=?1 AND status='reported' AND created_at>=?2
             AND (lower(decision) IN ('review','deny','reject','block') OR lower(action) IN ('verify','hold','block','deny'))
             ORDER BY created_at DESC LIMIT 1",
            params![uid, since],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(hit.map(|(id, decision, action)| format!("device_trust {decision}/{action} {id}")))
}

/// Candidate networks for this format: enabled, configured, and (for rewarded) able to verify callbacks.
fn candidates(state: &AppState, db: &Connection, cfg: &AdConfig, format: &str) -> Result<Vec<String>, ApiError> {
    let mut out = Vec::new();
    for (name, _) in NETWORKS {
        let Some(n) = cfg.networks.get(*name) else { continue };
        if !n.enabled || n.app_id.is_empty() || placement(n, format).is_empty() {
            continue;
        }
        if format == "rewarded" && secret(state, db, name)?.is_none() {
            continue;
        }
        out.push(name.to_string());
    }
    Ok(out)
}

/// Order candidates by the format rule. `ticket` is a uniform random number.
pub(super) fn order(rule: &FormatRule, candidates: &[String], ticket: u64) -> Vec<String> {
    if rule.mode == "ordered" {
        let mut ordered: Vec<String> =
            rule.order.iter().filter(|n| candidates.contains(n)).cloned().collect();
        for n in candidates {
            if !ordered.contains(n) {
                ordered.push(n.clone());
            }
        }
        return ordered;
    }
    let mut weighted: Vec<(String, u32)> = candidates
        .iter()
        .map(|n| (n.clone(), rule.weights.get(n).copied().unwrap_or(0)))
        .filter(|(_, w)| *w > 0)
        .collect();
    let total: u64 = weighted.iter().map(|(_, w)| u64::from(*w)).sum();
    if total == 0 {
        return Vec::new();
    }
    let mut pick = ticket % total;
    let mut first = 0;
    for (i, (_, w)) in weighted.iter().enumerate() {
        if pick < u64::from(*w) {
            first = i;
            break;
        }
        pick -= u64::from(*w);
    }
    let chosen = weighted.remove(first);
    weighted.sort_by(|a, b| b.1.cmp(&a.1));
    std::iter::once(chosen).chain(weighted).map(|(n, _)| n).collect()
}

/// The ordered list of {network, app_id, code_id} the app may try. Empty means "no ad".
pub(super) fn plan(
    state: &AppState,
    db: &Connection,
    uid: i64,
    format: &str,
    session: &str,
) -> Result<Vec<Value>, ApiError> {
    let cfg = config(db)?;
    if !cfg.enabled {
        log(db, Some(uid), "-", format, "", "empty", "server", session, "", "广告总开关关闭", json!({}))?;
        return Ok(Vec::new());
    }
    if cfg.risk_block && trusted_override(db, uid)?.is_none() {
        if let Some(reason) = risk_reason(db, uid, &cfg)? {
            log(db, Some(uid), "-", format, "", "risk_block", "server", session, "", &reason, json!({}))?;
            return Ok(Vec::new());
        }
    }
    let names = order(cfg.rule(format), &candidates(state, db, &cfg, format)?, Uuid::new_v4().as_u128() as u64);
    if names.is_empty() {
        log(db, Some(uid), "-", format, "", "empty", "server", session, "", "没有可用的广告平台", json!({}))?;
        return Ok(Vec::new());
    }
    let first = &cfg.networks[&names[0]];
    log(db, Some(uid), &names[0], format, placement(first, format), "plan", "server", session, "", "", json!({"order":names}))?;
    Ok(names
        .iter()
        .map(|n| {
            let net = &cfg.networks[n];
            json!({"network":n,"app_id":net.app_id,"code_id":placement(net, format)})
        })
        .collect())
}

/// 穿山甲 ids for clients that predate the multi-network plan.
pub(super) fn legacy_csj(db: &Connection) -> Result<Option<(String, String)>, ApiError> {
    let cfg = config(db)?;
    Ok(cfg
        .networks
        .get("csj")
        .filter(|n| n.enabled && !n.app_id.is_empty() && !n.rewarded.is_empty())
        .map(|n| (n.app_id.clone(), n.rewarded.clone())))
}

/// App ids the client should initialise (only enabled networks with an app id).
pub(super) fn public_networks(db: &Connection) -> Result<Value, ApiError> {
    let cfg = config(db)?;
    let mut out = serde_json::Map::new();
    if cfg.enabled {
        for (name, net) in &cfg.networks {
            if net.enabled && !net.app_id.is_empty() {
                out.insert(name.clone(), json!(net.app_id));
            }
        }
    }
    Ok(json!({"networks":out}))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn log(
    db: &Connection,
    uid: Option<i64>,
    network: &str,
    format: &str,
    placement: &str,
    event: &str,
    source: &str,
    session: &str,
    code: &str,
    message: &str,
    detail: Value,
) -> Result<(), ApiError> {
    db.execute(
        "INSERT INTO ad_events(created_at,day,user_id,network,format,placement,event,source,session_id,error_code,message,detail_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![now(), day(), uid, network, format, placement, event, source, session, code, message, detail.to_string()],
    )?;
    Ok(())
}

// ---------- reward callbacks ----------

pub(super) struct Decision {
    pub valid: bool,
    pub reason: &'static str,
    pub session: String,
    pub uid: Option<i64>,
}
fn reject(reason: &'static str) -> Decision {
    Decision { valid: false, reason, session: String::new(), uid: None }
}

/// Shared tail of every network's callback once its signature checked out.
pub(super) fn accept_reward(
    state: &AppState,
    network: &str,
    trans_id: &str,
    user_id: &str,
    extra: &str,
) -> Result<Decision, ApiError> {
    if state.config.ad_mode != "csj" {
        return Ok(reject("当前广告通道未开启"));
    }
    if trans_id.is_empty() || trans_id.len() > 160 || user_id.is_empty() {
        return Ok(reject("缺少奖励回调参数"));
    }
    let extra: Value = match serde_json::from_str(extra) {
        Ok(value) => value,
        Err(_) => return Ok(reject("extra 应为奖励会话 JSON")),
    };
    let Some(session_id) = extra.get("reward_session_id").and_then(Value::as_str) else {
        return Ok(reject("extra 缺少 reward_session_id"));
    };
    let mut db = open(state)?;
    let tx = db.transaction()?;
    let row: Option<(i64, String, i64, Option<i64>, Option<String>)> = tx
        .query_row(
            "SELECT user_id,provider,started_at,verified_at,csj_trans_id FROM reward_sessions WHERE id=?1",
            [session_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((uid, provider, started_at, verified_at, previous)) = row else {
        return Ok(reject("奖励会话不存在"));
    };
    let decision = |valid, reason| Decision { valid, reason, session: session_id.to_owned(), uid: Some(uid) };
    if uid.to_string() != user_id || provider != "csj" {
        return Ok(decision(false, "奖励会话归属不匹配"));
    }
    if now() - started_at > 1800 {
        return Ok(decision(false, "奖励会话已过期"));
    }
    // 穿山甲 keeps its historical bare trans id; other networks are prefixed so ids never collide.
    let stored = if network == "csj" { trans_id.to_owned() } else { format!("{network}:{trans_id}") };
    if verified_at.is_some() {
        let same = previous.as_deref() == Some(stored.as_str());
        return Ok(decision(same, "duplicate_transaction"));
    }
    let used: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM reward_sessions WHERE csj_trans_id=?1)",
        [&stored],
        |r| r.get(0),
    )?;
    if used {
        return Ok(decision(false, "交易编号已关联另一份奖励"));
    }
    tx.execute(
        "UPDATE reward_sessions SET verified_at=?1,csj_trans_id=?2,ad_network=?3 WHERE id=?4",
        params![now(), stored, network, session_id],
    )?;
    tx.commit()?;
    Ok(decision(true, "verified"))
}

/// Audit + statistics for a callback from any network.
pub(super) fn record_callback(
    state: &AppState,
    context: &RequestContext,
    network: &str,
    trans_id: &str,
    extra: &str,
    decision: &Decision,
) -> Result<(), ApiError> {
    let db = open(state)?;
    let query: HashMap<String, String> =
        [("trans_id".to_string(), trans_id.to_string()), ("extra".to_string(), extra.to_string())].into();
    user_admin::callback_audit(&db, context, &query, decision.valid, decision.reason)?;
    let placement: String = if decision.session.is_empty() {
        String::new()
    } else {
        config(&db).map(|c| c.networks.get(network).map(|n| n.rewarded.clone()).unwrap_or_default())?
    };
    log(
        &db,
        decision.uid,
        network,
        "rewarded",
        &placement,
        match (decision.valid, decision.reason) {
            // A platform retrying a callback we already accepted is not a new success.
            (true, "duplicate_transaction") => "callback_retry",
            (true, _) => "callback_ok",
            _ => "callback_reject",
        },
        "callback",
        &decision.session,
        "",
        decision.reason,
        json!({"trans_id":trans_id.chars().take(160).collect::<String>()}),
    )
}

fn hex_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

async fn network_callback(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(network): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let get = |key: &str| q.get(key).map(String::as_str).unwrap_or("");
    // Each network names its fields differently and orders the signed text differently.
    let (trans, user, extra, sign, expected): (&str, &str, &str, &str, fn(&str, &str) -> String) =
        match network.as_str() {
            // 快手：sign = md5(appSecurityKey:transId)
            "kuaishou" => (get("transId"), get("userId"), get("extra"), get("sign"), |k, t| md5_hex(&format!("{k}:{t}"))),
            // 百度：sign = sha256(trans_id:secret)
            "baidu" => (get("trans_id"), get("user_id"), get("extra"), get("sign"), |k, t| hex_sha256(&format!("{t}:{k}"))),
            // 优量汇：sig = sha256(transid:secret)
            "gdt" => (get("transid"), get("userid"), get("extrainfo"), get("sig"), |k, t| hex_sha256(&format!("{t}:{k}"))),
            _ => return Err(ApiError(StatusCode::NOT_FOUND, "接口不存在".into(), None)),
        };
    let db = open(&state)?;
    let decision = match secret(&state, &db, &network)? {
        None => reject("未配置回调密钥"),
        Some(key) if !equal_secret(&expected(&key, trans), &sign.to_ascii_lowercase()) => reject("签名不匹配"),
        Some(_) => accept_reward(&state, &network, trans, user, extra)?,
    };
    drop(db);
    if !decision.valid {
        eprintln!("{network} 奖励回调未通过：{}", decision.reason);
    }
    record_callback(&state, &context, &network, trans, extra, &decision)?;
    let body = if network == "baidu" {
        json!({"isValid": if decision.valid {"1"} else {"0"}, "code": 0, "msg": "success"})
    } else {
        json!({"isValid": decision.valid})
    };
    Ok(Json(body).into_response())
}

// ---------- client events ----------

#[derive(Deserialize)]
struct ClientEvent {
    network: String,
    format: String,
    event: String,
    #[serde(default)]
    code_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    error_code: Option<Value>,
    #[serde(default)]
    message: String,
    #[serde(default)]
    elapsed_ms: Option<u32>,
}
#[derive(Deserialize)]
struct ClientEvents {
    events: Vec<ClientEvent>,
}

async fn client_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ClientEvents>,
) -> ApiResult<Value> {
    if input.events.len() > 30 {
        return Err(invalid("事件过多"));
    }
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let today: i64 = db.query_row(
        "SELECT COUNT(*) FROM ad_events WHERE user_id=?1 AND created_at>=?2 AND source='app'",
        params![uid, today_start()],
        |r| r.get(0),
    )?;
    if today >= DAILY_EVENT_CAP {
        return Ok(Json(json!({"accepted":0})));
    }
    let tx = db.transaction()?;
    let mut accepted = 0;
    for e in input.events.iter().take((DAILY_EVENT_CAP - today) as usize) {
        if !NETWORKS.iter().any(|(n, _)| *n == e.network)
            || !FORMATS.iter().any(|(f, _)| *f == e.format)
            || !CLIENT_EVENTS.contains(&e.event.as_str())
            || e.code_id.len() > 40
            || e.session_id.len() > 64
        {
            continue;
        }
        let code = match &e.error_code {
            Some(Value::String(s)) => s.chars().take(24).collect(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let message: String = e.message.chars().take(200).collect();
        log(&tx, Some(uid), &e.network, &e.format, &e.code_id, &e.event, "app", &e.session_id, &code, &message, json!({"elapsed_ms":e.elapsed_ms}))?;
        accepted += 1;
    }
    tx.commit()?;
    Ok(Json(json!({"accepted":accepted})))
}

// ---------- admin ----------

fn callback_base(state: &AppState) -> String {
    env_value("ORANGE_PUBLIC_API_URL").unwrap_or_else(|| {
        if state.config.production {
            "https://orange-api.lingzidev.com".into()
        } else {
            "http://127.0.0.1:38180".into()
        }
    })
}

fn callbacks(state: &AppState) -> Value {
    let base = callback_base(state);
    json!({
        "csj": {"url": format!("{base}/csj/callback"),
                "how": "穿山甲后台 → 广告位 → 激励视频代码位 → 服务端回调：填写此地址，安全密钥填到下方“回调密钥”。奖励名称/数量与“奖励设置”一致。"},
        "kuaishou": {"url": format!("{base}/api/ads/callback/kuaishou?userId=__UID__&transId=__TRANSID__&sign=__SIGN__&amount=__RAMOUNT__&name=__RNAME__&extra=__EXTRA__"),
                "how": "快手联盟 → 激励视频广告位 → 服务端回调：整段地址原样粘贴（包含 __UID__ 等占位符），保存后得到的 appSecurityKey 填到“回调密钥”。"},
        "baidu": {"url": format!("{base}/api/ads/callback/baidu"),
                "how": "百青藤 → 激励视频代码位 → 服务端奖励验证：填写此地址（不要加参数），代码位生成的密钥填到“回调密钥”。"},
        "gdt": {"url": format!("{base}/api/ads/callback/gdt"),
                "how": "优量汇 → 激励视频广告位 → 服务端验证：填写此地址（不要加参数），获取的密钥填到“回调密钥”。"},
    })
}

fn admin_json(state: &AppState, db: &Connection) -> Result<Value, ApiError> {
    let (raw, revision, updated): (String, i64, i64) = db.query_row(
        "SELECT json,revision,updated_at FROM ad_config WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut value: Value = serde_json::from_str(&raw)?;
    let mut secrets = serde_json::Map::new();
    for (name, _) in NETWORKS {
        let stored: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM ad_secrets WHERE network=?1 AND secret!='')",
            [name],
            |r| r.get(0),
        )?;
        let source = if stored {
            "admin"
        } else if *name == "csj" && state.config.csj_reward_secret.is_some() {
            "env"
        } else {
            ""
        };
        secrets.insert(name.to_string(), json!(source));
    }
    value["revision"] = json!(revision);
    value["updated_at"] = json!(iso(updated));
    value["secret_source"] = Value::Object(secrets);
    value["callbacks"] = callbacks(state);
    value["network_labels"] = json!(NETWORKS.iter().map(|(n, l)| json!({"id":n,"label":l})).collect::<Vec<_>>());
    value["ad_mode"] = json!(state.config.ad_mode);
    Ok(value)
}

#[derive(Deserialize)]
struct AdminInput {
    #[serde(flatten)]
    config: AdConfig,
    /// New callback keys; a missing or empty value keeps the stored key.
    #[serde(default)]
    secrets: BTreeMap<String, String>,
    /// Networks whose stored key should be removed.
    #[serde(default)]
    clear_secrets: Vec<String>,
}

fn digits(value: &str, label: &str) -> Result<(), ApiError> {
    if value.len() > 32 || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(invalid(&format!("{label} 只能包含字母、数字、下划线或短横线，最多 32 位")));
    }
    Ok(())
}

fn validate(input: &mut AdminInput) -> Result<(), ApiError> {
    let cfg = &mut input.config;
    cfg.risk_window_hours = cfg.risk_window_hours.clamp(1, 24 * 30);
    for (name, label) in NETWORKS {
        let net = cfg.networks.entry(name.to_string()).or_default();
        net.app_id = net.app_id.trim().into();
        net.rewarded = net.rewarded.trim().into();
        net.feed = net.feed.trim().into();
        digits(&net.app_id, &format!("{label} AppID"))?;
        digits(&net.rewarded, &format!("{label} 激励视频代码位"))?;
        digits(&net.feed, &format!("{label} 信息流代码位"))?;
        if net.enabled && net.app_id.is_empty() {
            return Err(invalid(&format!("开启{label}前请填写 AppID")));
        }
        if net.enabled && net.rewarded.is_empty() && net.feed.is_empty() {
            return Err(invalid(&format!("开启{label}前请至少填写一个代码位")));
        }
    }
    cfg.networks.retain(|k, _| NETWORKS.iter().any(|(n, _)| n == k));
    for rule in [&mut cfg.rewarded, &mut cfg.feed] {
        if !["weighted", "ordered"].contains(&rule.mode.as_str()) {
            return Err(invalid("分配方式应为按概率或按顺序"));
        }
        rule.weights.retain(|k, _| NETWORKS.iter().any(|(n, _)| n == k));
        for w in rule.weights.values_mut() {
            *w = (*w).min(100);
        }
        let mut seen = Vec::new();
        for n in &rule.order {
            if NETWORKS.iter().any(|(id, _)| id == n) && !seen.contains(n) {
                seen.push(n.clone());
            }
        }
        for (n, _) in NETWORKS {
            if !seen.iter().any(|s| s == n) {
                seen.push(n.to_string());
            }
        }
        rule.order = seen;
    }
    for (name, key) in &mut input.secrets {
        *key = key.trim().to_owned();
        if !NETWORKS.iter().any(|(n, _)| n == name) || key.len() > 128 {
            return Err(invalid("回调密钥格式不正确"));
        }
    }
    Ok(())
}

async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(admin_json(&state, &open(&state)?)?))
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<AdminInput>,
) -> ApiResult<Value> {
    validate(&mut input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = admin_json(&state, &tx)?;
    tx.execute(
        "UPDATE ad_config SET json=?1,revision=revision+1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&input.config)?, now()],
    )?;
    for (name, key) in &input.secrets {
        if !key.is_empty() {
            tx.execute(
                "INSERT INTO ad_secrets(network,secret,updated_at) VALUES(?1,?2,?3) ON CONFLICT(network) DO UPDATE SET secret=excluded.secret,updated_at=excluded.updated_at",
                params![name, key, now()],
            )?;
        }
    }
    for name in &input.clear_secrets {
        tx.execute("DELETE FROM ad_secrets WHERE network=?1", [name])?;
    }
    let after = admin_json(&state, &tx)?;
    // Keys never go into the audit trail; only which networks changed.
    let changed: Vec<&String> = input.secrets.iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| k).collect();
    let mut audit_after = after.clone();
    audit_after["secrets_changed"] = json!(changed);
    audit_after["secrets_cleared"] = json!(input.clear_secrets);
    user_admin::action_audit(&tx, &context, None, "ad_config_update", "ad-config", "修改广告配置", before, audit_after)?;
    tx.commit()?;
    Ok(Json(after))
}

#[derive(Deserialize)]
struct StatsQuery {
    days: Option<i64>,
    format: Option<String>,
}

fn rate(num: i64, den: i64) -> Value {
    if den == 0 { Value::Null } else { json!(((num as f64) * 1000.0 / den as f64).round() / 10.0) }
}

fn rates(t: &BTreeMap<String, i64>) -> Value {
    let g = |k: &str| t.get(k).copied().unwrap_or(0);
    json!({
        "fill": rate(g("fill"), g("request")),
        "show": rate(g("show"), g("fill")),
        "click": rate(g("click"), g("show")),
        "callback": rate(g("callback_ok"), g("show")),
        "risk": rate(g("risk_block"), g("plan") + g("risk_block") + g("empty")),
    })
}

async fn admin_stats(State(state): State<AppState>, Query(q): Query<StatsQuery>) -> ApiResult<Value> {
    let days = q.days.unwrap_or(7).clamp(1, KEEP_DAYS);
    let format = q.format.filter(|f| FORMATS.iter().any(|(id, _)| id == f));
    let db = open(&state)?;
    let since = today_start() - (days - 1) * 86400;
    let mut stmt = db.prepare(
        "SELECT day,network,event,COUNT(*) FROM ad_events WHERE created_at>=?1 AND (?2 IS NULL OR format=?2) GROUP BY day,network,event",
    )?;
    let rows = stmt
        .query_map(params![since, format], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let day_list: Vec<String> = (0..days)
        .map(|i| {
            chrono::DateTime::from_timestamp(since + i * 86400 + 8 * 3600, 0)
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect();
    let mut overall: BTreeMap<String, i64> = BTreeMap::new();
    let mut per_net: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    let mut per_day: BTreeMap<(String, String), BTreeMap<String, i64>> = BTreeMap::new();
    for (d, net, event, count) in &rows {
        *overall.entry(event.clone()).or_default() += count;
        if net != "-" {
            *per_net.entry(net.clone()).or_default().entry(event.clone()).or_default() += count;
        }
        *per_day.entry((d.clone(), net.clone())).or_default().entry(event.clone()).or_default() += count;
    }
    let networks: Vec<Value> = NETWORKS
        .iter()
        .map(|(id, label)| {
            let totals = per_net.get(*id).cloned().unwrap_or_default();
            json!({"network":id,"label":label,"totals":totals,"rates":rates(&totals)})
        })
        .collect();
    let series: Vec<Value> = day_list
        .iter()
        .map(|d| {
            let mut entry = json!({"day":d});
            for (id, _) in NETWORKS {
                entry[*id] = json!(per_day.get(&(d.clone(), id.to_string())).cloned().unwrap_or_default());
            }
            let mut all: BTreeMap<String, i64> = BTreeMap::new();
            for ((dd, _), events) in &per_day {
                if dd == d {
                    for (e, c) in events {
                        *all.entry(e.clone()).or_default() += c;
                    }
                }
            }
            entry["all"] = json!(all);
            entry
        })
        .collect();
    let mut stmt = db.prepare(
        "SELECT network,format,event,error_code,message,COUNT(*) AS n FROM ad_events
         WHERE created_at>=?1 AND (?2 IS NULL OR format=?2) AND event IN ('error','no_fill','timeout','callback_reject','risk_block','empty')
         GROUP BY network,format,event,error_code,message ORDER BY n DESC LIMIT 20",
    )?;
    let errors = stmt
        .query_map(params![since, format], |r| {
            Ok(json!({"network":r.get::<_,String>(0)?,"format":r.get::<_,String>(1)?,"event":r.get::<_,String>(2)?,"code":r.get::<_,String>(3)?,"message":r.get::<_,String>(4)?,"count":r.get::<_,i64>(5)?}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let users: i64 = db.query_row(
        "SELECT COUNT(DISTINCT user_id) FROM ad_events WHERE created_at>=?1 AND (?2 IS NULL OR format=?2) AND event='show'",
        params![since, format],
        |r| r.get(0),
    )?;
    let rewarded_coins: i64 = db.query_row(
        "SELECT COALESCE(SUM(reward),0) FROM reward_sessions WHERE completed_at>=?1 AND verified_at IS NOT NULL",
        [since],
        |r| r.get(0),
    )?;
    Ok(Json(json!({
        "days": day_list, "format": format, "overall": {"totals": overall, "rates": rates(&overall)},
        "networks": networks, "series": series, "errors": errors, "show_users": users,
        "rewarded_coins": rewarded_coins,
        "formats": FORMATS.iter().map(|(id, l)| json!({"id":id,"label":l})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct EventsQuery {
    network: Option<String>,
    format: Option<String>,
    event: Option<String>,
    user_id: Option<i64>,
    session: Option<String>,
    page: Option<i64>,
}

async fn admin_events(State(state): State<AppState>, Query(q): Query<EventsQuery>) -> ApiResult<Value> {
    let db = open(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let filter = "(?1 IS NULL OR network=?1) AND (?2 IS NULL OR format=?2) AND (?3 IS NULL OR event=?3) AND (?4 IS NULL OR user_id=?4) AND (?5 IS NULL OR session_id=?5)";
    let empty = |v: Option<String>| v.filter(|s| !s.is_empty());
    let (network, format, event, session) = (empty(q.network), empty(q.format), empty(q.event), empty(q.session));
    let total: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM ad_events WHERE {filter}"),
        params![network, format, event, q.user_id, session],
        |r| r.get(0),
    )?;
    let mut stmt = db.prepare(&format!(
        "SELECT id,created_at,user_id,network,format,placement,event,source,session_id,error_code,message FROM ad_events WHERE {filter} ORDER BY id DESC LIMIT 50 OFFSET ?6"
    ))?;
    let items = stmt
        .query_map(params![network, format, event, q.user_id, session, (page - 1) * 50], |r| {
            Ok(json!({"id":r.get::<_,i64>(0)?,"created_at":iso(r.get(1)?),"user_id":r.get::<_,Option<i64>>(2)?,"network":r.get::<_,String>(3)?,"format":r.get::<_,String>(4)?,"placement":r.get::<_,String>(5)?,"event":r.get::<_,String>(6)?,"source":r.get::<_,String>(7)?,"session_id":r.get::<_,String>(8)?,"code":r.get::<_,String>(9)?,"message":r.get::<_,String>(10)?}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({"items":items,"total":total,"page":page,"page_size":50,
        "events": CLIENT_EVENTS.iter().chain(SERVER_EVENTS).collect::<Vec<_>>()})))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/ads/events", post(client_events))
        .route("/ads/callback/{network}", get(network_callback))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/ad-config", get(admin_get).put(admin_put))
        .route("/ad-stats", get(admin_stats))
        .route("/ad-events", get(admin_events))
}

// ---------- MD5 (快手 callback signature; no extra crate in the offline vendor set) ----------

pub(super) fn md5_hex(input: &str) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
        14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15,
        21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let mut msg = input.as_bytes().to_vec();
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|v| v.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_reference_vectors() {
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex("abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex("12345678901234567890123456789012345678901234567890123456789012345678901234567890"),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
        // 快手 doc sample shape: md5(appSecurityKey:transId), lowercase hex.
        assert_eq!(md5_hex("key:trans"), "8eaa98597c4f2426ae34d2b36898169e");
    }

    fn rule(mode: &str, weights: &[(&str, u32)]) -> FormatRule {
        FormatRule {
            mode: mode.into(),
            weights: weights.iter().map(|(n, w)| (n.to_string(), *w)).collect(),
            order: vec!["gdt".into(), "csj".into(), "kuaishou".into(), "baidu".into()],
        }
    }

    #[test]
    fn weighted_routing_follows_sliders_and_drops_zero_weight() {
        let all: Vec<String> = ["csj", "kuaishou", "baidu", "gdt"].iter().map(|s| s.to_string()).collect();
        let r = rule("weighted", &[("csj", 50), ("kuaishou", 30), ("baidu", 20), ("gdt", 0)]);
        let mut first = BTreeMap::new();
        for t in 0..100u64 {
            let o = order(&r, &all, t);
            assert_eq!(o.len(), 3, "zero weight network is never tried");
            *first.entry(o[0].clone()).or_insert(0) += 1;
        }
        assert_eq!(first["csj"], 50);
        assert_eq!(first["kuaishou"], 30);
        assert_eq!(first["baidu"], 20);
        // Fallbacks follow by weight.
        assert_eq!(order(&r, &all, 99), vec!["baidu", "csj", "kuaishou"]);
        // Only configured candidates participate.
        assert_eq!(order(&r, &["baidu".to_string()], 0), vec!["baidu"]);
        assert!(order(&r, &["gdt".to_string()], 0).is_empty());
        let o = order(&rule("ordered", &[]), &all, 7);
        assert_eq!(o, vec!["gdt", "csj", "kuaishou", "baidu"]);
    }

    #[test]
    fn admin_validation_requires_ids_before_enabling() {
        let mut input = AdminInput { config: AdConfig::default(), secrets: BTreeMap::new(), clear_secrets: vec![] };
        assert!(validate(&mut input).is_ok());
        input.config.networks.get_mut("baidu").unwrap().enabled = true;
        assert!(validate(&mut input).is_err());
        let net = input.config.networks.get_mut("baidu").unwrap();
        net.app_id = "d84da3ed".into();
        net.rewarded = "20552676".into();
        assert!(validate(&mut input).is_ok());
        input.config.networks.get_mut("baidu").unwrap().feed = "1 OR 1".into();
        assert!(validate(&mut input).is_err());
    }
}
