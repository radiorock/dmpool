// Workers endpoints
//
// Provides worker monitoring

use super::super::error::AdminError;
use super::AdminState;
use axum::{extract::Query, State};

pub async fn get_workers(
    State(_state): State<AdminState>,
    Query(_query): Query<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Ok(axum::Json(serde_json::json!({
        "total": 0,
        "workers": []
    })))
}
