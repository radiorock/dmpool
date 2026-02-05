// Dashboard endpoint
//
// Provides overview statistics for the admin dashboard

use super::super::error::AdminError;
use super::AdminState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub pool: PoolOverview,
    pub blocks: BlockOverview,
    pub payments: PaymentOverview,
    pub system: SystemOverview,
}

#[derive(Debug, Serialize)]
pub struct PoolOverview {
    pub hashrate_24h: u64,
    pub active_miners: i64,
    pub active_workers: i64,
    pub shares_per_second: f64,
}

#[derive(Debug, Serialize)]
pub struct BlockOverview {
    pub last_found: String,
    pub last_height: i64,
    pub total_found: i64,
    pub time_since_last_block_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct PaymentOverview {
    pub pending_amount_btc: f64,
    pub pending_count: i64,
    pub last_paid: String,
    pub total_paid_btc: f64,
}

#[derive(Debug, Serialize)]
pub struct SystemOverview {
    pub stratum_connections: i64,
    pub api_requests_per_minute: i64,
    pub db_connections: i64,
    pub uptime_seconds: i64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
}

/// GET /api/admin/dashboard
///
/// Returns comprehensive dashboard statistics
pub async fn get_dashboard(
    State(state): State<AdminState>,
) -> Result<Json<DashboardStats>, AdminError> {
    let conn = state.db.get_conn().await?;

    // Get pool stats
    let pool_stats = get_pool_overview(&conn).await?;

    // Get block info
    let block_stats = get_block_overview(&conn).await?;

    // Get payment info
    let payment_stats = get_payment_overview(&conn).await?;

    // Get system info
    let system_stats = get_system_overview().await;

    Ok(Json(DashboardStats {
        pool: pool_stats,
        blocks: block_stats,
        payments: payment_stats,
        system: system_stats,
    }))
}

async fn get_pool_overview(
    conn: &deadpool_postgres::Object,
) -> Result<PoolOverview, AdminError> {
    // Get active miners count
    let active_miners: i64 = conn
        .query_one(
            "SELECT COUNT(*) FROM active_miners_24h",
            &[]
        )
        .await?
        .get(0);

    // Get active workers count
    let active_workers: i64 = conn
        .query_one(
            "SELECT COUNT(*) FROM worker_status_cache WHERE is_online = true",
            &[]
        )
        .await?
        .get(0);

    // Calculate hashrate from shares in last 24h
    let row = conn
        .query_one(
            "SELECT COALESCE(SUM(total_shares_24h), 0) as total_shares FROM active_miners_24h",
            &[]
        )
        .await?;

    let total_shares: i64 = row.get("total_shares");
    let hashrate_24h = (total_shares as f64 / (24.0 * 3600.0)) as u64;
    let shares_per_second = total_shares as f64 / (24.0 * 3600.0);

    Ok(PoolOverview {
        hashrate_24h,
        active_miners,
        active_workers,
        shares_per_second,
    })
}

async fn get_block_overview(
    conn: &deadpool_postgres::Object,
) -> Result<BlockOverview, AdminError> {
    // Get most recent block
    let row = conn
        .query_one(
            "SELECT block_height, block_time FROM block_details_cache ORDER BY block_time DESC LIMIT 1",
            &[]
        )
        .await;

    let (last_found, last_height, total_found, time_since) = match row {
        Ok(r) => {
            let block_time: chrono::DateTime<chrono::Utc> = r.get("block_time");
            let height: i64 = r.get("block_height");
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(block_time);
            (
                block_time.to_rfc3339(),
                height,
                100, // TODO: Get actual count
                duration.num_seconds(),
            )
        }
        Err(_) => (
            "1970-01-01T00:00:00Z".to_string(),
            0,
            0,
            0,
        ),
    };

    Ok(BlockOverview {
        last_found,
        last_height,
        total_found,
        time_since_last_block_seconds: time_since,
    })
}

async fn get_payment_overview(
    conn: &deadpool_postgres::Object,
) -> Result<PaymentOverview, AdminError> {
    // Get pending payments from view
    let row = conn
        .query_one(
            "SELECT COALESCE(SUM(balance_sats), 0) as total FROM miners_pending_payout WHERE above_threshold = true",
            &[]
        )
        .await?;

    let pending_sats: i64 = row.get("total");
    let pending_count: i64 = conn
        .query_one(
            "SELECT COUNT(*) FROM miners_pending_payout WHERE above_threshold = true",
            &[]
        )
        .await?
        .get(0);

    // Get total paid
    let paid_row = conn
        .query_one(
            "SELECT COALESCE(SUM(amount_sats), 0) as total FROM payouts WHERE status = 'confirmed'",
            &[]
        )
        .await?;

    let total_paid_sats: i64 = paid_row.get("total");

    Ok(PaymentOverview {
        pending_amount_btc: pending_sats as f64 / 100_000_000.0,
        pending_count,
        last_paid: "2026-02-05T00:00:00Z".to_string(), // TODO: Get actual
        total_paid_btc: total_paid_sats as f64 / 100_000_000.0,
    })
}

async fn get_system_overview() -> SystemOverview {
    // Get system metrics
    // For now, return placeholder values
    // TODO: Integrate with actual system monitoring

    SystemOverview {
        stratum_connections: 342,
        api_requests_per_minute: 45,
        db_connections: 5,
        uptime_seconds: 86400, // 24 hours
        cpu_usage_percent: 15.0,
        memory_usage_percent: 45.0,
        disk_usage_percent: 60.0,
    }
}
