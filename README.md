# eth-uniswap-alloy

Production-grade Ethereum event indexer for Uniswap V2 using Alloy.

A high-performance, type-safe Rust implementation for tracking ETH/USDT prices on the Uniswap V2 WETH/USDT pool by monitoring `Sync` events with proper decimal handling and incremental block tracking.

## Features

- ✅ **Type-safe event decoding** using Alloy's `sol!` macro for compile-time verification
- ✅ **Proper decimal handling** for WETH (18 decimals) and USDT (6 decimals)
- ✅ **Incremental block tracking** - only fetches NEW events (not naive polling)
- ✅ **Graceful shutdown** with state persistence - press Ctrl+C to save and resume later
- ✅ **Colored CLI output** with timestamps, block numbers, and price change indicators
- ✅ **Production error handling** with unified `TrackerError` enum
- ✅ **Fully testable** with Anvil fork-based integration tests
- ✅ **Strict linting** - forbids unsafe code, unwrap, expect, and panic
- ✅ **Comprehensive documentation** with 23 doctests + 41 unit tests

## Architecture

5-layer design for modularity and maintainability:

1. **Config Layer** (`src/config.rs`) - Environment variable loading with validation
2. **RPC Layer** (`src/rpc.rs`) - Ethereum provider management with connection checks
3. **Events Layer** (`src/events.rs`) - Type-safe event decoding with `sol!` macro
4. **State Layer** (`src/state.rs`) - Reserve tracking with reorg detection
5. **Pricing Layer** (`src/pricing.rs`) - ETH/USDT price calculation with decimal adjustment

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed design documentation.

## Quick Start

### Prerequisites

- Rust 1.75+ (edition 2021)
- Alchemy API key for Ethereum mainnet access

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/gaveesh89/eth-price-tracker.git
   cd eth-price-tracker
   ```

2. Set up environment variables:
   ```bash
   cp .env.example .env
   # Edit .env and add your Alchemy API key
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

The CLI provides two main commands: `price` (one-time query) and `watch` (real-time monitoring).

### Price Command - One-time Price Fetch

Fetch the current ETH/USDT price from recent Sync events:

```bash
# Fetch from last 100 blocks (default)
cargo run --release -- price

# Fetch from last 500 blocks
cargo run --release -- price --blocks 500
```

**Output example:**
```
Fetching current ETH/USDT price from last 100 blocks...
[2024-01-15 14:23:45] Block 19000123 | $2,450.32 | 45.23 WETH | 110,789.45 USDT
```

### Watch Command - Real-time Price Monitoring

Monitor price changes in real-time with incremental block tracking:

```bash
# Watch with 12-second polling interval (default)
cargo run --release -- watch

# Watch with custom interval
cargo run --release -- watch --interval 15

# Watch starting from specific block
cargo run --release -- watch --start-block 19000000
```

**Graceful Shutdown:**
- Press `Ctrl+C` to stop monitoring gracefully
- State automatically saved to `state.json`
- Resume tracking from last block on restart
- No data loss on interruption

**Output example:**
```
Starting real-time price monitoring (interval: 12s)...
Last processed block: None (starting fresh)

[2024-01-15 14:23:45] Block 19000123 | $2,450.32 | 45.23 WETH | 110,789.45 USDT
[2024-01-15 14:24:00] Block 19000124 | $2,451.80 (+0.06%) | 45.18 WETH | 110,801.23 USDT
^C
🛑 Shutting down gracefully...
✅ State saved to ./state.json
📍 Last processed block: 19000124
👋 Shutdown complete
```

**Resume from saved state:**
```bash
cargo run --release -- watch
# INFO Resuming from saved state at block: 19000124
[2024-01-15 14:24:15] Block 19000125 | $2,449.15 (-0.11%) | 45.30 WETH | 110,785.67 USDT
```

**Features:**
- 🟢 **Green** - Price increased
- 🔴 **Red** - Price decreased
- ⚪ **White** - Price unchanged
- **Incremental tracking** - Only processes NEW blocks (no duplicate data)
- **State persistence** - Survives Ctrl+C and restarts

### CLI Options

```bash
# View all options
cargo run --release -- --help

# Price command options
cargo run --release -- price --help

# Watch command options
cargo run --release -- watch --help
```

## Configuration

Environment variables (see [.env.example](./.env.example)):

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ALCHEMY_API_KEY` | ✅ Yes | - | Your Alchemy API key for Ethereum mainnet |
| `POOL_ADDRESS` | ❌ No | `0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852` | Uniswap V2 WETH/USDT pair address |
| `ANVIL_FORK_BLOCK` | ❌ No | `19000000` | Block number for Anvil fork testing |
| `STATE_FILE` | ❌ No | `./state.json` | Path to state persistence file (future use) |
| `WATCH_MODE` | ❌ No | `false` | Enable watch mode (legacy, use CLI instead) |
| `POLL_INTERVAL_SECS` | ❌ No | `12` | Polling interval in seconds (legacy) |
| `BATCH_SIZE` | ❌ No | `1000` | Maximum blocks to fetch per RPC call |

## Development

### Build and Test

```bash
# Build with strict linting
make build

# Run all checks (format + clippy + test)
make check

# Run tests only (with single-threaded execution)
make test

# Run format check
make fmt

# Run clippy
make lint
```

Alternative (using `just`):
```bash
just check
```

### Testing Strategy

**Unit Tests (41 tests):**
- Config validation and RPC URL construction
- Error enum creation and trait implementations
- Event signature verification and filter creation
- State updates with reorg detection
- Price calculation with various reserve scenarios
- CLI argument parsing and formatting

**Doc Tests (23 tests):**
- All public API usage examples in documentation
- Compile-time verification of example code

**Integration Tests (4 tests):**
Anvil fork-based tests (require `ALCHEMY_API_KEY`):
```bash
# Run integration tests
export ALCHEMY_API_KEY="your_key_here"
cargo test --test anvil_setup -- --ignored
```

**Test Coverage:**
- ✅ Config loading and validation
- ✅ RPC provider creation and connection checks
- ✅ Event decoding with type safety
- ✅ State updates with reserve validation
- ✅ Price calculations with decimal adjustment
- ✅ Error handling across all layers
- ✅ CLI parsing and formatting

## Implementation Details

### Incremental Block Tracking

The `watch` command implements **incremental block tracking** to avoid redundant RPC calls:

1. Tracks `last_processed_block` in memory
2. On each interval, fetches `current_latest` block
3. **Skips if no new blocks:** `if current_latest <= last_processed_block { return }`
4. **Fetches only new events:** `filter.from_block(last_processed_block + 1).to_block(current_latest)`
5. Updates `last_processed_block = to_block` after processing

This ensures:
- ✅ No duplicate events processed
- ✅ Minimal RPC bandwidth usage
- ✅ Efficient incremental indexing
- ✅ NOT naive polling (only fetches NEW data)

### Decimal Handling

Proper decimal adjustment for price calculation:

- **WETH**: 18 decimals (ERC-20 standard)
- **USDT**: 6 decimals (Tether standard)
- **Price formula**: `(reserve1 * 10^12) / reserve0`
  - Adjusts USDT (6 decimals) to match WETH (18 decimals)
  - Result in standard 18-decimal format

### Error Handling

Unified `TrackerError` enum with 5 variants:
- `Config` - Configuration and environment variable errors
- `Rpc` - RPC provider and network errors
- `Decoding` - Event log decoding errors
- `State` - State validation and reserve errors
- `Math` - Arithmetic and overflow errors

All operations return `TrackerResult<T>` for consistent error propagation.

## Dependencies

Core dependencies:
- **Alloy v0.6** - Ethereum library (providers, sol-types, contract, node-bindings)
- **Tokio v1** - Async runtime with full features
- **Clap v4** - CLI argument parsing with derive macros
- **Colored v2** - Terminal color output
- **Chrono v0.4** - Timestamp formatting
- **Eyre v0.6** - Error context (wrapped by `TrackerError`)
- **Tracing v0.1** - Structured logging
- **Dotenvy v0.15** - Environment variable loading

See [Cargo.toml](./Cargo.toml) for complete dependency list.

## Project Structure

```
.
├── ARCHITECTURE.md           # Detailed architecture documentation
├── Cargo.toml                # Workspace dependencies and strict lints
├── .env.example              # Environment variable template
├── .gitignore                # Excludes .env, state files, build artifacts
├── justfile                  # Just build automation
├── Makefile                  # Make build automation
├── README.md                 # This file
├── src/
│   ├── lib.rs                # Module exports
│   ├── main.rs               # Binary entry point with async runtime
│   ├── cli.rs                # CLI interface (price/watch commands)
│   ├── config.rs             # Environment configuration
│   ├── error.rs              # Unified error handling
│   ├── rpc.rs                # RPC provider management
│   ├── events.rs             # Event definitions and filters
│   ├── state.rs              # State tracking
│   └── pricing.rs            # Price calculation
└── tests/
    └── anvil_setup.rs        # Anvil integration tests
```

## Documentation

### Generate and View Documentation

Generate the full API documentation:

```bash
# Generate documentation
cargo doc --no-deps

# Generate and open in browser
cargo doc --no-deps --open
```

The documentation is also available online at [docs.rs](https://docs.rs/eth-uniswap-alloy) (once published).

### Documentation Resources

- **[README.md](README.md)** - Project overview and quick start
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Detailed 5-layer architecture design
- **[USAGE.md](USAGE.md)** - Comprehensive usage guide with examples
- **[TESTING.md](TESTING.md)** - Complete testing documentation (80+ tests)
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Guidelines for contributors
- **[CLI_IMPLEMENTATION.md](CLI_IMPLEMENTATION.md)** - CLI module implementation details

### API Documentation

Every public API has comprehensive documentation including:

- **Description**: What the function/type does
- **Arguments**: Parameter descriptions
- **Returns**: Return value explanation
- **Errors**: When and why errors occur
- **Examples**: Working code examples
- **Panics**: Panic conditions (though we forbid panics)

Example from the codebase:

```rust
/// Calculate the ETH/USDT price from reserve amounts.
///
/// # Arguments
///
/// * `weth_reserve` - WETH reserve (18 decimals)
/// * `usdt_reserve` - USDT reserve (6 decimals)
///
/// # Returns
///
/// ETH price in USDT as f64
///
/// # Errors
///
/// Returns `TrackerError::Math` if reserves are zero
///
/// # Examples
///
/// ```
/// use eth_uniswap_alloy::pricing::calculate_eth_price;
/// use alloy::primitives::U256;
///
/// let weth = U256::from(50_000_000_000_000_000_000_u128);
/// let usdt = U256::from(125_000_000_000_u128);
/// let price = calculate_eth_price(weth, usdt).unwrap();
/// assert!(price > 2000.0 && price < 3000.0);
/// ```
pub fn calculate_eth_price(
    weth_reserve: U256,
    usdt_reserve: U256,
) -> TrackerResult<f64> {
    // Implementation...
}
```

## Linting and Code Quality

Strict linting configuration in [Cargo.toml](./Cargo.toml):

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "deny"
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

This ensures:
- ✅ No unsafe code anywhere
- ✅ No unwrap/expect/panic (all errors handled gracefully)
- ✅ Comprehensive documentation
- ✅ Pedantic and nursery clippy lints enabled

## Performance Considerations

1. **Incremental indexing** - Only fetches new blocks (not all history)
2. **Batch fetching** - Configurable `BATCH_SIZE` for RPC calls
3. **U256 arithmetic** - Native 256-bit integer operations (no decimal library overhead)
4. **Type-safe decoding** - Compile-time verification with `sol!` macro (no runtime parsing)
5. **Async/await** - Non-blocking I/O with Tokio runtime

## License

MIT OR Apache-2.0 (dual-licensed)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run `make check` to ensure all tests pass
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## Troubleshooting

### Common Issues

**Error: `ALCHEMY_API_KEY environment variable is required`**
- Solution: Create `.env` file with your Alchemy API key

**Error: `Failed to connect to RPC provider`**
- Check your API key is valid
- Verify network connectivity
- Ensure Alchemy endpoint is accessible

**Tests fail with parallel execution**
- Solution: Run tests with `--test-threads=1` (config tests modify env vars)

**Clippy warnings about `cast_precision_loss`**
- Expected behavior for reserve formatting (u128 → f64 conversion)
- Explicitly allowed with `#[allow(clippy::cast_precision_loss)]`

## Roadmap

Future enhancements:
- [ ] State persistence (save/load last processed block)
- [ ] Multiple pool support (track multiple pairs)
- [ ] Historical data export (CSV/JSON)
- [ ] WebSocket support (real-time updates without polling)
- [ ] Metrics/Prometheus integration
- [ ] Docker containerization
- [ ] GraphQL API for querying historical data

## Acknowledgments

- [Alloy](https://github.com/alloy-rs/alloy) - Modern Ethereum library for Rust
- [Uniswap V2](https://uniswap.org/) - Decentralized exchange protocol
- [Alchemy](https://www.alchemy.com/) - Ethereum API provider
- [Foundry](https://book.getfoundry.sh/) - Ethereum development toolkit (Anvil)

## Contact

For questions or support, please open an issue on GitHub.

