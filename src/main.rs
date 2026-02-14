//! CLI entry point for the Uniswap V2 event indexer.
//!
//! # Architecture Flow
//!
//! This binary delegates to the CLI module, which orchestrates all layers:
//!
//! ```text
//! main.rs (Runtime Initialization)
//!     ↓
//! CLI Layer (src/cli.rs)
//!     ↓
//! 1. Config Layer (src/config.rs)    → Load environment variables
//! 2. RPC Layer (src/rpc.rs)          → Create Ethereum provider
//! 3. Events Layer (src/events.rs)    → Fetch & decode Sync events
//! 4. State Layer (src/state.rs)      → Update reserves & validate
//! 5. Pricing Layer (src/pricing.rs)  → Calculate ETH/USDT price
//! 6. CLI Layer (output)              → Display formatted results
//! ```
//!
//! # Layer Separation
//!
//! - **main.rs**: Async runtime + tracing initialization only
//! - **CLI module**: User interface + layer orchestration
//! - **Core modules**: Independent, reusable, no upward dependencies
//!
//! All errors bubble up with context via `TrackerResult<T>`.

use eth_uniswap_alloy::cli;
use tracing_subscriber::EnvFilter;

/// Entry point for the Uniswap V2 event indexer.
///
/// Initializes:
/// - Tokio async runtime (via `#[tokio::main]`)
/// - Structured logging with tracing
/// - Environment filter (defaults to "info" level)
///
/// Then delegates to the CLI module for all business logic.
#[tokio::main]
async fn main() {
    // Initialize structured logging with environment-based filtering
    // Can be controlled via RUST_LOG environment variable
    // Example: RUST_LOG=debug cargo run
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Run CLI - all layer orchestration happens inside cli::run()
    // The CLI module coordinates:
    //   1. Config loading (environment variables)
    //   2. RPC provider creation (Ethereum connection)
    //   3. Event fetching (Sync events from Uniswap V2)
    //   4. State updates (reserve tracking)
    //   5. Price calculation (ETH/USDT)
    //   6. Output formatting (colored terminal output)
    if let Err(e) = cli::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
