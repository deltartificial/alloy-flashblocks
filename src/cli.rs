use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Stream {
        #[arg(short, long, default_value_t = 5)]
        blocks: usize,
    },
    QueryLatest,
    GetBalance {
        address: String,
    },
    GetReceipt {
        tx_hash: String,
    },
}
