use axum::{extract::State, Json};
use serde_json::json;
use crate::state::AppState;

pub async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1").fetch_one(state.pool()).await.is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "db":     db_ok,
        "chain":  "x1",
        "v":      "atlas.v1",
    }))
}
