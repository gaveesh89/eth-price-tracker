//! # Ethereum Uniswap V2 Event Indexer
//!
//! Production-grade event indexer for Uniswap V2 using Alloy.
//!
//! This library provides a modular, testable architecture for:
//! - Fetching and decoding Uniswap V2 Sync events
//! - Calculating ETH/USDT prices from reserve data
//! - Tracking indexing progress with persistent state
//! - Supporting watch mode for continuous monitoring
//!
//! ## Architecture
//!
//! The crate is organized into five layers:
//! - `rpc`: RPC provider management
//! - `events`: Event fetching and decoding
//! - `state`: Block cursor and persistence
//! - `pricing`: Price calculation from reserves
//! - `ui`: Display and formatting
//!
//! See `ARCHITECTURE.md` for detailed design documentation.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

// Module declarations will go here as we build them
pub mod cli;
pub mod config;
pub mod error;
pub mod events;
pub mod pricing;
pub mod rpc;
pub mod state;
