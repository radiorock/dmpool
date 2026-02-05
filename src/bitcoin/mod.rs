// Bitcoin RPC Client for DMPool
// Handles communication with Bitcoin node for transaction creation and broadcasting

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{error, info, warn};

/// Bitcoin RPC client
pub struct BitcoinRpcClient {
    url: String,
    username: String,
    password: String,
    client: reqwest::Client,
}

impl BitcoinRpcClient {
    /// Create a new Bitcoin RPC client
    pub fn new(url: String, username: String, password: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            url,
            username,
            password,
            client,
        }
    }

    /// Execute a raw RPC call
    async fn call(&self, method: &str, params: Vec<serde_json::Value>) -> Result<serde_json::Value> {
        let request_body = json!({
            "jsonrpc": "1.0",
            "id": "1",
            "method": method,
            "params": params
        });

        let response = self.client
            .post(&self.url)
            .basic_auth(&self.username, Some(&self.password))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send RPC request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "RPC request failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let response_text = response.text().await.context("Failed to read response")?;
        let rpc_response: RpcResponse = serde_json::from_str(&response_text)
            .context("Failed to parse RPC response")?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow::anyhow!("RPC error: {}", error.message));
        }

        rpc_response.result.ok_or_else(|| anyhow::anyhow!("RPC response missing result"))
    }

    /// Get blockchain info
    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        let result = self.call("getblockchaininfo", vec![]).await?;
        serde_json::from_value(result).context("Failed to parse blockchain info")
    }

    /// Get block count
    pub async fn get_block_count(&self) -> Result<u64> {
        let result = self.call("getblockcount", vec![]).await?;
        serde_json::from_value(result).context("Failed to parse block count")
    }

    /// Get network hashps (estimated network hashrate)
    pub async fn get_network_hash_ps(&self, blocks: u32, height: Option<u64>) -> Result<f64> {
        let params = if let Some(h) = height {
            vec![serde_json::json!(blocks), serde_json::json!(h)]
        } else {
            vec![serde_json::json!(blocks)]
        };
        let result = self.call("getnetworkhashps", params).await?;
        serde_json::from_value(result).context("Failed to parse network hashps")
    }

    /// Get mempool info
    pub async fn get_mempool_info(&self) -> Result<MempoolInfo> {
        let result = self.call("getmempoolinfo", vec![]).await?;
        serde_json::from_value(result).context("Failed to parse mempool info")
    }

    /// Get raw transaction
    pub async fn get_raw_transaction(&self, txid: &str) -> Result<String> {
        let result = self.call("getrawtransaction", vec![json!(txid)]).await?;
        serde_json::from_value(result).context("Failed to parse raw transaction")
    }

    /// Decode raw transaction
    pub async fn decode_raw_transaction(&self, hex: &str) -> Result<DecodedTransaction> {
        let result = self.call("decoderawtransaction", vec![json!(hex)]).await?;
        serde_json::from_value(result).context("Failed to decode transaction")
    }

    /// Create raw transaction
    pub async fn create_raw_transaction(
        &self,
        inputs: Vec<TxInput>,
        outputs: Vec<TxOutput>,
        locktime: Option<u32>,
    ) -> Result<String> {
        let inputs_json = serde_json::to_value(inputs)?;
        let outputs_json = serde_json::to_value(outputs)?;
        let mut params = vec![inputs_json, outputs_json];
        if let Some(lt) = locktime {
            params.push(json!(lt));
        }
        let result = self.call("createrawtransaction", params).await?;
        serde_json::from_value(result).context("Failed to create raw transaction")
    }

    /// Sign raw transaction with wallet
    pub async fn sign_raw_transaction_with_wallet(
        &self,
        hex: &str,
    ) -> Result<SignedTransaction> {
        let result = self.call("signrawtransactionwithwallet", vec![json!(hex)]).await?;
        serde_json::from_value(result).context("Failed to sign transaction")
    }

    /// Broadcast raw transaction
    pub async fn send_raw_transaction(&self, hex: &str) -> Result<String> {
        let result = self.call("sendrawtransaction", vec![json!(hex)]).await?;
        serde_json::from_value(result).context("Failed to broadcast transaction")
    }

    /// Get wallet info
    pub async fn get_wallet_info(&self) -> Result<WalletInfo> {
        let result = self.call("getwalletinfo", vec![]).await?;
        serde_json::from_value(result).context("Failed to parse wallet info")
    }

    /// List unspent outputs
    pub async fn list_unspent(
        &self,
        minconf: Option<u32>,
        maxconf: Option<u32>,
    ) -> Result<Vec<UnspentOutput>> {
        let minconf = minconf.unwrap_or(0);
        let maxconf = maxconf.unwrap_or(999999);
        let result = self.call(
            "listunspent",
            vec![json!(minconf), json!(maxconf)]
        ).await?;
        serde_json::from_value(result).context("Failed to parse unspent outputs")
    }

    /// Estimate smart fee
    pub async fn estimate_smart_fee(&self, conf_target: u32) -> Result<f64> {
        let result = self.call("estimatesmartfee", vec![json!(conf_target)]).await?;
        // Parse the response which may be a number or an object with "feerate" field
        if let Ok(feerate) = serde_json::from_value::<f64>(result.clone()) {
            return Ok(feerate);
        }
        if let Some(obj) = result.as_object() {
            if let Some(feerate) = obj.get("feerate").and_then(|v| v.as_f64()) {
                return Ok(feerate);
            }
        }
        Ok(0.00001) // Default fallback
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<bool> {
        match self.get_blockchain_info().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Bitcoin RPC connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

/// RPC response structure
#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
}

/// Blockchain info
#[derive(Debug, Clone, Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub difficulty: f64,
    pub initial_block_download: bool,
}

/// Mempool info
#[derive(Debug, Clone, Deserialize)]
pub struct MempoolInfo {
    pub size: u64,
    pub bytes: u64,
    pub usage: f64,
    pub maxmempool: f64,
}

/// Decoded transaction
#[derive(Debug, Clone, Deserialize)]
pub struct DecodedTransaction {
    pub txid: String,
    pub hash: String,
    pub version: u32,
    pub size: u64,
    pub vsize: u64,
    pub weight: u64,
    pub locktime: u32,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vin {
    pub txid: Option<String>,
    pub vout: Option<u32>,
    pub script_sig: Option<ScriptSig>,
    pub sequence: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptSig {
    pub asm: String,
    pub hex: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vout {
    pub value: f64,
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub addresses: Option<Vec<String>>,
}

/// Transaction input for creating transactions
#[derive(Debug, Clone, Serialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub sequence: Option<u32>,
}

/// Transaction output for creating transactions
#[derive(Debug, Clone, Serialize)]
pub struct TxOutput {
    pub address: String,
    pub amount: f64,
}

/// Signed transaction
#[derive(Debug, Clone, Deserialize)]
pub struct SignedTransaction {
    pub hex: String,
    pub complete: bool,
}

/// Wallet info
#[derive(Debug, Clone, Deserialize)]
pub struct WalletInfo {
    pub wallet_name: String,
    pub balance: f64,
    pub unconfirmed_balance: f64,
    pub immature_balance: f64,
    pub txcount: u64,
}

/// Unspent output
#[derive(Debug, Clone, Deserialize)]
pub struct UnspentOutput {
    pub txid: String,
    pub vout: u32,
    pub address: Option<String>,
    pub amount: f64,
    pub confirmations: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BitcoinRpcClient::new(
            "http://127.0.0.1:8332".to_string(),
            "user".to_string(),
            "pass".to_string()
        );
        assert_eq!(client.url, "http://127.0.0.1:8332");
    }
}
