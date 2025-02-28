use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use eyre::Result;
use serde_json::{json, Value};
use std::{borrow::Cow, time::Duration};
use tokio::time;
use tracing::info;
use url::Url;

pub struct FlashblocksRpcClient {
    provider: Box<dyn Provider<Ethereum>>,
}

impl FlashblocksRpcClient {
    pub fn new(url: Url) -> Result<Self> {
        let provider = Box::new(ProviderBuilder::default().on_http(url));
        Ok(Self { provider })
    }

    pub async fn query_latest_flashblock(&self) -> Result<()> {
        info!("Testing eth_getBlockByNumber with 'pending' tag...");
        let pending_block: Value = self
            .provider
            .client()
            .request(
                Cow::Borrowed("eth_getBlockByNumber"),
                (json!("pending"), json!(true)),
            )
            .await?;

        if let Some(block) = pending_block.get("result") {
            info!("Latest Flashblock:");
            info!(
                "  Number: {}",
                block.get("number").unwrap_or(&json!("unknown"))
            );
            info!("  Hash: {}", block.get("hash").unwrap_or(&json!("unknown")));
            if let Some(txs) = block.get("transactions").and_then(|t| t.as_array()) {
                info!("  Transaction count: {}", txs.len());
            }
        }

        Ok(())
    }

    pub async fn get_balance(&self, address: &str) -> Result<()> {
        let address = Address::parse_checksummed(address, None)?;
        info!("\nTesting eth_getBalance with 'pending' tag...");
        let balance: Value = self
            .provider
            .client()
            .request(
                Cow::Borrowed("eth_getBalance"),
                (json!(format!("{address:?}")), json!("pending")),
            )
            .await?;

        info!("Balance for {address:?}:");
        info!("  Raw: {}", balance.get("result").unwrap_or(&json!("0x0")));

        Ok(())
    }

    pub async fn get_receipt(&self, tx_hash: &str) -> Result<()> {
        info!("Querying receipt for transaction {}", tx_hash);
        let receipt: Value = self
            .provider
            .client()
            .request(Cow::Borrowed("eth_getTransactionReceipt"), [json!(tx_hash)])
            .await?;

        if let Some(receipt) = receipt.get("result") {
            info!("Receipt found:");
            info!(
                "  Block Number: {}",
                receipt.get("blockNumber").unwrap_or(&json!("unknown"))
            );
            info!(
                "  Status: {}",
                receipt.get("status").unwrap_or(&json!("unknown"))
            );
            info!(
                "  Gas Used: {}",
                receipt.get("gasUsed").unwrap_or(&json!("unknown"))
            );
        } else {
            info!("No receipt found for transaction {}", tx_hash);
        }

        Ok(())
    }

    pub async fn monitor_blocks(&self, duration: Duration) -> Result<()> {
        info!(
            "\nMonitoring for new blocks for {} seconds...",
            duration.as_secs()
        );
        let start_block = self.provider.get_block_number().await?;
        let end_time = tokio::time::Instant::now() + duration;

        while tokio::time::Instant::now() < end_time {
            let current_block = self.provider.get_block_number().await?;
            if current_block > start_block {
                info!("New block detected: {}", current_block);

                let block: Value = self
                    .provider
                    .client()
                    .request(
                        Cow::Borrowed("eth_getBlockByNumber"),
                        (json!(format!("0x{:x}", current_block)), json!(true)),
                    )
                    .await?;

                if let Some(block) = block.get("result") {
                    info!(
                        "  Timestamp: {}",
                        block.get("timestamp").unwrap_or(&json!("unknown"))
                    );
                    info!(
                        "  Gas Used: {}",
                        block.get("gasUsed").unwrap_or(&json!("unknown"))
                    );
                    if let Some(txs) = block.get("transactions").and_then(|t| t.as_array()) {
                        info!("  Transactions: {}", txs.len());
                    }
                }
            }
            time::sleep(Duration::from_millis(250)).await;
        }

        Ok(())
    }
}
