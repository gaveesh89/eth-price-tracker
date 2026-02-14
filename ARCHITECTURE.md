# Architecture: Production-Grade Uniswap V2 Event Indexer

## 1. Module Structure (5-Layer Separation)

```
eth_uniswap_alloy/
├── src/
│   ├── main.rs                 # Entry point, CLI parsing, runtime setup
│   ├── lib.rs                  # Library exports
│   │
│   ├── rpc/                    # Layer 1: RPC Connection Management
│   │   ├── mod.rs              # Provider abstraction
│   │   ├── provider.rs         # Alloy provider wrapper
│   │   ├── config.rs           # RPC config (URL, retry, timeout)
│   │   └── health.rs           # Connection health checks
│   │
│   ├── events/                 # Layer 2: Event Fetching & Decoding
│   │   ├── mod.rs              # Event system abstraction
│   │   ├── schema.rs           # sol! macro definitions (Sync event)
│   │   ├── fetcher.rs          # Log fetching with filters
│   │   ├── decoder.rs          # Event decoding utilities
│   │   └── pool_registry.rs    # Pool address registry (extensible)
│   │
│   ├── state/                  # Layer 3: State & Block Tracking
│   │   ├── mod.rs              # State manager
│   │   ├── cursor.rs           # Block cursor (last indexed block)
│   │   ├── persistence.rs      # JSON/DB persistence layer
│   │   └── cache.rs            # In-memory event cache
│   │
│   ├── pricing/                # Layer 4: Price Calculation
│   │   ├── mod.rs              # Pricing engine
│   │   ├── calculator.rs       # Price math (reserves -> price)
│   │   ├── decimals.rs         # Decimal handling (U256 -> float safely)
│   │   └── types.rs            # Price types (PricePoint, Reserve)
│   │
│   ├── ui/                     # Layer 5: User Interface
│   │   ├── mod.rs              # Display layer
│   │   ├── formatter.rs        # Pretty printing
│   │   ├── progress.rs         # Progress indicators
│   │   └── logger.rs           # Structured logging (tracing)
│   │
│   ├── error.rs                # Unified error types
│   ├── config.rs               # Global config
│   └── types.rs                # Shared types
│
├── tests/
│   ├── integration/            # Anvil-based integration tests
│   │   ├── mod.rs
│   │   ├── anvil_setup.rs      # Anvil fork helpers
│   │   ├── full_flow.rs        # End-to-end test
│   │   └── watch_mode.rs       # Watch mode test
│   └── unit/                   # Unit tests (per module)
│       └── pricing_tests.rs
│
├── examples/
│   ├── fetch_once.rs           # Fetch historical events
│   └── watch_live.rs           # Watch mode example
│
├── Cargo.toml
└── .env.example                # RPC_URL, pool addresses
```

---

## 2. Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                          Application Entry                            │
│  main.rs: CLI args → Config → Runtime → run() → Event Loop          │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Layer 1: RPC Provider                          │
│  - Initialize Alloy Provider (HTTP/WS)                               │
│  - Health checks & retry logic                                       │
│  - Returns: Arc<Provider>                                            │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Layer 3: State Manager                         │
│  - Load cursor (last indexed block)                                  │
│  - Determine: from_block = cursor.last + 1, to_block = latest       │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Layer 2: Event Fetcher                         │
│  - Build filter: address=POOL, topic=Sync, range=[from, to]         │
│  - provider.get_logs(filter) → Vec<Log>                             │
│  - Decode logs using sol! macro → Vec<SyncEvent>                    │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      Layer 4: Pricing Calculator                      │
│  - For each SyncEvent:                                               │
│    * Extract reserve0 (WETH), reserve1 (USDT)                       │
│    * Calculate: price = reserve1 / reserve0                          │
│    * Adjust for decimals: USDT (6) / WETH (18)                      │
│    * Return: PricePoint { block, timestamp, price, reserves }       │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Layer 3: State Update                          │
│  - Cache price points in memory                                      │
│  - Update cursor (last_block = to_block)                            │
│  - Persist cursor to disk (JSON/SQLite)                             │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Layer 5: UI Display                            │
│  - Format price points (table/JSON/CSV)                             │
│  - Show progress (blocks indexed, events found)                     │
│  - Log to console/file (tracing)                                    │
└──────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Watch Mode Loop?     │
                    │  If yes: sleep, goto  │
                    │  Layer 3 (State)      │
                    │  If no: exit          │
                    └───────────────────────┘
```

**Data Structures in Flow:**
```
RpcProvider → Filter → Vec<Log> → Vec<SyncEvent> → Vec<PricePoint> 
→ StateUpdate → Display
```

---

## 3. Error Handling Strategy

### Unified Error Enum

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    // Layer 1: RPC Errors
    #[error("RPC connection failed: {0}")]
    RpcConnection(String),
    
    #[error("RPC request failed: {0}")]
    RpcRequest(#[from] alloy::transports::TransportError),
    
    #[error("Provider initialization failed: {0}")]
    ProviderInit(String),
    
    // Layer 2: Event Errors
    #[error("Failed to fetch logs: {0}")]
    LogFetch(String),
    
    #[error("Event decoding failed: {0}")]
    EventDecode(String),
    
    #[error("Invalid pool address: {0}")]
    InvalidPool(String),
    
    // Layer 3: State Errors
    #[error("Failed to load state: {0}")]
    StateLoad(#[from] std::io::Error),
    
    #[error("Failed to persist state: {0}")]
    StatePersist(String),
    
    #[error("Invalid block cursor: {0}")]
    InvalidCursor(String),
    
    // Layer 4: Pricing Errors
    #[error("Price calculation failed: {0}")]
    PriceCalc(String),
    
    #[error("Decimal overflow: {0}")]
    DecimalOverflow(String),
    
    #[error("Division by zero in price calculation")]
    DivisionByZero,
    
    #[error("Invalid reserves: reserve0={0}, reserve1={1}")]
    InvalidReserves(String, String),
    
    // Layer 5: UI Errors
    #[error("Display formatting failed: {0}")]
    DisplayError(String),
    
    // Config Errors
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
```

### Error Handling Patterns

**1. RPC Layer:** Retry with exponential backoff
```rust
retry_policy: RetryPolicy {
    max_retries: 3,
    initial_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
}
```

**2. Event Decoding:** Skip invalid events, log warning
```rust
match decode_log(log) {
    Ok(event) => events.push(event),
    Err(e) => {
        warn!("Skipping invalid log: {e}");
        continue;
    }
}
```

**3. State Persistence:** Atomic writes with backup
```rust
// Write to temp file → fsync → rename
```

**4. Pricing Math:** Checked arithmetic, explicit error types
```rust
reserve0.checked_div(reserve1).ok_or(IndexerError::DivisionByZero)?
```

---

## 4. Testing Strategy

### Testing Pyramid

```
                    ┌──────────────────┐
                    │  E2E Integration │  (Anvil fork)
                    │   1-2 tests      │
                    └────────┬─────────┘
                             │
                ┌────────────▼────────────┐
                │  Integration Tests      │  (Anvil fork)
                │  5-10 tests             │
                └────────────┬────────────┘
                             │
            ┌────────────────▼────────────────┐
            │     Unit Tests                  │  (Mock/Pure functions)
            │     30+ tests                   │
            └─────────────────────────────────┘
```

### Anvil-Based Testing

**Setup (tests/integration/anvil_setup.rs):**
```rust
async fn setup_anvil_fork() -> AnvilInstance {
    Anvil::new()
        .fork("https://eth-mainnet.g.alchemy.com/v2/...")
        .fork_block_number(19_000_000)  // Known good block
        .spawn()
}

// Helper: Deploy mock Uniswap pool if needed
async fn deploy_test_pool(anvil: &AnvilInstance) -> Address { ... }

// Helper: Simulate Sync events
async fn simulate_sync_event(
    anvil: &AnvilInstance,
    pool: Address,
    reserve0: U256,
    reserve1: U256,
) { ... }
```

**Integration Test Cases:**

1. **Full Flow Test** (`tests/integration/full_flow.rs`)
   - Start Anvil fork
   - Fetch real Uniswap WETH/USDT events from fork
   - Decode all events
   - Calculate prices
   - Verify prices match expected range (e.g., $1500-$5000)
   - Check state persistence

2. **Watch Mode Test** (`tests/integration/watch_mode.rs`)
   - Start Anvil fork
   - Simulate new block with Sync event
   - Watch mode detects new event
   - Price calculated correctly
   - Cursor advances

3. **Error Recovery Test**
   - Simulate RPC disconnect
   - Verify retry logic works
   - Verify state not corrupted

### Unit Tests

**Per Layer:**

**RPC Layer:**
- Provider initialization
- Health check logic
- Retry policy

**Events Layer:**
- sol! macro event structure
- Log decoding (valid/invalid)
- Filter building

**State Layer:**
- Cursor save/load
- JSON serialization
- Invalid state handling

**Pricing Layer (Critical):**
- Price calculation: `(reserve1 * 10^18) / (reserve0 * 10^6)`
- Edge cases:
  - Zero reserves (should error)
  - Very large reserves (overflow check)
  - Very small reserves (precision check)
- Property-based tests (e.g., price always positive)

**UI Layer:**
- Formatting functions
- No crashes on edge cases

### Test Execution

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests (requires RPC_URL)
cargo test --test '*' -- --test-threads=1

# With coverage
cargo tarpaulin --out Html
```

### Quality Gates (CI)

```yaml
# .github/workflows/ci.yml
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test --all-features
- cargo build --release
```

---

## 5. Type Definitions and Structs

### Core Types

```rust
// src/types.rs

use alloy::primitives::{Address, U256, B256};
use serde::{Deserialize, Serialize};

/// Global configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub pools: Vec<PoolConfig>,
    pub state_file: PathBuf,
    pub watch_mode: bool,
    pub poll_interval_secs: u64,
    pub batch_size: u64,  // Max blocks per query
}

/// Pool configuration (extensible for multiple pools)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub name: String,
    pub address: Address,
    pub token0: TokenInfo,
    pub token1: TokenInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub decimals: u8,
}

/// Block cursor for incremental indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub last_indexed_block: u64,
    pub last_updated_at: i64,  // Unix timestamp
}

/// Decoded Sync event
#[derive(Debug, Clone)]
pub struct SyncEvent {
    pub block_number: u64,
    pub block_timestamp: u64,
    pub transaction_hash: B256,
    pub log_index: u64,
    pub pool_address: Address,
    pub reserve0: U256,  // WETH
    pub reserve1: U256,  // USDT
}

/// Calculated price point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_hash: String,
    pub pool: String,
    pub price: f64,  // USDT per WETH
    pub reserve0_raw: String,  // U256 as string
    pub reserve1_raw: String,
    pub reserve0_human: f64,
    pub reserve1_human: f64,
}
```

### sol! Macro Definitions

```rust
// src/events/schema.rs

use alloy::sol;

sol! {
    /// Uniswap V2 Sync event
    #[derive(Debug, Clone, PartialEq, Eq)]
    event Sync(
        uint112 reserve0,
        uint112 reserve1
    );
}

// Event signature hash (for filtering)
pub const SYNC_EVENT_SIGNATURE: &str = 
    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
```

### State Types

```rust
// src/state/types.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexerState {
    pub version: u32,  // Schema version
    pub cursors: HashMap<String, Cursor>,  // Pool name → cursor
    pub metadata: StateMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMetadata {
    pub total_events_indexed: u64,
    pub first_block: Option<u64>,
    pub last_block: Option<u64>,
}
```

---

## 6. Async Boundaries

### Tokio Runtime Setup

```rust
// src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    
    // Load config
    let config = Config::from_env()?;
    
    // Run indexer
    run(config).await
}
```

### Async Boundaries Map

```
┌─────────────────────┐
│  main() - async     │  Tokio runtime starts here
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  run(config) - async                    │
│  - Spawns RPC provider (async)          │
│  - Spawns event loop task               │
└──────────┬──────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  RPC Layer - async                      │
│  - provider.get_logs() → async          │
│  - provider.get_block_number() → async  │
└──────────┬──────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  Event Fetcher - async                  │
│  - fetch_events() → async               │
│  - decode_events() → sync (pure fn)     │
└──────────┬──────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  Pricing - sync                         │
│  - calculate_price() → pure function    │
│  - No async needed (math only)          │
└──────────┬──────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  State Manager - async/sync mix         │
│  - save_cursor() → async (tokio::fs)    │
│  - in-memory operations → sync          │
└──────────┬──────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│  UI Layer - sync                        │
│  - Formatting is synchronous            │
│  - println! in async context (safe)     │
└─────────────────────────────────────────┘
```

### Async Best Practices

**1. Minimize async scope:**
```rust
// Good: Only RPC calls are async
async fn fetch_and_process() -> Result<Vec<PricePoint>> {
    let logs = provider.get_logs(filter).await?;  // Async
    let events = decode_logs(logs);               // Sync
    let prices = calculate_prices(events);        // Sync
    Ok(prices)
}
```

**2. Use tokio::spawn for parallel work:**
```rust
// If supporting multiple pools
let handles: Vec<_> = pools.iter()
    .map(|pool| tokio::spawn(index_pool(pool.clone())))
    .collect();

for handle in handles {
    handle.await??;
}
```

**3. Use channels for streaming:**
```rust
// Watch mode: event stream
let (tx, mut rx) = mpsc::channel::<PricePoint>(100);

tokio::spawn(async move {
    // Producer: fetch events
    while let Some(price) = fetch_next_price().await {
        tx.send(price).await.unwrap();
    }
});

while let Some(price) = rx.recv().await {
    // Consumer: display
    println!("{}", price);
}
```

---

## 7. Quality Gates

### Pre-commit Checks

```bash
#!/bin/bash
# .githooks/pre-commit

set -e

echo "Running pre-commit checks..."

# 1. Format check
cargo fmt --check || {
    echo "❌ Code not formatted. Run: cargo fmt"
    exit 1
}

# 2. Clippy (no warnings allowed)
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy found issues"
    exit 1
}

# 3. Tests
cargo test --all-features || {
    echo "❌ Tests failed"
    exit 1
}

echo "✅ All checks passed"
```

### Cargo.toml Metadata

```toml
[package]
name = "eth-uniswap-alloy"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "deny"
pedantic = "warn"
nursery = "warn"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

### CI Pipeline

```yaml
# .github/workflows/ci.yml

name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: ~/.cargo
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Format check
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      
      - name: Build
        run: cargo build --release
      
      - name: Test (unit)
        run: cargo test --lib
      
      - name: Test (integration)
        env:
          RPC_URL: ${{ secrets.RPC_URL }}
        run: cargo test --test '*' -- --test-threads=1
      
      - name: Doc check
        run: cargo doc --no-deps --document-private-items
```

### Local Development Workflow

```bash
# Development iteration
cargo watch -x 'fmt' -x 'clippy' -x 'test'

# Pre-release checklist
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all-features
cargo build --release
cargo doc --open

# Benchmark (if needed)
cargo bench
```

### Documentation Requirements

```rust
// All public items must have docs
#![warn(missing_docs)]

/// Fetches Sync events from the specified Uniswap V2 pool.
///
/// # Arguments
/// * `provider` - Ethereum RPC provider
/// * `pool` - Pool address to query
/// * `from_block` - Starting block (inclusive)
/// * `to_block` - Ending block (inclusive)
///
/// # Returns
/// Vector of decoded `SyncEvent` structs
///
/// # Errors
/// Returns `IndexerError::LogFetch` if RPC call fails
/// Returns `IndexerError::EventDecode` if log decoding fails
pub async fn fetch_sync_events(
    provider: &Provider,
    pool: Address,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<SyncEvent>> {
    // ...
}
```

---

## 8. Dependencies (Cargo.toml)

```toml
[dependencies]
# Alloy (Ethereum library)
alloy = { version = "0.1", features = ["full"] }
alloy-sol-macro = "0.1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
dotenv = "0.15"

[dev-dependencies]
# Testing
alloy-node-bindings = "0.1"  # For Anvil spawning
tokio-test = "0.4"
proptest = "1"  # Property-based testing for pricing

# Benchmarking
criterion = { version = "0.5", features = ["async_tokio"] }
```

---

## 9. Configuration Management

### Environment Variables

```bash
# .env.example

# Required
RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY

# Optional
RUST_LOG=info
STATE_FILE=./state.json
WATCH_MODE=false
POLL_INTERVAL_SECS=12
BATCH_SIZE=1000

# Uniswap V2 WETH/USDT Pool
POOL_ADDRESS=0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852
```

### Config Loading

```rust
// src/config.rs

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        
        let rpc_url = env::var("RPC_URL")
            .map_err(|_| IndexerError::MissingEnv("RPC_URL".into()))?;
        
        let pool_address = env::var("POOL_ADDRESS")
            .map_err(|_| IndexerError::MissingEnv("POOL_ADDRESS".into()))?
            .parse()
            .map_err(|e| IndexerError::InvalidPool(format!("{e}")))?;
        
        // Default pool: WETH/USDT
        let pools = vec![PoolConfig {
            name: "WETH/USDT".into(),
            address: pool_address,
            token0: TokenInfo { symbol: "WETH".into(), decimals: 18 },
            token1: TokenInfo { symbol: "USDT".into(), decimals: 6 },
        }];
        
        Ok(Self {
            rpc_url,
            pools,
            state_file: env::var("STATE_FILE")
                .unwrap_or_else(|_| "./state.json".into())
                .into(),
            watch_mode: env::var("WATCH_MODE")
                .unwrap_or_else(|_| "false".into())
                .parse()
                .unwrap_or(false),
            poll_interval_secs: env::var("POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "12".into())
                .parse()
                .unwrap_or(12),
            batch_size: env::var("BATCH_SIZE")
                .unwrap_or_else(|_| "1000".into())
                .parse()
                .unwrap_or(1000),
        })
    }
}
```

---

## 10. Execution Flow (main.rs pseudocode)

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging
    init_tracing();
    
    // 2. Load config from env
    let config = Config::from_env()?;
    info!("Config loaded: {:?}", config);
    
    // 3. Initialize RPC provider
    let provider = RpcProvider::new(&config.rpc_url).await?;
    info!("Connected to RPC: {}", config.rpc_url);
    
    // 4. Load state (cursor)
    let mut state = StateManager::load(&config.state_file)?;
    info!("State loaded: last block = {}", state.cursor.last_indexed_block);
    
    // 5. Main indexing loop
    loop {
        // 5a. Determine block range
        let from_block = state.cursor.last_indexed_block + 1;
        let latest = provider.get_block_number().await?;
        let to_block = min(from_block + config.batch_size - 1, latest);
        
        if from_block > to_block {
            info!("No new blocks. Waiting...");
            if !config.watch_mode {
                break;  // Exit if not in watch mode
            }
            sleep(Duration::from_secs(config.poll_interval_secs)).await;
            continue;
        }
        
        info!("Indexing blocks {from_block}-{to_block}...");
        
        // 5b. Fetch events
        let events = fetch_sync_events(
            &provider,
            config.pools[0].address,
            from_block,
            to_block,
        ).await?;
        
        info!("Found {} Sync events", events.len());
        
        // 5c. Calculate prices
        let prices = calculate_prices(&events, &config.pools[0])?;
        
        // 5d. Display results
        display_prices(&prices);
        
        // 5e. Update state
        state.cursor.last_indexed_block = to_block;
        state.save(&config.state_file)?;
        
        info!("State saved. Cursor: {}", to_block);
        
        // If not watch mode, exit after one batch
        if !config.watch_mode {
            break;
        }
        
        // Wait before next iteration
        sleep(Duration::from_secs(config.poll_interval_secs)).await;
    }
    
    info!("Indexer finished");
    Ok(())
}
```

---

## 11. Extension Points for Future Features

### Multi-Pool Support
```rust
// Loop over all configured pools
for pool in &config.pools {
    let events = fetch_sync_events(&provider, pool.address, from, to).await?;
    let prices = calculate_prices(&events, pool)?;
    // Store prices with pool identifier
}
```

### Database Backend (SQLite/Postgres)
```rust
// Replace JSON persistence with DB
trait StatePersistence {
    async fn save_cursor(&self, cursor: &Cursor) -> Result<()>;
    async fn load_cursor(&self) -> Result<Cursor>;
    async fn save_price(&self, price: &PricePoint) -> Result<()>;
}

struct JsonPersistence { /* current impl */ }
struct SqlitePersistence { /* future impl */ }
```

### WebSocket Streaming
```rust
// Replace HTTP provider with WS for real-time events
let provider = ProviderBuilder::new()
    .on_ws(config.ws_url)
    .await?;

// Subscribe to new blocks
let mut block_stream = provider.subscribe_blocks().await?;
while let Some(block) = block_stream.next().await {
    // Process block immediately
}
```

### Metrics & Observability
```rust
// Add Prometheus metrics
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref EVENTS_PROCESSED: Counter = 
        Counter::new("events_processed", "Total events indexed").unwrap();
    
    static ref PRICE_CALCULATION_TIME: Histogram =
        Histogram::new("price_calc_duration_seconds", "Price calc time").unwrap();
}
```

---

## Summary Checklist

- ✅ **5-layer architecture**: rpc, events, state, pricing, ui
- ✅ **Data flow**: Clear separation of concerns
- ✅ **Error handling**: Unified `IndexerError` enum with layer-specific variants
- ✅ **Testing**: Anvil integration tests + comprehensive unit tests
- ✅ **Type safety**: sol! macro for events, strong typing everywhere
- ✅ **Async boundaries**: Minimal async scope, clear tokio usage
- ✅ **Quality gates**: Clippy, fmt, no warnings, CI pipeline
- ✅ **Extensibility**: Multi-pool ready, DB backend ready, WS ready
- ✅ **Production-ready**: Logging, config management, state persistence

---

**Next Steps:**
1. Review this architecture
2. Confirm requirements alignment
3. Begin implementation layer-by-layer:
   - Start with types/errors (foundation)
   - Then RPC layer (connectivity)
   - Then events layer (data fetching)
   - Then pricing (business logic)
   - Then state (persistence)
   - Finally UI (display)
