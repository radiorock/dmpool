// Payment Management endpoints
//
// Provides endpoints for viewing pending payments, manual payouts, and payment history

use super::super::error::AdminError;
use super::AdminState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PendingPaymentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub min_amount_btc: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PendingPaymentsResponse {
    pub total_btc: f64,
    pub count: i64,
    pub payments: Vec<PendingPayment>,
}

#[derive(Debug, Serialize)]
pub struct PendingPayment {
    pub address: String,
    pub balance_btc: f64,
    pub threshold_btc: f64,
    pub unpaid_since: String,
    pub can_pay: bool,
}

#[derive(Debug, Deserialize)]
pub struct TriggerPayoutRequest {
    pub amount_btc: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TriggerPayoutResponse {
    pub success: bool,
    pub address: String,
    pub amount_btc: f64,
    pub txid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub address: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentHistoryResponse {
    pub total: i64,
    pub payments: Vec<PaymentRecord>,
}

#[derive(Debug, Serialize)]
pub struct PaymentRecord {
    pub id: i64,
    pub address: String,
    pub amount_btc: f64,
    pub txid: Option<String>,
    pub block_height: Option<i64>,
    pub confirmations: i32,
    pub status: String,
    pub created_at: String,
}

/// GET /api/admin/payments/pending
///
/// Returns list of miners with pending payments (above threshold)
pub async fn get_pending_payouts(
    State(state): State<AdminState>,
    Query(query): Query<PendingPaymentsQuery>,
) -> Result<Json<PendingPaymentsResponse>, AdminError> {
    let conn = state.db.get_conn().await?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // Get pending payments from view
    let sql = if let Some(min_amount) = query.min_amount_btc {
        let min_sats = (min_amount * 100_000_000.0) as i64;
        format!(
            "SELECT * FROM miners_pending_payout WHERE above_threshold = true AND balance_sats >= $3 LIMIT $1 OFFSET $2",
            limit, offset, min_sats
        )
    } else {
        format!(
            "SELECT * FROM miners_pending_payout WHERE above_threshold = true ORDER BY balance_sats DESC LIMIT $1 OFFSET $2",
            limit, offset
        )
    };

    let rows = conn.query(&sql, &[]).await?;

    let mut payments = Vec::new();
    let mut total_btc = 0.0;

    for row in rows {
        let balance_sats: i64 = row.get("balance_sats");
        let threshold_sats: i64 = row.get("threshold_sats");

        payments.push(PendingPayment {
            address: row.get("address"),
            balance_btc: balance_sats as f64 / 100_000_000.0,
            threshold_btc: threshold_sats as f64 / 100_000_000.0,
            unpaid_since: "2026-02-01T00:00:00Z".to_string(), // TODO: Calculate
            can_pay: balance_sats >= threshold_sats,
        });

        total_btc += balance_sats as f64 / 100_000_000.0;
    }

    // Get count
    let count: i64 = conn
        .query_one("SELECT COUNT(*) FROM miners_pending_payout WHERE above_threshold = true", &[])
        .await?
        .get(0);

    Ok(Json(PendingPaymentsResponse {
        total_btc,
        count,
        payments,
    }))
}

/// POST /api/admin/payments/trigger/:address
///
/// Manually triggers a payout for a specific miner
pub async fn trigger_payout(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Json(req): Json<TriggerPayoutRequest>,
) -> Result<Json<TriggerPayoutResponse>, AdminError> {
    let conn = state.db.get_conn().await?;

    // Get miner's current balance
    let row = conn
        .query_one(
            "SELECT balance_sats FROM miners WHERE address = $1",
            &[&address]
        )
        .await
        .map_err(|_| AdminError::NotFound(format!("Miner not found: {}", address)))?;

    let balance_sats: i64 = row.get("balance_sats");

    // Use provided amount or full balance
    let payout_sats = if let Some(amount) = req.amount_btc {
        let sats = (amount * 100_000_000.0) as i64;
        if sats > balance_sats {
            return Err(AdminError::InvalidInput(
                format!("Requested payout {} BTC exceeds balance {} BTC",
                    amount, balance_sats as f64 / 100_000_000.0)
            ));
        }
        sats
    } else {
        // Get custom threshold or default
        let threshold_row = conn
            .query_one(
                "SELECT COALESCE(threshold_sats, 1000000) FROM custom_thresholds WHERE address = $1",
                &[&address]
            )
            .await?;

        let threshold_sats: i64 = threshold_row.get(0);
        if balance_sats < threshold_sats {
            return Err(AdminError::InvalidInput(
                format!("Balance {} BTC below threshold {} BTC",
                    balance_sats as f64 / 100_000_000.0,
                    threshold_sats as f64 / 100_000_000.0)
            ));
        }
        balance_sats
    };

    // TODO: Create actual payout transaction via Bitcoin RPC
    // For now, just return a placeholder response
    let txid = None;

    // Log audit
    conn.execute(
        "INSERT INTO admin_audit_logs (admin_user, action, target_type, target_id, new_value) VALUES ('admin', 'manual_payout', 'miner', $1, $2)",
        &[&address, &format!("amount_btc: {}", payout_sats as f64 / 100_000_000.0)]
    )
    .await
    .map_err(|e| AdminError::Internal(format!("Failed to log audit: {}", e)))?;

    Ok(Json(TriggerPayoutResponse {
        success: true,
        address,
        amount_btc: payout_sats as f64 / 100_000_000.0,
        txid,
    }))
}

/// GET /api/admin/payments/history
///
/// Returns payment history with optional filters
pub async fn get_payment_history(
    State(state): State<AdminState>,
    Query(query): Query<PaymentHistoryQuery>,
) -> Result<Json<PaymentHistoryResponse>, AdminError> {
    let conn = state.db.get_conn().await?;
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // Build query with filters
    let mut sql = "SELECT id, address, amount_sats, txid, block_height, confirmations, status, created_at FROM payout_history_view".to_string();
    let mut conditions = Vec::new();

    if let Some(address) = &query.address {
        conditions.push(format!("address = '{}'", address));
    }

    if let Some(status) = &query.status {
        conditions.push(format!("status = '{}'", status));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let rows = conn.query(&sql, &[]).await?;

    let mut payments = Vec::new();
    for row in rows {
        let amount_sats: i64 = row.get("amount_sats");

        payments.push(PaymentRecord {
            id: row.get("id"),
            address: row.get("address"),
            amount_btc: amount_sats as f64 / 100_000_000.0,
            txid: row.get("txid"),
            block_height: row.get("block_height"),
            confirmations: row.get("confirmations"),
            status: row.get("status"),
            created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at").to_rfc3339(),
        });
    }

    // Get total count
    let count_sql = if !conditions.is_empty() {
        format!("SELECT COUNT(*) FROM payout_history_view WHERE {}", conditions.join(" AND "))
    } else {
        "SELECT COUNT(*) FROM payout_history_view".to_string()
    };

    let total: i64 = conn.query_one(&count_sql, &[]).await?.get(0);

    Ok(Json(PaymentHistoryResponse {
        total,
        payments,
    }))
}
