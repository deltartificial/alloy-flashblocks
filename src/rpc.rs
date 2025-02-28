use eyre::Result;
use serde_json::Value;
use std::error::Error;
use tracing::{error, info};

pub struct FlashblocksRpcClient {
    endpoint: String,
}

impl FlashblocksRpcClient {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub async fn query_latest_flashblock(&self) -> Result<(), Box<dyn Error>> {
        info!("Testing eth_getBlockByNumber with 'pending' tag...");

        let client = reqwest::Client::new();
        let response = client
            .post(&self.endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": ["pending", true],
                "id": 1
            }))
            .send()
            .await?;

        let json: Value = response.json().await?;

        if let Some(error) = json.get("error") {
            error!("RPC error: {:?}", error);
            return Err("RPC error".into());
        }

        let block = match json.get("result") {
            Some(block) => block,
            None => {
                error!("No result field in response");
                return Err("No result field in response".into());
            }
        };

        let number = block["number"]
            .as_str()
            .map(|s| u64::from_str_radix(&s[2..], 16))
            .transpose()?
            .unwrap_or(0);

        let hash = block["hash"].as_str().unwrap_or("N/A");
        let transactions = block["transactions"]
            .as_array()
            .map(|t| t.len())
            .unwrap_or(0);
        let timestamp = block["timestamp"]
            .as_str()
            .map(|s| u64::from_str_radix(&s[2..], 16))
            .transpose()?
            .unwrap_or(0);
        let gas_used = block["gasUsed"]
            .as_str()
            .map(|s| u64::from_str_radix(&s[2..], 16))
            .transpose()?
            .unwrap_or(0);

        info!("Latest Flashblock:");
        info!("  Number: {}", number);
        info!("  Hash: {}", hash);
        info!("  Timestamp: {}", timestamp);
        info!("  Gas Used: {}", gas_used);
        info!("  Transactions: {}", transactions);

        Ok(())
    }

    pub async fn get_balance(&self, address: &str) -> Result<u64, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBalance",
                "params": [address, "latest"],
                "id": 1
            }))
            .send()
            .await?;

        let json: Value = response.json().await?;

        if let Some(error) = json.get("error") {
            error!("RPC error: {:?}", error);
            return Err("RPC error".into());
        }

        let balance = json
            .get("result")
            .and_then(|v| v.as_str())
            .map(|s| u64::from_str_radix(&s[2..], 16))
            .transpose()?
            .unwrap_or(0);

        Ok(balance)
    }

    pub async fn get_receipt(&self, tx_hash: &str) -> Result<Value, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash],
                "id": 1
            }))
            .send()
            .await?;

        let json: Value = response.json().await?;

        if let Some(error) = json.get("error") {
            error!("RPC error: {:?}", error);
            return Err("RPC error".into());
        }

        let receipt = json
            .get("result")
            .ok_or_else(|| "No receipt found".to_string())?;

        Ok(receipt.clone())
    }

    pub async fn monitor_blocks(&self, count: u64) -> Result<(), Box<dyn Error>> {
        let mut blocks_seen = 0;

        while blocks_seen < count {
            match self.query_latest_flashblock().await {
                Ok(_) => blocks_seen += 1,
                Err(e) => error!("Error monitoring block: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }
}
