//! Hybrid provider manager for intelligent RPC provider selection.
//!
//! Manages both HTTP and WebSocket providers, automatically selecting
//! the best option based on the use case and availability.
//!
//! # Provider Selection Strategy
//!
//! - **Historical queries**: Always use HTTP (reliable, simple)
//! - **Real-time subscriptions**: Prefer WebSocket, fallback to HTTP polling
//! - **Automatic recovery**: Reconnect WebSocket on disconnect
//!
//! # Example
//!
//! ```rust,ignore
//! use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut manager = HybridProviderManager::new(
//!         "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
//!         Some("wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
//!         ProviderMode::Hybrid,
//!     ).await?;
//!     
//!     // Use HTTP for historical data
//!     use alloy::providers::Provider;
//!     let block_number = manager.http().get_block_number().await?;
//!     println!("Latest block: {}", block_number);
//!     
//!     // Use WebSocket for real-time (with fallback)
//!     if manager.is_ws_available() {
//!         let ws = manager.ws().await?;
//!         // Subscribe to blocks or events...
//!     } else {
//!         println!("WebSocket unavailable, using HTTP polling");
//!     }
//!     
//!     Ok(())
//! }
//! ```

use eyre::Result;
use tracing::{info, warn};

use super::http;
use super::websocket::{ReconnectingWebSocket, WebSocketProvider};

/// Provider mode selection strategy.
///
/// Determines which provider type to use for different operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    /// Use only HTTP provider (traditional polling)
    Http,
    /// Use only WebSocket provider (fail if unavailable)
    WebSocket,
    /// Use both: prefer WebSocket, fallback to HTTP
    Hybrid,
}

/// Hybrid provider manager that intelligently selects between HTTP and WebSocket.
///
/// Maintains both provider types and automatically handles:
/// - Provider initialization
/// - WebSocket reconnection
/// - Graceful fallback to HTTP
/// - Connection health monitoring
///
/// # Architecture
///
/// ```text
/// HybridProviderManager
/// ├─ HTTP Provider (always available)
/// │  └─ For: Historical queries, fallback
/// └─ WebSocket Provider (optional)
///    └─ For: Real-time subscriptions
/// ```
pub struct HybridProviderManager {
    http_provider: http::Provider,
    ws_provider: Option<ReconnectingWebSocket>,
    mode: ProviderMode,
}

impl HybridProviderManager {
    /// Creates a new hybrid provider manager.
    ///
    /// # Arguments
    ///
    /// * `http_url` - HTTP RPC endpoint (required)
    /// * `ws_url` - WebSocket RPC endpoint (optional)
    /// * `mode` - Provider selection mode
    ///
    /// # Initialization Behavior
    ///
    /// - HTTP provider: Always initialized immediately
    /// - WebSocket provider: Initialized based on mode
    ///   - `Http`: WebSocket not created
    ///   - `WebSocket`: Must connect successfully or return error
    ///   - `Hybrid`: Connection attempted, failure logged but not fatal
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Prefer WebSocket, fallback to HTTP if unavailable
    /// let manager = HybridProviderManager::new(
    ///     "https://eth-mainnet.g.alchemy.com/v2/KEY".to_string(),
    ///     Some("wss://eth-mainnet.g.alchemy.com/v2/KEY".to_string()),
    ///     ProviderMode::Hybrid,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - HTTP provider initialization fails (always fatal)
    /// - WebSocket mode enabled but connection fails
    pub async fn new(http_url: String, ws_url: Option<String>, mode: ProviderMode) -> Result<Self> {
        // Always create HTTP provider (required for historical data)
        info!("Initializing HTTP provider: {}", http_url);
        let http_provider = http::create_provider(&http_url)
            .await
            .map_err(|e| eyre::eyre!("HTTP provider initialization failed: {}", e))?;

        // Optionally create WebSocket provider based on mode
        let ws_provider = if let Some(url) = ws_url {
            match mode {
                ProviderMode::Http => {
                    info!("HTTP mode selected, skipping WebSocket initialization");
                    None
                }
                ProviderMode::WebSocket => {
                    info!("WebSocket mode selected, connection required");
                    let mut reconnecting = ReconnectingWebSocket::new(url);
                    reconnecting.connect().await?;
                    Some(reconnecting)
                }
                ProviderMode::Hybrid => {
                    info!("Hybrid mode selected, attempting WebSocket connection");
                    let mut reconnecting = ReconnectingWebSocket::new(url);
                    match reconnecting.connect().await {
                        Ok(_) => {
                            info!("WebSocket connected successfully in hybrid mode");
                            Some(reconnecting)
                        }
                        Err(e) => {
                            warn!(
                                "WebSocket connection failed in hybrid mode: {}. Will use HTTP only.",
                                e
                            );
                            None
                        }
                    }
                }
            }
        } else {
            if mode == ProviderMode::WebSocket {
                return Err(eyre::eyre!(
                    "WebSocket mode selected but no WebSocket URL provided"
                ));
            }
            info!("No WebSocket URL provided, HTTP only");
            None
        };

        Ok(Self {
            http_provider,
            ws_provider,
            mode,
        })
    }

    /// Returns a reference to the HTTP provider.
    ///
    /// Always available for historical queries and as fallback.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
    /// # use alloy::providers::Provider;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let manager = HybridProviderManager::new(
    /// #     "https://...".to_string(), None, ProviderMode::Http
    /// # ).await?;
    /// let block_number = manager.http().get_block_number().await?;
    /// println!("Latest block: {}", block_number);
    /// # Ok(())
    /// # }
    /// ```
    pub fn http(&self) -> &http::Provider {
        &self.http_provider
    }

    /// Returns a reference to the WebSocket provider, connecting if necessary.
    ///
    /// # Reconnection Behavior
    ///
    /// If the WebSocket is disconnected, this method will attempt to reconnect
    /// before returning. This ensures you always get a connected provider or an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
    /// # use futures_util::stream::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut manager = HybridProviderManager::new(
    /// #     "https://...".to_string(),
    /// #     Some("wss://...".to_string()),
    /// #     ProviderMode::Hybrid
    /// # ).await?;
    /// let ws = manager.ws().await?;
    /// let mut stream = ws.subscribe_blocks().await?;
    ///
    /// while let Some(block) = stream.next().await {
    ///     println!("Block: {}", block.header.number);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - WebSocket not configured (no URL provided)
    /// - Connection/reconnection fails
    pub async fn ws(&mut self) -> Result<&WebSocketProvider> {
        let ws = self
            .ws_provider
            .as_mut()
            .ok_or_else(|| eyre::eyre!("WebSocket not configured"))?;

        // Connect if not already connected
        if !ws.is_connected() {
            info!("WebSocket not connected, attempting connection");
            ws.connect().await?;
        }

        ws.provider()
            .ok_or_else(|| eyre::eyre!("WebSocket provider not available"))
    }

    /// Checks if WebSocket is currently available.
    ///
    /// Returns `true` if WebSocket provider is configured AND connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let manager = HybridProviderManager::new(
    /// #     "https://...".to_string(), None, ProviderMode::Http
    /// # ).await?;
    /// if manager.is_ws_available() {
    ///     println!("WebSocket ready for subscriptions");
    /// } else {
    ///     println!("Using HTTP polling mode");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_ws_available(&self) -> bool {
        self.ws_provider
            .as_ref()
            .map(|ws| ws.is_connected())
            .unwrap_or(false)
    }

    /// Attempts to reconnect the WebSocket if it's configured.
    ///
    /// This is useful when the WebSocket disconnects and you want to
    /// explicitly trigger a reconnection attempt.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::hybrid::{HybridProviderManager, ProviderMode};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut manager = HybridProviderManager::new(
    /// #     "https://...".to_string(),
    /// #     Some("wss://...".to_string()),
    /// #     ProviderMode::Hybrid
    /// # ).await?;
    /// // Stream ended, try to reconnect
    /// if let Err(e) = manager.reconnect_ws().await {
    ///     eprintln!("Reconnection failed: {}, falling back to HTTP", e);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - WebSocket not configured
    /// - Reconnection fails after all retry attempts
    pub async fn reconnect_ws(&mut self) -> Result<()> {
        let ws = self
            .ws_provider
            .as_mut()
            .ok_or_else(|| eyre::eyre!("WebSocket not configured"))?;

        ws.reconnect().await
    }

    /// Returns the current provider mode.
    pub fn mode(&self) -> ProviderMode {
        self.mode
    }

    /// Returns the HTTP provider URL.
    pub fn http_url(&self) -> String {
        // Note: Alloy doesn't expose the URL from the provider directly
        // We'd need to store it separately if needed
        "HTTP provider (URL not stored)".to_string()
    }

    /// Returns the WebSocket URL if configured.
    pub fn ws_url(&self) -> Option<&str> {
        self.ws_provider.as_ref().map(|ws| ws.url())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_only_mode() {
        let http_url =
            std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());

        let manager = HybridProviderManager::new(http_url, None, ProviderMode::Http)
            .await
            .expect("HTTP provider should initialize");

        assert!(!manager.is_ws_available());
        assert_eq!(manager.mode(), ProviderMode::Http);
    }

    #[tokio::test]
    #[ignore] // Requires valid WebSocket URL
    async fn test_websocket_mode() {
        let http_url = std::env::var("RPC_URL").expect("RPC_URL not set");
        let ws_url = std::env::var("RPC_WS_URL").expect("RPC_WS_URL not set");

        let manager = HybridProviderManager::new(http_url, Some(ws_url), ProviderMode::WebSocket)
            .await
            .expect("WebSocket mode should initialize");

        assert!(manager.is_ws_available());
    }

    #[tokio::test]
    #[ignore] // Requires valid URLs
    async fn test_hybrid_mode() {
        let http_url = std::env::var("RPC_URL").expect("RPC_URL not set");
        let ws_url = std::env::var("RPC_WS_URL").ok();

        let manager = HybridProviderManager::new(http_url, ws_url, ProviderMode::Hybrid)
            .await
            .expect("Hybrid mode should initialize");

        // Should work even if WebSocket unavailable
        assert_eq!(manager.mode(), ProviderMode::Hybrid);
    }

    #[tokio::test]
    async fn test_http_provider_always_available() {
        let http_url =
            std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());

        let manager = HybridProviderManager::new(http_url, None, ProviderMode::Http)
            .await
            .expect("HTTP provider should initialize");

        // HTTP provider should always be accessible
        let _provider = manager.http();
    }
}
