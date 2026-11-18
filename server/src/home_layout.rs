//! 首页入口 (icon grid + floating entry) was retired: activities now have their own hall badges
//! (新人礼包, 幸运转盘). Older Apps still poll /app/home, so it answers with an empty layout.
use super::*;

async fn public_home() -> Json<Value> {
    Json(json!({"entries":[],"floating":null,"revision":0}))
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/app/home", get(public_home))
}
