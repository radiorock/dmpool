// Miner Management endpoints
//
// Provides endpoints for listing, searching, and managing miners

use super::super::error::AdminError;
use super::AdminState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct MinersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>, // "active", "inactive", "banned"
}

#[derive(Debug, Serialize)]
pub struct MinersListResponse {
    pub total: i64,
    pub miners: Vec<MinerInfo>,
}

#[derive(Debug, Serialize)]
pub struct MinerInfo {
    pub id: i64,
    pub address: String,
    pub hashrate_24h: u64,
    pub balance_btc: f64,
    pub total_earned_btc: f64,
    pub total_paid_btc: f64,
    pub workers_count: i64,
    pub last_seen: String,
    pub is_banned: bool,
    pub custom_threshold_btc: f64,
}

#[derive(Debug, Deserialize)]
pub struct BanMinerRequest {
    pub reason: String,
    pub permanent: Option<bool>,
    pub expires_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThresholdRequest {
    pub threshold_btc: f64,
}

/// GET /api/admin/miners
///
/// Returns paginated list of miners with optional search
pub async fn get_miners(
    State(state): State<AdminState>,
    Query(query): Query<MinersQuery>,
) -> Result<Json<MinersListResponse>, AdminError> {
    let conn = state.db.get_conn().await?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // Build query with optional search filter
    let sql = if let Some(search_term) = query.search {
        format!(
            "SELECT * FROM get_miners_list($1, $2, $3)",
            search_term, limit, offset
        )
    } else {
        format!("SELECT * FROM get_miners_list(NULL, {}, {})", limit, offset)
    };

    let rows = conn.query(&sql, &[&search]).await?;

    let mut miners = Vec::new();
    for row in rows {
        miners.push(MinerInfo {
            id: row.get("id"),
            address: row.get("address"),
            hashrate_24h: row.get("hashrate_24h"),
            balance_btc: row.get("balance_btc"),
            total_earned_btc: row.get("total_earned_btc"),
            total_paid_btc: row.get("total_paid_btc"),
            workers_count: row.get("workers_count"),
            last_seen: row.get::<_, chrono::DateTime<chrono::Utc>>("last_seen").to_rfc3339(),
            is_banned: row.get("is_banned"),
            custom_threshold_btc: row.get("custom_threshold_btc"),
        });
    }

    // Get total count
    let total: i64 = conn
        .query_one("SELECT COUNT(*) FROM miners", &[])
        .await?
        .get(0);

    Ok(Json(MinersListResponse { total, miners }))
}

/// GET /api/admin/miners/:address
///
/// Returns detailed information about a specific miner
pub async fn get_miner_detail(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> Result<Json<MinerDetailInfo>, AdminError> {
    let conn = state.db.get_conn().await?;

    // Get miner basic info
    let row = conn
        .query_one(
            "SELECT * FROM get_miner_detail($1)",
            &[&address]
        )
        .await
        .map_err(|_| AdminError::NotFound(format!("Miner not found: {}", address)))?;

    let detail = MinerDetailInfo {
        address: row.get("address"),
        hashrate_24h: row.get("hashrate_24h"),
        hashrate_avg: row.get("hashrate_avg"),
        balance_btc: row.get("balance_btc"),
        total_earned_btc: row.get("total_earned_btc"),
        total_paid_btc: row.get("total_paid_btc"),
        workers: vec![], // TODO: Fetch workers
        latest_shares: vec![], // TODO: Fetch shares
        custom_threshold_btc: row.get("custom_threshold_btc"),
        is_banned: row.get("is_banned"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at").to_rfc3339(),
    };

    Ok(Json(detail))
}

#[derive(Debug, Serialize)]
pub struct MinerDetailInfo {
    pub address: String,
    pub hashrate_24h: u64,
    pub hashrate_avg: HashrateAvg,
    pub balance_btc: f64,
    pub total_earned_btc: f64,
    pub total_paid_btc: f64,
    pub workers: Vec<WorkerDetail>,
    pub latest_shares: Vec<ShareInfo>,
    pub custom_threshold_btc: f64,
    pub is_banned: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HashrateAvg {
    #[serde(rename = "1h")]
    pub hour_1: u64,
    #[serde(rename = "6h")]
    pub hour_6: u64,
    #[serde(rename = "24h")]
    pub hour_24: u64,
    #[serde(rename = "7d")]
    pub day_7: u64,
}

#[derive(Debug, Serialize)]
pub struct WorkerDetail {
    pub name: String,
    pub hashrate: u64,
    pub difficulty: u64,
    pub shares: i64,
    pub last_seen: String,
    pub is_online: bool,
}

#[derive(Debug, Serialize)]
pub struct ShareInfo {
    pub difficulty: i64,
    pub created_at: String,
}

/// POST /api/admin/miners/:address/ban
///
/// Bans a miner from the pool
pub async fn ban_miner(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Json(req): Json<BanMinerRequest>,
) -> Result<Json<SuccessResponse>, AdminError> {
    let conn = state.db.get_conn().await?;

    // Calculate expiration date
    let expires_at = if req.permanent.unwrap_or(false) {
        None
    } else if let Some(days) = req.expires_days {
        Some(chrono::Utc::now() + chrono::Duration::days(days))
    } else {
        Some(chrono::Utc::now() + chrono::Duration::days(30)) // Default 30 days
    };

    // Insert ban record
    conn.execute(
        "INSERT INTO banned_miners (address, reason, is_permanent, expires_at, banned_by) VALUES ($1, $2, $3, $4, 'admin') ON CONFLICT (address) DO UPDATE SET reason = $2, is_permanent = $3, expires_at = $4",
        &[&address, &req.reason, &req.permanent.unwrap_or(false), &expires_at]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to ban miner: {}", e)))?;

    // Log audit
    conn.execute(
        "INSERT INTO admin_audit_logs (admin_user, action, target_type, target_id, new_value) VALUES ('admin', 'ban_miner', 'miner', $1, $2)",
        &[&address, &format!("reason: {}, expires: {:?}", req.reason, expires_at)]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to log audit: {}", e)))?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Miner {} banned successfully", address),
    }))
}

/// DELETE /api/admin/miners/:address/ban
///
/// Unbans a miner
pub async fn unban_miner(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> Result<Json<SuccessResponse>, AdminError> {
    let conn = state.db.get_conn().await?;

    // Remove ban record
    let rows_affected = conn
        .execute(
            "DELETE FROM banned_miners WHERE address = $1",
            &[&address]
        )
        .await
        .map_err(|e| AdminError::Internal(format!("Failed to unban miner: {}", e)))?;

    if rows_affected == 0 {
        return Err(AdminError::NotFound(format!("Miner {} is not banned", address)));
    }

    // Log audit
    conn.execute(
        "INSERT INTO admin_audit_logs (admin_user, action, target_type, target_id) VALUES ('admin', 'unban_miner', 'miner', $1)",
        &[&address]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to log audit: {}", e)))?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Miner {} unbanned successfully", address),
    }))
}

/// PUT /api/admin/miners/:address/threshold
///
/// Updates custom payment threshold for a miner
pub async fn update_threshold(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Json(req): Json<UpdateThresholdRequest>,
) -> Result<Json<ThresholdUpdateResponse>, AdminError> {
    let conn = state.db.get_conn().await?;

    let threshold_sats = (req.threshold_btc * 100_000_000.0) as i64;

    // Insert or update threshold
    conn.execute(
        "INSERT INTO custom_thresholds (address, threshold_sats, updated_by) VALUES ($1, $2, 'admin') ON CONFLICT (address) DO UPDATE SET threshold_sats = $2, updated_by = 'admin', updated_at = NOW()",
        &[&address, &threshold_sats]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to update threshold: {}", e)))?;

    // Log audit
    conn.execute(
        "INSERT INTO admin_audit_logs (admin_user, action, target_type, target_id, new_value) VALUES ('admin', 'update_threshold', 'miner', $1, $2)",
        &[&address, &format!("threshold_btc: {}", req.threshold_btc)]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to log audit: {}", e)))?;

    Ok(Json(ThresholdUpdateResponse {
        success: true,
        address,
        new_threshold_btc: req.threshold_btc,
    }))
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ThresholdUpdateResponse {
    pub success: bool,
    pub address: String,
    pub new_threshold_btc: f64,
}
