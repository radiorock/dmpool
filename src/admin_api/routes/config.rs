// System configuration endpoints
//
// Provides dynamic system configuration management

use super::super::error::AdminError;
use super::AdminState;
use axum::{extract::State, Json};

pub async fn get_config(
    State(_state): State<AdminState>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement - fetch from system_configs table
    Ok(axum::Json(serde_json::json!({
        "pool_fee_percent": 1.0,
        "min_payout_btc": 0.01,
        "pplns_window_days": 7,
        "stratum_port": 3333,
        "api_port": 8081
    })))
}

pub async fn update_config(
    State(_state): State<AdminState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement - update system_configs table
    Ok(axum::Json(serde_json::json!({
        "success": true,
        "reload_required": false
    })))
}
