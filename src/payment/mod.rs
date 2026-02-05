// Payment System Module for DMPool
// Handles miner balance tracking, payout calculations, and Bitcoin transactions

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use crate::bitcoin::BitcoinRpcClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Payout record representing a single payment to a miner
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payout {
    /// Unique payout ID
    pub id: String,
    /// Bitcoin address of the miner
    pub address: String,
    /// Amount in satoshis
    pub amount_satoshis: u64,
    /// Transaction ID (set after broadcast)
    pub txid: Option<String>,
    /// Block height when payout was created
    pub block_height: Option<u64>,
    /// Payout status
    pub status: PayoutStatus,
    /// Timestamp when payout was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when payout was broadcast
    pub broadcast_at: Option<DateTime<Utc>>,
    /// Number of confirmations (0 if pending)
    pub confirmations: u32,
    /// Error message if failed
    pub error: Option<String>,
}

/// Payout status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PayoutStatus {
    /// Pending - waiting to be broadcast
    Pending,
    /// Broadcast - waiting for confirmations
    Broadcast,
    /// Confirmed - has required confirmations
    Confirmed,
    /// Failed - transaction failed
    Failed,
}

/// Miner balance record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinerBalance {
    /// Bitcoin address
    pub address: String,
    /// Current balance in satoshis
    pub balance_satoshis: u64,
    /// Total earned (lifetime)
    pub total_earned_satoshis: u64,
    /// Total paid out (lifetime)
    pub total_paid_satoshis: u64,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Payment configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentConfig {
    /// Minimum payout threshold in satoshis (0.01 BTC = 1,000,000 satoshis)
    pub min_payout_satoshis: u64,
    /// Manual payout threshold in satoshis (0.001 BTC = 100,000 satoshis)
    pub manual_payout_satoshis: u64,
    /// Lightning payout threshold in satoshis (0.0001 BTC = 10,000 satoshis)
    pub lightning_payout_satoshis: u64,
    /// Required confirmations before considering payout complete
    pub required_confirmations: u32,
    /// Pool fee percentage (basis points: 100 = 1%)
    pub pool_fee_bps: u32,
    /// Donation percentage (basis points)
    pub donation_bps: u32,
    /// Enable automatic payouts
    pub auto_payout_enabled: bool,
    /// Auto payout interval in hours
    pub auto_payout_interval_hours: u32,
    /// Bitcoin RPC settings
    pub bitcoin_rpc_url: String,
    pub bitcoin_rpc_user: String,
    pub bitcoin_rpc_pass: String,
}

impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            min_payout_satoshis: 1_000_000,      // 0.01 BTC
            manual_payout_satoshis: 100_000,     // 0.001 BTC
            lightning_payout_satoshis: 10_000,   // 0.0001 BTC
            required_confirmations: 6,
            pool_fee_bps: 100,                   // 1%
            donation_bps: 0,
            auto_payout_enabled: false,
            auto_payout_interval_hours: 24,
            bitcoin_rpc_url: "http://127.0.0.1:8332".to_string(),
            bitcoin_rpc_user: "bitcoin".to_string(),
            bitcoin_rpc_pass: String::new(),
        }
    }
}

/// Payment manager
pub struct PaymentManager {
    /// Miner balances (address -> balance)
    balances: Arc<RwLock<HashMap<String, MinerBalance>>>,
    /// Payout history
    payouts: Arc<RwLock<Vec<Payout>>>,
    /// Configuration
    config: Arc<RwLock<PaymentConfig>>,
    /// Bitcoin RPC client
    bitcoin_client: Arc<BitcoinRpcClient>,
    /// Data directory for persistence
    data_dir: PathBuf,
    /// Maximum payouts to keep in memory
    max_payouts: usize,
}

impl PaymentManager {
    /// Create a new payment manager
    pub fn new(data_dir: PathBuf, config: PaymentConfig) -> Result<Self> {
        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir)
            .context("Failed to create payment data directory")?;

        // Create Bitcoin RPC client
        let bitcoin_client = Arc::new(BitcoinRpcClient::new(
            config.bitcoin_rpc_url.clone(),
            config.bitcoin_rpc_user.clone(),
            config.bitcoin_rpc_pass.clone(),
        ));

        Ok(Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            payouts: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(RwLock::new(config)),
            bitcoin_client,
            data_dir,
            max_payouts: 10000,
        })
    }

    /// Load persisted data from disk
    pub async fn load(&self) -> Result<()> {
        // Load balances
        let balances_path = self.data_dir.join("balances.json");
        if balances_path.exists() {
            let mut file = File::open(&balances_path).await
                .context("Failed to open balances file")?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;
            let balances: HashMap<String, MinerBalance> = serde_json::from_slice(&contents)
                .context("Failed to parse balances file")?;
            let count = balances.len();
            *self.balances.write().await = balances;
            info!("Loaded {} miner balances", count);
        }

        // Load payouts
        let payouts_path = self.data_dir.join("payouts.json");
        if payouts_path.exists() {
            let mut file = File::open(&payouts_path).await
                .context("Failed to open payouts file")?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;
            let payouts: Vec<Payout> = serde_json::from_slice(&contents)
                .context("Failed to parse payouts file")?;
            let count = payouts.len();
            *self.payouts.write().await = payouts;
            info!("Loaded {} payout records", count);
        }

        Ok(())
    }

    /// Save data to disk
    pub async fn save(&self) -> Result<()> {
        // Save balances
        let balances_path = self.data_dir.join("balances.json");
        let balances = self.balances.read().await;
        let balances_json = serde_json::to_vec_pretty(&*balances)
            .context("Failed to serialize balances")?;
        drop(balances);
        {
            let mut file = File::create(&balances_path).await
                .context("Failed to create balances file")?;
            file.write_all(&balances_json).await?;
        }

        // Save payouts
        let payouts_path = self.data_dir.join("payouts.json");
        let payouts = self.payouts.read().await;
        let payouts_json = serde_json::to_vec_pretty(&*payouts)
            .context("Failed to serialize payouts")?;
        drop(payouts);
        {
            let mut file = File::create(&payouts_path).await
                .context("Failed to create payouts file")?;
            file.write_all(&payouts_json).await?;
        }

        Ok(())
    }

    /// Add earnings to a miner's balance (call when block is found)
    pub async fn add_earnings(&self, address: String, amount_satoshis: u64, block_height: u64) -> Result<()> {
        let mut balances = self.balances.write().await;
        let balance = balances.entry(address.clone()).or_insert_with(|| MinerBalance {
            address: address.clone(),
            balance_satoshis: 0,
            total_earned_satoshis: 0,
            total_paid_satoshis: 0,
            updated_at: Utc::now(),
        });

        balance.balance_satoshis += amount_satoshis;
        balance.total_earned_satoshis += amount_satoshis;
        balance.updated_at = Utc::now();

        info!("Added {} satoshis to {} (block {}), new balance: {}",
            amount_satoshis, address, block_height, balance.balance_satoshis);

        Ok(())
    }

    /// Get miner balance
    pub async fn get_balance(&self, address: &str) -> Option<MinerBalance> {
        self.balances.read().await.get(address).cloned()
    }

    /// Get all balances
    pub async fn get_all_balances(&self) -> Vec<MinerBalance> {
        self.balances.read().await.values().cloned().collect()
    }

    /// Get pending payouts (balances above threshold)
    pub async fn get_pending_payouts(&self) -> Vec<(String, u64)> {
        let config = self.config.read().await;
        let threshold = config.min_payout_satoshis;
        drop(config);

        let balances = self.balances.read().await;
        balances.iter()
            .filter(|(_, b)| b.balance_satoshis >= threshold)
            .map(|(addr, b)| (addr.clone(), b.balance_satoshis))
            .collect()
    }

    /// Create a payout record (doesn't broadcast)
    pub async fn create_payout(&self, address: String, amount_satoshis: u64) -> Result<Payout> {
        // Check if miner has enough balance
        let balance = {
            let balances = self.balances.read().await;
            balances.get(&address).cloned()
        };

        let balance = balance.ok_or_else(|| anyhow::anyhow!("No balance found for address {}", address))?;

        if balance.balance_satoshis < amount_satoshis {
            return Err(anyhow::anyhow!(
                "Insufficient balance: requested {}, available {}",
                amount_satoshis, balance.balance_satoshis
            ));
        }

        // Create payout record
        let payout = Payout {
            id: uuid::Uuid::new_v4().to_string(),
            address: address.clone(),
            amount_satoshis,
            txid: None,
            block_height: None,
            status: PayoutStatus::Pending,
            created_at: Utc::now(),
            broadcast_at: None,
            confirmations: 0,
            error: None,
        };

        // Deduct from balance (marked as pending until confirmed)
        {
            let mut balances = self.balances.write().await;
            if let Some(b) = balances.get_mut(&address) {
                b.balance_satoshis -= amount_satoshis;
                b.updated_at = Utc::now();
            }
        }

        // Add to payouts
        {
            let mut payouts = self.payouts.write().await;
            payouts.push(payout.clone());

            // Trim if exceeded max
            if payouts.len() > self.max_payouts {
                let remove_count = payouts.len() - self.max_payouts;
                payouts.drain(0..remove_count);
            }
        }

        // Save to disk
        self.save().await?;

        info!("Created payout {} to {} for {} satoshis", payout.id, address, amount_satoshis);

        Ok(payout)
    }

    /// Broadcast a payout (build and send Bitcoin transaction)
    pub async fn broadcast_payout(&self, payout_id: &str) -> Result<Payout> {
        let config = self.config.read().await;

        // Find the payout
        let mut payout = {
            let payouts = self.payouts.read().await;
            payouts.iter()
                .find(|p| p.id == payout_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Payout {} not found", payout_id))?
        };

        if payout.status != PayoutStatus::Pending {
            return Err(anyhow::anyhow!("Payout {} is not pending", payout_id));
        }

        info!("Building transaction for payout {} to {} ({} satoshis)",
            payout.id, payout.address, payout.amount_satoshis);

        // Convert satoshis to BTC
        let amount_btc = payout.amount_satoshis as f64 / 100_000_000.0;

        // Get unspent outputs from wallet
        let unspent = self.bitcoin_client.list_unspent(Some(1), Some(999999)).await
            .context("Failed to get unspent outputs")?;

        if unspent.is_empty() {
            let error_msg = "No unspent outputs available in wallet".to_string();
            payout.status = PayoutStatus::Failed;
            payout.error = Some(error_msg.clone());

            // Update payouts
            {
                let mut payouts = self.payouts.write().await;
                if let Some(p) = payouts.iter_mut().find(|p| p.id == payout_id) {
                    *p = payout.clone();
                }
            }
            self.save().await?;

            return Err(anyhow::anyhow!("No unspent outputs available"));
        }

        // Select inputs (simple implementation - use first available utxo)
        // In production, you'd want to implement proper coin selection
        let utxo = &unspent[0];
        let total_input = (utxo.amount * 100_000_000.0) as u64; // Convert BTC to satoshis

        // Calculate change
        let change_satoshis = total_input - payout.amount_satoshis;
        let fee_estimate = config.donation_bps as u64; // Use a reasonable fee estimate
        let actual_change = change_satoshis.saturating_sub(fee_estimate);

        if actual_change < 546 { // Dust limit
            return Err(anyhow::anyhow!("Amount too small after fees"));
        }

        let change_btc = actual_change as f64 / 100_000_000.0;

        // Create transaction outputs
        let outputs = vec![
            crate::bitcoin::TxOutput {
                address: payout.address.clone(),
                amount: amount_btc,
            },
            crate::bitcoin::TxOutput {
                // Return change to the pool's address
                // In production, this should be configured separately
                address: utxo.address.clone().unwrap_or_else(|| utxo.txid.clone()), // Fallback to input address
                amount: change_btc,
            },
        ];

        // Create transaction input
        let inputs = vec![
            crate::bitcoin::TxInput {
                txid: utxo.txid.clone(),
                vout: utxo.vout,
                sequence: None,
            }
        ];

        // Create raw transaction
        let raw_tx = self.bitcoin_client.create_raw_transaction(inputs, outputs, None).await
            .context("Failed to create raw transaction")?;

        info!("Created raw transaction: {}", raw_tx);

        // Sign transaction with wallet
        let signed_tx = self.bitcoin_client.sign_raw_transaction_with_wallet(&raw_tx).await
            .context("Failed to sign transaction")?;

        if !signed_tx.complete {
            return Err(anyhow::anyhow!("Transaction signing incomplete"));
        }

        info!("Signed transaction: {}", signed_tx.hex);

        // Broadcast transaction
        let txid = self.bitcoin_client.send_raw_transaction(&signed_tx.hex).await
            .context("Failed to broadcast transaction")?;

        info!("Broadcast transaction {} for payout {}", txid, payout.id);

        // Update payout
        payout.txid = Some(txid.clone());
        payout.status = PayoutStatus::Broadcast;
        payout.broadcast_at = Some(Utc::now());

        // Update payouts
        {
            let mut payouts = self.payouts.write().await;
            if let Some(p) = payouts.iter_mut().find(|p| p.id == payout_id) {
                *p = payout.clone();
            }
        }

        self.save().await?;

        info!("Successfully broadcast payout {} to {} for {} satoshis (txid: {})",
            payout.id, payout.address, payout.amount_satoshis, txid);

        Ok(payout)
    }

    /// Get payout history for an address
    pub async fn get_payout_history(&self, address: &str, limit: usize) -> Vec<Payout> {
        let payouts = self.payouts.read().await;
        payouts.iter()
            .filter(|p| p.address == address)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get all pending payouts
    pub async fn get_pending_payout_records(&self) -> Vec<Payout> {
        let payouts = self.payouts.read().await;
        payouts.iter()
            .filter(|p| p.status == PayoutStatus::Pending || p.status == PayoutStatus::Broadcast)
            .cloned()
            .collect()
    }

    /// Get all payouts
    pub async fn get_all_payouts(&self) -> Vec<Payout> {
        self.payouts.read().await.clone()
    }

    /// Confirm a payout (called when transaction gets confirmations)
    pub async fn confirm_payout(&self, payout_id: &str, txid: String, block_height: u64, confirmations: u32) -> Result<()> {
        let config = self.config.read().await;
        let required = config.required_confirmations;
        drop(config);

        let mut payouts = self.payouts.write().await;
        if let Some(payout) = payouts.iter_mut().find(|p| p.id == payout_id) {
            payout.txid = Some(txid.clone());
            payout.block_height = Some(block_height);
            payout.confirmations = confirmations;

            if confirmations >= required {
                payout.status = PayoutStatus::Confirmed;

                // Update miner's total paid
                let mut balances = self.balances.write().await;
                if let Some(balance) = balances.get_mut(&payout.address) {
                    balance.total_paid_satoshis += payout.amount_satoshis;
                }

                info!("Payout {} confirmed with {} confirmations", payout_id, confirmations);
            }

            self.save().await?;
        }

        Ok(())
    }

    /// Get payment statistics
    pub async fn get_stats(&self) -> PaymentStats {
        let payouts = self.payouts.read().await;
        let balances = self.balances.read().await;

        let total_paid: u64 = payouts.iter()
            .filter(|p| p.status == PayoutStatus::Confirmed)
            .map(|p| p.amount_satoshis)
            .sum();

        let pending_amount: u64 = payouts.iter()
            .filter(|p| p.status == PayoutStatus::Pending || p.status == PayoutStatus::Broadcast)
            .map(|p| p.amount_satoshis)
            .sum();

        let total_balance: u64 = balances.values()
            .map(|b| b.balance_satoshis)
            .sum();

        PaymentStats {
            total_miners: balances.len(),
            total_balance_satoshis: total_balance,
            total_paid_satoshis: total_paid,
            pending_payouts_satoshis: pending_amount,
            confirmed_payouts: payouts.iter().filter(|p| p.status == PayoutStatus::Confirmed).count(),
            pending_payouts: payouts.iter().filter(|p| p.status == PayoutStatus::Pending || p.status == PayoutStatus::Broadcast).count(),
        }
    }

    /// Update configuration
    pub async fn update_config(&self, config: PaymentConfig) -> Result<()> {
        *self.config.write().await = config;
        self.save().await?;
        Ok(())
    }

    /// Get configuration
    pub async fn get_config(&self) -> PaymentConfig {
        self.config.read().await.clone()
    }

    /// Process automatic payouts (call periodically)
    pub async fn process_auto_payouts(&self) -> Result<Vec<Payout>> {
        let config = self.config.read().await;
        if !config.auto_payout_enabled {
            return Ok(Vec::new());
        }
        drop(config);

        let pending = self.get_pending_payouts().await;
        let mut created = Vec::new();

        for (address, amount) in pending {
            match self.create_payout(address.clone(), amount).await {
                Ok(payout) => {
                    created.push(payout);
                }
                Err(e) => {
                    error!("Failed to create payout for {}: {}", address, e);
                }
            }
        }

        // Broadcast all created payouts
        for payout in &created {
            if let Err(e) = self.broadcast_payout(&payout.id).await {
                error!("Failed to broadcast payout {}: {}", payout.id, e);
            }
        }

        Ok(created)
    }
}

/// Payment statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentStats {
    pub total_miners: usize,
    pub total_balance_satoshis: u64,
    pub total_paid_satoshis: u64,
    pub pending_payouts_satoshis: u64,
    pub confirmed_payouts: usize,
    pub pending_payouts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_add_earnings() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PaymentManager::new(temp_dir.path().to_path_buf(), PaymentConfig::default())
            .unwrap();

        manager.add_earnings("bc1qtest".to_string(), 500_000, 123).await.unwrap();

        let balance = manager.get_balance("bc1qtest").await;
        assert!(balance.is_some());
        assert_eq!(balance.unwrap().balance_satoshis, 500_000);
    }

    #[tokio::test]
    async fn test_create_payout() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PaymentConfig::default();
        config.min_payout_satoshis = 100_000;
        let manager = PaymentManager::new(temp_dir.path().to_path_buf(), config)
            .unwrap();

        // Add earnings
        manager.add_earnings("bc1qtest".to_string(), 500_000, 123).await.unwrap();

        // Create payout
        let payout = manager.create_payout("bc1qtest".to_string(), 100_000).await.unwrap();
        assert_eq!(payout.amount_satoshis, 100_000);
        assert_eq!(payout.status, PayoutStatus::Pending);

        // Balance should be reduced
        let balance = manager.get_balance("bc1qtest").await.unwrap();
        assert_eq!(balance.balance_satoshis, 400_000);
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PaymentManager::new(temp_dir.path().to_path_buf(), PaymentConfig::default())
            .unwrap();

        manager.add_earnings("bc1qtest".to_string(), 50_000, 123).await.unwrap();

        let result = manager.create_payout("bc1qtest".to_string(), 100_000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PaymentManager::new(temp_dir.path().to_path_buf(), PaymentConfig::default())
            .unwrap();

        manager.add_earnings("bc1qtest".to_string(), 500_000, 123).await.unwrap();
        manager.save().await.unwrap();

        // Create new manager and load
        let manager2 = PaymentManager::new(temp_dir.path().to_path_buf(), PaymentConfig::default())
            .unwrap();
        manager2.load().await.unwrap();

        let balance = manager2.get_balance("bc1qtest").await;
        assert!(balance.is_some());
        assert_eq!(balance.unwrap().balance_satoshis, 500_000);
    }
}
