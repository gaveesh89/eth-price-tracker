# Usage Guide

Complete guide for using the Ethereum Uniswap V2 Event Indexer.

## Table of Contents

- [Installation](#installation)
- [Configuration](#configuration)
- [CLI Usage](#cli-usage)
- [Library Usage](#library-usage)
- [Advanced Examples](#advanced-examples)
- [Troubleshooting](#troubleshooting)

## Installation

### Prerequisites

- Rust 1.75+ ([install via rustup](https://rustup.rs/))
- Alchemy API key ([get one free](https://www.alchemy.com/))
- Git

### Clone and Build

```bash
git clone https://github.com/gaveesh89/eth-price-tracker.git
cd eth-price-tracker
cargo build --release
```

The binary will be at `target/release/eth-uniswap-alloy`.

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```bash
# Required
ALCHEMY_API_KEY=your_alchemy_api_key_here

# Optional (with defaults)
POOL_ADDRESS=0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852
ANVIL_FORK_BLOCK=19000000
STATE_FILE=./state.json
WATCH_MODE=false
POLL_INTERVAL_SECS=12
BATCH_SIZE=1000
```

### Configuration Details

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `ALCHEMY_API_KEY` | String | *Required* | Your Alchemy API key for Ethereum mainnet |
| `POOL_ADDRESS` | Address | `0x0d4a...1852` | Uniswap V2 WETH/USDT pair address |
| `ANVIL_FORK_BLOCK` | u64 | `19000000` | Block number for Anvil fork testing |
| `STATE_FILE` | Path | `./state.json` | Path to state persistence file |
| `WATCH_MODE` | bool | `false` | Enable continuous monitoring (legacy) |
| `POLL_INTERVAL_SECS` | u64 | `12` | Polling interval in seconds |
| `BATCH_SIZE` | u64 | `1000` | Maximum blocks per RPC call |

## CLI Usage

### Price Command

Fetch the current ETH/USDT price from recent Sync events.

**Basic Usage:**
```bash
cargo run --release -- price
```

**With Custom Block Range:**
```bash
# Scan last 500 blocks
cargo run --release -- price --blocks 500

# Or using short form
cargo run --release -- price -b 500
```

**Output:**
```
[2024-01-15 14:23:45] Block 19000123 | $2,450.32 | 45.23 WETH | 110,789.45 USDT
```

### Watch Command

Monitor price changes in real-time with incremental block tracking.

**Basic Usage:**
```bash
cargo run --release -- watch
```

**With Custom Options:**
```bash
# Custom polling interval (15 seconds)
cargo run --release -- watch --interval 15

# Start from specific block
cargo run --release -- watch --start-block 19000000

# Combined options
cargo run --release -- watch -i 15 -s 19000000
```

**Output:**
```
🔍 Watching for ETH/USDT price updates...

[2024-01-15 14:23:45] Block 19000123 | $2,450.32 | 45.23 WETH | 110,789.45 USDT
[2024-01-15 14:24:00] Block 19000124 | $2,451.80 (+0.06%) | 45.18 WETH | 110,801.23 USDT
[2024-01-15 14:24:15] Block 19000125 | $2,449.15 (-0.11%) | 45.30 WETH | 110,785.67 USDT
```

**Color Legend:**
- 🟢 **Green**: Price increased
- 🔴 **Red**: Price decreased
- ⚪ **White**: Price unchanged

### Help Commands

```bash
# Main help
cargo run --release -- --help

# Price command help
cargo run --release -- price --help

# Watch command help
cargo run --release -- watch --help

# Version info
cargo run --release -- --version
```

## Library Usage

### Basic Example

```rust
use eth_uniswap_alloy::{
    config::Config,
    rpc::{create_provider, get_latest_block},
    events::{create_sync_filter_for_pair, UNISWAP_V2_WETH_USDT_PAIR},
    state::State,
    pricing::calculate_eth_price,
    error::TrackerResult,
};

#[tokio::main]
async fn main() -> TrackerResult<()> {
    // 1. Load configuration
    let config = Config::from_env()?;
    
    // 2. Create RPC provider
    let provider = create_provider(config.rpc_url()).await?;
    
    // 3. Get latest block
    let latest_block = get_latest_block(&provider).await?;
    let from_block = latest_block.saturating_sub(100);
    
    // 4. Fetch Sync events
    let filter = create_sync_filter_for_pair(
        UNISWAP_V2_WETH_USDT_PAIR,
        from_block,
        latest_block
    );
    let logs = provider.get_logs(&filter).await
        .map_err(|e| eth_uniswap_alloy::error::TrackerError::rpc(
            format!("Failed to fetch logs: {e}"),
            None
        ))?;
    
    // 5. Process events
    let mut state = State::new();
    for log in logs {
        // Decode event (simplified - see events module for full decoding)
        println!("Processing log at block {}", log.block_number.unwrap_or_default());
    }
    
    // 6. Calculate price
    let (weth, usdt) = state.get_reserves();
    if state.is_initialized() {
        let price = calculate_eth_price(weth, usdt)?;
        println!("Current ETH price: ${:.2}", price);
    }
    
    Ok(())
}
```

### Advanced Example: Custom Pool Tracking

```rust
use eth_uniswap_alloy::{
    config::Config,
    rpc::create_provider,
    events::create_sync_filter_for_pair,
    error::TrackerResult,
};
use alloy::primitives::Address;

#[tokio::main]
async fn main() -> TrackerResult<()> {
    let config = Config::from_env()?;
    let provider = create_provider(config.rpc_url()).await?;
    
    // Track a different pool (e.g., ETH/USDC)
    let eth_usdc_pair: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
        .parse()
        .expect("Valid address");
    
    let filter = create_sync_filter_for_pair(
        eth_usdc_pair,
        19000000,
        19000100
    );
    
    let logs = provider.get_logs(&filter).await
        .map_err(|e| eth_uniswap_alloy::error::TrackerError::rpc(
            format!("Failed to fetch logs: {e}"),
            None
        ))?;
    
    println!("Found {} Sync events", logs.len());
    
    Ok(())
}
```

### Error Handling Example

```rust
use eth_uniswap_alloy::{
    config::Config,
    error::{TrackerError, TrackerResult},
};

fn load_config_with_fallback() -> TrackerResult<Config> {
    match Config::from_env() {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            // Could implement fallback logic here
            Err(e)
        }
    }
}

fn main() {
    match load_config_with_fallback() {
        Ok(config) => {
            println!("Configuration loaded successfully");
            println!("RPC URL: {}", config.rpc_url());
        }
        Err(TrackerError::Config { message, .. }) => {
            eprintln!("Config error: {}", message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
            std::process::exit(2);
        }
    }
}
```

## Advanced Examples

### Incremental Block Processing

```rust
use eth_uniswap_alloy::{
    rpc::{create_provider, get_latest_block},
    events::{create_sync_filter_for_pair, UNISWAP_V2_WETH_USDT_PAIR},
    error::TrackerResult,
};

async fn process_incrementally() -> TrackerResult<()> {
    let provider = create_provider("your_rpc_url").await?;
    let mut last_processed = 19000000_u64;
    
    loop {
        let current_latest = get_latest_block(&provider).await?;
        
        // Only process if there are new blocks
        if current_latest > last_processed {
            let filter = create_sync_filter_for_pair(
                UNISWAP_V2_WETH_USDT_PAIR,
                last_processed + 1,
                current_latest
            );
            
            let logs = provider.get_logs(&filter).await
                .map_err(|e| eth_uniswap_alloy::error::TrackerError::rpc(
                    format!("RPC error: {e}"),
                    None
                ))?;
            
            println!("Processing {} new events", logs.len());
            last_processed = current_latest;
        }
        
        // Wait before next check
        tokio::time::sleep(tokio::time::Duration::from_secs(12)).await;
    }
}
```

### Batch Processing

```rust
use eth_uniswap_alloy::{
    rpc::create_provider,
    events::{create_sync_filter_for_pair, UNISWAP_V2_WETH_USDT_PAIR},
    error::TrackerResult,
};

async fn batch_process(from_block: u64, to_block: u64) -> TrackerResult<()> {
    let provider = create_provider("your_rpc_url").await?;
    const BATCH_SIZE: u64 = 1000;
    
    let mut current = from_block;
    while current <= to_block {
        let batch_end = (current + BATCH_SIZE).min(to_block);
        
        let filter = create_sync_filter_for_pair(
            UNISWAP_V2_WETH_USDT_PAIR,
            current,
            batch_end
        );
        
        let logs = provider.get_logs(&filter).await
            .map_err(|e| eth_uniswap_alloy::error::TrackerError::rpc(
                format!("RPC error: {e}"),
                None
            ))?;
        
        println!("Blocks {}-{}: {} events", current, batch_end, logs.len());
        
        current = batch_end + 1;
    }
    
    Ok(())
}
```

## Troubleshooting

### Common Issues

#### 1. "ALCHEMY_API_KEY environment variable is required"

**Solution:** Create a `.env` file with your API key:
```bash
echo "ALCHEMY_API_KEY=your_key_here" > .env
```

#### 2. "Failed to connect to RPC provider"

**Possible causes:**
- Invalid API key
- Network connectivity issues
- Rate limiting

**Solutions:**
```bash
# Test your API key
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY

# Check network connectivity
ping eth-mainnet.g.alchemy.com
```

#### 3. "No Sync events found"

**Possible causes:**
- No swaps occurred in the specified block range
- Pool address is incorrect
- Block range is too narrow

**Solutions:**
```bash
# Increase block range
cargo run --release -- price --blocks 1000

# Verify pool address in .env
echo $POOL_ADDRESS
```

#### 4. Tests fail with parallel execution

**Solution:** Run tests single-threaded:
```bash
cargo test -- --test-threads=1
```

### Debug Mode

Enable debug logging:

```bash
# Set log level
export RUST_LOG=debug

# Run with verbose output
cargo run --release -- price
```

Available log levels:
- `error` - Only errors
- `warn` - Errors and warnings
- `info` - Informational messages (default)
- `debug` - Detailed debugging
- `trace` - Very verbose

### Performance Tips

1. **Use release builds** for better performance:
   ```bash
   cargo build --release
   ./target/release/eth-uniswap-alloy watch
   ```

2. **Adjust batch size** for large ranges:
   ```bash
   export BATCH_SIZE=500  # Smaller batches for slower connections
   ```

3. **Use appropriate polling intervals**:
   ```bash
   # Ethereum block time is ~12 seconds
   cargo run --release -- watch --interval 12
   ```

## Additional Resources

- [Architecture Documentation](ARCHITECTURE.md)
- [Testing Guide](TESTING.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [API Documentation](https://docs.rs/eth-uniswap-alloy)
- [GitHub Repository](https://github.com/gaveesh89/eth-price-tracker)

## Getting Help

- Open an [issue](https://github.com/gaveesh89/eth-price-tracker/issues) on GitHub
- Check [existing issues](https://github.com/gaveesh89/eth-price-tracker/issues?q=is%3Aissue)
- Review [documentation](https://docs.rs/eth-uniswap-alloy)
