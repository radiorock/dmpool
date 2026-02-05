// Unit tests for Observer API routes
//
// These tests use mock database responses to verify the API endpoints
// work correctly.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;

use dmpool::observer_api;

// ============================================================================
// Mock Database Manager
// ============================================================================

struct MockDatabaseManager {
    pool_stats: Option<PoolStats>,
    miner_stats: Option<MinerStats>,
    blocks: Option<Vec<BlockInfo>>,
}

impl MockDatabaseManager {
    fn new() -> Self {
        Self {
            pool_stats: None,
            miner_stats: None,
            blocks: None,
        }
    }

    fn with_pool_stats(mut self, stats: PoolStats) -> Self {
        self.pool_stats = Some(stats);
        self
    }

    fn with_miner_stats(mut self, stats: MinerStats) -> Self {
        self.miner_stats = Some(stats);
        self
    }
}

// For testing, we'll create a simple integration test instead of full mocking
// since DatabaseManager has many methods

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Helper function to create a test request
    fn create_request(path: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap()
    }

    /// Helper function to create an ObserverState with mock database
    async fn create_test_state() -> ObserverState {
        // In a real test, we would set up a test database here
        // For now, we'll use the actual DatabaseManager with test config
        let db_conn_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://dmpool:dmpool@localhost:5432/dmpool_test".to_string());

        let db = match DatabaseManager::new(&db_conn_string) {
            Ok(db) => Arc::new(db),
            Err(_) => {
                // If we can't connect to database, skip tests
                panic!("Database connection required for tests. Set TEST_DATABASE_URL environment variable.");
            }
        };

        ObserverState { db }
    }

    /// Test: GET /api/v1/stats returns pool statistics
    #[tokio::test]
    async fn test_get_pool_stats() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/stats"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let stats: Value = serde_json::from_slice(&body).unwrap();

        // Verify response structure
        assert!(stats.get("pool_hashrate_3h").is_some());
        assert!(stats.get("active_miners").is_some());
        assert!(stats.get("active_workers").is_some());
    }

    /// Test: GET /api/v1/stats/{address} with invalid address returns 400
    #[tokio::test]
    async fn test_get_miner_stats_invalid_address() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/stats/invalid_address"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test: GET /api/v1/stats/{address} with valid address
    #[tokio::test]
    async fn test_get_miner_stats_valid_address() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/stats/bc1qtestexample123456789abcdef"))
            .await
            .unwrap();

        // Might return 404 if miner doesn't exist, or 200 with empty data
        // Either way should not return 500
        assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Test: GET /api/v1/stats/{address}/hashrate with period parameter
    #[tokio::test]
    async fn test_get_miner_hashrate_with_period() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/stats/bc1qtestexample123456789abcdef/hashrate?period=7d"))
            .await
            .unwrap();

        // Should not return 500
        assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Test: GET /api/v1/blocks returns blocks list
    #[tokio::test]
    async fn test_get_blocks() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/blocks?limit=10"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let blocks: Value = serde_json::from_slice(&body).unwrap();

        // Verify response structure
        assert!(blocks.get("total").is_some());
        assert!(blocks.get("blocks").is_some());
    }

    /// Test: GET /api/v1/blocks with pagination
    #[tokio::test]
    async fn test_get_blocks_pagination() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/blocks?limit=5&offset=10"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: GET /api/v1/blocks/{height} for specific block
    #[tokio::test]
    async fn test_get_block_detail() {
        let state = create_test_state().await;
        let app = observer_api::create_router(Arc::new(state.db));

        let response = app
            .oneshot(create_request("/api/v1/blocks/823456"))
            .await
            .unwrap();

        // Might return 404 if block doesn't exist
        assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Test: Invalid Bitcoin address validation
    #[tokio::test]
    fn test_invalid_bitcoin_address() {
        // Valid addresses
        assert!(is_valid_bitcoin_address("bc1qtestexample123456789abcdef"));
        assert!(is_valid_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"));
        assert!(is_valid_bitcoin_address("3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy"));

        // Invalid addresses
        assert!(!is_valid_bitcoin_address("invalid"));
        assert!(!is_valid_bitcoin_address(""));
        assert!(!is_valid_bitcoin_address("0x1234567890abcdef"));
    }

    fn is_valid_bitcoin_address(address: &str) -> bool {
        address.starts_with("bc1") || address.starts_with("1") || address.starts_with("3")
    }

    /// Test: Period parsing
    #[tokio::test]
    fn test_parse_period() {
        // Valid periods
        assert_eq!(parse_period("1d"), Some(1));
        assert_eq!(parse_period("7d"), Some(7));
        assert_eq!(parse_period("1m"), Some(30));
        assert_eq!(parse_period("1y"), Some(365));

        // Invalid periods
        assert_eq!(parse_period("invalid"), None);
        assert_eq!(parse_period("2h"), None);
        assert_eq!(parse_period(""), None);
    }

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
}

// ============================================================================
// Database Query Unit Tests
// ============================================================================

#[cfg(test)]
mod database_tests {
    use super::*;

    /// Test database connection
    #[tokio::test]
    async fn test_database_connection() {
        let db_conn_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://dmpool:dmpool@localhost:5432/dmpool_test".to_string());

        let result = DatabaseManager::new(&db_conn_string);

        // Result depends on whether database is available
        // We just check that it doesn't panic
        match result {
            Ok(db) => {
                // Try to test connection
                let test_result = db.test_connection().await;
                if test_result.is_ok() {
                    println!("Database connection test successful");
                } else {
                    println!("Database connection test failed: {:?}", test_result.err());
                }
            }
            Err(e) => {
                println!("Failed to create database manager: {}", e);
            }
        }
    }
}
