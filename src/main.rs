use alloy_flashblocks::FlashblocksRpcClient;
use clap::{Parser, Subcommand};
use std::error::Error;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query the latest Flashblock
    QueryLatest {
        /// RPC URL to connect to
        #[arg(long, default_value = "https://sepolia-preconf.base.org")]
        rpc_url: Url,
    },
    /// Stream blocks from the network
    Stream {
        /// Number of blocks to stream
        #[arg(long, default_value_t = 5)]
        blocks: u64,
        /// RPC URL to connect to
        #[arg(long, default_value = "https://sepolia-preconf.base.org")]
        rpc_url: Url,
    },
    /// Get balance for an address
    GetBalance {
        /// Address to query
        address: String,
        /// RPC URL to connect to
        #[arg(long, default_value = "https://sepolia-preconf.base.org")]
        rpc_url: Url,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::QueryLatest { rpc_url } => {
            let client = FlashblocksRpcClient::new(rpc_url.to_string());
            client.query_latest_flashblock().await?;
        }
        Commands::Stream { blocks, rpc_url } => {
            let client = FlashblocksRpcClient::new(rpc_url.to_string());
            client.monitor_blocks(blocks).await?;
        }
        Commands::GetBalance { address, rpc_url } => {
            let client = FlashblocksRpcClient::new(rpc_url.to_string());
            let balance = client.get_balance(&address).await?;
            println!("Balance: {}", balance);
        }
    }

    Ok(())
}
