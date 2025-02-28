use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use eyre::Result;
use std::str::FromStr;
use std::time::Duration;
use tokio::time;
use tracing::info;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let rpc_url = Url::parse("https://sepolia-preconf.base.org")?;
    let provider = ProviderBuilder::new().on_http(rpc_url);

    let latest_block = provider.get_block_number().await?;
    info!("Connected to Base Sepolia. Latest block: {}", latest_block);

    test_flashblocks_methods(&provider).await?;

    Ok(())
}

async fn test_flashblocks_methods(provider: &impl Provider) -> Result<()> {
    info!("Testing eth_getBlockByNumber with 'pending' tag...");
    let method = "eth_getBlockByNumber";
    let pending_block: serde_json::Value = provider
        .raw_request(method.into(), serde_json::json!(["pending", true]))
        .await?;

    if let Some(block) = pending_block.get("result") {
        info!("Latest Flashblock:");
        info!(
            "  Number: {}",
            block.get("number").unwrap_or(&serde_json::json!("unknown"))
        );
        info!(
            "  Hash: {}",
            block.get("hash").unwrap_or(&serde_json::json!("unknown"))
        );
        if let Some(txs) = block.get("transactions").and_then(|t| t.as_array()) {
            info!("  Transaction count: {}", txs.len());
        }
    }

    let test_address = Address::from_str("0x4200000000000000000000000000000000000011")?;
    info!("\nTesting eth_getBalance with 'pending' tag...");
    let method = "eth_getBalance";
    let balance: serde_json::Value = provider
        .raw_request(
            method.into(),
            serde_json::json!([format!("{test_address:?}"), "pending"]),
        )
        .await?;

    info!("Balance for {test_address:?}:");
    info!(
        "  Raw: {}",
        balance.get("result").unwrap_or(&serde_json::json!("0x0"))
    );

    info!("\nMonitoring for new blocks for 30 seconds...");
    let start_block = provider.get_block_number().await?;
    let end_time = tokio::time::Instant::now() + Duration::from_secs(30);

    while tokio::time::Instant::now() < end_time {
        let current_block = provider.get_block_number().await?;
        if current_block > start_block {
            info!("New block detected: {}", current_block);

            let block: serde_json::Value = provider
                .raw_request(
                    "eth_getBlockByNumber".into(),
                    serde_json::json!([format!("0x{:x}", current_block), true]),
                )
                .await?;

            if let Some(block) = block.get("result") {
                info!(
                    "  Timestamp: {}",
                    block
                        .get("timestamp")
                        .unwrap_or(&serde_json::json!("unknown"))
                );
                info!(
                    "  Gas Used: {}",
                    block
                        .get("gasUsed")
                        .unwrap_or(&serde_json::json!("unknown"))
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
