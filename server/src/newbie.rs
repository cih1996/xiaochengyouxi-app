//! 新人礼包 / 邀请大红包.
//!
//! * Each "round" (期) is one run of the activity; the admin can start a new round, which lets the
//!   same people take part again (old users invited in a new round count as "old" invitees).
//! * Welcome packet: one per user per round — users registered during the round, or users who
//!   accepted an invitation in this round. Amount follows the opener's own DeviceTrust score; new
//!   users who came through an invitation get a bonus on top (双向奖励).
//! * Invite packets: the inviter gets one packet per invitee once that invitee has done something
//!   real (first check-in or first completed red-packet video after joining) and has a device
//!   score. Amount follows the INVITEE's score, so a farm of black devices only yields the floor.
//! * Every amount is decided here at open time; the page only shows what the server returns.
//!   Ledger source `newbie:{packet_id}` is unique, so a packet can never pay twice.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Tier {
    /// Applies to scores up to and including this value (0 = cleanest, 100 = worst).
    pub max_score: i64,
    pub min_coins: i64,
    pub max_coins: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Config {
    pub enabled: bool,
    pub title: String,
    pub subtitle: String,
    /// Text on the hall's floating badge.
    pub entry_label: String,
    /// Pop the big packet automatically once per round when the user has an unopened welcome packet.
    pub auto_popup: bool,
    pub round: i64,
    pub round_started_at: i64,
    pub welcome_tiers: Vec<Tier>,
    /// Extra percentage on the welcome packet for new users who came through an invitation.
    pub invited_bonus_percent: i64,
    pub inviter_new_tiers: Vec<Tier>,
    pub inviter_old_tiers: Vec<Tier>,
    /// Score used when no DeviceTrust score exists yet for the scored user.
    pub unscored_score: i64,
    /// Invite packets per inviter per round (0 = unlimited).
    pub max_invite_packets: i64,
    pub share_text: String,
    /// After the welcome packet: this many more packets, each claimed by watching a rewarded
    /// video (amount by the user's own score, credited only on the verified ad callback).
    #[serde(default = "default_video_packets")]
    pub video_packets: i64,
    #[serde(default = "default_video_tiers")]
    pub video_tiers: Vec<Tier>,
    #[serde(default)]
    pub revision: i64,
}
fn default_video_packets() -> i64 {
    3
}
fn default_video_tiers() -> Vec<Tier> {
    vec![tier(10, 1000, 3000), tier(30, 500, 1000), tier(60, 100, 300), tier(100, 10, 10)]
}

fn tier(max_score: i64, min_coins: i64, max_coins: i64) -> Tier {
    Tier {
        max_score,
        min_coins,
        max_coins,
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            title: "新人礼包".into(),
            subtitle: "拆开大红包，邀请好友再领更多".into(),
            entry_label: "新人礼包".into(),
            auto_popup: true,
            round: 1,
            round_started_at: now(),
            welcome_tiers: vec![
                tier(10, 3000, 5000),
                tier(30, 1500, 3000),
                tier(60, 300, 800),
                tier(100, 10, 10),
            ],
            invited_bonus_percent: 50,
            inviter_new_tiers: vec![
                tier(10, 3000, 5000),
                tier(30, 1000, 2000),
                tier(60, 100, 300),
                tier(100, 10, 10),
            ],
            inviter_old_tiers: vec![
                tier(10, 200, 500),
                tier(30, 50, 150),
                tier(100, 10, 10),
            ],
            unscored_score: 60,
            max_invite_packets: 100,
            share_text: "我在小橙有喜领到了新人大红包，扫码下载，你也能拆一个！".into(),
            video_packets: default_video_packets(),
            video_tiers: default_video_tiers(),
            revision: 0,
        }
    }
}

fn validate_tiers(name: &str, tiers: &[Tier]) -> Result<(), ApiError> {
    if tiers.is_empty() || tiers.len() > 10 {
        return Err(invalid(&format!("{name}需要 1–10 档")));
    }
    let mut last = -1;
    for t in tiers {
        if t.max_score <= last || !(0..=100).contains(&t.max_score) {
            return Err(invalid(&format!("{name}的分数上限需在 0–100 之间递增")));
        }
        if t.min_coins < 1 || t.max_coins < t.min_coins || t.max_coins > 100_000 {
            return Err(invalid(&format!("{name}的金币范围需为 1–100000，且最小不大于最大")));
        }
        last = t.max_score;
    }
    if last != 100 {
        return Err(invalid(&format!("{name}最后一档的分数上限必须是 100")));
    }
    Ok(())
}

fn validate(c: &Config) -> Result<(), ApiError> {
    for (name, text, max) in [
        ("活动名称", &c.title, 12),
        ("副标题", &c.subtitle, 40),
        ("入口文字", &c.entry_label, 6),
    ] {
        let n = text.trim().chars().count();
        if n == 0 || n > max {
            return Err(invalid(&format!("{name}需为 1–{max} 个字")));
        }
    }
    if c.share_text.chars().count() > 120 {
        return Err(invalid("分享文案最多 120 字"));
    }
    validate_tiers("新人红包档位", &c.welcome_tiers)?;
    validate_tiers("邀请新用户档位", &c.inviter_new_tiers)?;
    validate_tiers("邀请老用户档位", &c.inviter_old_tiers)?;
    validate_tiers("看视频红包档位", &c.video_tiers)?;
    if !(0..=10).contains(&c.video_packets) {
        return Err(invalid("看视频红包个数需在 0–10 之间"));
    }
    if !(0..=300).contains(&c.invited_bonus_percent) {
        return Err(invalid("受邀新用户加成需在 0–300% 之间"));
    }
    if !(0..=100).contains(&c.unscored_score) {
        return Err(invalid("无评分时按的分数需在 0–100 之间"));
    }
    if !(0..=10_000).contains(&c.max_invite_packets) {
        return Err(invalid("每人邀请红包上限需在 0–10000 之间"));
    }
    Ok(())
}

pub(super) fn initialize(db: &Connection) -> Result<(), ApiError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS newbie_config(id INTEGER PRIMARY KEY CHECK(id=1),config_json TEXT NOT NULL,updated_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS newbie_members(round INTEGER NOT NULL,user_id INTEGER NOT NULL,inviter_id INTEGER,kind TEXT NOT NULL,joined_at INTEGER NOT NULL,qualified_at INTEGER,PRIMARY KEY(round,user_id));
         CREATE INDEX IF NOT EXISTS idx_newbie_members_inviter ON newbie_members(round,inviter_id);
         CREATE TABLE IF NOT EXISTS newbie_packets(id TEXT PRIMARY KEY,round INTEGER NOT NULL,user_id INTEGER NOT NULL,kind TEXT NOT NULL,invitee_id INTEGER NOT NULL DEFAULT 0,status TEXT NOT NULL DEFAULT 'ready',coins INTEGER,score INTEGER,created_at INTEGER NOT NULL,opened_at INTEGER,UNIQUE(round,user_id,kind,invitee_id));
         CREATE INDEX IF NOT EXISTS idx_newbie_packets_user ON newbie_packets(user_id,round);",
    )?;
    let has_session: bool = db
        .query_row("SELECT COUNT(*) FROM pragma_table_info('newbie_packets') WHERE name='session_id'", [], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !has_session {
        db.execute_batch("ALTER TABLE newbie_packets ADD COLUMN session_id TEXT")?;
    }
    db.execute(
        "INSERT OR IGNORE INTO newbie_config VALUES(1,?1,?2)",
        params![serde_json::to_string(&Config::default())?, now()],
    )?;
    Ok(())
}

fn config(db: &Connection) -> Result<Config, ApiError> {
    Ok(serde_json::from_str(&db.query_row::<String, _, _>(
        "SELECT config_json FROM newbie_config WHERE id=1",
        [],
        |r| r.get(0),
    )?)?)
}

/// Latest DeviceTrust score for a user; restricted accounts count as the worst score.
fn score_of(db: &Connection, uid: i64, c: &Config) -> Result<(i64, bool), ApiError> {
    if risk::check_account(db, uid).is_err() {
        return Ok((100, true));
    }
    let score: Option<i64> = db
        .query_row(
            "SELECT score FROM risk_events WHERE user_id=?1 AND score IS NOT NULL ORDER BY created_at DESC LIMIT 1",
            [uid],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match score {
        Some(s) => (s.clamp(0, 100), true),
        None => (c.unscored_score, false),
    })
}

fn amount(tiers: &[Tier], score: i64) -> i64 {
    let t = tiers
        .iter()
        .find(|t| score <= t.max_score)
        .unwrap_or_else(|| tiers.last().unwrap());
    let span = (t.max_coins - t.min_coins + 1).max(1) as u128;
    t.min_coins + (Uuid::new_v4().as_u128() % span) as i64
}

/// Highest amount a tier list can pay (shown as "最高 xxx 金币").
fn top(tiers: &[Tier]) -> i64 {
    tiers.iter().map(|t| t.max_coins).max().unwrap_or(0)
}

fn member(db: &Connection, round: i64, uid: i64) -> Result<Option<(Option<i64>, String)>, ApiError> {
    Ok(db
        .query_row(
            "SELECT inviter_id,kind FROM newbie_members WHERE round=?1 AND user_id=?2",
            params![round, uid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

/// Brings membership, qualification and packets up to date for `uid` (as invitee and inviter).
fn sync(tx: &Connection, uid: i64, c: &Config) -> Result<(), ApiError> {
    let round = c.round;
    // New users invited during this round become members automatically.
    tx.execute(
        "INSERT OR IGNORE INTO newbie_members(round,user_id,inviter_id,kind,joined_at)
         SELECT ?1,id,inviter_id,'new',COALESCE(invitation_bound_at,created_at) FROM users
         WHERE (id=?2 OR inviter_id=?2) AND inviter_id IS NOT NULL AND created_at>=?3 AND invitation_bound_at IS NOT NULL",
        params![round, uid, c.round_started_at],
    )?;
    // Welcome packet: every user, new or existing, once per round (amount follows their own score).
    tx.execute(
        "INSERT OR IGNORE INTO newbie_packets(id,round,user_id,kind,invitee_id,created_at) VALUES(?1,?2,?3,'welcome',0,?4)",
        params![Uuid::new_v4().to_string(), round, uid, now()],
    )?;
    // Video packets appear once the welcome packet has been opened.
    let welcome_opened: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='welcome' AND status='opened')",
        params![round, uid],
        |r| r.get(0),
    )?;
    if welcome_opened {
        for i in 1..=c.video_packets {
            tx.execute(
                "INSERT OR IGNORE INTO newbie_packets(id,round,user_id,kind,invitee_id,created_at) VALUES(?1,?2,?3,'video',?4,?5)",
                params![Uuid::new_v4().to_string(), round, uid, i, now()],
            )?;
        }
    }
    // Invitees qualify after a real action (first check-in or completed packet video) + a score.
    let pending: Vec<(i64, i64)> = tx
        .prepare("SELECT user_id,joined_at FROM newbie_members WHERE round=?1 AND inviter_id=?2 AND qualified_at IS NULL")?
        .query_map(params![round, uid], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (invitee, joined) in pending {
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM ledger WHERE user_id=?1 AND created_at>=?2 AND title IN ('每日签到','连续签到补签'))
                 OR EXISTS(SELECT 1 FROM reward_sessions WHERE user_id=?1 AND completed_at>=?2)",
            params![invitee, joined],
            |r| r.get(0),
        )?;
        let scored: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM risk_events WHERE user_id=?1 AND score IS NOT NULL)",
            [invitee],
            |r| r.get(0),
        )?;
        if active && scored {
            tx.execute(
                "UPDATE newbie_members SET qualified_at=?1 WHERE round=?2 AND user_id=?3",
                params![now(), round, invitee],
            )?;
        }
    }
    let qualified: Vec<i64> = tx
        .prepare("SELECT user_id FROM newbie_members WHERE round=?1 AND inviter_id=?2 AND qualified_at IS NOT NULL ORDER BY qualified_at")?
        .query_map(params![round, uid], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut have: i64 = tx.query_row(
        "SELECT COUNT(*) FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='invite'",
        params![round, uid],
        |r| r.get(0),
    )?;
    for invitee in qualified {
        if c.max_invite_packets > 0 && have >= c.max_invite_packets {
            break;
        }
        have += tx.execute(
            "INSERT OR IGNORE INTO newbie_packets(id,round,user_id,kind,invitee_id,created_at) VALUES(?1,?2,?3,'invite',?4,?5)",
            params![Uuid::new_v4().to_string(), round, uid, invitee, now()],
        )? as i64;
    }
    Ok(())
}

fn mask_name(nickname: &str, phone: &str) -> String {
    let nick = nickname.trim();
    if !nick.is_empty() && !nick.chars().all(|c| c.is_ascii_digit()) {
        let chars: Vec<char> = nick.chars().collect();
        return if chars.len() <= 1 {
            format!("{nick}*")
        } else {
            format!("{}*{}", chars[0], chars[chars.len() - 1])
        };
    }
    if phone.len() >= 11 {
        format!("{}****{}", &phone[..3], &phone[phone.len() - 4..])
    } else {
        "新朋友".into()
    }
}

fn state_json(state: &AppState, db: &Connection, uid: i64, c: &Config) -> Result<Value, ApiError> {
    let round = c.round;
    let welcome = db
        .query_row(
            "SELECT id,status,coins FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='welcome'",
            params![round, uid],
            |r| Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"coins":r.get::<_,Option<i64>>(2)?})),
        )
        .optional()?;
    let me = member(db, round, uid)?;
    let invited_new = matches!(&me, Some((Some(_), kind)) if kind == "new");
    let ready: Vec<String> = db
        .prepare("SELECT id FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='invite' AND status='ready' ORDER BY created_at")?
        .query_map(params![round, uid], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let (opened_count, invite_coins): (i64, i64) = db.query_row(
        "SELECT COUNT(*),COALESCE(SUM(coins),0) FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='invite' AND status='opened'",
        params![round, uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let friends: Vec<Value> = db
        .prepare(
            "SELECT m.kind,m.joined_at,m.qualified_at,u.nickname,u.phone,p.status,p.coins FROM newbie_members m JOIN users u ON u.id=m.user_id
             LEFT JOIN newbie_packets p ON p.round=m.round AND p.user_id=m.inviter_id AND p.kind='invite' AND p.invitee_id=m.user_id
             WHERE m.round=?1 AND m.inviter_id=?2 ORDER BY m.joined_at DESC LIMIT 100",
        )?
        .query_map(params![round, uid], |r| {
            let qualified = r.get::<_, Option<i64>>(2)?.is_some();
            let status: Option<String> = r.get(5)?;
            let progress = match (qualified, status.as_deref()) {
                (_, Some("opened")) => "opened",
                (_, Some(_)) => "ready",
                (true, None) => "limit",
                (false, _) => "pending",
            };
            Ok(json!({
                "name": mask_name(&r.get::<_, String>(3)?, &r.get::<_, String>(4)?),
                "kind": r.get::<_, String>(0)?,
                "joined_at": iso(r.get(1)?),
                "progress": progress,
                "coins": r.get::<_, Option<i64>>(6)?,
            }))
        })?
        .collect::<Result<_, _>>()?;
    let code: String = db.query_row("SELECT invite_code FROM users WHERE id=?1", [uid], |r| r.get(0))?;
    let bonus = if invited_new { c.invited_bonus_percent } else { 0 };
    // Packets the user can open now, in the order the page offers them: welcome, a revealed video
    // packet still waiting for its video, unopened video packets, then invite packets.
    let queue: Vec<Value> = db
        .prepare(
            "SELECT id,kind,status,CASE WHEN kind='video' THEN NULL ELSE coins END FROM newbie_packets WHERE round=?1 AND user_id=?2 AND status IN ('ready','revealed','ad_started')
             ORDER BY CASE kind WHEN 'welcome' THEN 0 WHEN 'video' THEN 1 ELSE 2 END, CASE status WHEN 'ready' THEN 1 ELSE 0 END, invitee_id, created_at",
        )?
        .query_map(params![round, uid], |r| {
            Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"coins":r.get::<_,Option<i64>>(3)?}))
        })?
        .collect::<Result<_, _>>()?;
    let (video_total, video_done, video_coins): (i64, i64, i64) = db.query_row(
        "SELECT COUNT(*),COALESCE(SUM(status='opened'),0),COALESCE(SUM(CASE WHEN status='opened' THEN coins END),0) FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='video'",
        params![round, uid],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let welcome_coins: i64 = db.query_row(
        "SELECT COALESCE(SUM(coins),0) FROM newbie_packets WHERE round=?1 AND user_id=?2 AND kind='welcome' AND status='opened'",
        params![round, uid],
        |r| r.get(0),
    )?;
    Ok(json!({
        "enabled": c.enabled,
        "round": round,
        "title": c.title,
        "subtitle": c.subtitle,
        "entry_label": c.entry_label,
        "auto_popup": c.auto_popup,
        "welcome": welcome,
        "invited": invited_new,
        "bonus_percent": bonus,
        "welcome_max": top(&c.welcome_tiers) * (100 + bonus) / 100,
        "invited_max": top(&c.welcome_tiers) * (100 + c.invited_bonus_percent) / 100,
        "invite_max": top(&c.inviter_new_tiers),
        "ready_invites": ready,
        "queue": queue,
        "available": queue.len(),
        "video_total": video_total,
        "video_left": video_total - video_done,
        "video_max": top(&c.video_tiers),
        "total_coins": welcome_coins + video_coins + invite_coins,
        "opened_invites": opened_count,
        "invite_coins": invite_coins,
        "friends": friends,
        "invite_code": code,
        "invite_url": invitations::share_url(state, &code),
        "share_text": c.share_text,
    }))
}

async fn runtime(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    let c = config(&db)?;
    if !c.enabled {
        return Ok(Json(json!({"enabled":false})));
    }
    let tx = db.transaction()?;
    sync(&tx, uid, &c)?;
    let result = state_json(&state, &tx, uid, &c)?;
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct OpenInput {
    packet_id: String,
}

async fn open_packet(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<OpenInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    invitations::require_bound(&db, uid)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("活动已结束，敬请期待下一期"));
    }
    sync(&tx, uid, &c)?;
    let row: Option<(String, i64, String, Option<i64>)> = tx
        .query_row(
            "SELECT kind,invitee_id,status,coins FROM newbie_packets WHERE id=?1 AND user_id=?2 AND round=?3",
            params![input.packet_id, uid, c.round],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((kind, invitee, status, coins)) = row else {
        return Err(ApiError(StatusCode::NOT_FOUND, "红包不存在或已过期".into(), None));
    };
    if status == "opened" {
        let result = json!({"coins":coins.unwrap_or(0),"kind":kind,"already":true,"user":user(&tx,uid)?});
        return Ok(Json(result));
    }
    if kind == "video" {
        // Reveal only: the amount is fixed now and paid through the rewarded-video pipeline
        // (task newbie:{id}), i.e. only after the ad network's verified callback.
        let coins = match coins {
            Some(c) => c,
            None => {
                let (score, scored) = score_of(&tx, uid, &c)?;
                let amount = amount(&c.video_tiers, score);
                tx.execute(
                    "UPDATE newbie_packets SET status='revealed',coins=?1,score=?2 WHERE id=?3 AND status='ready'",
                    params![amount, score, input.packet_id],
                )?;
                user_admin::behavior(
                    &tx,
                    &context,
                    Some(uid),
                    None,
                    "newbie_reveal",
                    "success",
                    &input.packet_id,
                    Some(amount as f64 / COINS_PER_YUAN as f64),
                    json!({"round":c.round,"score":score,"scored":scored}),
                )?;
                amount
            }
        };
        // Blind box: the amount is locked here but only shown after the video has paid it.
        let _ = coins;
        let result = json!({"kind":kind,"already":false,"needs_video":true,"task_id":format!("newbie:{}",input.packet_id),"state":state_json(&state,&tx,uid,&c)?});
        tx.commit()?;
        return Ok(Json(result));
    }
    let (coins, score, scored) = if kind == "welcome" {
        let (score, scored) = score_of(&tx, uid, &c)?;
        let base = amount(&c.welcome_tiers, score);
        let bonus = match member(&tx, c.round, uid)? {
            Some((Some(_), k)) if k == "new" => c.invited_bonus_percent,
            _ => 0,
        };
        (base * (100 + bonus) / 100, score, scored)
    } else {
        let (score, scored) = score_of(&tx, invitee, &c)?;
        let old = member(&tx, c.round, invitee)?.is_some_and(|(_, k)| k == "old");
        let tiers = if old { &c.inviter_old_tiers } else { &c.inviter_new_tiers };
        (amount(tiers, score), score, scored)
    };
    let changed = tx.execute(
        "UPDATE newbie_packets SET status='opened',coins=?1,score=?2,opened_at=?3 WHERE id=?4 AND status='ready'",
        params![coins, score, now(), input.packet_id],
    )?;
    if changed == 0 {
        return Err(conflict("红包正在打开，请稍候"));
    }
    let title = if kind == "welcome" { "新人大红包" } else { "邀请好友大红包" };
    credit(&tx, uid, coins, title, &format!("newbie:{}", input.packet_id))?;
    user_admin::behavior(
        &tx,
        &context,
        Some(uid),
        None,
        "newbie_open",
        "success",
        &input.packet_id,
        Some(coins as f64 / COINS_PER_YUAN as f64),
        json!({"kind":kind,"round":c.round,"invitee_id":invitee,"score":score,"scored":scored}),
    )?;
    sync(&tx, uid, &c)?; // opening the welcome packet unlocks the video packets
    let result = json!({"coins":coins,"kind":kind,"already":false,"user":user(&tx,uid)?,"state":state_json(&state,&tx,uid,&c)?});
    tx.commit()?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct AcceptInput {
    invite_code: String,
}

/// An existing user scanned someone's poster (App opened through the invite link): joins this
/// round as that person's invitee. New users are attributed through the normal invite binding.
async fn accept(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(input): Json<AcceptInput>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let uid = auth(&db, &headers)?;
    invitations::require_bound(&db, uid)?;
    let tx = db.transaction()?;
    let c = config(&tx)?;
    if !c.enabled {
        return Err(conflict("活动已结束，敬请期待下一期"));
    }
    let code = input.invite_code.trim().to_ascii_uppercase();
    let inviter: Option<i64> = tx
        .query_row(
            "SELECT id FROM users WHERE invite_code=?1 AND invitation_bound_at IS NOT NULL",
            [&code],
            |r| r.get(0),
        )
        .optional()?;
    let Some(inviter) = inviter else {
        return Err(invalid("邀请码无效"));
    };
    if inviter == uid {
        return Err(invalid("不能接受自己的邀请"));
    }
    sync(&tx, uid, &c)?;
    let existing = member(&tx, c.round, uid)?;
    let joined = if existing.is_none() {
        let created: i64 = tx.query_row("SELECT created_at FROM users WHERE id=?1", [uid], |r| r.get(0))?;
        let kind = if created >= c.round_started_at { "new" } else { "old" };
        tx.execute(
            "INSERT OR IGNORE INTO newbie_members(round,user_id,inviter_id,kind,joined_at) VALUES(?1,?2,?3,?4,?5)",
            params![c.round, uid, inviter, kind, now()],
        )?;
        sync(&tx, uid, &c)?;
        user_admin::behavior(
            &tx,
            &context,
            Some(uid),
            None,
            "newbie_accept",
            "success",
            &code,
            None,
            json!({"round":c.round,"inviter_id":inviter,"kind":kind}),
        )?;
        true
    } else {
        false
    };
    let result = json!({"joined":joined,"state":state_json(&state,&tx,uid,&c)?});
    tx.commit()?;
    Ok(Json(result))
}

async fn admin_get(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let c = config(&db)?;
    let round = c.round;
    let count = |sql: &str| -> Result<i64, ApiError> { Ok(db.query_row(sql, [round], |r| r.get(0))?) };
    let stats = json!({
        "members": count("SELECT COUNT(*) FROM newbie_members WHERE round=?1")?,
        "members_new": count("SELECT COUNT(*) FROM newbie_members WHERE round=?1 AND kind='new'")?,
        "members_old": count("SELECT COUNT(*) FROM newbie_members WHERE round=?1 AND kind='old'")?,
        "qualified": count("SELECT COUNT(*) FROM newbie_members WHERE round=?1 AND qualified_at IS NOT NULL")?,
        "welcome_opened": count("SELECT COUNT(*) FROM newbie_packets WHERE round=?1 AND kind='welcome' AND status='opened'")?,
        "welcome_coins": count("SELECT COALESCE(SUM(coins),0) FROM newbie_packets WHERE round=?1 AND kind='welcome' AND status='opened'")?,
        "invite_opened": count("SELECT COUNT(*) FROM newbie_packets WHERE round=?1 AND kind='invite' AND status='opened'")?,
        "invite_ready": count("SELECT COUNT(*) FROM newbie_packets WHERE round=?1 AND kind='invite' AND status='ready'")?,
        "invite_coins": count("SELECT COALESCE(SUM(coins),0) FROM newbie_packets WHERE round=?1 AND kind='invite' AND status='opened'")?,
    });
    let recent: Vec<Value> = db
        .prepare(
            "SELECT p.opened_at,p.kind,p.coins,p.score,u.id,u.username,u.phone,p.invitee_id,i.phone FROM newbie_packets p JOIN users u ON u.id=p.user_id
             LEFT JOIN users i ON i.id=p.invitee_id WHERE p.round=?1 AND p.status='opened' ORDER BY p.opened_at DESC LIMIT 50",
        )?
        .query_map([round], |r| {
            Ok(json!({"opened_at":iso(r.get(0)?),"kind":r.get::<_,String>(1)?,"coins":r.get::<_,i64>(2)?,"score":r.get::<_,Option<i64>>(3)?,
                "user_id":r.get::<_,i64>(4)?,"username":r.get::<_,Option<String>>(5)?,"phone":r.get::<_,String>(6)?,
                "invitee_id":r.get::<_,i64>(7)?,"invitee_phone":r.get::<_,Option<String>>(8)?}))
        })?
        .collect::<Result<_, _>>()?;
    let mut value = serde_json::to_value(&c)?;
    value["round_started_at"] = json!(iso(c.round_started_at));
    Ok(Json(json!({"config":value,"stats":stats,"recent":recent})))
}

#[derive(Deserialize)]
struct AdminInput {
    #[serde(flatten)]
    config: Value,
    #[serde(default)]
    new_round: bool,
}

async fn admin_put(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(input): Json<AdminInput>,
) -> ApiResult<Value> {
    {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    let old = config(&tx)?;
    let mut body = input.config;
    // Round bookkeeping is server-owned.
    body["round"] = json!(old.round);
    body["round_started_at"] = json!(old.round_started_at);
    body["revision"] = json!(old.revision);
    let mut next: Config = serde_json::from_value(body).map_err(|e| invalid(&format!("配置格式错误：{e}")))?;
    validate(&next)?;
    if input.new_round {
        next.round = old.round + 1;
        next.round_started_at = now();
    }
    next.revision = old.revision + 1;
    tx.execute(
        "UPDATE newbie_config SET config_json=?1,updated_at=?2 WHERE id=1",
        params![serde_json::to_string(&next)?, now()],
    )?;
    user_admin::action_audit(
        &tx,
        &context,
        None,
        if input.new_round { "newbie_new_round" } else { "newbie_config_update" },
        "newbie",
        if input.new_round { "开启新人礼包新一期" } else { "修改新人礼包配置" },
        serde_json::to_value(&old)?,
        serde_json::to_value(&next)?,
    )?;
    tx.commit()?;
    }
    admin_get(State(state)).await
}

// ---------- rewarded-video pipeline hooks (task id "newbie:{packet_id}") ----------

/// Quote/start: a revealed video packet with its fixed amount.
pub(super) fn video_task(state: &AppState, db: &Connection, uid: i64, pid: &str) -> Result<rain::GrandTask, ApiError> {
    if state.config.ad_mode != "csj" {
        return Err(conflict("请在已接入广告的 APP 中领取"));
    }
    let row: Option<(i64, String, Option<i64>, Option<String>)> = db
        .query_row(
            "SELECT round,status,coins,session_id FROM newbie_packets WHERE id=?1 AND user_id=?2 AND kind='video'",
            params![pid, uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((round, status, coins, session)) = row else {
        return Err(ApiError(StatusCode::NOT_FOUND, "红包不存在".into(), None));
    };
    let c = config(db)?;
    if session.is_none() && (!c.enabled || round != c.round) {
        return Err(conflict("本期活动已结束"));
    }
    if !["revealed", "ad_started"].contains(&status.as_str()) {
        return Err(conflict("请先拆开这个红包"));
    }
    Ok(rain::GrandTask {
        packet_id: pid.into(),
        coins: coins.unwrap_or(0),
        session_id: session,
    })
}

/// Start: ties the reward session to the packet. Returns false when `pid` is not ours.
pub(super) fn attach_session(db: &Connection, uid: i64, pid: &str, sid: &str) -> Result<bool, ApiError> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM newbie_packets WHERE id=?1 AND user_id=?2 AND kind='video')",
        params![pid, uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(false);
    }
    let changed = db.execute(
        "UPDATE newbie_packets SET status='ad_started',session_id=?1 WHERE id=?2 AND user_id=?3 AND kind='video' AND status='revealed' AND session_id IS NULL",
        params![sid, pid, uid],
    )?;
    if changed != 1 {
        return Err(conflict("该红包已处理，请刷新后重试"));
    }
    Ok(true)
}

pub(super) fn completion_packet(db: &Connection, uid: i64, pid: &str, sid: &str) -> Result<String, ApiError> {
    let valid: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM newbie_packets WHERE id=?1 AND user_id=?2 AND kind='video' AND status='ad_started' AND session_id=?3)",
        params![pid, uid, sid],
        |r| r.get(0),
    )?;
    if !valid {
        return Err(conflict("红包奖励状态不匹配"));
    }
    Ok(pid.into())
}

pub(super) fn finish_video(db: &Connection, uid: i64, pid: &str, sid: &str, coins: i64) -> Result<(), ApiError> {
    let changed = db.execute(
        "UPDATE newbie_packets SET status='opened',opened_at=?1 WHERE id=?2 AND user_id=?3 AND session_id=?4 AND coins=?5 AND status='ad_started'",
        params![now(), pid, uid, sid, coins],
    )?;
    if changed != 1 {
        return Err(conflict("红包奖励状态不匹配"));
    }
    Ok(())
}

// ---------- winners board (cached: one query per 5 minutes per round) ----------
static BOARD: std::sync::Mutex<Option<(i64, i64, Value)>> = std::sync::Mutex::new(None);

fn mask_phone(phone: &str) -> String {
    if phone.len() >= 11 {
        format!("{}****{}", &phone[..3], &phone[phone.len() - 2..])
    } else {
        "小橙用户".into()
    }
}

async fn board(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    let c = config(&db)?;
    if !c.enabled {
        return Ok(Json(json!({"items":[]})));
    }
    if let Some((round, at, value)) = BOARD.lock().unwrap().as_ref() {
        if *round == c.round && now() - at < 300 {
            return Ok(Json(value.clone()));
        }
    }
    let items: Vec<Value> = db
        .prepare(
            "SELECT u.phone,SUM(p.coins) total FROM newbie_packets p JOIN users u ON u.id=p.user_id
             WHERE p.round=?1 AND p.status='opened' GROUP BY p.user_id ORDER BY total DESC LIMIT 20",
        )?
        .query_map([c.round], |r| Ok(json!({"name":mask_phone(&r.get::<_,String>(0)?),"coins":r.get::<_,i64>(1)?})))?
        .collect::<Result<_, _>>()?;
    let value = json!({"items":items,"cached_at":iso(now())});
    *BOARD.lock().unwrap() = Some((c.round, now(), value.clone()));
    Ok(Json(value))
}

/// For the invite landing page (no login): whether the activity runs and the headline amount an
/// invited new user can get.
async fn public_info(State(state): State<AppState>) -> ApiResult<Value> {
    let db = open(&state)?;
    // Landing page shows the SMS form only when the human check is set up.
    let captcha = human_check::public_app_id(&db)?;
    let download = state.config.app_download_url.clone().unwrap_or_default();
    let c = config(&db)?;
    if !c.enabled {
        return Ok(Json(json!({"enabled":false,"captcha_app_id":captcha,"download_url":download})));
    }
    Ok(Json(json!({
        "enabled": true,
        "captcha_app_id": captcha,
        "download_url": download,
        "title": c.title,
        "invited_max": top(&c.welcome_tiers) * (100 + c.invited_bonus_percent) / 100,
        "bonus_percent": c.invited_bonus_percent,
    })))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/newbie/public", get(public_info))
        .route("/newbie/board", get(board))
        .route("/newbie", get(runtime))
        .route("/newbie/open", post(open_packet))
        .route("/newbie/accept", post(accept))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/newbie", get(admin_get).put(admin_put))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::collections::HashMap;

    /// Welcome packet pays directly; a video packet only reveals its amount, and the coins arrive
    /// only through the rewarded-video pipeline after a correctly signed ad callback — once.
    #[tokio::test]
    async fn video_packet_pays_only_after_verified_callback() {
        let (state, context, mut headers, _) = rain::tests::fixture(true, true);
        let db = open(&state).unwrap();
        let mut c = config(&db).unwrap();
        c.enabled = true;
        db.execute("UPDATE newbie_config SET config_json=?1", [serde_json::to_string(&c).unwrap()]).unwrap();
        let st = runtime(State(state.clone()), headers.clone()).await.unwrap().0;
        assert_eq!(st["available"], 1);
        let welcome = st["welcome"]["id"].as_str().unwrap().to_string();
        let opened = open_packet(State(state.clone()), Extension(context.clone()), headers.clone(), Json(OpenInput { packet_id: welcome })).await.unwrap().0;
        let after_welcome = opened["user"]["coins"].as_i64().unwrap();
        let st = opened["state"].clone();
        assert_eq!(st["video_total"], 3);
        assert_eq!(st["available"], 3);
        let pid = st["queue"][0]["id"].as_str().unwrap().to_string();
        assert_eq!(st["queue"][0]["kind"], "video");
        let revealed = open_packet(State(state.clone()), Extension(context.clone()), headers.clone(), Json(OpenInput { packet_id: pid.clone() })).await.unwrap().0;
        assert_eq!(revealed["needs_video"], true);
        // Blind box: the response never carries the locked amount.
        assert!(revealed.get("coins").is_none());
        let coins: i64 = db.query_row("SELECT coins FROM newbie_packets WHERE id=?1", [&pid], |r| r.get(0)).unwrap();
        assert!(coins >= 10);
        // Revealing pays nothing.
        assert_eq!(user(&db, 2).unwrap().coins, after_welcome);
        let task = format!("newbie:{pid}");
        let quote = reward_policy::test_quote(&state, &headers, &task, None).await;
        assert_eq!(quote["reward"], coins);
        let qid = quote["quote_id"].as_str().unwrap().to_owned();
        business_guard::test_event(&state, "newbie-proof", 2, "reward", json!({"task_id":task,"quote_id":qid}));
        headers.insert("x-risk-event", HeaderValue::from_static("newbie-proof"));
        let session = super::super::reward_start(
            State(state.clone()),
            Extension(context.clone()),
            headers.clone(),
            Json(super::super::RewardInput { task_id: task.clone(), quote_id: qid }),
        )
        .await
        .unwrap()
        .0;
        let sid = session["id"].as_str().unwrap().to_string();
        // No callback yet: nothing to complete.
        assert!(super::super::reward_complete(State(state.clone()), Extension(context.clone()), headers.clone(), Path(sid.clone())).await.is_err());
        let trans = "newbie-callback-transaction";
        let mut query = HashMap::from([
            ("user_id".to_string(), "2".to_string()),
            ("trans_id".to_string(), trans.to_string()),
            ("reward_name".to_string(), "金币".to_string()),
            ("reward_amount".to_string(), "99999".to_string()),
            ("extra".to_string(), session["media_extra"].as_str().unwrap().to_string()),
            ("sign".to_string(), "forged".to_string()),
        ]);
        assert!(!super::super::verify_csj_callback(&state, &query).unwrap().valid);
        query.insert("sign".into(), format!("{:x}", Sha256::digest(format!("rain-test-secret:{trans}").as_bytes())));
        assert!(super::super::verify_csj_callback(&state, &query).unwrap().valid);
        let paid = super::super::reward_complete(State(state.clone()), Extension(context.clone()), headers.clone(), Path(sid.clone())).await.unwrap().0;
        assert_eq!(paid["reward"], coins);
        assert_eq!(paid["user"]["coins"].as_i64().unwrap(), after_welcome + coins);
        let repeat = super::super::reward_complete(State(state.clone()), Extension(context), headers.clone(), Path(sid)).await.unwrap().0;
        assert_eq!(repeat["already_completed"], true);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM ledger WHERE source=?1", [format!("newbie:{pid}")], |r| r.get::<_, i64>(0)).unwrap(), 1);
        let st = runtime(State(state), headers).await.unwrap().0;
        assert_eq!(st["video_left"], 2);
        assert_eq!(st["available"], 2);
    }

    #[test]
    fn tiers_pick_by_score_and_stay_in_range() {
        let c = Config::default();
        for _ in 0..200 {
            let clean = amount(&c.welcome_tiers, 0);
            assert!((3000..=5000).contains(&clean));
            assert_eq!(amount(&c.inviter_new_tiers, 100), 10);
            let mid = amount(&c.inviter_new_tiers, 25);
            assert!((1000..=2000).contains(&mid));
        }
        assert!(validate(&c).is_ok());
        let mut bad = c.clone();
        bad.welcome_tiers = vec![tier(50, 10, 20)];
        assert!(validate(&bad).is_err());
    }
}
