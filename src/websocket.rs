use crate::types::Flashblock;
use eyre::{Result, WrapErr};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::Message, Error as WsError},
};
use tracing::{debug, error, info};
use url::Url;

pub struct FlashblocksWsClient {
    url: Url,
    max_blocks: usize,
    reconnect_delay: Duration,
}

impl FlashblocksWsClient {
    pub fn new(url: Url, max_blocks: usize) -> Self {
        Self {
            url,
            max_blocks,
            reconnect_delay: Duration::from_secs(1),
        }
    }

    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    pub async fn stream_blocks(&self) -> Result<()> {
        info!("Connecting to Flashblocks WebSocket at {}", self.url);

        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            match self.connect_and_stream().await {
                Ok(_) => break,
                Err(e) => {
                    attempts += 1;
                    error!(
                        "WebSocket error (attempt {}/{}): {}",
                        attempts, max_attempts, e
                    );
                    if attempts < max_attempts {
                        time::sleep(self.reconnect_delay).await;
                    }
                }
            }
        }

        if attempts == max_attempts {
            error!("Failed to connect after {} attempts", max_attempts);
            return Err(eyre::eyre!("Max connection attempts reached"));
        }

        Ok(())
    }

    async fn connect_and_stream(&self) -> Result<()> {
        let (mut ws_stream, _) = connect_async(self.url.as_str())
            .await
            .wrap_err("Failed to establish WebSocket connection")?;
        info!("WebSocket connection established");

        let init_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "params": ["flashblocks"],
            "id": 1
        });
        let init_str = init_msg.to_string();
        ws_stream
            .send(Message::Text(init_str.as_str().into()))
            .await
            .wrap_err("Failed to send subscription request")?;
        info!("Sent subscription request");

        let mut block_count = 0;
        info!("Awaiting Flashblocks...");

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => {
                        if let Some(error) = json.get("error") {
                            error!("Received JSON-RPC error: {}", error);
                            continue;
                        }

                        match serde_json::from_value::<Flashblock>(json.clone()) {
                            Ok(flashblock) => {
                                self.handle_flashblock(&flashblock, &mut block_count)
                                    .await?;
                                if block_count >= self.max_blocks && flashblock.is_initial() {
                                    info!(
                                        "\nReached maximum block count ({}), exiting",
                                        self.max_blocks
                                    );
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Not a Flashblock message: {}", e);
                                debug!("Raw message: {}", text);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message as JSON: {}", e);
                        error!("Raw message: {}", text);
                    }
                },
                Ok(Message::Binary(data)) => match String::from_utf8(data.to_vec()) {
                    Ok(text) => {
                        debug!("Received binary message: {}", text);
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(json) => {
                                if let Ok(flashblock) =
                                    serde_json::from_value::<Flashblock>(json.clone())
                                {
                                    self.handle_flashblock(&flashblock, &mut block_count)
                                        .await?;
                                    if block_count >= self.max_blocks && flashblock.is_initial() {
                                        info!(
                                            "\nReached maximum block count ({}), exiting",
                                            self.max_blocks
                                        );
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse binary message as JSON: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode binary message as UTF-8: {}", e);
                    }
                },
                Ok(Message::Ping(data)) => {
                    ws_stream
                        .send(Message::Pong(data))
                        .await
                        .wrap_err("Failed to respond to ping")?;
                }
                Ok(Message::Pong(_)) => {}
                Ok(Message::Close(frame)) => {
                    info!("WebSocket connection closed by server: {:?}", frame);
                    break;
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => match e {
                    WsError::Protocol(p) => {
                        error!("WebSocket protocol error: {}", p);
                        break;
                    }
                    WsError::ConnectionClosed => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    _ => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                },
            }
        }

        Ok(())
    }

    async fn handle_flashblock(
        &self,
        flashblock: &Flashblock,
        block_count: &mut usize,
    ) -> Result<()> {
        if flashblock.is_initial() {
            *block_count += 1;
            info!("\nNew block started (#{}/{})", block_count, self.max_blocks);
            info!("Payload ID: {}", flashblock.payload_id);

            if let Some(base) = &flashblock.base {
                if let Some(number) = flashblock.block_number() {
                    info!("Block number: {}", number);
                }
                info!("Parent hash: {}", base.parent_hash);
                info!("Gas limit: {}", base.gas_limit);
                info!("Base fee: {} wei", base.base_fee_per_gas);
            }
        } else {
            info!(
                "\nDiff update #{} for payload {}",
                flashblock.index, flashblock.payload_id
            );

            let tx_count = flashblock.transaction_count();
            if tx_count > 0 {
                info!("New transactions: {}", tx_count);
            }

            if let Some(gas_used) = &flashblock.diff.gas_used {
                info!("Gas used: {}", gas_used);
            }

            if let Some(block_hash) = &flashblock.diff.block_hash {
                info!("Block hash: {}", block_hash);
            }
        }

        self.log_metadata(flashblock).await;
        Ok(())
    }

    async fn log_metadata(&self, flashblock: &Flashblock) {
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
    }
}
