use crate::types::Flashblock;
use eyre::{Result, WrapErr};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::Message, Error as WsError},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};
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
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .wrap_err("Failed to connect to WebSocket")?;
        info!("WebSocket connection established");

        self.process_messages(ws_stream).await
    }

    async fn process_messages(
        &self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        let (_, mut read) = ws_stream.split();
        let mut block_count = 0;

        info!("Awaiting Flashblocks...");

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let flashblock: Flashblock = serde_json::from_str(&text)
                        .wrap_err("Failed to parse Flashblock")?;

                    self.handle_flashblock(&flashblock, &mut block_count).await?;

                    if block_count >= self.max_blocks && flashblock.is_initial() {
                        info!("\nReached maximum block count ({}), exiting", self.max_blocks);
                        break;
                    }
                }
                Ok(Message::Binary(_)) => warn!("Received unexpected binary message"),
                Ok(Message::Ping(_)) => debug!("Received ping"),
                Ok(Message::Pong(_)) => debug!("Received pong"),
                Ok(Message::Frame(_)) => debug!("Received raw frame"),
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    match e {
                        WsError::Protocol(_) | WsError::Utf8 => {
                            warn!("WebSocket protocol error: {}", e);
                            continue;
                        }
                        _ => return Err(e.into()),
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_flashblock(&self, flashblock: &Flashblock, block_count: &mut usize) -> Result<()> {
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
            info!("\nDiff update #{} for payload {}", flashblock.index, flashblock.payload_id);
            
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