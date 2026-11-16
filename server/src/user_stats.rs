use super::*;
use chrono::NaiveDate;
use std::collections::BTreeMap;

// Only server-recorded account actions count. SDK reporting and passive pushes do not.
const ACTIVE_KINDS: &[&str] = &[
    "register",
    "login",
    "app_open",
    "password_reset",
    "logout",
    "invite_bind",
    "hall_request",
    "reward_start",
    "reward_complete",
    "check_in",
    "withdrawal",
    "rain_join",
    "rain_small_claim",
    "rain_large_open",
    "rain_large_claim",
];
const CHINA_OFFSET: i64 = 8 * 3600;

#[derive(Deserialize, Default)]
struct StatsQuery {
    days: Option<u32>,
    start_date: Option<String>,
    end_date: Option<String>,
}

struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
    days: i64,
}

fn local_date(timestamp: i64) -> NaiveDate {
    chrono::DateTime::from_timestamp(timestamp + CHINA_OFFSET, 0)
        .unwrap()
        .date_naive()
}

fn start_of_day(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() - CHINA_OFFSET
}

fn parse_date(raw: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .filter(|date| date.to_string() == raw)
        .ok_or_else(|| invalid("日期请使用 YYYY-MM-DD 格式"))
}

impl DateRange {
    fn parse(query: &StatsQuery, at: i64) -> Result<Self, ApiError> {
        let today = local_date(at);
        let (start, end) = match (&query.start_date, &query.end_date) {
            (None, None) => {
                let days = query.days.unwrap_or(30);
                if ![7, 30, 90].contains(&days) {
                    return Err(invalid("请选择近7、30或90天，或填写日期区间"));
                }
                (today - chrono::Duration::days(i64::from(days) - 1), today)
            }
            (Some(start), Some(end)) => {
                if query.days.is_some() {
                    return Err(invalid("天数与自选日期区间请选择一种"));
                }
                (parse_date(start)?, parse_date(end)?)
            }
            _ => return Err(invalid("请同时填写开始日期和结束日期")),
        };
        let days = (end - start).num_days() + 1;
        if !(1..=366).contains(&days) {
            return Err(invalid("日期区间应为1至366天，结束日期不得早于开始日期"));
        }
        if end > today {
            return Err(invalid("结束日期不能晚于今天"));
        }
        Ok(Self {
            start,
            end,
            today,
            days,
        })
    }
}

#[derive(Default)]
struct Activity {
    today: i64,
    seven_days: i64,
    thirty_days: i64,
    selected: i64,
    buckets: [i64; 4],
    daily: BTreeMap<String, (i64, i64)>,
}

fn activity(db: &Connection, range: &DateRange, at: i64) -> Result<Activity, ApiError> {
    let seven = range.today - chrono::Duration::days(6);
    let thirty = range.today - chrono::Duration::days(29);
    let earliest = range.start.min(thirty);
    let kinds = ACTIVE_KINDS
        .iter()
        .map(|kind| format!("'{kind}'"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "WITH active_days AS MATERIALIZED (
           SELECT b.user_id,
             date(b.created_at,'unixepoch','+8 hours') AS day,
             date(u.created_at,'unixepoch','+8 hours') AS registration_day
           FROM user_behavior b JOIN users u ON u.id=b.user_id
           WHERE b.created_at>=?1 AND b.created_at<?2 AND b.created_at>=u.created_at
             AND b.status='success' AND b.kind IN ({kinds})
           GROUP BY b.user_id,day,registration_day
         ), per_user AS (
           SELECT user_id,MAX(day=?3) AS today,MAX(day>=?4) AS seven,
             MAX(day>=?5) AS thirty,
             COUNT(CASE WHEN day BETWEEN ?6 AND ?7 THEN 1 END) AS selected_days
           FROM active_days GROUP BY user_id
         )
         SELECT '',COALESCE(SUM(today),0),COALESCE(SUM(seven),0),
           COALESCE(SUM(thirty),0),COALESCE(SUM(selected_days>0),0),
           COALESCE(SUM(selected_days=1),0),
           COALESCE(SUM(selected_days BETWEEN 2 AND 3),0),
           COALESCE(SUM(selected_days BETWEEN 4 AND 7),0),
           COALESCE(SUM(selected_days>=8),0)
         FROM per_user
         UNION ALL
         SELECT day,COUNT(*),SUM(registration_day<day),0,0,0,0,0,0
         FROM active_days WHERE day BETWEEN ?6 AND ?7 GROUP BY day"
    );
    let mut statement = db.prepare(&sql)?;
    let mut rows = statement.query(params![
        start_of_day(earliest),
        at + 1,
        range.today.to_string(),
        seven.to_string(),
        thirty.to_string(),
        range.start.to_string(),
        range.end.to_string(),
    ])?;
    let mut result = Activity::default();
    while let Some(row) = rows.next()? {
        let date: String = row.get(0)?;
        if date.is_empty() {
            result.today = row.get(1)?;
            result.seven_days = row.get(2)?;
            result.thirty_days = row.get(3)?;
            result.selected = row.get(4)?;
            result.buckets = [row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?];
        } else {
            result.daily.insert(date, (row.get(1)?, row.get(2)?));
        }
    }
    Ok(result)
}

fn statistics(db: &Connection, query: &StatsQuery, at: i64) -> Result<Value, ApiError> {
    let range = DateRange::parse(query, at)?;
    let registrations = db
        .prepare(
            "SELECT date(created_at,'unixepoch','+8 hours'),COUNT(*) FROM users
             WHERE created_at<?1 GROUP BY 1 ORDER BY 1",
        )?
        .query_map([at + 1], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let active = activity(db, &range, at)?;
    let collection_since: Option<i64> = db
        .query_row(
            "SELECT value FROM investigation_meta WHERE key='collection_since'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let start = range.start.to_string();
    let total_users: i64 = registrations.values().sum();
    let mut running: i64 = registrations
        .iter()
        .filter(|(date, _)| *date < &start)
        .map(|(_, count)| count)
        .sum();
    let mut range_registrations = 0;
    let mut daily = Vec::with_capacity(range.days as usize);
    for offset in 0..range.days {
        let date = (range.start + chrono::Duration::days(offset)).to_string();
        let registered = registrations.get(&date).copied().unwrap_or(0);
        let (active_users, returning_active) = active.daily.get(&date).copied().unwrap_or_default();
        running += registered;
        range_registrations += registered;
        daily.push(json!({
            "date":date,"registrations":registered,"active_users":active_users,
            "total_users":running,"returning_active":returning_active
        }));
    }
    let distribution = [
        ("1", "1天"),
        ("2_3", "2–3天"),
        ("4_7", "4–7天"),
        ("8_plus", "8天及以上"),
    ]
    .into_iter()
    .zip(active.buckets)
    .map(|((key, label), users)| json!({"key":key,"label":label,"users":users}))
    .collect::<Vec<_>>();
    Ok(json!({
        "selected_range":{"start_date":start,"end_date":range.end.to_string(),"days":range.days},
        "summary":{
            "total_users":total_users,
            "today_registrations":registrations.get(&range.today.to_string()).copied().unwrap_or(0),
            "yesterday_registrations":registrations.get(&(range.today-chrono::Duration::days(1)).to_string()).copied().unwrap_or(0),
            "range_registrations":range_registrations,
            "today_active_users":active.today,
            "active_users_7d":active.seven_days,
            "active_users_30d":active.thirty_days,
            "range_active_users":active.selected,
            "range_inactive_users":running-active.selected,
            "range_total_users":running
        },
        "daily":daily,
        "activity_days":distribution,
        "activity_definition":"按北京时间自然日，对成功登录、打开应用或参与业务的用户去重统计。",
        "metadata":{
            "timezone":"UTC+08:00","generated_at":iso(at),
            "behavior_collection_since":collection_since.map(iso),
            "returning_active_definition":"注册日期早于该日的活跃用户。",
            "inactive_definition":"区间结束时已注册、期间无有效活跃记录的用户。",
            "history_note":"活跃仅依据已采集的成功行为；采集前的历史不补算。",
            "active_kinds":ACTIVE_KINDS,
            "active_status":"success"
        }
    }))
}

async fn stats(State(state): State<AppState>, Query(query): Query<StatsQuery>) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let result = statistics(&tx, &query, now())?;
    tx.commit()?;
    Ok(Json(result))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/user-center/stats", get(stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(value: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(value)
            .unwrap()
            .timestamp()
    }

    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE users(id INTEGER PRIMARY KEY,created_at INTEGER NOT NULL);
             CREATE TABLE user_behavior(user_id INTEGER,kind TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL);
             CREATE TABLE investigation_meta(key TEXT PRIMARY KEY,value INTEGER NOT NULL);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO investigation_meta VALUES('collection_since',?1)",
            [stamp("2026-10-01T00:00:00+08:00")],
        )
        .unwrap();
        db
    }

    fn add_user(db: &Connection, uid: i64, at: &str) {
        db.execute("INSERT INTO users VALUES(?1,?2)", params![uid, stamp(at)])
            .unwrap();
    }

    fn event(db: &Connection, uid: Option<i64>, kind: &str, status: &str, at: &str) {
        db.execute(
            "INSERT INTO user_behavior VALUES(?1,?2,?3,?4)",
            params![uid, kind, status, stamp(at)],
        )
        .unwrap();
    }

    fn range(start: &str, end: &str) -> StatsQuery {
        StatsQuery {
            start_date: Some(start.into()),
            end_date: Some(end.into()),
            ..Default::default()
        }
    }

    #[test]
    fn date_ranges_use_china_calendar_and_validate_inclusive_bounds() {
        let at = stamp("2026-11-11T00:30:00+08:00");
        let default = DateRange::parse(&StatsQuery::default(), at).unwrap();
        assert_eq!(default.start.to_string(), "2026-10-13");
        assert_eq!(default.end.to_string(), "2026-11-11");
        let seven = DateRange::parse(
            &StatsQuery {
                days: Some(7),
                ..Default::default()
            },
            at,
        )
        .unwrap();
        assert_eq!(seven.start.to_string(), "2026-11-05");
        assert_eq!(start_of_day(seven.start), stamp("2026-11-04T16:00:00Z"));
        assert_eq!(
            DateRange::parse(&range("2024-01-01", "2024-12-31"), at)
                .unwrap()
                .days,
            366
        );
        for query in [
            range("2024-01-01", "2025-01-01"),
            range("2026-11-12", "2026-11-11"),
            range("2026-11-11", "2026-11-12"),
            range("2026-2-01", "2026-02-28"),
            range("2026-02-30", "2026-03-01"),
            StatsQuery {
                days: Some(14),
                ..Default::default()
            },
            StatsQuery {
                start_date: Some("2026-11-01".into()),
                ..Default::default()
            },
            StatsQuery {
                days: Some(7),
                ..range("2026-11-01", "2026-11-07")
            },
        ] {
            assert_eq!(
                DateRange::parse(&query, at).err().unwrap().0,
                StatusCode::BAD_REQUEST
            );
        }
    }

    #[test]
    fn empty_data_has_zero_metrics_and_every_calendar_day() {
        let value = statistics(
            &db(),
            &range("2026-11-09", "2026-11-11"),
            stamp("2026-11-11T12:00:00+08:00"),
        )
        .unwrap();
        assert!(
            value["summary"]
                .as_object()
                .unwrap()
                .values()
                .all(|n| n == 0)
        );
        let daily = value["daily"].as_array().unwrap();
        assert_eq!(daily.len(), 3);
        assert_eq!(daily[0]["date"], "2026-11-09");
        assert_eq!(daily[2]["date"], "2026-11-11");
        assert!(
            value["activity_days"]
                .as_array()
                .unwrap()
                .iter()
                .all(|bucket| bucket["users"] == 0)
        );
        assert_eq!(
            value["metadata"]["behavior_collection_since"],
            "2026-09-30T16:00:00Z"
        );
    }

    #[test]
    fn registrations_obey_local_midnight_end_day_and_current_second() {
        let db = db();
        add_user(&db, 1, "2026-11-08T00:00:00+08:00");
        add_user(&db, 2, "2026-11-10T15:59:59Z");
        add_user(&db, 3, "2026-11-10T16:00:00Z");
        add_user(&db, 4, "2026-11-11T12:00:00+08:00");
        add_user(&db, 5, "2026-11-11T12:00:01+08:00");
        let value = statistics(
            &db,
            &range("2026-11-09", "2026-11-10"),
            stamp("2026-11-11T12:00:00+08:00"),
        )
        .unwrap();
        assert_eq!(value["summary"]["total_users"], 4);
        assert_eq!(value["summary"]["today_registrations"], 2);
        assert_eq!(value["summary"]["yesterday_registrations"], 1);
        assert_eq!(value["summary"]["range_registrations"], 1);
        assert_eq!(value["summary"]["range_total_users"], 2);
        assert_eq!(value["summary"]["range_inactive_users"], 2);
        assert_eq!(value["daily"][0]["registrations"], 0);
        assert_eq!(value["daily"][0]["total_users"], 1);
        assert_eq!(value["daily"][1]["registrations"], 1);
        assert_eq!(value["daily"][1]["total_users"], 2);
    }

    #[test]
    fn real_activity_is_deduplicated_and_excludes_sdk_failures_and_other_people_prebinding() {
        let db = db();
        add_user(&db, 1, "2026-11-01T10:00:00+08:00");
        add_user(&db, 2, "2026-11-08T10:00:00+08:00");
        add_user(&db, 3, "2026-11-01T10:00:00+08:00");
        add_user(&db, 4, "2026-11-11T10:00:00+08:00");
        for kind in ACTIVE_KINDS {
            event(&db, Some(1), kind, "success", "2026-11-08T12:00:00+08:00");
        }
        event(
            &db,
            Some(1),
            "app_open",
            "success",
            "2026-11-09T12:00:00+08:00",
        );
        event(
            &db,
            Some(2),
            "register",
            "success",
            "2026-11-08T10:00:00+08:00",
        );
        event(
            &db,
            Some(2),
            "rain_large_open",
            "success",
            "2026-11-10T23:59:59+08:00",
        );
        event(
            &db,
            Some(4),
            "app_open",
            "success",
            "2026-11-11T10:00:00+08:00",
        );
        for kind in [
            "risk_assessment",
            "risk_sdk_failure",
            "ad_client_reward_arrived",
            "request",
            "admin_access",
            "invite_prebind",
        ] {
            event(&db, Some(3), kind, "success", "2026-11-10T12:00:00+08:00");
        }
        event(&db, Some(3), "login", "failed", "2026-11-10T12:00:00+08:00");
        event(
            &db,
            Some(3),
            "login",
            "reported",
            "2026-11-10T12:00:00+08:00",
        );
        event(&db, None, "login", "success", "2026-11-10T12:00:00+08:00");
        event(
            &db,
            Some(999),
            "login",
            "success",
            "2026-11-10T12:00:00+08:00",
        );
        event(
            &db,
            Some(2),
            "login",
            "success",
            "2026-11-07T12:00:00+08:00",
        );
        event(
            &db,
            Some(3),
            "login",
            "success",
            "2026-11-11T12:00:01+08:00",
        );
        let value = statistics(
            &db,
            &range("2026-11-08", "2026-11-10"),
            stamp("2026-11-11T12:00:00+08:00"),
        )
        .unwrap();
        let summary = &value["summary"];
        assert_eq!(summary["range_active_users"], 2);
        assert_eq!(summary["range_inactive_users"], 1);
        assert_eq!(summary["range_total_users"], 3);
        assert_eq!(summary["today_active_users"], 1);
        assert_eq!(summary["active_users_7d"], 3);
        assert_eq!(summary["active_users_30d"], 3);
        assert_eq!(value["daily"][0]["active_users"], 2);
        assert_eq!(value["daily"][0]["returning_active"], 1);
        assert_eq!(value["daily"][1]["active_users"], 1);
        assert_eq!(value["daily"][2]["active_users"], 1);
        assert_eq!(value["daily"][2]["returning_active"], 1);
        assert_eq!(value["activity_days"][1]["users"], 2);
    }

    #[test]
    fn distribution_counts_active_days_once_and_rolling_windows_have_exact_boundaries() {
        let db = db();
        for uid in 1..=7 {
            add_user(&db, uid, "2026-10-01T00:00:00+08:00");
        }
        for (uid, days) in [(1, 1), (2, 3), (3, 7), (4, 8)] {
            for day in 0..days {
                let at = format!("2026-11-{:02}T12:00:00+08:00", 10 - day);
                event(&db, Some(uid), "check_in", "success", &at);
                event(&db, Some(uid), "reward_complete", "success", &at);
            }
        }
        event(
            &db,
            Some(5),
            "login",
            "success",
            "2026-10-12T23:59:59+08:00",
        );
        event(
            &db,
            Some(6),
            "login",
            "success",
            "2026-10-13T00:00:00+08:00",
        );
        event(
            &db,
            Some(7),
            "login",
            "success",
            "2026-11-04T23:59:59+08:00",
        );
        let query = range("2026-11-01", "2026-11-10");
        let at = stamp("2026-11-11T12:00:00+08:00");
        let before = statistics(&db, &query, at).unwrap();
        assert_eq!(before["summary"]["range_active_users"], 5);
        assert_eq!(before["summary"]["active_users_7d"], 4);
        assert_eq!(before["summary"]["active_users_30d"], 6);
        assert_eq!(before["activity_days"][0]["users"], 2);
        for bucket in before["activity_days"].as_array().unwrap().iter().skip(1) {
            assert_eq!(bucket["users"], 1);
        }
        event(
            &db,
            Some(7),
            "app_open",
            "success",
            "2026-11-05T00:00:00+08:00",
        );
        let after = statistics(&db, &query, at).unwrap();
        assert_eq!(after["summary"]["active_users_7d"], 5);
        assert_eq!(after["summary"]["range_active_users"], 5);
        assert_eq!(after["activity_days"][0]["users"], 1);
        assert_eq!(after["activity_days"][1]["users"], 2);
    }
}
