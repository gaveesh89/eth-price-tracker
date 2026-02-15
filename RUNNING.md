# Running the Ethereum Price Tracker

## Quick Start

### Option 1: Using cargo with environment variable (Recommended for Testing)

```bash
# Set the RPC_URL and run the price command
RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY" cargo run -- price --blocks 5
```

### Option 2: Load .env and use cargo run

```bash
# Make sure .env is in the project root with RPC_URL set
source .env
cargo run -- price --blocks 5
```

### Option 3: Using the binary after building

```bash
# Build the project
cargo build --release

# Run with environment variable
RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY" ./target/release/eth-uniswap-alloy price --blocks 5
```

## Available Commands

### 1. Fetch Current Price (One-time)

```bash
# Default: scan last 100 blocks
RPC_URL="your_rpc_url" cargo run -- price

# Scan last 5 blocks (works with Alchemy free tier)
RPC_URL="your_rpc_url" cargo run -- price --blocks 5
```

**Output Example:**
```
INFO eth_uniswap_alloy::cli: Fetching current ETH/USDT price
INFO eth_uniswap_alloy::rpc: Latest block number: 24456586
INFO eth_uniswap_alloy::cli: Latest block: 24456586
INFO eth_uniswap_alloy::cli: Scanning blocks 24456581 to 24456586
✓ Current ETH Price: 2544.32 USDT
```

### 2. Watch Price Updates (Real-time Monitoring)

```bash
# Monitor with 12 second polling interval (default)
RPC_URL="your_rpc_url" cargo run -- watch

# Custom polling interval (30 seconds)
RPC_URL="your_rpc_url" cargo run -- watch --interval 30

# Start from specific block
RPC_URL="your_rpc_url" cargo run -- watch --start-block 24456000
```

**Graceful Shutdown:**
- Press `Ctrl+C` to stop the watcher gracefully
- State is automatically saved to `state.json` before exit
- Resume tracking from last processed block on restart
- No data loss on interruption

**Output Example with Shutdown:**
```
📊 Block: 24461565 | Price: $2060.00 USDT | Change: ▲ +0.50%
📊 Block: 24461566 | Price: $2061.00 USDT | Change: ▲ +0.05%
^C
🛑 Shutting down gracefully...
✅ State saved to ./state.json
📍 Last processed block: 24461566
👋 Shutdown complete
```

**Restart resumes from saved state:**
```
RPC_URL="your_rpc_url" cargo run -- watch
INFO Resuming from saved state at block: 24461566
📊 Block: 24461567 | Price: $2062.00 USDT | Change: ▲ +0.05%
```

## Alchemy API Tier Limits

**Free Tier:**
- ✅ `eth_getLogs` with up to **10 block range** per request
- ✅ Full access to Ethereum mainnet data
- ✅ **Watch command now batches queries** to stay within limits

**Solution:**
- ✅ Use `--blocks 5` for price command (single query within 10-block limit)
- ✅ Use `--interval 12` for watch command (now has automatic batching!)
  - Queries are automatically batched into 10-block chunks
  - Processes all blocks incrementally without hitting rate limits
  - No more "10 block range" errors!
- Upgrade to **PAYG** for unlimited block ranges

## Environment Configuration

You have two options for providing the RPC URL:

### Option A: RPC_URL (Recommended - New)
```bash
export RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
cargo run -- price
```

### Option B: ALCHEMY_API_KEY (Backward Compatible - Old)
```bash
export ALCHEMY_API_KEY="YOUR_KEY_ONLY"
cargo run -- price
```

The application will construct the RPC URL: `https://eth-mainnet.g.alchemy.com/v2/{ALCHEMY_API_KEY}`

## Troubleshooting

### "RPC_URL or ALCHEMY_API_KEY environment variable is required"
Make sure to set the environment variable before running:
```bash
export RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
cargo run -- price
```

### "eth_getLogs requests with up to a 10 block range"
Reduce the block range for the price command:
```bash
RPC_URL="..." cargo run -- price --blocks 5
```

### "No Sync events found in the last X blocks"
This is normal - try a larger block range or use watch mode to wait for new trades:
```bash
RPC_URL="..." cargo run -- watch
```

## View Logs

To see detailed debug information:

```bash
RUST_LOG=debug RPC_URL="..." cargo run -- price --blocks 5
```

## Configuration Files

- **`.env.example`** - Template showing all available options
- **`.env`** - Your actual secrets (ignored by git, add your API key here)
- **`README_SETUP.md`** - Detailed setup instructions
