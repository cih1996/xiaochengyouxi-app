use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Deserialize)]
struct GraphQuery {
    up_depth: Option<usize>,
    down_depth: Option<usize>,
    limit: Option<usize>,
}

fn missing_user() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "用户不存在".into(), None)
}

fn basic(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let profile = db.query_row(
        "SELECT u.id,u.nickname,u.phone,u.invite_code,u.coins,u.created_at,COALESCE(a.status,'active'),
          (SELECT COUNT(*) FROM users c WHERE c.inviter_id=u.id AND c.invitation_bound_at IS NOT NULL),
          (SELECT COUNT(*) FROM account_devices d WHERE d.user_id=u.id),
          (SELECT MAX(created_at) FROM user_behavior b WHERE b.user_id=u.id AND b.status='success' AND b.kind IN ('register','login','app_open','check_in','hall_request','reward_start','reward_complete','withdrawal','rain_join','rain_small_claim','rain_large_open','rain_large_claim'))
         FROM users u LEFT JOIN account_restrictions a ON a.user_id=u.id WHERE u.id=?1",
        [uid],
        |r| Ok(json!({"id":r.get::<_,i64>(0)?,"nickname":r.get::<_,String>(1)?,"phone":r.get::<_,String>(2)?,"invite_code":r.get::<_,String>(3)?,"coins":r.get::<_,i64>(4)?,"created_at":iso(r.get(5)?),"account_status":r.get::<_,String>(6)?,"children_count":r.get::<_,i64>(7)?,"device_count":r.get::<_,i64>(8)?,"last_active_at":r.get::<_,Option<i64>>(9)?.map(iso)})),
    ).optional()?.ok_or_else(missing_user)?;
    let mut profile = profile;
    profile["risk"] = behavior_audit::risk_summary(db, uid)?;
    Ok(profile)
}

// Paths retain their parents, so a broad first generation still shows deeper relationships.
fn graph_data(db: &Connection, uid: i64, query: &GraphQuery) -> Result<Value, ApiError> {
    basic(db, uid)?;
    let up = query.up_depth.unwrap_or(3);
    let down = query.down_depth.unwrap_or(3);
    let limit = query.limit.unwrap_or(20);
    if !(1..=6).contains(&up) || !(1..=6).contains(&down) || !(1..=20).contains(&limit) {
        return Err(invalid("关系层级为1至6级，每图1至20位用户"));
    }
    let mut selected = BTreeMap::from([(uid, 0_i64)]);
    let mut ancestors = BTreeSet::from([uid]);
    let mut current = uid;
    let mut cycle = false;
    let mut upstream_total = 0;
    for depth in 1..=up {
        let parent: Option<i64> = db
            .query_row(
                "SELECT inviter_id FROM users WHERE id=?1 AND invitation_bound_at IS NOT NULL",
                [current],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        let Some(parent) = parent else { break };
        if !ancestors.insert(parent) {
            cycle = true;
            break;
        }
        upstream_total += 1;
        if selected.len() < limit {
            selected.insert(parent, -(depth as i64));
        }
        current = parent;
    }
    let tree = "WITH RECURSIVE relatives(id,depth,path,cycle) AS (
      SELECT ?1,0,','||?1||',',0
      UNION ALL
      SELECT u.id,r.depth+1,r.path||u.id||',',instr(r.path,','||u.id||',')>0
      FROM users u JOIN relatives r ON u.inviter_id=r.id
      WHERE u.invitation_bound_at IS NOT NULL AND r.depth<?2 AND r.cycle=0
    ) ";
    let total: i64 = db.query_row(
        &format!("{tree} SELECT COUNT(DISTINCT id) FROM relatives WHERE depth>0 AND cycle=0"),
        params![uid, down],
        |r| r.get(0),
    )?;
    cycle |= db.query_row(
        &format!("{tree} SELECT EXISTS(SELECT 1 FROM relatives WHERE cycle=1)"),
        params![uid, down],
        |r| r.get::<_, bool>(0),
    )?;
    let paths = db.prepare(&format!("{tree} SELECT path FROM relatives WHERE depth>0 AND cycle=0 ORDER BY depth DESC,id DESC LIMIT 80"))?
        .query_map(params![uid,down], |r| r.get::<_,String>(0))?
        .collect::<Result<Vec<_>,_>>()?;
    for path in paths {
        let ids = path
            .trim_matches(',')
            .split(',')
            .map(|v| v.parse::<i64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
        for (depth, id) in ids.into_iter().enumerate().skip(1) {
            if selected.contains_key(&id) {
                continue;
            }
            if selected.len() == limit {
                break;
            }
            selected.insert(id, depth as i64);
        }
        if selected.len() == limit {
            break;
        }
    }
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (id, depth) in &selected {
        let mut node = basic(db, *id)?;
        node["depth"] = json!(depth);
        node["is_center"] = json!(*id == uid);
        let visible_children = db
            .prepare(
                "SELECT id FROM users WHERE inviter_id=?1 AND invitation_bound_at IS NOT NULL",
            )?
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .filter(|child| selected.contains_key(child))
            .count();
        node["hidden_children_count"] =
            json!((node["children_count"].as_i64().unwrap() - visible_children as i64).max(0));
        let relation: Option<(i64,i64)> = db.query_row(
            "SELECT inviter_id,invitation_bound_at FROM users WHERE id=?1 AND inviter_id IS NOT NULL AND invitation_bound_at IS NOT NULL",
            [id], |r| Ok((r.get(0)?,r.get(1)?)),
        ).optional()?;
        if let Some((parent, stamp)) = relation {
            if selected.contains_key(&parent) {
                edges.push(
                    json!({"source":parent,"target":id,"kind":"invitation","bound_at":iso(stamp)}),
                );
            }
        }
        nodes.push(node);
    }
    nodes.sort_by_key(|n| (n["depth"].as_i64().unwrap(), n["id"].as_i64().unwrap()));
    let mut shared: BTreeMap<String, (String, i64, Vec<i64>)> = BTreeMap::new();
    for id in selected.keys() {
        let rows = db.prepare("SELECT device_id,model,(SELECT COUNT(*) FROM account_devices x WHERE x.device_id=d.device_id) FROM account_devices d WHERE user_id=?1")?
            .query_map([id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?)))?
            .collect::<Result<Vec<_>,_>>()?;
        for (device, model, count) in rows {
            shared
                .entry(device)
                .or_insert((model, count, Vec::new()))
                .2
                .push(*id);
        }
    }
    let mut device_links = Vec::new();
    for (device, (model, count, user_ids)) in shared {
        if user_ids.len() > 1 {
            let score: Option<i64> = db.query_row("SELECT MAX(score) FROM risk_events WHERE device_identity=?1 AND status='reported' AND updated_at>=?2",params![device,now()-30*86400],|r|r.get(0))?;
            device_links.push(json!({"device_id":device,"user_ids":user_ids,"model":model,"account_count":count,"risk_score":score}));
        }
    }
    let scope = 1 + upstream_total + total;
    let hidden = (scope - selected.len() as i64).max(0);
    Ok(
        json!({"center_user_id":uid,"up_depth":up,"down_depth":down,"max_nodes":20,"total_users_in_scope":scope,"hidden_users":hidden,"truncated":hidden>0,"cycle_detected":cycle,"server_now":iso(now()),"nodes":nodes,"edges":edges,"device_links":device_links}),
    )
}

fn summary_data(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let profile = basic(db, uid)?;
    let mut devices = db.prepare("SELECT device_id,platform,model,os_version,first_seen,last_seen,evidence_status,key_id FROM account_devices WHERE user_id=?1 ORDER BY last_seen DESC")?
        .query_map([uid], |r| Ok(json!({"device_id":r.get::<_,String>(0)?,"platform":r.get::<_,String>(1)?,"model":r.get::<_,String>(2)?,"os_version":r.get::<_,String>(3)?,"first_seen":iso(r.get(4)?),"last_seen":iso(r.get(5)?),"evidence_status":r.get::<_,String>(6)?,"key_id":r.get::<_,String>(7)?})))?
        .collect::<Result<Vec<_>,_>>()?;
    for device in &mut devices {
        let id = device["device_id"].as_str().unwrap();
        let related = db
            .prepare("SELECT user_id FROM account_devices WHERE device_id=?1 ORDER BY user_id")?
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let score: Option<i64> = db.query_row("SELECT MAX(score) FROM risk_events WHERE status='reported' AND (device_identity=?1 OR key_id=?2) AND updated_at>=?3",params![id,device["key_id"].as_str().unwrap(),now()-30*86400],|r|r.get(0))?;
        device["account_count"] = json!(related.len());
        device["related_user_ids"] = json!(related);
        device["risk_score"] = json!(score);
    }
    let since7 = now() - 7 * 86400;
    let activity: (i64,i64,i64,Option<i64>) = db.query_row("SELECT COUNT(DISTINCT CASE WHEN created_at>=?2 THEN date(created_at,'unixepoch','+8 hours') END),COUNT(DISTINCT date(created_at,'unixepoch','+8 hours')),COUNT(CASE WHEN created_at>=?2 THEN 1 END),MAX(created_at) FROM user_behavior WHERE user_id=?1 AND status='success' AND created_at>=?3 AND kind IN ('register','login','app_open','check_in','hall_request','reward_start','reward_complete','withdrawal','rain_join','rain_small_claim','rain_large_open','rain_large_claim')",params![uid,since7,now()-30*86400],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    let recent = db.prepare("SELECT id,kind,status,created_at,related_id,detail_json FROM user_behavior WHERE user_id=?1 ORDER BY created_at DESC,id DESC LIMIT 20")?
        .query_map([uid], |r| { let raw:String=r.get(5)?; let detail:Value=serde_json::from_str(&raw).map_err(|e|rusqlite::Error::FromSqlConversionFailure(5,rusqlite::types::Type::Text,Box::new(e)))?; Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"created_at":iso(r.get(3)?),"related_id":r.get::<_,String>(4)?,"detail":detail})) })?
        .collect::<Result<Vec<_>,_>>()?;
    let inviter: Option<Value> = db.query_row("SELECT p.id,p.nickname,p.phone FROM users u JOIN users p ON p.id=u.inviter_id WHERE u.id=?1 AND u.invitation_bound_at IS NOT NULL",[uid],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"nickname":r.get::<_,String>(1)?,"phone":r.get::<_,String>(2)?}))).optional()?;
    let descendants:i64=db.query_row("WITH RECURSIVE related(id) AS (SELECT id FROM users WHERE inviter_id=?1 AND invitation_bound_at IS NOT NULL UNION SELECT u.id FROM users u JOIN related r ON u.inviter_id=r.id WHERE u.invitation_bound_at IS NOT NULL) SELECT COUNT(*) FROM related WHERE id!=?1",[uid],|r|r.get(0))?;
    let summary = db.query_row("SELECT COUNT(*),COALESCE(SUM(CASE WHEN coins>0 THEN coins ELSE 0 END),0),(SELECT COUNT(*) FROM withdrawals WHERE user_id=?1) FROM ledger WHERE user_id=?1",[uid],|r|Ok(json!({"ledger_count":r.get::<_,i64>(0)?,"total_earned_coins":r.get::<_,i64>(1)?,"withdrawal_count":r.get::<_,i64>(2)?})))?;
    Ok(
        json!({"user":profile,"devices":devices,"activity":{"last_active_at":activity.3.map(iso),"active_days_7d":activity.0,"active_days_30d":activity.1,"events_7d":activity.2},"recent_events":recent,"inviter":inviter,"invitation":{"children_count":profile["children_count"],"descendants_count":descendants},"summary":summary}),
    )
}

async fn graph(
    State(state): State<AppState>,
    Path(uid): Path<i64>,
    Query(query): Query<GraphQuery>,
) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    Ok(Json(graph_data(&tx, uid, &query)?))
}
async fn summary(State(state): State<AppState>, Path(uid): Path<i64>) -> ApiResult<Value> {
    let mut db = open(&state)?;
    let tx = db.transaction()?;
    Ok(Json(summary_data(&tx, uid)?))
}
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/users/{id}/graph", get(graph))
        .route("/users/{id}/graph-summary", get(summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (AppState, Connection) {
        let (state, _, _, _) = rain::tests::fixture(false, false);
        let db = open(&state).unwrap();
        for id in 10..=65 {
            db.execute("INSERT INTO users(id,phone,password_hash,nickname,coins,created_at,invite_code,invitation_bound_at) VALUES(?1,?2,'unused',?3,100,?4,?5,?4)",params![id,format!("139{:08}",id),format!("用户{id}"),now(),format!("G{id:05}")]).unwrap();
        }
        (state, db)
    }
    #[test]
    fn broad_tree_keeps_complete_paths_both_three_levels_and_twenty_user_cap() {
        let (_state, db) = fixture();
        for (child, parent) in [(11, 10), (12, 11), (13, 12), (14, 13), (15, 14), (16, 15)] {
            db.execute(
                "UPDATE users SET inviter_id=?1 WHERE id=?2",
                params![parent, child],
            )
            .unwrap();
        }
        for child in 20..=60 {
            db.execute("UPDATE users SET inviter_id=13 WHERE id=?1", [child])
                .unwrap();
        }
        let data = graph_data(&db, 13, &GraphQuery::default()).unwrap();
        let nodes = data["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 20);
        assert!(nodes.iter().any(|n| n["id"] == 10 && n["depth"] == -3));
        assert!(nodes.iter().any(|n| n["id"] == 16 && n["depth"] == 3));
        assert!(data["truncated"].as_bool().unwrap());
        assert!(data["hidden_users"].as_i64().unwrap() > 0);
        let ids: BTreeSet<i64> = nodes.iter().map(|n| n["id"].as_i64().unwrap()).collect();
        for edge in data["edges"].as_array().unwrap() {
            assert!(
                ids.contains(&edge["source"].as_i64().unwrap())
                    && ids.contains(&edge["target"].as_i64().unwrap())
            );
        }
        assert_eq!(
            db.query_row("SELECT coins FROM users WHERE id=13", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            100
        );
    }
    #[test]
    fn malformed_cycles_stop_and_shared_devices_stay_distinct_from_invitation_edges() {
        let (_state, db) = fixture();
        db.execute("UPDATE users SET inviter_id=11 WHERE id=10", [])
            .unwrap();
        db.execute("UPDATE users SET inviter_id=10 WHERE id=11", [])
            .unwrap();
        for id in [10, 11] {
            db.execute("INSERT INTO account_devices VALUES(?1,'same-device','Android','测试设备','15','','','signature_only',?2,?2)",params![id,now()]).unwrap();
        }
        let data = graph_data(&db, 10, &GraphQuery::default()).unwrap();
        assert_eq!(data["cycle_detected"], true);
        assert_eq!(data["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(data["device_links"].as_array().unwrap().len(), 1);
        assert_eq!(data["device_links"][0]["account_count"], 2);
        let summary = summary_data(&db, 10).unwrap();
        assert_eq!(summary["devices"][0]["related_user_ids"], json!([10, 11]));
        assert_eq!(
            graph_data(&db, 9999, &GraphQuery::default()).unwrap_err().0,
            StatusCode::NOT_FOUND
        );
    }
}
