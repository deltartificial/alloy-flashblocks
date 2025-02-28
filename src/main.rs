use alloy_flashblocks::{cli::{Cli, Commands}, FlashblocksRpcClient, FlashblocksWsClient};
use clap::Parser;
use eyre::Result;
use std::time::Duration;
use tracing::info;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Stream { blocks } => {
            let ws_url = Url::parse("wss://sepolia.flashblocks.base.org/ws")?;
            let client = FlashblocksWsClient::new(ws_url, blocks);
            client.stream_blocks().await?;
        }
        Commands::QueryLatest => {
            let rpc_url = Url::parse("https://sepolia-preconf.base.org")?;
            let client = FlashblocksRpcClient::new(rpc_url)?;
            client.query_latest_flashblock().await?;
        }
        Commands::GetBalance { address } => {
            let rpc_url = Url::parse("https://sepolia-preconf.base.org")?;
            let client = FlashblocksRpcClient::new(rpc_url)?;
            client.get_balance(&address).await?;
        }
        Commands::GetReceipt { tx_hash } => {
            let rpc_url = Url::parse("https://sepolia-preconf.base.org")?;
            let client = FlashblocksRpcClient::new(rpc_url)?;
            client.get_receipt(&tx_hash).await?;
        }
    }

    Ok(())
}
