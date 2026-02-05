// Database connection and query module for DMPool
//
// This module provides database access for:
// - Observer API (read-only access to Hydrapool data)
// - Admin API (full access to admin tables)

use anyhow::{Context, Result};
use deadpool_postgres::{Config, Pool, Runtime};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_postgres::NoTls;
use tracing::{debug, error, info};

/// Database connection pool manager
pub struct DatabaseManager {
    pool: Pool,
}

impl DatabaseManager {
    /// Create a new database manager from connection string
    pub fn new(conn_string: &str) -> Result<Self> {
        info!("Connecting to database: {}", conn_string);

        let mut cfg = Config::new();
        cfg.url = Some(conn_string.to_string());
        cfg.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 16,
            min_idle: 2,
            ..Default::default()
        });
        cfg.timeouts = Some(deadpool_postgres::Timeouts {
            wait: Some(Duration::from_secs(30)),
            ..Default::default()
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create database pool")?;

        info!("Database pool created successfully");
        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub async fn get_conn(&self) -> Result<deadpool_postgres::Object> {
        self.pool
            .get()
            .await
            .context("Failed to get database connection")
    }

    /// Test database connection
    pub async fn test_connection(&self) -> Result<()> {
        let conn = self.get_conn().await?;
        let row = conn
            .query_one("SELECT version()", &[])
            .await
            .context("Failed to query database version")?;

        let version: String = row.get(0);
        info!("Database connection test successful. Version: {}", version);
        Ok(())
    }

    /// Initialize admin tables (run migrations)
    pub async fn init_admin_tables(&self) -> Result<()> {
        info!("Initializing admin tables...");

        let migration_sql = include_str!("../../migrations/001_admin_tables.sql");
        let conn = self.get_conn().await?;

        conn.batch_execute(migration_sql)
            .await
            .context("Failed to execute admin tables migration")?;

        info!("Admin tables initialized successfully");
        Ok(())
    }
}

// ============================================================================
// Data Models for API Responses
// ============================================================================

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub pool_hashrate_3h: u64,
    pub active_miners: i64,
    pub active_workers: i64,
    pub last_block_height: i64,
    pub next_block_eta_seconds: i64,
    pub pool_fee_percent: f64,
    pub network_difficulty: u64,
    pub block_reward: f64,
}

/// Miner statistics (for Observer API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub address: String,
    pub shares_in_window: u64,
    pub estimated_reward_window: f64,
    pub estimated_next_block: f64,
    pub hashrate_3h: u64,
    pub hashrate_avg: HashrateAverage,
    pub workers: Vec<WorkerInfo>,
    pub latest_earnings: Vec<EarningRecord>,
}

/// Hashrate averages at different time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashrateAverage {
    #[serde(rename = "1h")]
    pub hour_1: u64,
    #[serde(rename = "6h")]
    pub hour_6: u64,
    #[serde(rename = "24h")]
    pub hour_24: u64,
    #[serde(rename = "7d")]
    pub day_7: u64,
}

/// Worker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub name: String,
    pub hashrate: u64,
    pub shares: u64,
    pub last_seen: String,
    pub is_online: bool,
}

/// Earning record (payout)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningRecord {
    pub block_height: i64,
    pub time: String,
    pub amount_btc: f64,
    pub txid: Option<String>,
    pub confirmations: i32,
}

/// Hashrate data point for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashrateDataPoint {
    pub timestamp: String,
    pub hashrate: u64,
}

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: i64,
    pub time: String,
    pub reward_btc: f64,
    pub pool_fee_percent: f64,
    pub txid: Option<String>,
    pub confirmations: i32,
    pub payouts_count: i64,
}

/// Block detail with PPLNS distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDetail {
    pub height: i64,
    pub time: String,
    pub reward_btc: f64,
    pub pool_fee_btc: f64,
    pub network_difficulty: u64,
    pub txid: Option<String>,
    pub confirmations: i32,
    pub pplns_window_shares: i64,
    pub payouts: Vec<PayoutDetail>,
}

/// Payout detail for a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutDetail {
    pub address: String,
    pub amount_btc: f64,
    pub shares: u64,
    #[serde(rename = "share_percent")]
    pub share_percent: f64,
}

// ============================================================================
// Query Functions
// ============================================================================

impl DatabaseManager {
    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> Result<PoolStats> {
        let conn = self.get_conn().await?;

        // Get pool config values
        let fee_percent: f64 = conn
            .query_one(
                "SELECT value::float FROM system_configs WHERE key = 'pool.fee_percent'",
                &[]
            )
            .await?
            .get(0);

        let min_payout_sats: i64 = conn
            .query_one(
                "SELECT value::bigint FROM system_configs WHERE key = 'pool.min_payout_sats'",
                &[]
            )
            .await?
            .get(0);

        // Get active miners count (from Hydrapool's miners table)
        let active_miners: i64 = conn
            .query_one(
                "SELECT COUNT(*) FROM miners WHERE balance_sats > 0 OR id IN (SELECT DISTINCT miner_id FROM shares WHERE created_at > NOW() - INTERVAL '24 hours')",
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

        // Calculate pool hashrate from shares in last 3 hours
        let row = conn
            .query_one(
                "SELECT COALESCE(SUM(difficulty), 0) as total_difficulty FROM shares WHERE created_at > NOW() - INTERVAL '3 hours'",
                &[]
            )
            .await?;

        let total_difficulty: i64 = row.get("total_difficulty");
        let pool_hashrate_3h = (total_difficulty as f64 / (3.0 * 3600.0)) as u64;

        Ok(PoolStats {
            pool_hashrate_3h,
            active_miners,
            active_workers,
            last_block_height: 0, // TODO: Get from Bitcoin node
            next_block_eta_seconds: 3600, // TODO: Calculate
            pool_fee_percent: fee_percent,
            network_difficulty: 0, // TODO: Get from Bitcoin node
            block_reward: 3.125, // Current Bitcoin reward
        })
    }

    /// Get miner statistics
    pub async fn get_miner_stats(&self, address: &str) -> Result<Option<MinerStats>> {
        let conn = self.get_conn().await?;

        // Check if miner exists
        let miner_exists: bool = conn
            .query_one("SELECT EXISTS(SELECT 1 FROM miners WHERE address = $1)", &[&address])
            .await?
            .get(0);

        if !miner_exists {
            return Ok(None);
        }

        // Get shares in PPLNS window
        let row = conn
            .query_one(
                "SELECT COALESCE(SUM(difficulty), 0) as shares FROM shares WHERE miner_id = (SELECT id FROM miners WHERE address = $1) AND created_at > NOW() - INTERVAL '7 days'",
                &[&address]
            )
            .await?;

        let shares_in_window: i64 = row.get("shares");

        // Calculate hashrate averages
        let hashrate_avg = self.calculate_miner_hashrate_avg(&conn, address).await?;

        // Get workers
        let workers = self.get_miner_workers(&conn, address).await?;

        // Get latest earnings
        let latest_earnings = self.get_miner_earnings(&conn, address, 10).await?;

        // Calculate estimated rewards
        let estimated_reward_window = 0.0; // TODO: Calculate based on shares_in_window
        let estimated_next_block = 0.0; // TODO: Calculate

        Ok(Some(MinerStats {
            address: address.to_string(),
            shares_in_window: shares_in_window as u64,
            estimated_reward_window,
            estimated_next_block,
            hashrate_3h: hashrate_avg.hour_1,
            hashrate_avg,
            workers,
            latest_earnings,
        })
    }

    /// Calculate miner hashrate at different time periods
    async fn calculate_miner_hashrate_avg(&self, conn: &deadpool_postgres::Object, address: &str) -> Result<HashrateAverage> {
        let periods = [3600, 21600, 86400, 604800]; // 1h, 6h, 24h, 7d in seconds

        let mut hashrates = Vec::new();
        for period_seconds in periods {
            let row = conn
                .query_one(
                    "SELECT COALESCE(SUM(difficulty), 0) as total_difficulty FROM shares WHERE miner_id = (SELECT id FROM miners WHERE address = $1) AND created_at > NOW() - INTERVAL '1 second' * $2",
                    &[&address, &(period_seconds as i64)]
                )
                .await?;

            let total_difficulty: i64 = row.get("total_difficulty");
            let hashrate = (total_difficulty as f64 / period_seconds as f64) as u64;
            hashrates.push(hashrate);
        }

        Ok(HashrateAverage {
            hour_1: hashrates[0],
            hour_6: hashrates[1],
            hour_24: hashrates[2],
            day_7: hashrates[3],
        })
    }

    /// Get miner's workers
    async fn get_miner_workers(&self, conn: &deadpool_postgres::Object, address: &str) -> Result<Vec<WorkerInfo>> {
        let rows = conn
            .query(
                "SELECT worker_name, current_hashrate, total_shares, last_seen, is_online FROM worker_status_cache WHERE miner_address = $1 ORDER BY last_seen DESC",
                &[&address]
            )
            .await?;

        let mut workers = Vec::new();
        for row in rows {
            workers.push(WorkerInfo {
                name: row.get("worker_name"),
                hashrate: row.get("current_hashrate"),
                shares: row.get("total_shares"),
                last_seen: row.get::<_, chrono::DateTime<chrono::Utc>>("last_seen").to_rfc3339(),
                is_online: row.get("is_online"),
            });
        }

        Ok(workers)
    }

    /// Get miner's earnings (payouts)
    async fn get_miner_earnings(&self, conn: &deadpool_postgres::Object, address: &str, limit: i64) -> Result<Vec<EarningRecord>> {
        // Check block_details_cache first, then fallback to payouts table
        let rows = conn
            .query(
                "SELECT block_height, block_time, reward_sats, coinbase_txid FROM block_details_cache WHERE block_height IN (SELECT block_height FROM payouts WHERE miner_id = (SELECT id FROM miners WHERE address = $1)) ORDER BY block_time DESC LIMIT $2",
                &[&address, &limit]
            )
            .await?;

        let mut earnings = Vec::new();
        for row in rows {
            let reward_sats: i64 = row.get("reward_sats");
            let txid: Option<String> = row.get("coinbase_txid");

            earnings.push(EarningRecord {
                block_height: row.get("block_height"),
                time: row.get::<_, chrono::DateTime<chrono::Utc>>("block_time").to_rfc3339(),
                amount_btc: reward_sats as f64 / 100_000_000.0,
                txid,
                confirmations: 100, // TODO: Calculate from current block height
            });
        }

        Ok(earnings)
    }

    /// Get hashrate history for charts
    pub async fn get_miner_hashrate_history(&self, address: &str, period_days: i64) -> Result<Vec<HashrateDataPoint>> {
        let conn = self.get_conn().await?;

        let rows = conn
            .query(
                "SELECT date_trunc('hour', created_at) as hour, SUM(difficulty) as total_difficulty FROM shares WHERE miner_id = (SELECT id FROM miners WHERE address = $1) AND created_at > NOW() - INTERVAL '1 day' * $2 GROUP BY date_trunc('hour', created_at) ORDER BY hour ASC",
                &[&address, &period_days]
            )
            .await?;

        let mut data_points = Vec::new();
        for row in rows {
            let hour: chrono::DateTime<chrono::Utc> = row.get("hour");
            let total_difficulty: i64 = row.get("total_difficulty");

            data_points.push(HashrateDataPoint {
                timestamp: hour.to_rfc3339(),
                hashrate: (total_difficulty as f64 / 3600.0) as u64,
            });
        }

        Ok(data_points)
    }

    /// Get block list
    pub async fn get_blocks(&self, limit: i64, offset: i64) -> Result<Vec<BlockInfo>> {
        let conn = self.get_conn().await?;

        let rows = conn
            .query(
                "SELECT block_height, block_time, reward_sats, pool_fee_sats, coinbase_txid, payout_count FROM block_details_cache ORDER BY block_time DESC LIMIT $1 OFFSET $2",
                &[&limit, &offset]
            )
            .await?;

        let mut blocks = Vec::new();
        for row in rows {
            let reward_sats: i64 = row.get("reward_sats");
            let fee_sats: i64 = row.get("pool_fee_sats");

            blocks.push(BlockInfo {
                height: row.get("block_height"),
                time: row.get::<_, chrono::DateTime<chrono::Utc>>("block_time").to_rfc3339(),
                reward_btc: reward_sats as f64 / 100_000_000.0,
                pool_fee_percent: (fee_sats as f64 / reward_sats as f64) * 100.0,
                txid: row.get("coinbase_txid"),
                confirmations: 100, // TODO: Calculate
                payouts_count: row.get("payout_count"),
            });
        }

        Ok(blocks)
    }

    /// Get block detail with PPLNS distribution
    pub async fn get_block_detail(&self, height: i64) -> Result<Option<BlockDetail>> {
        let conn = self.get_conn().await?;

        let block_row = match conn
            .query_one(
                "SELECT * FROM block_details_cache WHERE block_height = $1",
                &[&height]
            )
            .await
        {
            Ok(row) => row,
            Err(_) => return Ok(None),
        };

        let reward_sats: i64 = block_row.get("reward_sats");
        let fee_sats: i64 = block_row.get("pool_fee_sats");

        // Get PPLNS payouts for this block
        let payout_rows = conn
            .query(
                "SELECT miner_address, shares, reward_sats FROM block_payouts WHERE block_height = $1 ORDER BY reward_sats DESC",
                &[&height]
            )
            .await?;

        let mut payouts = Vec::new();
        for row in payout_rows {
            let shares: i64 = row.get("shares");
            let payout_sats: i64 = row.get("reward_sats");
            let total_difficulty: i64 = block_row.get("pplns_total_difficulty");

            payouts.push(PayoutDetail {
                address: row.get("miner_address"),
                amount_btc: payout_sats as f64 / 100_000_000.0,
                shares: shares as u64,
                share_percent: (shares as f64 / total_difficulty as f64) * 100.0,
            });
        }

        Ok(Some(BlockDetail {
            height,
            time: block_row.get::<_, chrono::DateTime<chrono::Utc>>("block_time").to_rfc3339(),
            reward_btc: reward_sats as f64 / 100_000_000.0,
            pool_fee_btc: fee_sats as f64 / 100_000_000.0,
            network_difficulty: 0, // TODO: Get from Bitcoin node
            txid: block_row.get("coinbase_txid"),
            confirmations: 100, // TODO: Calculate
            pplns_window_shares: block_row.get("pplns_window_shares"),
            payouts,
        }))
    }
}
