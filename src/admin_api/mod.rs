// Admin API Module for DMPool
//
// This module provides internal-only API endpoints for:
// - Dashboard monitoring
// - Miner management
// - Worker monitoring
// - Payment management
// - Block management
// - System monitoring
// - Notification configuration
// - System configuration
//
// These endpoints require authentication and should only be accessible
// from internal network or VPN.

pub mod routes;
pub mod error;
pub mod middleware;

use anyhow::Result;
use axum::{Router, routing::get, routing::post, routing::put, routing::delete};
use std::sync::Arc;
use tracing::info;

use crate::db::DatabaseManager;

/// Application state for Admin API
#[derive(Clone)]
pub struct AdminState {
    pub db: Arc<DatabaseManager>,
}

/// Create the Admin API router (with authentication middleware)
pub fn create_router(db: Arc<DatabaseManager>) -> Router {
    let state = AdminState { db };

    Router::new()
        // Dashboard
        .route("/api/admin/dashboard", get(routes::dashboard::get_dashboard))

        // Miner management
        .route("/api/admin/miners", get(routes::miners::get_miners))
        .route("/api/admin/miners/:address", get(routes::miners::get_miner_detail))
        .route("/api/admin/miners/:address/ban", post(routes::miners::ban_miner))
        .route("/api/admin/miners/:address/ban", delete(routes::miners::unban_miner))
        .route("/api/admin/miners/:address/threshold", put(routes::miners::update_threshold))

        // Workers
        .route("/api/admin/workers", get(routes::workers::get_workers))

        // Payments
        .route("/api/admin/payments/pending", get(routes::payments::get_pending_payouts))
        .route("/api/admin/payments/trigger/:address", post(routes::payments::trigger_payout))
        .route("/api/admin/payments/history", get(routes::payments::get_payment_history))

        // Blocks
        .route("/api/admin/blocks", get(routes::blocks::get_blocks))
        .route("/api/admin/blocks/:height", get(routes::blocks::get_block_detail))
        .route("/api/admin/blocks/:height/pplns", get(routes::blocks::get_block_pplns))

        // Monitoring
        .route("/api/admin/monitoring/stratum", get(routes::monitoring::get_stratum_stats))
        .route("/api/admin/monitoring/database", get(routes::monitoring::get_database_stats))
        .route("/api/admin/logs", get(routes::monitoring::get_logs))

        // Notifications
        .route("/api/admin/notifications/config", get(routes::notifications::get_config))
        .route("/api/admin/notifications/config", put(routes::notifications::update_config))
        .route("/api/admin/notifications/history", get(routes::notifications::get_history))

        // System Config
        .route("/api/admin/config", get(routes::config::get_config))
        .route("/api/admin/config", put(routes::config::update_config))

        .with_state(state)
}

/// Start the Admin API server
pub async fn start_admin_api(
    db: Arc<DatabaseManager>,
    host: String,
    port: u16,
) -> Result<tokio::task::JoinHandle<()>> {
    let app = create_router(db);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Admin API listening on http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap();
    });

    Ok(handle)
}
