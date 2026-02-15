//! # Ethereum Uniswap V2 Event Indexer
//!
//! Production-grade event indexer for Uniswap V2 using [Alloy](https://github.com/alloy-rs/alloy).
//!
//! This library provides a modular, testable architecture for tracking ETH/USDT prices
//! on Uniswap V2 by monitoring `Sync` events with proper decimal handling and
//! incremental block tracking.
//!
//! ## Features
//!
//! - **Type-safe event decoding** using Alloy's `sol!` macro
//! - **Proper decimal handling** for WETH (18) and USDT (6)
//! - **Incremental block tracking** - only fetches NEW events
//! - **Production error handling** with unified `TrackerError`
//! - **Full async/await** support with Tokio
//! - **Comprehensive testing** (80+ tests)
//!
//! ## Architecture
//!
//! The crate is organized into five independent layers:
//!
//! 1. **Config Layer** ([`config`]) - Environment variable loading
//! 2. **RPC Layer** ([`rpc`]) - Ethereum provider management
//! 3. **Events Layer** ([`events`]) - Event fetching and decoding
//! 4. **State Layer** ([`state`]) - Reserve tracking and validation
//! 5. **Pricing Layer** ([`pricing`]) - ETH/USDT price calculation
//!
//! See [`ARCHITECTURE.md`](https://github.com/gaveesh89/eth-price-tracker/blob/master/ARCHITECTURE.md)
//! for detailed design documentation.
//!
//! ## Quick Start
//!
//! ### Using the CLI
//!
//! ```bash
//! # One-time price fetch
//! cargo run --release -- price
//!
//! # Real-time monitoring
//! cargo run --release -- watch
//! ```
//!
//! ### Using as a Library
//!
//! ```rust,no_run
//! use eth_uniswap_alloy::{config::Config, rpc::create_provider, events::*, pricing::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = Config::from_env()?;
//!     
//!     // Create RPC provider
//!     let provider = create_provider(config.rpc_url()).await?;
//!     
//!     // Fetch events and calculate price
//!     let filter = create_sync_filter_for_pair(UNISWAP_V2_WETH_USDT_PAIR, 19000000, 19000100);
//!     let logs = provider.get_logs(&filter).await?;
//!     
//!     // Process events and calculate price
//!     for log in logs {
//!         // Decode and process...
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Environment Setup
//!
//! Create a `.env` file with your Alchemy API key:
//!
//! ```text
//! ALCHEMY_API_KEY=your_key_here
//! ```
//!
//! ## Error Handling
//!
//! All operations return [`error::TrackerResult<T>`](error::TrackerResult) for
//! consistent error propagation:
//!
//! ```rust
//! use eth_uniswap_alloy::error::{TrackerError, TrackerResult};
//!
//! fn example() -> TrackerResult<()> {
//!     // Operations that can fail return TrackerResult
//!     Ok(())
//! }
//! ```
//!
//! ## Testing
//!
//! Run the test suite:
//!
//! ```bash
//! # All tests
//! cargo test
//!
//! # Unit tests only
//! cargo test --lib
//!
//! # Integration tests
//! cargo test --test '*'
//! ```
//!
//! See [`TESTING.md`](https://github.com/gaveesh89/eth-price-tracker/blob/master/TESTING.md)
//! for comprehensive testing documentation.
//!
//! ## Documentation
//!
//! Generate and view the documentation:
//!
//! ```bash
//! cargo doc --no-deps --open
//! ```
//!
//! ## License
//!
//! Licensed under either of:
//!
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
//!
//! at your option.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

// Module declarations will go here as we build them
pub mod cli;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod api;
pub mod app_state;
pub mod observability;
pub mod pricing;
pub mod reorg;
pub mod rpc;
pub mod state;
