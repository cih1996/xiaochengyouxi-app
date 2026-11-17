use super::*;
use qrcode::{Color, EcLevel, QrCode};

const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const PLATFORM_CODE: &str = "JZ8F6A";

fn normalize_code(input: &str) -> Result<String, ApiError> {
    let code = input.trim().to_ascii_uppercase();
    if code.len() != 6
        || !code.bytes().all(|c| CODE_ALPHABET.contains(&c))
        || !code.bytes().any(|c| c.is_ascii_alphabetic())
        || !code.bytes().any(|c| c.is_ascii_digit())
    {
        return Err(invalid("请输入 6 位字母和数字组成的邀请码"));
    }
    Ok(code)
}
fn platform_code(db: &Connection) -> Result<String, ApiError> {
    Ok(db.query_row(
        "SELECT platform_code FROM invitation_settings WHERE id=1",
        [],
        |r| r.get(0),
    )?)
}
pub(super) fn new_code(db: &Connection) -> Result<String, ApiError> {
    loop {
        let mut random = Uuid::new_v4().as_u128();
        let code: String = (0..6)
            .map(|_| {
                let c = CODE_ALPHABET[(random % CODE_ALPHABET.len() as u128) as usize] as char;
                random /= CODE_ALPHABET.len() as u128;
                c
            })
            .collect();
        if !code.bytes().any(|c| c.is_ascii_alphabetic())
            || !code.bytes().any(|c| c.is_ascii_digit())
        {
            continue;
        }
        let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM users WHERE invite_code=?1) OR EXISTS(SELECT 1 FROM invitation_settings WHERE platform_code=?1)",[&code],|r|r.get(0))?;
        if !exists {
            return Ok(code);
        }
    }
}
pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS invitation_settings(id INTEGER PRIMARY KEY CHECK(id=1),platform_code TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS invitation_prebindings(phone TEXT PRIMARY KEY,invite_code TEXT NOT NULL,inviter_id INTEGER,created_at INTEGER NOT NULL,consumed_at INTEGER,user_id INTEGER);")?;
    db.execute(
        "INSERT OR IGNORE INTO invitation_settings(id,platform_code) VALUES(1,?1)",
        [PLATFORM_CODE],
    )?;
    let columns = db
        .prepare("PRAGMA table_info(users)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    for (name, definition) in [
        ("invite_code", "TEXT"),
        ("inviter_id", "INTEGER"),
        ("inviter_code", "TEXT"),
        ("invitation_bound_at", "INTEGER"),
        ("username", "TEXT"),
    ] {
        if !columns.iter().any(|column| column == name) {
            db.execute_batch(&format!("ALTER TABLE users ADD COLUMN {name} {definition}"))?;
        }
    }
    let ids = db
        .prepare("SELECT id FROM users WHERE invite_code IS NULL OR invite_code=''")?
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids {
        db.execute(
            "UPDATE users SET invite_code=?1 WHERE id=?2",
            params![new_code(db)?, id],
        )?;
    }
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_invite_code ON users(invite_code);
        CREATE INDEX IF NOT EXISTS idx_users_inviter ON users(inviter_id);
        CREATE INDEX IF NOT EXISTS idx_prebindings_inviter ON invitation_prebindings(inviter_id,consumed_at);")?;
    Ok(())
}
pub(super) fn require_bound(db: &Connection, uid: i64) -> Result<(), ApiError> {
    let bound: bool = db.query_row(
        "SELECT invitation_bound_at IS NOT NULL FROM users WHERE id=?1",
        [uid],
        |r| r.get(0),
    )?;
    if !bound {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "请先绑定邀请码".into(),
            None,
        ));
    }
    Ok(())
}
fn resolve_inviter(
    db: &Connection,
    code: &str,
    joining_user: Option<i64>,
) -> Result<Option<i64>, ApiError> {
    if code == platform_code(db)? {
        return Ok(None);
    }
    let inviter: Option<(i64, Option<i64>)> = db
        .query_row(
            "SELECT id,invitation_bound_at FROM users WHERE invite_code=?1",
            [code],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((id, bound_at)) = inviter else {
        return Err(invalid("邀请码不存在"));
    };
    if Some(id) == joining_user {
        return Err(invalid("不能填写自己的邀请码"));
    }
    if bound_at.is_none() {
        return Err(invalid("该邀请码暂不可用"));
    }
    Ok(Some(id))
}
fn bind_relation(
    db: &Connection,
    uid: i64,
    code: &str,
    inviter: Option<i64>,
) -> Result<(), ApiError> {
    if inviter == Some(uid) {
        return Err(invalid("不能填写自己的邀请码"));
    }
    if let Some(id) = inviter {
        require_bound(db, id)?;
    }
    db.execute("UPDATE users SET inviter_id=?1,inviter_code=?2,invitation_bound_at=?3 WHERE id=?4 AND invitation_bound_at IS NULL",params![inviter,code,now(),uid])?;
    Ok(())
}
pub(super) fn consume_pending(db: &Connection, uid: i64) -> Result<(), ApiError> {
    let (phone, bound): (String, Option<i64>) = db.query_row(
        "SELECT phone,invitation_bound_at FROM users WHERE id=?1",
        [uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if bound.is_some() {
        return Ok(());
    }
    let pending:Option<(String,Option<i64>)>=db.query_row("SELECT invite_code,inviter_id FROM invitation_prebindings WHERE phone=?1 AND consumed_at IS NULL",[&phone],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((code, inviter)) = pending {
        bind_relation(db, uid, &code, inviter)?;
        db.execute("UPDATE invitation_prebindings SET consumed_at=?1,user_id=?2 WHERE phone=?3 AND consumed_at IS NULL",params![now(),uid,phone])?;
    }
    Ok(())
}
#[derive(Deserialize)]
pub(super) struct CodeInput {
    invite_code: String,
}
pub(super) async fn bind(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<CodeInput>,
) -> ApiResult<Value> {
    let code = normalize_code(&input.invite_code)?;
    let mut db = open(&state)?;
    let uid = account_auth(&db, &headers)?;
    let tx = db.transaction()?;
    let (bound, previous): (Option<i64>, Option<String>) = tx.query_row(
        "SELECT invitation_bound_at,inviter_code FROM users WHERE id=?1",
        [uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if bound.is_some() {
        if previous.as_deref() == Some(&code) {
            return Ok(Json(json!({"user":user(&tx,uid)?})));
        }
        return Err(conflict("邀请码已绑定，不能更换"));
    }
    let pending:Option<String>=tx.query_row("SELECT invite_code FROM invitation_prebindings WHERE phone=(SELECT phone FROM users WHERE id=?1) AND consumed_at IS NULL",[uid],|r|r.get(0)).optional()?;
    if let Some(pending) = pending {
        if pending != code {
            return Err(conflict("该手机号已预绑定其他邀请码"));
        }
        consume_pending(&tx, uid)?;
    } else {
        let inviter = resolve_inviter(&tx, &code, Some(uid))?;
        bind_relation(&tx, uid, &code, inviter)?;
    }
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "invite_bind",
        "success",
        &code,
        None,
        json!({}),
    )?;
    let result = json!({"user":user(&tx,uid)?});
    tx.commit()?;
    Ok(Json(result))
}
#[derive(Deserialize)]
pub(super) struct PrebindInput {
    phone: String,
    invite_code: String,
}
pub(super) async fn prebind(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<PrebindInput>,
) -> ApiResult<Value> {
    validate_phone(&input.phone)?;
    let code = normalize_code(&input.invite_code)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let existing_user: Option<(i64, Option<i64>)> = tx
        .query_row(
            "SELECT id,invitation_bound_at FROM users WHERE phone=?1",
            [&input.phone],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if existing_user
        .as_ref()
        .is_some_and(|(_, bound)| bound.is_some())
    {
        return Err(conflict("该手机号已绑定邀请码，请直接登录"));
    }
    let existing: Option<String> = tx
        .query_row(
            "SELECT invite_code FROM invitation_prebindings WHERE phone=?1",
            [&input.phone],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        if existing != code {
            return Err(conflict("该手机号已预绑定其他邀请码"));
        }
    } else {
        let inviter = resolve_inviter(&tx, &code, existing_user.as_ref().map(|(id, _)| *id))?;
        tx.execute("INSERT INTO invitation_prebindings(phone,invite_code,inviter_id,created_at) VALUES(?1,?2,?3,?4)",params![input.phone,code,inviter,now()])?;
    }
    user_admin::behavior(
        &tx,
        &context,
        existing_user.map(|(id, _)| id),
        Some(&input.phone),
        "invite_prebind",
        "success",
        &code,
        None,
        json!({}),
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"status":"pending","invite_code":code,"download_url":state.config.app_download_url.as_deref().unwrap_or("")}),
    ))
}
fn share_url(state: &AppState, code: &str) -> String {
    format!(
        "{}/i/{code}",
        state.config.invite_base_url.trim_end_matches('/')
    )
}
pub(super) async fn summary(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let current = user(&db, uid)?;
    let registered: i64 = db.query_row(
        "SELECT COUNT(*) FROM users WHERE inviter_id=?1 AND invitation_bound_at IS NOT NULL",
        [uid],
        |r| r.get(0),
    )?;
    let pending: i64 = db.query_row(
        "SELECT COUNT(*) FROM invitation_prebindings WHERE inviter_id=?1 AND consumed_at IS NULL",
        [uid],
        |r| r.get(0),
    )?;
    let mut result = json!({"invite_code":current.invite_code,"invite_url":share_url(&state,&current.invite_code),"invited_count":registered+pending,"registered_count":registered,"pending_count":pending});
    if let Some(code) = current.inviter_code {
        result["inviter_code"] = json!(code);
    }
    Ok(Json(result))
}
pub(super) async fn qr(
    State(state): State<AppState>,
    Path(input): Path<String>,
) -> ApiResult<Value> {
    let code = normalize_code(&input)?;
    resolve_inviter(&open(&state)?, &code, None)?;
    let url = share_url(&state, &code);
    let qr = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::M)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    let size = qr.width();
    let modules: Vec<String> = qr
        .to_colors()
        .chunks(size)
        .map(|row| {
            row.iter()
                .map(|color| if *color == Color::Dark { '1' } else { '0' })
                .collect()
        })
        .collect();
    Ok(Json(
        json!({"code":code,"url":url,"size":size,"modules":modules}),
    ))
}
fn admin_data(state: &AppState, db: &Connection) -> Result<Value, ApiError> {
    let platform = platform_code(db)?;
    let registered: i64 = db.query_row(
        "SELECT COUNT(*) FROM users WHERE invitation_bound_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let unbound: i64 = db.query_row(
        "SELECT COUNT(*) FROM users WHERE invitation_bound_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let pending: i64 = db.query_row(
        "SELECT COUNT(*) FROM invitation_prebindings WHERE consumed_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let mut stmt=db.prepare("SELECT u.id,u.phone,u.invite_code,COALESCE(u.inviter_code,p.invite_code),u.created_at,u.invitation_bound_at,CASE WHEN u.invitation_bound_at IS NOT NULL THEN '已绑定' WHEN p.phone IS NOT NULL THEN '待登录' ELSE '待绑定' END FROM users u LEFT JOIN invitation_prebindings p ON p.phone=u.phone AND p.consumed_at IS NULL ORDER BY u.id DESC LIMIT 200")?;
    let mut records=stmt.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"phone":r.get::<_,String>(1)?,"invite_code":r.get::<_,String>(2)?,"inviter_code":r.get::<_,Option<String>>(3)?,"created_at":iso(r.get(4)?),"bound_at":r.get::<_,Option<i64>>(5)?.map(iso),"status":r.get::<_,String>(6)?})))?.collect::<Result<Vec<_>,_>>()?;
    let mut pending_stmt=db.prepare("SELECT p.phone,p.invite_code,p.created_at FROM invitation_prebindings p LEFT JOIN users u ON u.phone=p.phone WHERE p.consumed_at IS NULL AND u.id IS NULL ORDER BY p.created_at DESC LIMIT 200")?;
    records.extend(pending_stmt.query_map([],|r|Ok(json!({"phone":r.get::<_,String>(0)?,"invite_code":Value::Null,"inviter_code":r.get::<_,String>(1)?,"created_at":iso(r.get(2)?),"bound_at":Value::Null,"status":"待注册"})))?.collect::<Result<Vec<_>,_>>()?);
    Ok(
        json!({"platform_code":platform,"platform_url":share_url(state,&platform),"invited_count":registered+pending,"registered_count":registered,"pending_count":pending,"unbound_count":unbound,"records":records}),
    )
}
pub(super) async fn admin_overview(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(admin_data(&state, &open(&state)?)?))
}
#[derive(Deserialize)]
pub(super) struct PlatformInput {
    platform_code: String,
}
pub(super) async fn update_platform_code(
    State(state): State<AppState>,
    Json(input): Json<PlatformInput>,
) -> ApiResult<Value> {
    let code = normalize_code(&input.platform_code)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE invite_code=?1)",
        [&code],
        |r| r.get(0),
    )?;
    if exists {
        return Err(conflict("该邀请码已被用户使用"));
    }
    tx.execute(
        "UPDATE invitation_settings SET platform_code=?1 WHERE id=1",
        [code],
    )?;
    let result = admin_data(&state, &tx)?;
    tx.commit()?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_accounts_keep_balances_and_receive_stable_unique_codes() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE users(id INTEGER PRIMARY KEY,phone TEXT,coins INTEGER);
            INSERT INTO users(id,phone,coins) VALUES(1,'13000130000',4321);",
        )
        .unwrap();
        for id in 2..=80 {
            db.execute(
                "INSERT INTO users(id,phone,coins) VALUES(?1,?2,987)",
                params![id, format!("130{id:08}")],
            )
            .unwrap();
        }
        initialize(&db).unwrap();
        let before: Vec<String> = db
            .prepare("SELECT invite_code FROM users ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(before.len(), 80);
        assert_eq!(
            before
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            80
        );
        assert!(
            before
                .iter()
                .all(|code| normalize_code(code).is_ok_and(|normalized| normalized == *code))
        );
        assert_eq!(
            db.query_row("SELECT coins FROM users WHERE id=1", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            4321
        );
        assert_eq!(require_bound(&db, 1).unwrap_err().0, StatusCode::FORBIDDEN);
        initialize(&db).unwrap();
        let after: Vec<String> = db
            .prepare("SELECT invite_code FROM users ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn pending_invitation_consumption_is_stable_and_preserves_money() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE users(id INTEGER PRIMARY KEY,phone TEXT,coins INTEGER);
            INSERT INTO users(id,phone,coins) VALUES(1,'13000130000',4321),(2,'13000130001',987);",
        )
        .unwrap();
        initialize(&db).unwrap();
        let inviter_code: String = db
            .query_row("SELECT invite_code FROM users WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert!(resolve_inviter(&db, &inviter_code, Some(2)).is_err());
        bind_relation(&db, 1, PLATFORM_CODE, None).unwrap();
        assert_eq!(
            resolve_inviter(&db, &inviter_code, Some(2)).unwrap(),
            Some(1)
        );
        assert!(resolve_inviter(&db, &inviter_code, Some(1)).is_err());
        db.execute("INSERT INTO invitation_prebindings(phone,invite_code,inviter_id,created_at) VALUES('13000130001',?1,1,?2)",params![inviter_code,now()]).unwrap();
        consume_pending(&db, 2).unwrap();
        consume_pending(&db, 2).unwrap();
        let relation: (Option<i64>, String, i64) = db
            .query_row(
                "SELECT inviter_id,inviter_code,coins FROM users WHERE id=2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(relation, (Some(1), inviter_code, 987));
        assert_eq!(db.query_row("SELECT COUNT(*) FROM invitation_prebindings WHERE consumed_at IS NOT NULL AND user_id=2",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
}
