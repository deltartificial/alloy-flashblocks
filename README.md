# Alloy Flashblocks

A Rust implementation for interacting with Base's Flashblocks service. Provides WebSocket streaming, monitoring, and RPC functionality for real-time block data, using Alloy.

## Features

- WebSocket streaming of Flashblocks data
- Real-time block monitoring with statistics
- RPC client for blockchain queries
- Automatic reconnection handling
- Comprehensive transaction and block tracking

## Installation

```bash
git clone https://github.com/deltartificial/alloy-flashblocks
cd alloy-flashblocks
cargo build --release
```

## Usage

### WebSocket Client
Stream raw Flashblocks data:
```bash
cargo run --bin flashblocks_ws
```

### Block Monitor
Monitor blocks with detailed statistics:
```bash
cargo run --bin flashblocks_monitor
```

### RPC Client
Query blockchain data:
```bash
cargo run --bin flashblocks_rpc
```

## Configuration

Default endpoint: `wss://sepolia.flashblocks.base.org/ws`

To use a custom endpoint, modify the URL in the respective binary files.

## Example Output

```
=== Flashblocks Statistics ===
Block #123456: payload_id=0x...
  Sub-blocks: 3
  Total transactions: 150
  Duration: 500ms
  Average sub-block interval: 166.67ms
  Transactions per second: 300.00
===========================
```

