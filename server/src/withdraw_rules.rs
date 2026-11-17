//! Withdrawal tiers: users pick a fixed amount; each tier may cap daily and lifetime uses.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct WithdrawTier {
    pub id: String,
    pub amount_yuan: f64,
    /// 0 = unlimited.
    #[serde(default)]
    pub daily_limit: i64,
    /// 0 = unlimited (e.g. 1 for a once-only newcomer tier).
    #[serde(default)]
    pub lifetime_limit: i64,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub note: String,
}
fn enabled() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct WithdrawRules {
    pub tiers: Vec<WithdrawTier>,
}
impl Default for WithdrawRules {
    fn default() -> Self {
        let tier = |id: &str, amount: f64, daily: i64, note: &str| WithdrawTier {
            id: id.into(),
            amount_yuan: amount,
            daily_limit: daily,
            lifetime_limit: 0,
            enabled: true,
            note: note.into(),
        };
        Self {
            tiers: vec![
                tier("daily-small", 0.5, 1, "每天限 1 次"),
                tier("t20", 20.0, 0, ""),
                tier("t50", 50.0, 0, ""),
            ],
        }
    }
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS withdraw_rules(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL,revision INTEGER NOT NULL,updated_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO withdraw_rules(id,json,revision,updated_at) VALUES(1,?1,1,?2)",
        params![serde_json::to_string(&WithdrawRules::default())?, now()],
    )?;
    Ok(())
}

pub(super) fn rules(db: &Connection) -> Result<WithdrawRules, ApiError> {
    let raw: String = db.query_row("SELECT json FROM withdraw_rules WHERE id=1", [], |r| r.get(0))?;
    Ok(serde_json::from_str(&raw)?)
}

fn cents(value: f64) -> i64 {
    (value * 100.0).round() as i64
}

/// Smallest enabled tier, shown to older app versions as the threshold.
pub(super) fn minimum(db: &Connection) -> Result<f64, ApiError> {
    Ok(rules(db)?
        .tiers
        .iter()
        .filter(|t| t.enabled)
        .map(|t| t.amount_yuan)
        .fold(f64::INFINITY, f64::min)
        .min(1e9))
}

fn usage(db: &Connection, uid: i64, amount: f64, since: Option<i64>) -> Result<i64, ApiError> {
    // Refunded (rejected) withdrawals do not consume a tier.
    Ok(db.query_row(
        "SELECT COUNT(*) FROM withdrawals WHERE user_id=?1 AND CAST(ROUND(amount_yuan*100) AS INTEGER)=?2 AND status!='模拟已退回' AND (?3 IS NULL OR created_at>=?3)",
        params![uid, cents(amount), since],
        |r| r.get(0),
    )?)
}

fn amount_text(amount: f64) -> String {
    let c = cents(amount);
    if c % 100 == 0 { (c / 100).to_string() } else { format!("{:.2}", amount).trim_end_matches('0').to_string() }
}

/// Rejects amounts that are not an enabled tier or whose limits are used up.
pub(super) fn check(db: &Connection, uid: i64, amount: f64) -> Result<(), ApiError> {
    let rules = rules(db)?;
    let enabled: Vec<&WithdrawTier> = rules.tiers.iter().filter(|t| t.enabled).collect();
    let Some(tier) = enabled.iter().find(|t| cents(t.amount_yuan) == cents(amount)) else {
        let list = enabled.iter().map(|t| format!("{} 元", amount_text(t.amount_yuan))).collect::<Vec<_>>().join("、");
        return Err(invalid(&if list.is_empty() { "提现暂未开放".to_string() } else { format!("请选择提现档位：{list}") }));
    };
    if tier.daily_limit > 0 && usage(db, uid, tier.amount_yuan, Some(today_start()))? >= tier.daily_limit {
        return Err(invalid(&format!("{} 元档今天已提现，明天再来", amount_text(tier.amount_yuan))));
    }
    if tier.lifetime_limit > 0 && usage(db, uid, tier.amount_yuan, None)? >= tier.lifetime_limit {
        return Err(invalid(&format!("{} 元档的提现次数已用完", amount_text(tier.amount_yuan))));
    }
    Ok(())
}

async fn options(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let balance: i64 = db.query_row("SELECT coins FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    let mut tiers = Vec::new();
    for t in rules(&db)?.tiers.into_iter().filter(|t| t.enabled) {
        let today = usage(&db, uid, t.amount_yuan, Some(today_start()))?;
        let total = usage(&db, uid, t.amount_yuan, None)?;
        let need = (t.amount_yuan * COINS_PER_YUAN as f64).round() as i64;
        let (available, reason) = if t.daily_limit > 0 && today >= t.daily_limit {
            (false, "今日已提".to_string())
        } else if t.lifetime_limit > 0 && total >= t.lifetime_limit {
            (false, "次数已用完".to_string())
        } else if balance < need {
            (false, format!("还差 ¥{:.2}", (need - balance) as f64 / COINS_PER_YUAN as f64))
        } else {
            (true, String::new())
        };
        let limit = if t.daily_limit > 0 {
            format!("每天限 {} 次", t.daily_limit)
        } else if t.lifetime_limit > 0 {
            format!("限 {} 次", t.lifetime_limit)
        } else {
            format!("满 {} 元可提", amount_text(t.amount_yuan))
        };
        tiers.push(json!({"id":t.id,"amount_yuan":t.amount_yuan,"label":format!("{} 元",amount_text(t.amount_yuan)),"limit":if t.note.is_empty(){limit}else{t.note.clone()},"available":available,"reason":reason,"used_today":today}));
    }
    Ok(Json(json!({"cash_yuan":balance as f64/COINS_PER_YUAN as f64,"tiers":tiers})))
}

fn rules_json(db: &Connection) -> Result<Value, ApiError> {
    let (raw, revision, updated): (String, i64, i64) = db.query_row(
        "SELECT json,revision,updated_at FROM withdraw_rules WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut value: Value = serde_json::from_str(&raw)?;
    value["revision"] = json!(revision);
    value["updated_at"] = json!(iso(updated));
    Ok(value)
}
async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    Ok(Json(rules_json(&open(&state)?)?))
}
fn validate(input: &mut WithdrawRules) -> Result<(), ApiError> {
    if input.tiers.len() > 8 {
        return Err(invalid("提现档位最多 8 个"));
    }
    let mut seen = std::collections::HashSet::new();
    for t in &mut input.tiers {
        t.id = t.id.trim().to_owned();
        t.note = t.note.trim().to_owned();
        if t.id.is_empty() || t.id.len() > 32 || !seen.insert(t.id.clone()) {
            return Err(invalid("档位标识不能为空且不能重复"));
        }
        if !t.amount_yuan.is_finite() || t.amount_yuan < 0.1 || t.amount_yuan > 5000.0 || (t.amount_yuan * 100.0 - (t.amount_yuan * 100.0).round()).abs() > 1e-6 {
            return Err(invalid("档位金额应在 0.1 至 5000 元之间，最多两位小数"));
        }
        if !(0..=100).contains(&t.daily_limit) || !(0..=1000).contains(&t.lifetime_limit) {
            return Err(invalid("次数限制超出范围"));
        }
        if t.note.chars().count() > 20 {
            return Err(invalid("档位说明最多 20 字"));
        }
    }
    let amounts: std::collections::HashSet<i64> = input.tiers.iter().filter(|t| t.enabled).map(|t| cents(t.amount_yuan)).collect();
    if amounts.len() != input.tiers.iter().filter(|t| t.enabled).count() {
        return Err(invalid("启用的档位金额不能重复"));
    }
    input.tiers.sort_by(|a, b| a.amount_yuan.total_cmp(&b.amount_yuan));
    Ok(())
}
async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<WithdrawRules>,
) -> ApiResult<Value> {
    validate(&mut input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = rules_json(&tx)?;
    tx.execute(
        "UPDATE withdraw_rules SET json=?1,revision=revision+1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&input)?, now()],
    )?;
    let after = rules_json(&tx)?;
    user_admin::action_audit(&tx, &context, None, "withdraw_rules_update", "withdraw-rules", "修改提现档位", before, after.clone())?;
    tx.commit()?;
    Ok(Json(after))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/withdraw/options", get(options))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/withdraw-rules", get(admin_get).put(admin_put))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiers_reject_duplicates_and_bad_amounts() {
        let mut ok = WithdrawRules::default();
        assert!(validate(&mut ok).is_ok());
        let mut dup = WithdrawRules::default();
        dup.tiers[1].amount_yuan = 0.5;
        assert!(validate(&mut dup).is_err());
        let mut bad = WithdrawRules::default();
        bad.tiers[0].amount_yuan = 0.555;
        assert!(validate(&mut bad).is_err());
        assert_eq!(amount_text(0.5), "0.5");
        assert_eq!(amount_text(20.0), "20");
    }
}
