//! Local notification reminders (no push service): the App schedules them itself in the phone's
//! notification bar, using this server-side configuration for times, texts and switches.
//! Rain: the next round only, a few minutes before it starts (once per App session).
//! Check-in / wheel: once a day at a set time while still undone. Nothing in quiet hours.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Config {
    pub enabled: bool,
    pub rain_enabled: bool,
    pub rain_lead_minutes: i64,
    pub rain_title: String,
    pub rain_body: String,
    pub checkin_enabled: bool,
    pub checkin_time: String,
    pub checkin_title: String,
    pub checkin_body: String,
    pub wheel_enabled: bool,
    pub wheel_time: String,
    pub wheel_title: String,
    pub wheel_body: String,
    pub quiet_start: String,
    pub quiet_end: String,
    #[serde(default)]
    pub revision: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            rain_enabled: true,
            rain_lead_minutes: 3,
            rain_title: "红包雨马上开始".into(),
            rain_body: "{minutes} 分钟后开抢，准时来领红包～".into(),
            checkin_enabled: true,
            checkin_time: "20:00".into(),
            checkin_title: "今天还没签到哦".into(),
            checkin_body: "签到领金币，连续签到奖励更多".into(),
            wheel_enabled: true,
            wheel_time: "12:30".into(),
            wheel_title: "幸运转盘免费次数待使用".into(),
            wheel_body: "今天的免费转盘还没转，转一转试试手气".into(),
            quiet_start: "22:30".into(),
            quiet_end: "08:00".into(),
            revision: 0,
        }
    }
}

fn valid_time(t: &str) -> bool {
    let Some((h, m)) = t.split_once(':') else { return false };
    h.len() == 2 && m.len() == 2 && h.parse::<u32>().is_ok_and(|h| h < 24) && m.parse::<u32>().is_ok_and(|m| m < 60)
}

fn validate(c: &Config) -> Result<(), ApiError> {
    for t in [&c.checkin_time, &c.wheel_time, &c.quiet_start, &c.quiet_end] {
        if !valid_time(t) {
            return Err(invalid("时间格式应为 HH:MM，例如 20:00"));
        }
    }
    if !(1..=30).contains(&c.rain_lead_minutes) {
        return Err(invalid("红包雨提前提醒需为 1–30 分钟"));
    }
    for (name, v, max) in [
        ("标题", &c.rain_title, 20), ("标题", &c.checkin_title, 20), ("标题", &c.wheel_title, 20),
        ("内容", &c.rain_body, 40), ("内容", &c.checkin_body, 40), ("内容", &c.wheel_body, 40),
    ] {
        let n = v.trim().chars().count();
        if n == 0 || n > max {
            return Err(invalid(&format!("通知{name}需为 1–{max} 个字")));
        }
    }
    Ok(())
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS reminder_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,updated_at INTEGER NOT NULL);")?;
    db.execute(
        "INSERT OR IGNORE INTO reminder_config VALUES(1,?1,?2)",
        params![serde_json::to_string(&Config::default())?, now()],
    )?;
    Ok(())
}

fn config(db: &Connection) -> Result<Config, ApiError> {
    Ok(serde_json::from_str(&db.query_row::<String, _, _>(
        "SELECT config_json FROM reminder_config WHERE id=1",
        [],
        |r| r.get(0),
    )?)?)
}

async fn public(State(state): State<AppState>) -> ApiResult<Config> {
    Ok(Json(config(&open(&state)?)?))
}

async fn admin_get(State(state): State<AppState>) -> ApiResult<Config> {
    Ok(Json(config(&open(&state)?)?))
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(mut input): Json<Config>,
) -> ApiResult<Config> {
    validate(&input)?;
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let before = config(&tx)?;
    input.revision = before.revision + 1;
    tx.execute(
        "UPDATE reminder_config SET config_json=?1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&input)?, now()],
    )?;
    user_admin::action_audit(&tx, &context, None, "reminder_config", "reminders", "修改通知提醒配置", serde_json::to_value(&before)?, serde_json::to_value(&input)?)?;
    tx.commit()?;
    Ok(Json(input))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/app/reminders", get(public))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/reminders", get(admin_get).put(admin_put))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_validate_and_times_are_checked() {
        assert!(validate(&Config::default()).is_ok());
        let mut c = Config::default();
        c.checkin_time = "25:00".into();
        assert!(validate(&c).is_err());
        assert!(valid_time("08:05") && !valid_time("8:05") && !valid_time("12:60"));
    }
}
