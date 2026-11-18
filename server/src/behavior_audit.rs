use super::*;
use axum::extract::DefaultBodyLimit;
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
    signature,
};

const HEADER: usize = 108;
const MAX_FRAME: usize = 65536;
const MAX_EVENTS: usize = 200;
const SPKI_PREFIX: &[u8] = &[
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];
const SCREENS: [&str; 15] = [
    "unknown",
    "hall",
    "shop",
    "mine",
    "packet",
    "reward_video",
    "product_detail",
    "withdrawal",
    "invite",
    "rain",
    "login",
    "register",
    "settings",
    "check_in",
    "reward_result",
];
const TYPES: [&str; 9] = [
    "unknown",
    "screen_enter",
    "screen_leave",
    "foreground",
    "background",
    "click",
    "gesture",
    "business",
    "heartbeat",
];
const CONTROLS: [&str; 18] = [
    "unknown",
    "red_packet",
    "check_in",
    "tab_hall",
    "tab_shop",
    "tab_mine",
    "product",
    "buy",
    "withdraw",
    "invite",
    "rain_packet",
    "reward_continue",
    "close",
    "login",
    "register",
    "settings",
    "hall_request_packet",
    "hall_request_product",
];
const ACTIONS: [&str; 13] = [
    "none",
    "reward_start",
    "reward_complete",
    "check_in",
    "rain_join",
    "rain_claim",
    "withdrawal",
    "product_open",
    "product_jump",
    "red_packet_open",
    "video_start",
    "video_close",
    "hall_request",
];

#[derive(Clone, Serialize, Deserialize)]
struct Weights {
    click_frequency: i64,
    regular_intervals: i64,
    short_dwell: i64,
    business_mismatch: i64,
    session_anomaly: i64,
}
#[derive(Clone, Serialize, Deserialize)]
struct Policy {
    enabled: bool,
    mode: String,
    #[serde(default)]
    revision: i64,
    session_ttl_seconds: i64,
    batch_interval_seconds: i64,
    touch_sample_percent: i64,
    click_window_seconds: i64,
    click_count_threshold: i64,
    min_interval_samples: i64,
    interval_tolerance_ms: i64,
    repeated_interval_ratio: f64,
    short_dwell_ms: i64,
    short_dwell_samples: i64,
    business_mismatch_threshold: i64,
    session_anomaly_threshold: i64,
    review_score: i64,
    hold_score: i64,
    weights: Weights,
    /// Where review_score/hold_score came from: DeviceTrust's live policy when available
    /// (read-only in the admin), otherwise the stored fallback.
    #[serde(default, skip_deserializing)]
    score_source: Value,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "observe".into(),
            revision: 1,
            session_ttl_seconds: 1800,
            batch_interval_seconds: 15,
            touch_sample_percent: 100,
            click_window_seconds: 10,
            click_count_threshold: 30,
            min_interval_samples: 12,
            interval_tolerance_ms: 40,
            repeated_interval_ratio: 0.9,
            short_dwell_ms: 800,
            short_dwell_samples: 12,
            business_mismatch_threshold: 5,
            session_anomaly_threshold: 3,
            review_score: 40,
            hold_score: 75,
            score_source: Value::Null,
            weights: Weights {
                click_frequency: 25,
                regular_intervals: 30,
                short_dwell: 20,
                business_mismatch: 25,
                session_anomaly: 20,
            },
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS behavior_policy(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS behavior_sessions(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,binding_event_id TEXT NOT NULL UNIQUE,key_id TEXT NOT NULL,device_id TEXT NOT NULL,proof_level TEXT NOT NULL,public_key BLOB NOT NULL,aes_key BLOB NOT NULL,nonce_prefix BLOB NOT NULL,challenge BLOB NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,last_received_at INTEGER,last_seq INTEGER NOT NULL DEFAULT 0,last_hash TEXT NOT NULL,last_elapsed_ms INTEGER NOT NULL DEFAULT 0,last_event_ms INTEGER NOT NULL DEFAULT 0,current_screen INTEGER NOT NULL DEFAULT 0,is_foreground INTEGER NOT NULL DEFAULT 0,viewport_width INTEGER NOT NULL DEFAULT 0,viewport_height INTEGER NOT NULL DEFAULT 0,event_count INTEGER NOT NULL DEFAULT 0,batch_count INTEGER NOT NULL DEFAULT 0,foreground_ms INTEGER NOT NULL DEFAULT 0,click_count INTEGER NOT NULL DEFAULT 0,dropped_events INTEGER NOT NULL DEFAULT 0,behavior_score INTEGER,assessment_json TEXT NOT NULL DEFAULT '{}',policy_revision INTEGER NOT NULL,last_assessed_at INTEGER);
      CREATE INDEX IF NOT EXISTS idx_behavior_sessions_user ON behavior_sessions(user_id,created_at DESC);
      CREATE TABLE IF NOT EXISTS behavior_batches(session_id TEXT NOT NULL,seq INTEGER NOT NULL,packet_hash TEXT NOT NULL,previous_hash TEXT NOT NULL,next_challenge BLOB NOT NULL,received_at INTEGER NOT NULL,event_count INTEGER NOT NULL,dropped_events INTEGER NOT NULL,first_elapsed_ms INTEGER NOT NULL,last_elapsed_ms INTEGER NOT NULL,PRIMARY KEY(session_id,seq));
      CREATE TABLE IF NOT EXISTS behavior_events(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,user_id INTEGER NOT NULL,batch_seq INTEGER NOT NULL,time_ms INTEGER NOT NULL,type INTEGER NOT NULL,screen INTEGER NOT NULL,control INTEGER NOT NULL,action INTEGER NOT NULL,x INTEGER,y INTEGER,end_x INTEGER,end_y INTEGER,duration_ms INTEGER NOT NULL,operation_ref TEXT NOT NULL,server_confirmed INTEGER NOT NULL,server_state TEXT NOT NULL,credited_coins INTEGER,credit_source TEXT,server_received_at INTEGER NOT NULL,dwell_ms INTEGER NOT NULL DEFAULT 0,dwell_screen INTEGER NOT NULL DEFAULT 0);
      CREATE INDEX IF NOT EXISTS idx_behavior_events_session ON behavior_events(session_id,time_ms,id);
      CREATE INDEX IF NOT EXISTS idx_behavior_events_user ON behavior_events(user_id,server_received_at);
      CREATE UNIQUE INDEX IF NOT EXISTS idx_behavior_credit_once ON behavior_events(user_id,credit_source) WHERE credited_coins>0;
      CREATE TABLE IF NOT EXISTS behavior_alerts(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,session_id TEXT NOT NULL,score INTEGER NOT NULL,level TEXT NOT NULL,action TEXT NOT NULL,policy_revision INTEGER NOT NULL,evidence_json TEXT NOT NULL,policy_json TEXT NOT NULL,created_at INTEGER NOT NULL);
      CREATE INDEX IF NOT EXISTS idx_behavior_alert_user ON behavior_alerts(user_id,created_at DESC);
      CREATE TABLE IF NOT EXISTS behavior_failures(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,session_id TEXT NOT NULL,stage TEXT NOT NULL,code TEXT NOT NULL,detail TEXT NOT NULL,created_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO behavior_policy VALUES(1,?1,?2)",
        params![serde_json::to_string(&Policy::default())?, now()],
    )?;
    Ok(())
}
/// DeviceTrust's current review/hold score lines, read from the newest verified event.
fn device_trust_scores(db: &Connection) -> Result<Option<(i64, i64, i64)>, ApiError> {
    let row: Option<(Option<i64>, Option<i64>, i64)> = db
        .query_row(
            "SELECT json_extract(detail_json,'$.policy.reviewScore'),json_extract(detail_json,'$.policy.holdScore'),updated_at
             FROM (SELECT detail_json,updated_at FROM risk_events WHERE status='reported' ORDER BY rowid DESC LIMIT 50)
             WHERE json_extract(detail_json,'$.policy.reviewScore') IS NOT NULL LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((Some(review), Some(hold), at)) if (1..=100).contains(&review) && review < hold && hold <= 100 => {
            Some((review, hold, at))
        }
        _ => None,
    })
}
/// The behaviour policy, with its score lines following DeviceTrust's platform policy so there
/// is one place (the DeviceTrust console) that defines them.
fn policy(db: &Connection) -> Result<Policy, ApiError> {
    let raw: String = db.query_row(
        "SELECT config_json FROM behavior_policy WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let mut p: Policy = serde_json::from_str(&raw)?;
    p.score_source = match device_trust_scores(db)? {
        Some((review, hold, at)) => {
            p.review_score = review;
            p.hold_score = hold;
            json!({"source":"DeviceTrust","review_score":review,"hold_score":hold,"synced_at":iso(at)})
        }
        None => json!({"source":"local"}),
    };
    Ok(p)
}
fn random<const N: usize>() -> Result<[u8; N], ApiError> {
    let mut bytes = [0; N];
    SystemRandom::new().fill(&mut bytes).map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "随机数生成失败".into(),
            None,
        )
    })?;
    Ok(bytes)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn public_key(body: &Value) -> Result<(Vec<u8>, Vec<u8>), ApiError> {
    let encoded = body["public_key"]
        .as_str()
        .ok_or_else(|| invalid("行为公钥缺失"))?;
    if encoded.len() > 256 {
        return Err(invalid("行为公钥格式错误"));
    }
    let der = STANDARD
        .decode(encoded)
        .map_err(|_| invalid("行为公钥格式错误"))?;
    if der.len() != 91 || !der.starts_with(SPKI_PREFIX) || der[26] != 4 {
        return Err(invalid("行为公钥应为P256 SPKI"));
    }
    Ok((der[26..].to_vec(), der))
}
pub(super) fn binding_summary(body: &Value) -> Result<Value, ApiError> {
    if body["consent"] != true {
        return Err(invalid("请先同意行为采集"));
    }
    let (_, der) = public_key(body)?;
    let nonce = body["client_nonce"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| invalid("行为会话随机标识缺失"))?;
    let text_field = |name: &str| -> Result<String, ApiError> {
        let value = body[name].as_str().unwrap_or("");
        if value.chars().count() > 128 || value.chars().any(|c| c.is_control()) {
            return Err(invalid("设备元信息格式无效"));
        }
        Ok(value.to_owned())
    };
    Ok(
        json!({"protocol":"OBA1","public_key_sha256":hex(&Sha256::digest(der)),"client_nonce":nonce.to_string(),"consent":true,"device_model":text_field("device_model")?,"os_version":text_field("os_version")?,"app_version":text_field("app_version")?}),
    )
}
async fn config(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    account_auth(&db, &headers)?;
    let p = policy(&db)?;
    Ok(Json(
        json!({"enabled":p.enabled,"protocol":"OBA1","batch_interval_seconds":p.batch_interval_seconds,"heartbeat_seconds":15,"touch_sample_percent":p.touch_sample_percent,"max_events":MAX_EVENTS,"session_ttl_seconds":p.session_ttl_seconds,"consent_required":true}),
    ))
}
fn disabled() -> ApiError {
    ApiError(
        StatusCode::CONFLICT,
        "当前采集服务暂不可用".into(),
        Some("BEHAVIOR_DISABLED"),
    )
}
async fn create_session(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Value> {
    let summary = binding_summary(&body)?;
    let (public, _) = public_key(&body)?;
    let id = headers
        .get("x-risk-event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| invalid("请先完成设备校验"))?;
    let mut db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let p = policy(&db)?;
    if !p.enabled {
        return Err(disabled());
    }
    if state.config.device_trust.is_none() {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "设备校验服务尚未配置".into(),
            None,
        ));
    }
    let intent = business_guard::Intent {
        method: "POST".into(),
        path: "/api/behavior/session".into(),
        body: summary,
    };
    let hash = business_guard::request_hash(&risk::account_id(&state, uid), &intent)?;
    let tx = db.transaction()?;
    let row:Option<(String,String,String,String,i64,Option<String>,String)>=tx.query_row("SELECT request_hash,key_id,device_identity,planned_operation,created_at,operation_ref,detail_json FROM risk_events WHERE id=?1 AND user_id=?2 AND kind='behavior' AND status='reported' AND binding_version=1",params![id,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
    let (stored, key, device, operation, created, consumed, detail) =
        row.ok_or_else(|| invalid("设备校验记录无效"))?;
    if hash != stored || key.is_empty() || device.is_empty() {
        return Err(invalid("设备校验与当前会话不匹配"));
    }
    let detail: Value = serde_json::from_str(&detail)?;
    let level = detail
        .pointer("/reportProof/level")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !["signature_only", "tee", "strongbox"].contains(&level) {
        return Err(invalid("设备签名校验记录缺失"));
    }
    let existing: Option<(String, Vec<u8>)> = tx
        .query_row(
            "SELECT id,public_key FROM behavior_sessions WHERE binding_event_id=?1 AND user_id=?2",
            params![id, uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((sid, prior_key)) = existing {
        if prior_key != public {
            return Err(conflict("行为会话已绑定其他公钥"));
        }
        let result = session_response(&tx, &sid, &p)?;
        tx.commit()?;
        return Ok(Json(result));
    }
    if now() - created > 600 || created > now() + 5 || consumed.is_some() {
        return Err(conflict("设备校验已过期或已使用"));
    }
    Uuid::parse_str(&operation).map_err(|_| invalid("行为会话标识无效"))?;
    let aes = random::<32>()?;
    let prefix = random::<4>()?;
    let challenge = random::<32>()?;
    let at = now();
    let changed=tx.execute("UPDATE risk_events SET operation_ref=?1 WHERE id=?2 AND user_id=?3 AND operation_ref IS NULL",params![operation,id,uid])?;
    if changed != 1 {
        return Err(conflict("设备校验已使用"));
    }
    tx.execute("INSERT INTO behavior_sessions(id,user_id,binding_event_id,key_id,device_id,proof_level,public_key,aes_key,nonce_prefix,challenge,created_at,expires_at,last_hash,policy_revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![operation,uid,id,key,device,level,public,aes.as_slice(),prefix.as_slice(),challenge.as_slice(),at,at+p.session_ttl_seconds,"0".repeat(64),p.revision])?;
    let model = body["device_model"].as_str().unwrap_or("");
    let os = body["os_version"].as_str().unwrap_or("");
    let app = body["app_version"].as_str().unwrap_or("");
    if !model.is_empty() || !os.is_empty() || !app.is_empty() {
        user_admin::record_device(
            &tx,
            uid,
            &user_admin::DeviceObservation {
                device_id: device.clone(),
                platform: "Android".into(),
                model: model.into(),
                os_version: format!("{os} / app {app}"),
                key_id: key.clone(),
                report_id: id.to_owned(),
                evidence_status: format!("{level};model_source=client_signed"),
            },
        )?;
    }
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "behavior_session",
        "success",
        &operation,
        None,
        json!({"binding_event_id":id,"protocol":"OBA1"}),
    )?;
    let result = session_response(&tx, &operation, &p)?;
    tx.commit()?;
    Ok(Json(result))
}
fn session_response(db: &Connection, id: &str, p: &Policy) -> Result<Value, ApiError> {
    let (key,prefix,challenge,expiry,seq,previous,elapsed):(Vec<u8>,Vec<u8>,Vec<u8>,i64,i64,String,i64)=db.query_row("SELECT aes_key,nonce_prefix,challenge,expires_at,last_seq,last_hash,last_elapsed_ms FROM behavior_sessions WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)))?;
    if expiry <= now() {
        return Err(conflict("行为会话已过期，请重新开始"));
    }
    Ok(
        json!({"session_id":id,"key_base64":STANDARD.encode(key),"nonce_prefix":STANDARD.encode(prefix),"challenge":STANDARD.encode(challenge),"expires_at":iso(expiry),"server_now":iso(now()),"protocol":"OBA1","batch_interval_seconds":p.batch_interval_seconds,"max_events":MAX_EVENTS,"max_elapsed_ms":(expiry-now()).max(0)*1000+elapsed,"next_seq":seq+1,"previous_hash":previous,"elapsed_offset_ms":elapsed}),
    )
}
#[derive(Clone)]
struct CapturedEvent {
    elapsed: i64,
    kind: u8,
    screen: u8,
    control: u8,
    action: u8,
    coords: [Option<u16>; 4],
    duration: i64,
    reference: String,
}
struct Payload {
    wall_ms: u64,
    width: u16,
    height: u16,
    dropped: u32,
    events: Vec<CapturedEvent>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ApiError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| invalid("行为包长度错误"))?;
        let value = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, ApiError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ApiError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, ApiError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, ApiError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
}
fn payload(bytes: &[u8]) -> Result<Payload, ApiError> {
    let mut r = Reader { bytes, pos: 0 };
    let wall_ms = r.u64()?;
    let width = r.u16()?;
    let height = r.u16()?;
    let dropped = r.u32()?;
    let count = r.u16()? as usize;
    if count > MAX_EVENTS || width == 0 || height == 0 || width > 10000 || height > 10000 {
        return Err(invalid("行为包采集范围错误"));
    }
    let mut events = Vec::with_capacity(count);
    let mut last = 0;
    for _ in 0..count {
        let elapsed = i64::from(r.u32()?);
        let kind = r.u8()?;
        let screen = r.u8()?;
        let control = r.u8()?;
        let action = r.u8()?;
        if elapsed < last
            || !(1..TYPES.len() as u8).contains(&kind)
            || screen as usize >= SCREENS.len()
            || control as usize >= CONTROLS.len()
            || action as usize >= ACTIONS.len()
        {
            return Err(invalid("行为包事件枚举或顺序错误"));
        }
        last = elapsed;
        let mut coords = [None; 4];
        for coordinate in &mut coords {
            let value = r.u16()?;
            if value != u16::MAX {
                if value > 10000 {
                    return Err(invalid("行为触点越界"));
                }
                *coordinate = Some(value);
            }
        }
        let duration = i64::from(r.u32()?);
        let len = r.u8()? as usize;
        if len > 96 {
            return Err(invalid("行为业务关联号过长"));
        }
        let reference = r.take(len)?;
        if !reference
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || b":_-".contains(c))
        {
            return Err(invalid("行为业务关联号格式错误"));
        }
        // Gestures carry their press time; screen_leave carries the client's dwell time (the
        // server still computes dwell itself). Every other event has no duration.
        if duration > elapsed || (kind != 6 && kind != 2 && duration != 0) {
            return Err(invalid("行为持续时间错误"));
        }
        events.push(CapturedEvent {
            elapsed,
            kind,
            screen,
            control,
            action,
            coords,
            duration,
            reference: String::from_utf8(reference.to_vec()).unwrap(),
        });
    }
    if r.pos != bytes.len() {
        return Err(invalid("行为包含未知尾部数据"));
    }
    Ok(Payload {
        wall_ms,
        width,
        height,
        dropped,
        events,
    })
}
struct Frame<'a> {
    session: String,
    seq: i64,
    challenge: &'a [u8],
    nonce: [u8; 12],
    previous: String,
    header: &'a [u8],
    cipher: &'a [u8],
    signed: &'a [u8],
    signature: &'a [u8],
    hash: String,
}
fn frame(bytes: &[u8]) -> Result<Frame<'_>, ApiError> {
    if bytes.len() > MAX_FRAME || bytes.len() < HEADER + 16 + 2 + 8 || &bytes[..4] != b"OBA1" {
        return Err(invalid("行为加密帧格式错误"));
    }
    let session = Uuid::from_slice(&bytes[4..20])
        .map_err(|_| invalid("行为会话标识格式错误"))?
        .to_string();
    let seq = u64::from_be_bytes(bytes[20..28].try_into().unwrap());
    if seq == 0 || seq > i64::MAX as u64 {
        return Err(invalid("行为批次序号错误"));
    }
    let length = u32::from_be_bytes(bytes[104..108].try_into().unwrap()) as usize;
    let end = HEADER
        .checked_add(length)
        .filter(|end| length >= 16 && *end + 2 <= bytes.len())
        .ok_or_else(|| invalid("行为加密帧长度错误"))?;
    let signature_len = u16::from_be_bytes(bytes[end..end + 2].try_into().unwrap()) as usize;
    if !(8..=80).contains(&signature_len) || end + 2 + signature_len != bytes.len() {
        return Err(invalid("行为签名长度错误"));
    }
    Ok(Frame {
        session,
        seq: seq as i64,
        challenge: &bytes[28..60],
        nonce: bytes[60..72].try_into().unwrap(),
        previous: hex(&bytes[72..104]),
        header: &bytes[..HEADER],
        cipher: &bytes[HEADER..end],
        signed: &bytes[..end],
        signature: &bytes[end + 2..],
        hash: hex(&Sha256::digest(&bytes[..end])),
    })
}
fn decode(f: &Frame<'_>, public: &[u8], key: &[u8]) -> Result<Payload, ApiError> {
    signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, public)
        .verify(f.signed, f.signature)
        .map_err(|_| invalid("行为P256签名校验失败"))?;
    let key =
        aead::UnboundKey::new(&aead::AES_256_GCM, key).map_err(|_| invalid("行为加密密钥无效"))?;
    let mut bytes = f.cipher.to_vec();
    let plaintext = aead::LessSafeKey::new(key)
        .open_in_place(
            aead::Nonce::assume_unique_for_key(f.nonce),
            aead::Aad::from(f.header),
            &mut bytes,
        )
        .map_err(|_| invalid("行为AES-GCM完整性校验失败"))?;
    payload(plaintext)
}
fn signed_error(message: &str, code: &'static str) -> ApiError {
    ApiError(StatusCode::CONFLICT, message.into(), Some(code))
}
fn acknowledge(sid: &str, seq: i64, hash: &str, challenge: &[u8], duplicate: bool) -> Value {
    json!({"accepted":true,"duplicate":duplicate,"session_id":sid,"seq":seq,"packet_hash":hash,"next_challenge":STANDARD.encode(challenge),"server_now":iso(now()),"status":"received"})
}
fn ledger_credit(
    db: &Connection,
    uid: i64,
    source: &str,
    since: i64,
) -> Result<Option<i64>, ApiError> {
    Ok(db
        .query_row(
            "SELECT coins FROM ledger WHERE user_id=?1 AND source=?2 AND created_at>=?3",
            params![uid, source, since],
            |r| r.get(0),
        )
        .optional()?)
}
fn reconcile(
    db: &Connection,
    uid: i64,
    event: &CapturedEvent,
    since: i64,
) -> Result<(bool, &'static str, Option<i64>, Option<String>), ApiError> {
    if event.reference.is_empty() || event.kind != 7 {
        return Ok((false, "client_reported", None, None));
    }
    let source = match event.action {
        1 | 2 | 10 | 11 => {
            let row:Option<(String,Option<i64>,i64)>=db.query_row("SELECT task_id,completed_at,started_at FROM reward_sessions WHERE id=?1 AND user_id=?2",params![event.reference,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
            let Some((task, completed, started)) = row else {
                return Ok((false, "unmatched", None, None));
            };
            if started < since {
                return Ok((false, "unmatched", None, None));
            }
            if event.action != 2 {
                return Ok((true, "created", None, None));
            }
            if completed.is_none() {
                return Ok((true, "pending", None, None));
            }
            if let Some(packet) = task.strip_prefix("rain:") {
                format!("rain:{packet}")
            } else if let Some(packet) = task.strip_prefix("newbie:") {
                format!("newbie:{packet}")
            } else if let Some(chance) = task.strip_prefix("wheel:") {
                format!("wheel:{chance}")
            } else {
                format!("reward:{}", event.reference)
            }
        }
        3 | 5 => {
            if event.action == 5 {
                let packet = event
                    .reference
                    .strip_prefix("rain:")
                    .unwrap_or(&event.reference);
                let status: Option<String> = db
                    .query_row(
                        "SELECT status FROM rain_packets WHERE id=?1 AND user_id=?2",
                        params![packet, uid],
                        |r| r.get(0),
                    )
                    .optional()?;
                if status.as_deref().is_some_and(|s| s != "claimed") {
                    return Ok((true, "pending", None, None));
                }
                format!("rain:{packet}")
            } else {
                event.reference.clone()
            }
        }
        4 => {
            let found:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM rain_joins WHERE user_id=?1 AND round_id=?2 AND created_at>=?3)",params![uid,event.reference,since],|r|r.get(0))?;
            return Ok((
                found,
                if found { "created" } else { "unmatched" },
                None,
                None,
            ));
        }
        6 => {
            let found:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM withdrawals WHERE user_id=?1 AND id=?2 AND created_at>=?3)",params![uid,event.reference,since],|r|r.get(0))?;
            return Ok((
                found,
                if found { "created" } else { "unmatched" },
                None,
                None,
            ));
        }
        _ => return Ok((false, "client_reported", None, None)),
    };
    let Some(coins) = ledger_credit(db, uid, &source, since)? else {
        return Ok((false, "unmatched", None, None));
    };
    if coins <= 0 {
        return Ok((true, "created", None, None));
    }
    let already:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM behavior_events WHERE user_id=?1 AND credit_source=?2 AND credited_coins>0)",params![uid,source],|r|r.get(0))?;
    Ok((
        true,
        "credited",
        Some(if already { 0 } else { coins }),
        Some(source),
    ))
}
fn process(
    state: &AppState,
    context: &RequestContext,
    uid: i64,
    bytes: &[u8],
) -> Result<Value, ApiError> {
    let f = frame(bytes)?;
    let mut db = open(state)?;
    let p = policy(&db)?;
    if !p.enabled {
        return Err(disabled());
    }
    let tx = db.transaction()?;
    let row:Option<(Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,i64,i64,i64,String,i64,i64,u8,bool)>=tx.query_row("SELECT public_key,aes_key,nonce_prefix,challenge,created_at,expires_at,last_seq,last_hash,last_elapsed_ms,last_event_ms,current_screen,is_foreground FROM behavior_sessions WHERE id=?1 AND user_id=?2",params![f.session,uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional()?;
    let (
        public,
        key,
        prefix,
        challenge,
        created,
        expires,
        last_seq,
        last_hash,
        last_elapsed,
        mut event_time,
        mut screen,
        mut foreground,
    ) = row.ok_or_else(|| invalid("行为会话账号归属不匹配"))?;
    if expires <= now() {
        return Err(conflict("行为会话已过期"));
    }
    let body = decode(&f, &public, &key)?;
    let duplicate:Option<(String,Vec<u8>)>=tx.query_row("SELECT packet_hash,next_challenge FROM behavior_batches WHERE session_id=?1 AND seq=?2",params![f.session,f.seq],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((hash, next)) = duplicate {
        if hash != f.hash {
            return Err(signed_error("同序号行为包内容冲突", "signed_sequence"));
        }
        return Ok(acknowledge(&f.session, f.seq, &hash, &next, true));
    }
    if f.seq != last_seq + 1 {
        return Err(signed_error("行为序号未连续", "signed_sequence"));
    }
    if f.previous != last_hash || f.challenge != challenge {
        return Err(signed_error("行为批次链或挑战不匹配", "signed_chain"));
    }
    if prefix.len() != 4 || f.nonce[..4] != prefix || f.nonce[4..] != (f.seq as u64).to_be_bytes() {
        return Err(signed_error("行为随机数与序号不匹配", "signed_sequence"));
    }
    let first = body
        .events
        .first()
        .map(|e| e.elapsed)
        .unwrap_or(last_elapsed);
    let last = body
        .events
        .last()
        .map(|e| e.elapsed)
        .unwrap_or(last_elapsed);
    if first < last_elapsed
        || last > (now() - created) * 1000 + 5000
        || last > (expires - created) * 1000
    {
        return Err(signed_error("行为单调时钟超出服务端窗口", "signed_time"));
    }
    let at = now();
    let mut dwell = 0;
    let mut clicks = 0;
    for (index, event) in body.events.iter().enumerate() {
        let delta = event.elapsed - event_time;
        let tracked = if foreground
            && screen > 0
            && delta <= (p.batch_interval_seconds * 3000 + 5000).max(45000)
        {
            delta
        } else {
            0
        };
        dwell += tracked;
        let recorded_screen = if event.kind == 2 {
            screen
        } else {
            event.screen
        };
        let (confirmed, state, coins, source) = reconcile(&tx, uid, event, created - 30)?;
        tx.execute("INSERT INTO behavior_events(id,session_id,user_id,batch_seq,time_ms,type,screen,control,action,x,y,end_x,end_y,duration_ms,operation_ref,server_confirmed,server_state,credited_coins,credit_source,server_received_at,dwell_ms,dwell_screen) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",params![format!("{}:{}:{index:03}",f.session,f.seq),f.session,uid,f.seq,event.elapsed,event.kind,recorded_screen,event.control,event.action,event.coords[0],event.coords[1],event.coords[2],event.coords[3],event.duration,event.reference,confirmed,state,coins,source,at,tracked,screen])?;
        if event.kind == 5 {
            clicks += 1;
        }
        match event.kind {
            1 => screen = event.screen,
            2 => screen = 0,
            3 => {
                foreground = true;
                screen = event.screen;
            }
            4 => foreground = false,
            _ => {}
        }
        event_time = event.elapsed;
    }
    let next = random::<32>()?;
    tx.execute(
        "INSERT INTO behavior_batches VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            f.session,
            f.seq,
            f.hash,
            f.previous,
            next.as_slice(),
            at,
            body.events.len() as i64,
            body.dropped,
            first,
            last
        ],
    )?;
    tx.execute("UPDATE behavior_sessions SET last_received_at=?1,last_seq=?2,last_hash=?3,challenge=?4,last_elapsed_ms=?5,last_event_ms=?6,current_screen=?7,is_foreground=?8,viewport_width=?9,viewport_height=?10,event_count=event_count+?11,batch_count=batch_count+1,foreground_ms=foreground_ms+?12,click_count=click_count+?13,dropped_events=dropped_events+?14 WHERE id=?15",params![at,f.seq,f.hash,next.as_slice(),last,event_time,screen,foreground,body.width,body.height,body.events.len()as i64,dwell,clicks,body.dropped,f.session])?;
    let _ = body.wall_ms; // 客户端墙钟不参与发币、排序、停留或有效期判断。
    if body
        .events
        .iter()
        .any(|event| [1, 3, 5, 6, 7].contains(&event.kind))
    {
        user_admin::behavior(
            &tx,
            context,
            Some(uid),
            None,
            "app_interaction",
            "success",
            &f.session,
            None,
            json!({"event_count":body.events.len(),"seq":f.seq}),
        )?;
    }
    assess(&tx, uid, &f.session, &p, context)?;
    tx.commit()?;
    Ok(acknowledge(&f.session, f.seq, &f.hash, &next, false))
}
fn failure_record(
    db: &Connection,
    uid: i64,
    sid: &str,
    stage: &str,
    code: &str,
    detail: &str,
) -> Result<String, ApiError> {
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO behavior_failures VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, uid, sid, stage, code, detail, now()],
    )?;
    Ok(id)
}
async fn batches(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    bytes: axum::body::Bytes,
) -> ApiResult<Value> {
    let uid = account_auth(&open(&state)?, &headers)?;
    match process(&state, &context, uid, &bytes) {
        Ok(value) => Ok(Json(value)),
        Err(error) => {
            let sid = frame(&bytes).map(|f| f.session).unwrap_or_default();
            let mut db = open(&state)?;
            let tx = db.transaction()?;
            let trace = failure_record(
                &tx,
                uid,
                &sid,
                "server_validation",
                error.2.unwrap_or("invalid_frame"),
                &error.1,
            )?;
            if error.2.is_some_and(|code| code.starts_with("signed_")) {
                let owned: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM behavior_sessions WHERE id=?1 AND user_id=?2)",
                    params![sid, uid],
                    |r| r.get(0),
                )?;
                if owned {
                    assess(&tx, uid, &sid, &policy(&tx)?, &context)?;
                }
            }
            tx.commit()?;
            Err(ApiError(
                error.0,
                format!("数据提交未完成，请稍后重试。参考号：{trace}"),
                Some("BEHAVIOR_REJECTED"),
            ))
        }
    }
}
#[derive(Deserialize)]
struct FailureInput {
    #[serde(default)]
    session_id: String,
    stage: String,
    code: String,
    #[serde(default)]
    detail: String,
}
async fn client_failure(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FailureInput>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    if !["session", "encode", "encrypt", "upload"].contains(&input.stage.as_str())
        || input.code.len() > 80
        || input.detail.len() > 1024
    {
        return Err(invalid("行为诊断字段无效"));
    }
    let mut detail = input.detail.replace(token(&headers)?, "[redacted]");
    if !input.session_id.is_empty() {
        let secret: Option<Vec<u8>> = db
            .query_row(
                "SELECT aes_key FROM behavior_sessions WHERE id=?1 AND user_id=?2",
                params![input.session_id, uid],
                |r| r.get(0),
            )
            .optional()?;
        let secret = secret.ok_or_else(|| invalid("行为会话不存在"))?;
        detail = detail.replace(&STANDARD.encode(secret), "[redacted]");
    }
    let id = failure_record(
        &db,
        uid,
        &input.session_id,
        &input.stage,
        &format!("client:{}", input.code),
        &detail,
    )?;
    Ok(Json(json!({"reference":id})))
}
fn clicks(
    db: &Connection,
    uid: i64,
    sid: Option<&str>,
    since: i64,
) -> Result<Vec<(String, i64)>, ApiError> {
    Ok(db.prepare("SELECT session_id,time_ms FROM behavior_events WHERE user_id=?1 AND type=5 AND (?2 IS NULL OR session_id=?2) AND server_received_at>=?3 ORDER BY session_id,time_ms,id")?.query_map(params![uid,sid,since],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<Result<Vec<_>,_>>()?)
}
fn intervals(values: &[(String, i64)]) -> Vec<i64> {
    values
        .windows(2)
        .filter(|pair| pair[0].0 == pair[1].0)
        .map(|pair| pair[1].1 - pair[0].1)
        .collect()
}
fn level(score: Option<i64>, p: &Policy) -> &'static str {
    match score {
        None => "unknown",
        Some(s) if s >= p.hold_score => "high",
        Some(s) if s >= p.review_score => "medium",
        _ => "low",
    }
}
fn assess(
    db: &Connection,
    uid: i64,
    sid: &str,
    p: &Policy,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let click = clicks(db, uid, Some(sid), 0)?;
    let differences = intervals(&click);
    let mut peak = 0;
    let mut left = 0;
    for right in 0..click.len() {
        while click[right].1 - click[left].1 > p.click_window_seconds * 1000 {
            left += 1;
        }
        peak = peak.max(right - left + 1);
    }
    let mut visits = Vec::new();
    let mut entered: Option<i64> = None;
    let events=db.prepare("SELECT time_ms,type,screen FROM behavior_events WHERE session_id=?1 ORDER BY time_ms,id")?.query_map([sid],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,u8>(1)?,r.get::<_,u8>(2)?)))?.collect::<Result<Vec<_>,_>>()?;
    for (time, kind, screen) in events {
        match kind {
            1 => {
                if let Some(start) = entered {
                    visits.push(time - start);
                }
                entered = Some(time);
            }
            2 | 4 => {
                if let Some(start) = entered.take() {
                    visits.push(time - start);
                }
            }
            3 => {
                if screen > 0 {
                    entered = Some(time);
                }
            }
            _ => {}
        }
    }
    let short = visits
        .iter()
        .filter(|duration| **duration < p.short_dwell_ms)
        .count();
    let mismatch:i64=db.query_row("SELECT COUNT(*) FROM behavior_events WHERE session_id=?1 AND type=7 AND action BETWEEN 1 AND 6 AND server_state='unmatched' AND operation_ref!=''",[sid],|r|r.get(0))?;
    let anomalies:i64=db.query_row("SELECT COUNT(*) FROM behavior_failures WHERE session_id=?1 AND user_id=?2 AND code IN ('signed_time','signed_sequence','signed_chain')",params![sid,uid],|r|r.get(0))?;
    let mut repeated = 0;
    for candidate in &differences {
        let count = differences
            .iter()
            .filter(|d| (**d - *candidate).abs() <= p.interval_tolerance_ms)
            .count();
        repeated = repeated.max(count);
    }
    let enough = click.len() as i64 >= p.min_interval_samples + 1
        || visits.len() as i64 >= p.short_dwell_samples
        || mismatch >= p.business_mismatch_threshold
        || anomalies >= p.session_anomaly_threshold
        || peak as i64 >= p.click_count_threshold;
    let mut evidence = Vec::new();
    for (hit, code, label, points, observed, threshold) in [
        (
            peak as i64 >= p.click_count_threshold,
            "click_frequency",
            "短时点击频率",
            p.weights.click_frequency,
            peak as i64,
            p.click_count_threshold,
        ),
        (
            differences.len() as i64 >= p.min_interval_samples
                && repeated as f64 / differences.len().max(1) as f64 >= p.repeated_interval_ratio,
            "regular_intervals",
            "点击间隔重复",
            p.weights.regular_intervals,
            repeated as i64,
            (differences.len() as f64 * p.repeated_interval_ratio).ceil() as i64,
        ),
        (
            visits.len() as i64 >= p.short_dwell_samples && short as i64 >= p.short_dwell_samples,
            "short_dwell",
            "连续短页面停留",
            p.weights.short_dwell,
            short as i64,
            p.short_dwell_samples,
        ),
        (
            mismatch >= p.business_mismatch_threshold,
            "business_mismatch",
            "业务关联待核对",
            p.weights.business_mismatch,
            mismatch,
            p.business_mismatch_threshold,
        ),
        (
            anomalies >= p.session_anomaly_threshold,
            "session_anomaly",
            "已签名批次时序冲突",
            p.weights.session_anomaly,
            anomalies,
            p.session_anomaly_threshold,
        ),
    ] {
        if hit {
            evidence.push(json!({"code":code,"label":label,"source":"behavior","points":points,"observed":observed,"threshold":threshold,"version":p.revision,"assessed_at":iso(now())}));
        }
    }
    let score = if enough {
        Some(
            evidence
                .iter()
                .map(|e| e["points"].as_i64().unwrap())
                .sum::<i64>()
                .min(100),
        )
    } else {
        None
    };
    let assessment = json!({"evidence":evidence,"samples":{"clicks":click.len(),"intervals":differences.len(),"page_visits":visits.len(),"business_unmatched":mismatch,"signed_anomalies":anomalies},"enough_samples":enough,"policy":p});
    db.execute("UPDATE behavior_sessions SET behavior_score=?1,assessment_json=?2,policy_revision=?3,last_assessed_at=?4 WHERE id=?5",params![score,assessment.to_string(),p.revision,now(),sid])?;
    if evidence.is_empty() {
        return Ok(());
    }
    let score = score.unwrap_or(0);
    let action = if p.mode == "hold" && score >= p.hold_score {
        "hold"
    } else if p.mode != "observe" && score >= p.review_score {
        "review"
    } else {
        "observe"
    };
    let duplicate:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM behavior_alerts WHERE session_id=?1 AND policy_revision=?2 AND score=?3 AND action=?4)",params![sid,p.revision,score,action],|r|r.get(0))?;
    if duplicate {
        return Ok(());
    }
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO behavior_alerts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            id,
            uid,
            sid,
            score,
            level(Some(score), p),
            action,
            p.revision,
            serde_json::to_string(&evidence)?,
            serde_json::to_string(p)?,
            now()
        ],
    )?;
    if action != "observe" && trusted_override(db, uid)?.is_none() {
        let status = if action == "hold" {
            "restricted"
        } else {
            "review"
        };
        let before: Option<String> = db
            .query_row(
                "SELECT status FROM account_restrictions WHERE user_id=?1",
                [uid],
                |r| r.get(0),
            )
            .optional()?;
        let changed=db.execute("INSERT INTO account_restrictions(user_id,status,reason,updated_at,updated_by,trace_id,risk_event_id) VALUES(?1,?2,?3,?4,'BehaviorAudit',?5,'') ON CONFLICT(user_id) DO UPDATE SET status=CASE WHEN account_restrictions.status='restricted' THEN 'restricted' ELSE excluded.status END,reason=excluded.reason,updated_at=excluded.updated_at,updated_by=excluded.updated_by,trace_id=excluded.trace_id,risk_event_id='' WHERE account_restrictions.updated_by IN ('DeviceTrust','BehaviorAudit')",params![uid,status,format!("行为审计评估：{}",evidence.iter().filter_map(|e|e["label"].as_str()).collect::<Vec<_>>().join("、")),now(),id])?;
        if changed > 0 {
            let mut actor = context.clone();
            actor.admin_username = Some("BehaviorAudit".into());
            user_admin::action_audit(
                db,
                &actor,
                Some(uid),
                "behavior_restriction",
                &id,
                "行为审计策略处置",
                json!({"status":before}),
                json!({"status":status,"score":score,"policy_revision":p.revision,"alert_id":id}),
            )?;
        }
    }
    Ok(())
}
pub(super) fn risk_summary(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let p = policy(db)?;
    let sdk:Option<(i64,i64,String)>=db.query_row("SELECT score,updated_at,detail_json FROM risk_events WHERE user_id=?1 AND status='reported' AND score IS NOT NULL ORDER BY updated_at DESC,id DESC LIMIT 1",[uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let behavior:Option<(i64,i64,i64,String)>=db.query_row("SELECT behavior_score,last_assessed_at,policy_revision,assessment_json FROM behavior_sessions WHERE user_id=?1 AND behavior_score IS NOT NULL ORDER BY last_assessed_at DESC,id DESC LIMIT 1",[uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let sdk_score = sdk.as_ref().map(|s| s.0);
    let behavior_score = behavior.as_ref().map(|s| s.0);
    let score = sdk_score.into_iter().chain(behavior_score).max();
    let mut factors = Vec::new();
    let mut assessed = None;
    if let Some((score, at, raw)) = sdk {
        let detail: Value = serde_json::from_str(&raw)?;
        factors.push(json!({"code":"sdk_assessment","label":"设备证明评估","source":"DeviceTrust","score":score,"version":detail["modelVersion"],"assessed_at":iso(at),"hits":detail["hits"]}));
        assessed = Some(at);
    }
    let mut revision = p.revision;
    if let Some((_, at, version, raw)) = behavior {
        let detail: Value = serde_json::from_str(&raw)?;
        if let Some(evidence) = detail["evidence"].as_array() {
            factors.extend(evidence.iter().cloned());
        }
        assessed = Some(assessed.map_or(at, |old| old.max(at)));
        revision = version;
    }
    Ok(
        json!({"score":score,"level":level(score,&p),"sdk_score":sdk_score,"behavior_score":behavior_score,"evidence_count":factors.len(),"last_assessed_at":assessed.map(iso),"policy_revision":revision,"factors":factors}),
    )
}

#[derive(Deserialize, Default)]
struct PageQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    days: Option<i64>,
}
fn page(q: &PageQuery) -> (i64, i64) {
    (
        q.page.unwrap_or(1).max(1),
        q.page_size.unwrap_or(20).clamp(1, 100),
    )
}
async fn admin_policy(State(state): State<AppState>) -> ApiResult<Policy> {
    Ok(Json(policy(&open(&state)?)?))
}
async fn update_policy(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<Policy>,
) -> ApiResult<Policy> {
    if !["observe", "review", "hold"].contains(&input.mode.as_str())
        || !(0..=100).contains(&input.touch_sample_percent)
        || !(5..=300).contains(&input.batch_interval_seconds)
        || !(60..=86400).contains(&input.session_ttl_seconds)
        || input.review_score < 1
        || input.hold_score <= input.review_score
        || input.hold_score > 100
    {
        return Err(invalid("行为审计策略范围无效"));
    }
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = policy(&tx)?;
    input.revision = before.revision + 1;
    tx.execute(
        "UPDATE behavior_policy SET config_json=?1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&input)?, now()],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        "behavior_policy_update",
        "behavior-policy",
        "修改行为审计策略",
        serde_json::to_value(before)?,
        serde_json::to_value(&input)?,
    )?;
    tx.commit()?;
    // Re-read so the response carries the DeviceTrust-synced score lines and their source.
    Ok(Json(policy(&open(&state)?)?))
}
async fn alerts(State(state): State<AppState>, Query(q): Query<PageQuery>) -> ApiResult<Value> {
    let db = open(&state)?;
    let (page, size) = page(&q);
    let offset = (page - 1) * size;
    let total: i64 = db.query_row("SELECT COUNT(*) FROM behavior_alerts", [], |r| r.get(0))?;
    let rows=db.prepare("SELECT id,user_id,session_id,score,level,action,policy_revision,evidence_json,created_at FROM behavior_alerts ORDER BY created_at DESC,id DESC LIMIT ?1 OFFSET ?2")?.query_map(params![size,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"user_id":r.get::<_,i64>(1)?,"session_id":r.get::<_,String>(2)?,"score":r.get::<_,i64>(3)?,"level":r.get::<_,String>(4)?,"action":r.get::<_,String>(5)?,"policy_revision":r.get::<_,i64>(6)?,"evidence":serde_json::from_str::<Value>(&r.get::<_,String>(7)?).unwrap_or(json!([])),"created_at":iso(r.get(8)?)})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(
        json!({"items":rows,"total":total,"page":page,"page_size":size}),
    ))
}
async fn profile(
    State(state): State<AppState>,
    Path(uid): Path<i64>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let days = q.days.unwrap_or(30);
    if ![7, 30, 90].contains(&days) {
        return Err(invalid("days应为7、30或90"));
    }
    let since = now() - days * 86400;
    let sessions: i64 = db.query_row(
        "SELECT COUNT(*) FROM behavior_sessions WHERE user_id=?1 AND created_at>=?2",
        params![uid, since],
        |r| r.get(0),
    )?;
    let event_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND server_received_at>=?2",
        params![uid, since],
        |r| r.get(0),
    )?;
    let click_count:i64=db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND type=5 AND server_received_at>=?2",params![uid,since],|r|r.get(0))?;
    let values = intervals(&clicks(&db, uid, None, since)?);
    let risk = risk_summary(&db, uid)?;
    Ok(Json(
        json!({"collection":{"status":if sessions==0{"unknown"}else{"available"},"since":iso(since),"session_count":sessions,"missing":[],"notes":[]},"summary":{"foreground_ms":db.query_row("SELECT COALESCE(SUM(foreground_ms),0) FROM behavior_sessions WHERE user_id=?1 AND created_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"event_count":event_count,"click_count":click_count,"gesture_count":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND type=6 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"red_packet_opens":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND action=9 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"ad_starts":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND action=10 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"ad_completions":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND action=2 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"claimed_coins":db.query_row("SELECT COALESCE(SUM(credited_coins),0) FROM behavior_events WHERE user_id=?1 AND credited_coins>0 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"claimed_count":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND credited_coins>0 AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"unmatched_business_count":db.query_row("SELECT COUNT(*) FROM behavior_events WHERE user_id=?1 AND server_state='unmatched' AND server_received_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?,"dropped_events":db.query_row("SELECT COALESCE(SUM(dropped_events),0) FROM behavior_sessions WHERE user_id=?1 AND created_at>=?2",params![uid,since],|r|r.get::<_,i64>(0))?},"click_intervals":{"sample_count":values.len(),"median_ms":values.get(values.len()/2),"p95_ms":values.get((values.len()*95/100).min(values.len().saturating_sub(1)))},"pages":[],"daily":[],"risk":risk,"business_reconciliation":[],"sessions":sessions,"missing":[]}),
    ))
}
async fn sessions(
    State(state): State<AppState>,
    Path(uid): Path<i64>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let (page, size) = page(&q);
    let offset = (page - 1) * size;
    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM behavior_sessions WHERE user_id=?1",
        [uid],
        |r| r.get(0),
    )?;
    let rows=db.prepare("SELECT id,started_at,last_received_at,expires_at,key_id,device_id,proof_level,event_count,batch_count,foreground_ms,click_count,dropped_events,behavior_score,last_assessed_at,policy_revision FROM behavior_sessions WHERE user_id=?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3")?.query_map(params![uid,size,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"started_at":iso(r.get(1)?),"last_received_at":r.get::<_,Option<i64>>(2)?.map(iso),"expires_at":iso(r.get(3)?),"key_id":r.get::<_,String>(4)?,"device_id":r.get::<_,String>(5)?,"proof_level":r.get::<_,String>(6)?,"event_count":r.get::<_,i64>(7)?,"batch_count":r.get::<_,i64>(8)?,"foreground_ms":r.get::<_,i64>(9)?,"click_count":r.get::<_,i64>(10)?,"dropped_events":r.get::<_,i64>(11)?,"behavior_score":r.get::<_,Option<i64>>(12)?,"last_assessed_at":r.get::<_,Option<i64>>(13)?.map(iso),"policy_revision":r.get::<_,i64>(14)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(
        json!({"items":rows,"total":total,"page":page,"page_size":size}),
    ))
}
async fn session_detail(
    State(state): State<AppState>,
    Path((uid, sid)): Path<(i64, String)>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Value> {
    let db = open(&state)?;
    let limit = q.page_size.unwrap_or(1000).clamp(1, 5000);
    let offset = q.page.unwrap_or(0).max(0);
    let owner: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM behavior_sessions WHERE id=?1 AND user_id=?2)",
        params![sid, uid],
        |r| r.get(0),
    )?;
    if !owner {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "行为会话不存在".into(),
            None,
        ));
    }
    let events=db.prepare("SELECT id,time_ms,type,screen,control,action,x,y,end_x,end_y,duration_ms,operation_ref,server_confirmed,server_state,credited_coins,server_received_at,batch_seq FROM behavior_events WHERE session_id=?1 ORDER BY time_ms,id LIMIT ?2 OFFSET ?3")?.query_map(params![sid,limit,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"time_ms":r.get::<_,i64>(1)?,"type":TYPES.get(r.get::<_,u8>(2)? as usize).unwrap_or(&"unknown"),"screen":SCREENS.get(r.get::<_,u8>(3)? as usize).unwrap_or(&"unknown"),"control_id":CONTROLS.get(r.get::<_,u8>(4)? as usize).unwrap_or(&"unknown"),"action":ACTIONS.get(r.get::<_,u8>(5)? as usize).unwrap_or(&"none"),"x":r.get::<_,Option<u16>>(6)?,"y":r.get::<_,Option<u16>>(7)?,"end_x":r.get::<_,Option<u16>>(8)?,"end_y":r.get::<_,Option<u16>>(9)?,"duration_ms":r.get::<_,i64>(10)?,"operation_ref":r.get::<_,String>(11)?,"server_confirmed":r.get::<_,bool>(12)?,"server_state":r.get::<_,String>(13)?,"credited_coins":r.get::<_,Option<i64>>(14)?,"server_received_at":iso(r.get(15)?),"batch_seq":r.get::<_,i64>(16)?})))?.collect::<Result<Vec<_>,_>>()?;
    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM behavior_events WHERE session_id=?1",
        [sid.as_str()],
        |r| r.get(0),
    )?;
    Ok(Json(
        json!({"session":{"id":sid,"protocol":"OBA1"},"events":events,"events_total":total,"next_offset":if offset+limit<total{json!(offset+limit)}else{Value::Null},"metadata":{"availability":"available"}}),
    ))
}
pub(super) fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/behavior/config", get(config))
        .route("/behavior/session", post(create_session))
        .route("/behavior/batch", post(batches))
        .route("/behavior/batches", post(batches))
        .route("/behavior/failure", post(client_failure))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/behavior/policy", get(admin_policy).put(update_policy))
        .route("/behavior/alerts", get(alerts))
        .route("/users/{id}/behavior/profile", get(profile))
        .route("/users/{id}/behavior/sessions", get(sessions))
        .route("/users/{id}/behavior/sessions/{sid}", get(session_detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vector() -> Value {
        serde_json::from_str(include_str!("behavior-vector-v1.json")).unwrap()
    }
    #[test]
    fn shared_oba1_vector_verifies_signature_and_aes_gcm_payload() {
        let value = vector();
        let bytes = STANDARD
            .decode(value["frame_base64"].as_str().unwrap())
            .unwrap();
        let f = frame(&bytes).unwrap();
        let public = STANDARD
            .decode(value["public_key"].as_str().unwrap())
            .unwrap();
        let key = STANDARD
            .decode(value["key_base64"].as_str().unwrap())
            .unwrap();
        let parsed = decode(&f, &public[26..], &key).unwrap();
        assert_eq!(parsed.events.len(), 3);
        assert_eq!(parsed.events[1].coords[0], Some(2500));
        assert_eq!(parsed.events[1].coords[1], Some(5000));
        assert_eq!(f.hash, value["packet_hash"]);
        assert_eq!(f.seq, 1);
    }
    #[test]
    fn signature_or_cipher_mutation_is_rejected() {
        let value = vector();
        let mut bytes = STANDARD
            .decode(value["frame_base64"].as_str().unwrap())
            .unwrap();
        let public = STANDARD
            .decode(value["public_key"].as_str().unwrap())
            .unwrap();
        let key = STANDARD
            .decode(value["key_base64"].as_str().unwrap())
            .unwrap();
        bytes[150] ^= 1;
        let f = frame(&bytes).unwrap();
        assert!(decode(&f, &public[26..], &key).is_err());
    }
    #[test]
    fn score_lines_follow_device_trust_policy_when_known() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE behavior_policy(id INTEGER PRIMARY KEY,config_json TEXT NOT NULL,updated_at INTEGER);
            CREATE TABLE risk_events(id TEXT,status TEXT,detail_json TEXT,updated_at INTEGER);").unwrap();
        db.execute("INSERT INTO behavior_policy VALUES(1,?1,0)", [serde_json::to_string(&Policy::default()).unwrap()]).unwrap();
        let p = policy(&db).unwrap();
        assert_eq!((p.review_score, p.hold_score), (40, 75));
        assert_eq!(p.score_source["source"], "local");
        db.execute("INSERT INTO risk_events VALUES('a','reported',?1,100)", [json!({"policy":{"reviewScore":30,"holdScore":70}}).to_string()]).unwrap();
        db.execute("INSERT INTO risk_events VALUES('b','prepared','{}',200)", []).unwrap();
        let p = policy(&db).unwrap();
        assert_eq!((p.review_score, p.hold_score), (30, 70));
        assert_eq!(p.score_source["source"], "DeviceTrust");
        assert_eq!(level(Some(50), &p), "medium");
        // Nonsense platform values are ignored rather than applied.
        db.execute("INSERT INTO risk_events VALUES('c','reported',?1,300)", [json!({"policy":{"reviewScore":80,"holdScore":20}}).to_string()]).unwrap();
        assert_eq!(policy(&db).unwrap().score_source["source"], "local");
    }

    #[test]
    fn policy_defaults_and_event_enumerations_are_explicit() {
        let p = Policy::default();
        assert_eq!(p.mode, "observe");
        assert_eq!(p.touch_sample_percent, 100);
        assert_eq!(TYPES[5], "click");
        assert_eq!(SCREENS[14], "reward_result");
        assert_eq!(ACTIONS[2], "reward_complete");
    }
}
