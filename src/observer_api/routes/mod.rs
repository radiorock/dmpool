// Observer API Routes
//
// Public, read-only endpoints for pool and miner statistics

use super::error::ObserverError;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::db::{DatabaseManager, BlockInfo, BlockDetail, HashrateDataPoint};

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for hashrate history
#[derive(Debug, Deserialize)]
pub struct HashrateQuery {
    pub period: Option<String>, // "7d", "1m", "3m", etc.
}

// ============================================================================
// Pool Statistics Endpoints
// ============================================================================

/// GET /api/v1/stats
///
/// Returns pool-wide statistics
pub async fn get_pool_stats(
    State(state): State<super::ObserverState>,
) -> Result<Json<crate::db::PoolStats>, ObserverError> {
    let stats = state.db.get_pool_stats().await?;
    Ok(Json(stats))
}

// ============================================================================
// Miner Statistics Endpoints
// ============================================================================

/// GET /api/v1/stats/:address
///
/// Returns detailed statistics for a specific miner
pub async fn get_miner_stats(
    State(state): State<super::ObserverState>,
    Path(address): Path<String>,
) -> Result<Json<crate::db::MinerStats>, ObserverError> {
    // Validate Bitcoin address
    if !is_valid_bitcoin_address(&address) {
        return Err(ObserverError::InvalidInput("Invalid Bitcoin address".to_string()));
    }

    match state.db.get_miner_stats(&address).await? {
        Some(stats) => Ok(Json(stats)),
        None => Err(ObserverError::NotFound(format!("Miner not found: {}", address))),
    }
}

/// GET /api/v1/stats/:address/hashrate?period=7d
///
/// Returns hashrate history for a specific miner
pub async fn get_miner_hashrate_history(
    State(state): State<super::ObserverState>,
    Path(address): Path<String>,
    Query(query): Query<HashrateQuery>,
) -> Result<Json<HashrateHistoryResponse>, ObserverError> {
    // Validate Bitcoin address
    if !is_valid_bitcoin_address(&address) {
        return Err(ObserverError::InvalidInput("Invalid Bitcoin address".to_string()));
    }

    // Parse period (default: 7 days)
    let period_days = parse_period(query.period.as_deref()).unwrap_or(7);

    let data_points = state.db.get_miner_hashrate_history(&address, period_days).await?;

    Ok(Json(HashrateHistoryResponse {
        address,
        period: format!("{}d", period_days),
        interval: "1h".to_string(),
        data_points,
    }))
}

/// Response for hashrate history
#[derive(Debug, Serialize)]
pub struct HashrateHistoryResponse {
    pub address: String,
    pub period: String,
    pub interval: String,
    pub data_points: Vec<HashrateDataPoint>,
}

// ============================================================================
// Block Information Endpoints
// ============================================================================

/// GET /api/v1/blocks?limit=20&offset=0
///
/// Returns list of blocks found by the pool
pub async fn get_blocks(
    State(state): State<super::ObserverState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<BlocksResponse>, ObserverError> {
    let limit = query.limit.unwrap_or(20).min(100); // Max 100
    let offset = query.offset.unwrap_or(0);

    let blocks = state.db.get_blocks(limit, offset).await?;

    Ok(Json(BlocksResponse {
        total: blocks.len() as i64, // TODO: Get actual count
        blocks,
    }))
}

/// Response for blocks list
#[derive(Debug, Serialize)]
pub struct BlocksResponse {
    pub total: i64,
    pub blocks: Vec<BlockInfo>,
}

/// GET /api/v1/blocks/:height
///
/// Returns detailed information about a specific block including PPLNS distribution
pub async fn get_block_detail(
    State(state): State<super::ObserverState>,
    Path(height): Path<i64>,
) -> Result<Json<BlockDetail>, ObserverError> {
    match state.db.get_block_detail(height).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(ObserverError::NotFound(format!("Block not found: {}", height))),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate Bitcoin address (basic check)
fn is_valid_bitcoin_address(address: &str) -> bool {
    // Basic validation - should use proper Bitcoin address validation
    // Prefixes: bc1 (Bech32), 1 (P2PKH), 3 (P2SH)
    address.starts_with("bc1") || address.starts_with("1") || address.starts_with("3")
}

/// Parse period string to days
fn parse_period(period: &str) -> Option<i64> {
    match period {
        "1d" => Some(1),
        "3d" => Some(3),
        "7d" => Some(7),
        "1m" => Some(30),
        "3m" => Some(90),
        "6m" => Some(180),
        "1y" => Some(365),
        _ => None,
    }
}

// ============================================================================
// Module Re-exports
// ============================================================================

pub mod blocks;
pub mod miners;
pub mod pool;
