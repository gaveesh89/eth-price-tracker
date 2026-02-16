//! RPC provider management for Ethereum connections.
//!
//! This module provides both HTTP and WebSocket providers with automatic
//! fallback and reconnection logic.
//!
//! # Provider Types
//!
//! - **HTTP Provider** ([`http`]): Traditional request/response for historical data
//! - **WebSocket Provider** ([`websocket`]): Real-time subscriptions with push notifications
//! - **Hybrid Provider** ([`hybrid`]): Intelligent manager that uses both
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │     HybridProviderManager           │
//! │  (Intelligent Provider Selection)   │
//! └─────────────────────────────────────┘
//!          │                    │
//!          ├────────────────────┤
//!          │                    │
//!    ┌─────▼─────┐        ┌────▼────┐
//!    │   HTTP    │        │   WS    │
//!    │ Provider  │        │Provider │
//!    └───────────┘        └─────────┘
//!         │                     │
//!    Historical Data      Real-time
//!    (fetch command)      (watch command)
//! ```
//!
//! # Usage Patterns
//!
//! ## Historical Data Fetching
//! ```rust,ignore
//! use eth_uniswap_alloy::rpc::{create_provider, get_latest_block};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/KEY").await?;
//! let block = get_latest_block(&provider).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Real-time Subscriptions
//! ```rust,ignore
//! use eth_uniswap_alloy::rpc::websocket::WebSocketProvider;
//! use futures_util::stream::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ws = WebSocketProvider::connect("wss://eth-mainnet.g.alchemy.com/v2/KEY".to_string()).await?;
//! let mut stream = ws.subscribe_blocks().await?;
//!
//! while let Some(block) = stream.next().await {
//!     println!("New block: {}", block.header.number);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Hybrid Mode (Recommended)
//! ```rust,ignore
//! use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = HybridProviderManager::new(
//!     "https://eth-mainnet.g.alchemy.com/v2/KEY".to_string(),
//!     Some("wss://eth-mainnet.g.alchemy.com/v2/KEY".to_string()),
//!     ProviderMode::Hybrid,
//! ).await?;
//!
//! // Use HTTP for historical data
//! let block = manager.http().get_block_number().await?;
//!
//! // Use WebSocket for real-time (with fallback)
//! if let Ok(ws) = manager.ws().await {
//!     // Subscribe to blocks...
//! }
//! # Ok(())
//! # }
//! ```

pub mod http;
pub mod hybrid;
pub mod websocket;

// Re-export commonly used types
pub use http::{check_connection, create_provider, get_latest_block, Provider};
pub use hybrid::{HybridProviderManager, ProviderMode};
pub use websocket::{ReconnectingWebSocket, WebSocketProvider};
