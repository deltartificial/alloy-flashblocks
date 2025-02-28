use alloy_flashblocks::types::Flashblock;
use eyre::Result;
use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};
use url::Url;

fn hex_to_decimal(hex: &str) -> String {
    if let Some(hex_value) = hex.strip_prefix("0x") {
        if let Ok(value) = u64::from_str_radix(hex_value, 16) {
            return value.to_string();
        }
    }
    hex.to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let ws_url = Url::parse("wss://sepolia.flashblocks.base.org/ws")?;
    info!("Connecting to Flashblocks WebSocket at {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
    info!("WebSocket connection established");

    let (_, mut read) = ws_stream.split();

    info!("Awaiting Flashblocks...");
    let mut block_count = 0;
    let max_blocks = 5;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let flashblock: Flashblock = serde_json::from_str(&text)?;

                if flashblock.index == 0 {
                    block_count += 1;
                    info!("\nNew block started (#{}/{})", block_count, max_blocks);
                    info!(
                        "Payload ID: {} ({})",
                        flashblock.payload_id,
                        hex_to_decimal(&flashblock.payload_id)
                    );

                    if let Some(base) = &flashblock.base {
                        if let Some(hex) = base.block_number.strip_prefix("0x") {
                            if let Ok(number) = u64::from_str_radix(hex, 16) {
                                info!("Block number: {}", number);
                            }
                        }
                        info!("Parent hash: {}", base.parent_hash);
                        info!("Gas limit: {}", base.gas_limit);
                        info!("Base fee: {} wei", base.base_fee_per_gas);
                    }
                } else {
                    info!(
                        "\nDiff update #{} for payload {} ({})",
                        flashblock.index,
                        flashblock.payload_id,
                        hex_to_decimal(&flashblock.payload_id)
                    );

                    if let Some(txs) = &flashblock.diff.transactions {
                        info!("New transactions: {}", txs.len());
                    }

                    if let Some(gas_used) = &flashblock.diff.gas_used {
                        info!("Gas used: {}", hex_to_decimal(gas_used));
                    }

                    if let Some(block_hash) = &flashblock.diff.block_hash {
                        info!("Block hash: {}", block_hash);
                    }
                }

                if let Some(balances) = &flashblock.metadata.new_account_balances {
                    let balance_count = balances.as_object().map_or(0, |obj| obj.len());
                    if balance_count > 0 {
                        info!("Updated balances for {} accounts", balance_count);
                    }
                }

                if let Some(receipts) = &flashblock.metadata.receipts {
                    let receipt_count = receipts.as_object().map_or(0, |obj| obj.len());
                    if receipt_count > 0 {
                        info!("New receipts: {}", receipt_count);
                    }
                }

                if block_count >= max_blocks && flashblock.index == 0 {
                    info!("\nReached maximum block count ({}), exiting", max_blocks);
                    break;
                }
            }
            Ok(Message::Binary(binary)) => match serde_json::from_slice::<Flashblock>(&binary) {
                Ok(flashblock) => {
                    if flashblock.index == 0 {
                        block_count += 1;
                        info!("\nNew block started (#{}/{})", block_count, max_blocks);
                        info!(
                            "Payload ID: {} ({})",
                            flashblock.payload_id,
                            hex_to_decimal(&flashblock.payload_id)
                        );

                        if let Some(base) = &flashblock.base {
                            if let Some(hex) = base.block_number.strip_prefix("0x") {
                                if let Ok(number) = u64::from_str_radix(hex, 16) {
                                    info!("Block number: {}", number);
                                }
                            }
                            info!("Parent hash: {}", base.parent_hash);
                            info!("Gas limit: {}", base.gas_limit);
                            info!("Base fee: {} wei", base.base_fee_per_gas);
                        }
                    } else {
                        info!(
                            "\nDiff update #{} for payload {} ({})",
                            flashblock.index,
                            flashblock.payload_id,
                            hex_to_decimal(&flashblock.payload_id)
                        );

                        if let Some(txs) = &flashblock.diff.transactions {
                            info!("New transactions: {}", txs.len());
                        }

                        if let Some(gas_used) = &flashblock.diff.gas_used {
                            info!("Gas used: {}", hex_to_decimal(gas_used));
                        }

                        if let Some(block_hash) = &flashblock.diff.block_hash {
                            info!("Block hash: {}", block_hash);
                        }
                    }

                    if let Some(balances) = &flashblock.metadata.new_account_balances {
                        let balance_count = balances.as_object().map_or(0, |obj| obj.len());
                        if balance_count > 0 {
                            info!("Updated balances for {} accounts", balance_count);
                        }
                    }

                    if let Some(receipts) = &flashblock.metadata.receipts {
                        let receipt_count = receipts.as_object().map_or(0, |obj| obj.len());
                        if receipt_count > 0 {
                            info!("New receipts: {}", receipt_count);
                        }
                    }

                    if block_count >= max_blocks && flashblock.index == 0 {
                        info!("\nReached maximum block count ({}), exiting", max_blocks);
                        break;
                    }
                }
                Err(e) => error!("Failed to parse binary message: {}", e),
            },
            Ok(Message::Ping(_)) => debug!("Received ping"),
            Ok(Message::Pong(_)) => debug!("Received pong"),
            Ok(Message::Frame(_)) => debug!("Received frame"),
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by server");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
