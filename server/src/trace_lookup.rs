//! Admin lookup by the reference number users see ("参考号" / "奖励编号"): finds the account,
//! device, risk decision and restriction behind it, so support can explain or lift it quickly.
use super::*;

fn ref_ok(r: &str) -> bool {
    !r.is_empty() && r.len() <= 80 && r.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':' || c == '_')
}

fn ts(v: Option<i64>) -> Value {
    v.map(|t| json!(iso(t))).unwrap_or(Value::Null)
}

fn user_summary(db: &Connection, uid: i64) -> Result<Value, ApiError> {
    let row: Option<(String, String, i64, i64)> = db
        .query_row(
            "SELECT phone,nickname,coins,created_at FROM users WHERE id=?1",
            [uid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((phone, nickname, coins, created)) = row else {
        return Ok(json!({"id":uid,"missing":true}));
    };
    let restriction = db
        .query_row(
            "SELECT status,reason,updated_by,updated_at,trace_id,risk_event_id FROM account_restrictions WHERE user_id=?1",
            [uid],
            |r| {
                Ok(json!({"status":r.get::<_,String>(0)?,"reason":r.get::<_,String>(1)?,"updated_by":r.get::<_,String>(2)?,
                    "updated_at":iso(r.get(3)?),"trace_id":r.get::<_,String>(4)?,"risk_event_id":r.get::<_,String>(5)?}))
            },
        )
        .optional()?
        .unwrap_or_else(|| json!({"status":"active"}));
    let shown_phone = if phone.starts_with("u:") { String::new() } else { phone.clone() };
    let username = phone.strip_prefix("u:").unwrap_or("");
    Ok(json!({"id":uid,"phone":shown_phone,"username":username,"nickname":nickname,"coins":coins,
        "created_at":iso(created),"restriction":restriction,"review_account":is_review_account(db, uid)?}))
}

fn devices(db: &Connection, uid: i64, focus: &str) -> Result<Vec<Value>, ApiError> {
    let mut stmt = db.prepare(
        "SELECT device_id,platform,model,os_version,evidence_status,first_seen,last_seen FROM account_devices WHERE user_id=?1 ORDER BY last_seen DESC LIMIT 20",
    )?;
    let bare = focus.strip_prefix("device:").unwrap_or(focus);
    let rows = stmt
        .query_map([uid], |r| {
            let id: String = r.get(0)?;
            let matches = !bare.is_empty() && (id == focus || id == bare || id.strip_prefix("device:") == Some(bare));
            Ok(json!({"device_id":id,"platform":r.get::<_,String>(1)?,"model":r.get::<_,String>(2)?,"os_version":r.get::<_,String>(3)?,
                "evidence":r.get::<_,String>(4)?,"first_seen":iso(r.get(5)?),"last_seen":iso(r.get(6)?),"this_event":matches}))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// DeviceTrust details worth showing: decision, reasons, rule hits, IP location.
fn risk_view(detail: &Value) -> Value {
    let ip = &detail["resources"]["ip"];
    let location = ["province", "city", "isp"]
        .iter()
        .filter_map(|k| ip[*k].as_str().filter(|s| !s.is_empty()))
        .collect::<Vec<_>>()
        .join(" ");
    let hits: Vec<Value> = detail["hits"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|h| {
                    if let Some(s) = h.as_str() {
                        json!({"code":s})
                    } else {
                        json!({"code":h.get("code").or(h.get("rule")).or(h.get("id")).cloned().unwrap_or(Value::Null),
                            "label":h.get("label").or(h.get("name")).or(h.get("title")).cloned().unwrap_or(Value::Null),
                            "score":h.get("score").or(h.get("weight")).cloned().unwrap_or(Value::Null),
                            "detail":h.get("detail").or(h.get("evidence")).or(h.get("value")).cloned().unwrap_or(Value::Null)})
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    json!({"decision":detail["decision"],"action":detail["action"],"score":detail["score"],
        "reasons":detail["reasons"],"hits":hits,"ip_location":location,"scene":detail["scene"],
        "attestation":detail["reportProof"]["level"],"model_version":detail["modelVersion"]})
}

async fn lookup(State(state): State<AppState>, Path(reference): Path<String>) -> ApiResult<Value> {
    let reference = reference.trim().to_owned();
    if !ref_ok(&reference) {
        return Err(invalid("请输入完整的参考号或奖励编号"));
    }
    let db = open(&state)?;
    let mut matches: Vec<Value> = Vec::new();
    let mut uid: Option<i64> = None;
    let mut device_focus = String::new();

    // 1) DeviceTrust risk event (参考号 on "服务暂不可用" and DeviceTrust restrictions).
    let event: Option<(i64, String, String, String, String, Option<i64>, String, String, String, i64, String)> = db
        .query_row(
            "SELECT user_id,kind,status,decision,action,score,detail_json,error,client_ip,created_at,COALESCE(device_identity,'') FROM risk_events WHERE id=?1",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
        )
        .optional()?;
    if let Some((u, kind, status, decision, action, score, detail, error, ip, created, device)) = event {
        uid = Some(u);
        device_focus = device.clone();
        let detail: Value = serde_json::from_str(&detail).unwrap_or(Value::Null);
        matches.push(json!({"type":"risk_event","title":"设备核验事件","time":iso(created),"operation":kind,"status":status,
            "decision":decision,"action":action,"score":score,"error":error,"client_ip":ip,"device_identity":device,"risk":risk_view(&detail)}));
    }

    // 2) Restriction whose trace is this reference (behaviour audit / admin changes use their own ids).
    let restriction: Option<(i64, String, String, String, i64)> = db
        .query_row(
            "SELECT user_id,status,reason,updated_by,updated_at FROM account_restrictions WHERE trace_id=?1 OR (risk_event_id!='' AND risk_event_id=?1)",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    if let Some((u, status, reason, by, at)) = restriction {
        uid.get_or_insert(u);
        matches.push(json!({"type":"restriction","title":"账号限制","status":status,"reason":reason,"updated_by":by,"time":iso(at),"current":true}));
    }
    let audit: Vec<Value> = {
        let mut stmt = db.prepare(
            "SELECT user_id,actor,action,reason,created_at FROM admin_audit WHERE related_id=?1 ORDER BY created_at DESC LIMIT 10",
        )?;
        let rows = stmt
            .query_map([&reference], |r| {
                Ok((r.get::<_, Option<i64>>(0)?, json!({"actor":r.get::<_,String>(1)?,"action":r.get::<_,String>(2)?,"reason":r.get::<_,String>(3)?,"time":iso(r.get(4)?)})))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(u) = rows.iter().find_map(|(u, _)| *u) {
            uid.get_or_insert(u);
        }
        rows.into_iter().map(|(_, v)| v).collect()
    };
    if !audit.is_empty() {
        matches.push(json!({"type":"audit","title":"处置记录","items":audit}));
    }

    // 3) Reward session (奖励编号) with its ad trail.
    let reward: Option<(i64, String, i64, i64, Option<i64>, Option<i64>, String, Option<String>, Option<String>)> = db
        .query_row(
            "SELECT user_id,task_id,reward,started_at,verified_at,completed_at,provider,ad_network,csj_trans_id FROM reward_sessions WHERE id=?1",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .optional()?;
    if let Some((u, task, coins, started, verified, completed, provider, network, trans)) = reward {
        uid.get_or_insert(u);
        let mut stmt = db.prepare(
            "SELECT created_at,network,event,source,error_code,message FROM ad_events WHERE session_id=?1 ORDER BY id LIMIT 40",
        )?;
        let trail = stmt
            .query_map([&reference], |r| {
                Ok(json!({"time":iso(r.get(0)?),"network":r.get::<_,String>(1)?,"event":r.get::<_,String>(2)?,"source":r.get::<_,String>(3)?,"code":r.get::<_,String>(4)?,"message":r.get::<_,String>(5)?}))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = db.prepare(
            "SELECT status,reason_code,client_ip,created_at FROM callback_events WHERE reward_session_id=?1 ORDER BY created_at",
        )?;
        let callbacks = stmt
            .query_map([&reference], |r| {
                Ok(json!({"status":r.get::<_,String>(0)?,"reason":r.get::<_,String>(1)?,"client_ip":r.get::<_,String>(2)?,"time":iso(r.get(3)?)}))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let state_label = if completed.is_some() { "已到账" } else if verified.is_some() { "平台已确认，待领取" } else { "等待平台回调" };
        matches.push(json!({"type":"reward","title":"激励红包","task_id":task,"coins":coins,"status":state_label,"provider":provider,
            "network":network,"trans_id":trans,"started_at":iso(started),"verified_at":ts(verified),"completed_at":ts(completed),
            "ad_trail":trail,"callbacks":callbacks}));
    }

    // 4) Withdrawal id.
    let withdrawal: Option<(i64, f64, String, String, i64)> = db
        .query_row(
            "SELECT user_id,amount_yuan,channel,status,created_at FROM withdrawals WHERE id=?1",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    if let Some((u, amount, channel, status, at)) = withdrawal {
        uid.get_or_insert(u);
        matches.push(json!({"type":"withdrawal","title":"提现申请","amount_yuan":amount,"channel":channel,"status":status,"time":iso(at)}));
    }

    // 5) Any recorded business behaviour with this id.
    let behaviour: Option<(Option<i64>, String, String, String, i64)> = db
        .query_row(
            "SELECT user_id,kind,status,detail_json,created_at FROM user_behavior WHERE id=?1 OR related_id=?1 ORDER BY created_at DESC LIMIT 1",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    if let Some((u, kind, status, detail, at)) = behaviour {
        if let Some(u) = u {
            uid.get_or_insert(u);
        }
        let detail: Value = serde_json::from_str(&detail).unwrap_or(Value::Null);
        matches.push(json!({"type":"behavior","title":"业务记录","kind":kind,"status":status,"detail":detail,"time":iso(at)}));
    }

    let user = match uid {
        Some(u) => Some(user_summary(&db, u)?),
        None => None,
    };
    let device_list = match uid {
        Some(u) => devices(&db, u, &device_focus)?,
        None => Vec::new(),
    };
    Ok(Json(json!({"reference":reference,"found":!matches.is_empty(),"user":user,"devices":device_list,"matches":matches})))
}

pub(super) fn admin_routes() -> Router<AppState> {
    Router::new().route("/trace/{reference}", get(lookup))
}
