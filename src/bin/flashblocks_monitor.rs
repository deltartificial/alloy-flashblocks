use alloy_flashblocks::types::Flashblock;
use chrono::{DateTime, Utc};
use eyre::Result;
use futures_util::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};
use url::Url;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BlockStats {
    block_number: u64,
    payload_id: String,
    sub_blocks: u64,
    total_transactions: usize,
    start_time: DateTime<Utc>,
    last_update: DateTime<Utc>,
}

type BlockStatsMap = Arc<Mutex<HashMap<String, BlockStats>>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let block_stats: BlockStatsMap = Arc::new(Mutex::new(HashMap::new()));
    let stats_clone = Arc::clone(&block_stats);

    tokio::spawn(async move {
        loop {
            time::sleep(Duration::from_secs(5)).await;
            display_stats(&stats_clone).await;
        }
    });

    stream_flashblocks(block_stats).await?;
    Ok(())
}

async fn display_stats(stats: &BlockStatsMap) {
    let stats_lock = stats.lock().await;
    if stats_lock.is_empty() {
        return;
    }

    info!("=== Flashblocks Statistics ===");

    for (payload_id, stats) in stats_lock.iter() {
        let duration = stats.last_update - stats.start_time;
        let duration_ms = duration.num_milliseconds();

        info!("Block #{}: payload_id={}", stats.block_number, payload_id);
        info!("  Sub-blocks: {}", stats.sub_blocks);
        info!("  Total transactions: {}", stats.total_transactions);
        info!("  Duration: {}ms", duration_ms);

        if stats.sub_blocks > 0 && duration_ms > 0 {
            let avg_interval = duration_ms as f64 / stats.sub_blocks as f64;
            info!("  Average sub-block interval: {:.2}ms", avg_interval);
        }

        if stats.total_transactions > 0 {
            let tps = if duration_ms > 0 {
                (stats.total_transactions as f64 * 1000.0) / duration_ms as f64
            } else {
                0.0
            };
            info!("  Transactions per second: {:.2}", tps);
        }
    }

    info!("=============================");
}

async fn stream_flashblocks(block_stats: BlockStatsMap) -> Result<()> {
    let ws_url = Url::parse("wss://sepolia.flashblocks.base.org/ws")?;
    info!("Connecting to Flashblocks WebSocket at {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
    info!("WebSocket connection established");

    let (_, mut read) = ws_stream.split();
    info!("Awaiting Flashblocks...");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let flashblock: Flashblock = serde_json::from_str(&text)?;
                process_flashblock(&flashblock, &block_stats).await?;
            }
            Ok(Message::Binary(_)) => warn!("Received binary message"),
            Ok(Message::Ping(_)) => debug!("Received ping"),
            Ok(Message::Pong(_)) => debug!("Received pong"),
            Ok(Message::Frame(_)) => debug!("Received frame"),
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
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

async fn process_flashblock(flashblock: &Flashblock, block_stats: &BlockStatsMap) -> Result<()> {
    let now = Utc::now();
    let payload_id = flashblock.payload_id.clone();
    let tx_count = flashblock
        .diff
        .transactions
        .as_ref()
        .map_or(0, |txs| txs.len());
    let block_number = flashblock.metadata.block_number.unwrap_or_else(|| {
        flashblock
            .base
            .as_ref()
            .and_then(|base| base.block_number.strip_prefix("0x"))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok())
            .unwrap_or(0)
    });

    let mut stats = block_stats.lock().await;

    if flashblock.index == 0 {
        info!(
            "New block #{} started: payload_id={}",
            block_number, payload_id
        );

        stats.insert(
            payload_id.clone(),
            BlockStats {
                block_number,
                payload_id: payload_id.clone(),
                sub_blocks: 1,
                total_transactions: tx_count,
                start_time: now,
                last_update: now,
            },
        );
    } else {
        if let Some(stat) = stats.get_mut(&payload_id) {
            stat.sub_blocks += 1;
            stat.total_transactions += tx_count;
            stat.last_update = now;

            debug!(
                "Sub-block #{} for block #{}: {} transactions",
                flashblock.index, block_number, tx_count
            );
        } else {
            warn!(
                "Received diff without initial block: payload_id={}",
                payload_id
            );
            stats.insert(
                payload_id.clone(),
                BlockStats {
                    block_number,
                    payload_id: payload_id.clone(),
                    sub_blocks: 1,
                    total_transactions: tx_count,
                    start_time: now,
                    last_update: now,
                },
            );
        }
    }

    if stats.len() > 10 {
        if let Some(key) = stats
            .iter()
            .min_by_key(|(_, s)| s.start_time)
            .map(|(k, _)| k.clone())
        {
            stats.remove(&key);
        }
    }

    Ok(())
}
