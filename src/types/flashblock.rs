use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct FlashblockBase {
    pub parent_hash: String,
    pub fee_recipient: String,
    pub block_number: String,
    pub gas_limit: String,
    pub timestamp: String,
    pub base_fee_per_gas: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashblockDiff {
    pub state_root: Option<String>,
    pub block_hash: Option<String>,
    pub gas_used: Option<String>,
    pub transactions: Option<Vec<String>>,
    pub withdrawals: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashblockMetadata {
    pub block_number: Option<u64>,
    pub new_account_balances: Option<Value>,
    pub receipts: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Flashblock {
    pub payload_id: String,
    pub index: u64,
    #[serde(default)]
    pub base: Option<FlashblockBase>,
    pub diff: FlashblockDiff,
    pub metadata: FlashblockMetadata,
}

impl Flashblock {
    pub fn block_number(&self) -> Option<u64> {
        self.metadata.block_number.or_else(|| {
            self.base.as_ref().and_then(|base| {
                base.block_number
                    .strip_prefix("0x")
                    .and_then(|hex| u64::from_str_radix(hex, 16).ok())
            })
        })
    }

    pub fn transaction_count(&self) -> usize {
        self.diff.transactions.as_ref().map_or(0, |txs| txs.len())
    }

    pub fn is_initial(&self) -> bool {
        self.index == 0
    }
}
