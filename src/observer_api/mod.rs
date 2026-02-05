// Observer API Module for DMPool
//
// This module provides public, read-only API endpoints for:
// - Pool statistics
// - Miner statistics
// - Hashrate history
// - Block information
//
// These endpoints are accessible without authentication and are
// designed to be consumed by the observer frontend.

pub mod routes;
pub mod error;

use anyhow::Result;
use axum::{Router, routing::get};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::db::DatabaseManager;

/// Application state for Observer API
#[derive(Clone)]
pub struct ObserverState {
    pub db: Arc<DatabaseManager>,
}

/// Create the Observer API router
pub fn create_router(db: Arc<DatabaseManager>) -> Router {
    let state = ObserverState { db };

    Router::new()
        // Pool statistics
        .route("/api/v1/stats", get(routes::get_pool_stats))

        // Miner statistics
        .route("/api/v1/stats/:address", get(routes::get_miner_stats))
        .route("/api/v1/stats/:address/hashrate", get(routes::get_miner_hashrate_history))

        // Block information
        .route("/api/v1/blocks", get(routes::get_blocks))
        .route("/api/v1/blocks/:height", get(routes::get_block_detail))

        .with_state(state)
}

/// Start the Observer API server
pub async fn start_observer_api(
    db: Arc<DatabaseManager>,
    host: String,
    port: u16,
) -> Result<tokio::task::JoinHandle<()>> {
    let app = create_router(db.clone());
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Observer API listening on http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap();
    });

    Ok(handle)
}
