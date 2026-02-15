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

use eth_uniswap_alloy::{cli, observability};
use tracing::error;

/// Entry point for the Uniswap V2 event indexer.
///
/// Initializes:
/// - Tokio async runtime (via `#[tokio::main]`)
/// - Production-grade structured logging with tracing
/// - Environment-based filtering (RUST_LOG, LOG_JSON, LOG_FILE)
///
/// Then delegates to the CLI module for all business logic.
#[tokio::main]
async fn main() {
    // Initialize structured logging FIRST (before any other operations)
    // Configuration can be controlled via environment variables:
    // - RUST_LOG: Set log level (e.g., "debug", "info", "trace")
    // - LOG_JSON: Enable JSON output for production ("true" or "false")
    // - LOG_FILE: Write logs to file with daily rotation
    //
    // Examples:
    //   RUST_LOG=debug cargo run -- watch
    //   RUST_LOG=eth_uniswap_alloy=trace,sqlx=warn cargo run
    //   LOG_JSON=true LOG_FILE=./logs/indexer.log cargo run
    let log_level = std::env::var("RUST_LOG").ok();
    let log_file = std::env::var("LOG_FILE").ok().map(std::path::PathBuf::from);
    let json_output = std::env::var("LOG_JSON")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if let Err(e) = observability::init_tracing(log_level, log_file, json_output) {
        eprintln!("Failed to initialize tracing: {e}");
        std::process::exit(1);
    }

    // Run CLI - all layer orchestration happens inside cli::run()
    // The CLI module coordinates:
    //   1. Config loading (environment variables)
    //   2. RPC provider creation (Ethereum connection)
    //   3. Event fetching (Sync events from Uniswap V2)
    //   4. State updates (reserve tracking)
    //   5. Price calculation (ETH/USDT)
    //   6. Output formatting (colored terminal output)
    if let Err(e) = cli::run().await {
        error!(error = %e, "Application error");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
