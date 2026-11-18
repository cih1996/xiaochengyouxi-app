//! Where withdrawals are paid: bound once in the App, not typed on every withdrawal.
//!
//! * 支付宝: account + real name (payouts are checked against the name).
//! * 微信: the 收款码 is decoded on the phone; only its content is uploaded and it must be a
//!   WeChat personal payment code (wxp://…). No image is ever stored; the admin sees a QR code
//!   re-rendered from that content. Optional real name.
//! Every withdrawal stores a snapshot of the account it was paid to (`withdrawals.account`, and
//! `payout_ref` = the 收款码 content), so later changes never alter a submitted request.
use super::*;
use qrcode::{Color, EcLevel, QrCode};

/// WeChat personal 收款码 content, e.g. wxp://f2f0AbC…
pub(super) fn valid_wechat_code(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("wxp://") else {
        return false;
    };
    (12..=120).contains(&rest.len()) && rest.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS payout_accounts(user_id INTEGER PRIMARY KEY,alipay_account TEXT,alipay_name TEXT,wechat_code TEXT,wechat_name TEXT,updated_at INTEGER NOT NULL);",
    )?;
    let has_ref: bool = db
        .query_row("SELECT COUNT(*) FROM pragma_table_info('withdrawals') WHERE name='payout_ref'", [], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !has_ref {
        db.execute_batch("ALTER TABLE withdrawals ADD COLUMN payout_ref TEXT")?;
    }
    Ok(())
}

pub(super) struct Bound {
    pub account: String,
    pub payout_ref: Option<String>,
}

/// The snapshot used for a withdrawal through `channel`, or a clear message to bind first.
pub(super) fn for_withdrawal(db: &Connection, uid: i64, channel: &str) -> Result<Bound, ApiError> {
    let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>)> = db
        .query_row(
            "SELECT alipay_account,alipay_name,wechat_code,wechat_name FROM payout_accounts WHERE user_id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let (alipay, alipay_name, qr, wechat_name) = row.unwrap_or((None, None, None, None));
    match channel {
        "alipay" => match (alipay, alipay_name) {
            (Some(a), Some(n)) => Ok(Bound { account: format!("{a}（{n}）"), payout_ref: None }),
            _ => Err(ApiError(StatusCode::CONFLICT, "请先绑定支付宝收款账户".into(), Some("PAYOUT_UNBOUND"))),
        },
        _ => match qr.filter(|c| valid_wechat_code(c)) {
            Some(id) => Ok(Bound {
                account: match wechat_name.filter(|n| !n.is_empty()) {
                    Some(n) => format!("微信收款码（{n}）"),
                    None => "微信收款码".into(),
                },
                payout_ref: Some(id),
            }),
            None => Err(ApiError(StatusCode::CONFLICT, "请先绑定微信收款码".into(), Some("PAYOUT_UNBOUND"))),
        },
    }
}

fn mask_account(a: &str) -> String {
    let chars: Vec<char> = a.chars().collect();
    if let Some((name, domain)) = a.split_once('@') {
        let head: String = name.chars().take(2).collect();
        return format!("{head}***@{domain}");
    }
    if chars.len() <= 4 {
        return a.into();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

fn mask_name(n: &str) -> String {
    let chars: Vec<char> = n.chars().collect();
    match chars.len() {
        0 => String::new(),
        1 => n.into(),
        _ => format!("*{}", chars[1..].iter().collect::<String>()),
    }
}

fn summary(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>)> = db
        .query_row(
            "SELECT alipay_account,alipay_name,wechat_code,wechat_name FROM payout_accounts WHERE user_id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let (alipay, alipay_name, qr, wechat_name) = row.unwrap_or((None, None, None, None));
    Ok(json!({
        "alipay": match (alipay, alipay_name) {
            (Some(a), Some(n)) => json!({"account":mask_account(&a),"name":mask_name(&n)}),
            _ => Value::Null,
        },
        "wechat": match qr {
            Some(_) => json!({"name":wechat_name.as_deref().map(mask_name).unwrap_or_default()}),
            None => Value::Null,
        },
    }))
}

fn valid_name(n: &str) -> bool {
    let len = n.chars().count();
    (2..=20).contains(&len) && n.chars().all(|c| ('\u{4e00}'..='\u{9fff}').contains(&c) || c == '·' || c.is_ascii_alphabetic() || c == ' ')
}

async fn get_account(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    Ok(Json(summary(&db, uid)?))
}

#[derive(Deserialize)]
struct AlipayInput {
    account: String,
    real_name: String,
}

async fn bind_alipay(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<AlipayInput>,
) -> ApiResult<Value> {
    let account = input.account.trim().to_string();
    let name = input.real_name.trim().to_string();
    let phone = account.len() == 11 && account.starts_with('1') && account.bytes().all(|c| c.is_ascii_digit());
    let email = account.len() <= 64 && account.contains('@') && account.contains('.') && !account.contains(' ');
    if !phone && !email {
        return Err(invalid("请输入支付宝账号（手机号或邮箱）"));
    }
    if !valid_name(&name) {
        return Err(invalid("请输入支付宝实名认证的真实姓名"));
    }
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO payout_accounts(user_id,alipay_account,alipay_name,updated_at) VALUES(?1,?2,?3,?4)
         ON CONFLICT(user_id) DO UPDATE SET alipay_account=excluded.alipay_account,alipay_name=excluded.alipay_name,updated_at=excluded.updated_at",
        params![uid, account, name, now()],
    )?;
    user_admin::behavior(&tx, &context, Some(uid), None, "payout_bind", "success", "alipay", None, json!({"account":mask_account(&account)}))?;
    let result = summary(&tx, uid)?;
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct WechatInput {
    /// Content decoded from the 收款码 on the phone.
    code: String,
    #[serde(default)]
    real_name: String,
}

async fn bind_wechat(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<WechatInput>,
) -> ApiResult<Value> {
    let code = input.code.trim().to_string();
    if !valid_wechat_code(&code) {
        return Err(invalid("这不是微信收款码，请在微信“收付款 → 二维码收款”中保存收款码后重新选择"));
    }
    let name = input.real_name.trim().to_string();
    if !name.is_empty() && !valid_name(&name) {
        return Err(invalid("真实姓名格式不正确"));
    }
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO payout_accounts(user_id,wechat_code,wechat_name,updated_at) VALUES(?1,?2,?3,?4)
         ON CONFLICT(user_id) DO UPDATE SET wechat_code=excluded.wechat_code,wechat_name=excluded.wechat_name,updated_at=excluded.updated_at",
        params![uid, code, name, now()],
    )?;
    let digest: String = Sha256::digest(code.as_bytes()).iter().take(8).map(|b| format!("{b:02x}")).collect();
    user_admin::behavior(&tx, &context, Some(uid), None, "payout_bind", "success", "wechat", None, json!({"code_hash":digest}))?;
    let result = summary(&tx, uid)?;
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct MatrixInput {
    code: String,
}

/// Admin: QR matrix for a stored 收款码 content (rendered in the browser; no image stored).
async fn admin_matrix(Json(input): Json<MatrixInput>) -> ApiResult<Value> {
    if !valid_wechat_code(&input.code) {
        return Err(invalid("收款码内容无效"));
    }
    let qr = QrCode::with_error_correction_level(input.code.as_bytes(), EcLevel::M)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    let size = qr.width();
    let modules: Vec<String> = qr
        .to_colors()
        .chunks(size)
        .map(|row| row.iter().map(|c| if *c == Color::Dark { '1' } else { '0' }).collect())
        .collect();
    Ok(Json(json!({"size":size,"modules":modules})))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/payout-account", get(get_account))
        .route("/payout-account/alipay", axum::routing::put(bind_alipay))
        .route("/payout-account/wechat", post(bind_wechat))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/payout-qr", post(admin_matrix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masks_and_names() {
        assert_eq!(mask_account("13800138000"), "138****8000");
        assert_eq!(mask_account("hello@qq.com"), "he***@qq.com");
        assert_eq!(mask_name("张三丰"), "*三丰");
        assert!(valid_name("张三") && valid_name("阿依古丽·买买提") && !valid_name("张") && !valid_name("1234"));
        assert!(valid_wechat_code("wxp://f2f0Ab3dEfGhIjKlMnOpQrStUv"));
        for bad in ["https://u.wechat.com/abc", "wxp://", "wxp://short", "WXP://f2f0Ab3dEfGhIjKl", "wxp://f2f0 Ab3dEfGhIjKl", "weixin://wxpay/bizpayurl?pr=x"] {
            assert!(!valid_wechat_code(bad), "{bad}");
        }
    }
}
