// Notification configuration endpoints
//
// Provides notification config management

use super::super::error::AdminError;
use super::AdminState;
use axum::{extract::State, Json};

pub async fn get_config(
    State(_state): State<AdminState>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Ok(axum::Json(serde_json::json!({
        "admin_telegram_enabled": false,
        "admin_email_enabled": false,
        "notify_block_found": true,
        "notify_payment": true,
        "notify_alert": true
    })))
}

pub async fn update_config(
    State(_state): State<AdminState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Ok(axum::Json(serde_json::json!({
        "success": true
    })))
}

pub async fn get_history(
    State(_state): State<AdminState>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Ok(axum::Json(serde_json::json!({
        "total": 0,
        "notifications": []
    })))
}
