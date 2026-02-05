// Block Management endpoints
//
// Provides endpoints for viewing blocks and PPLNS distribution

use super::super::error::AdminError;
use super::AdminState;
use axum::{extract::Path, Query, State};

pub async fn get_blocks(
    State(_state): State<AdminState>,
    Query(_query): Query<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Ok(axum::Json(serde_json::json!({
        "total": 0,
        "blocks": []
    })))
}

pub async fn get_block_detail(
    State(_state): State<AdminState>,
    Path(_height): Path<i64>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Err(AdminError::NotFound("Block not found".to_string()))
}

pub async fn get_block_pplns(
    State(_state): State<AdminState>,
    Path(_height): Path<i64>,
) -> Result<axum::Json<serde_json::Value>, AdminError> {
    // TODO: Implement
    Err(AdminError::NotFound("Block PPLNS data not found".to_string()))
}
