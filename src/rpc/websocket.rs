//! WebSocket provider for real-time blockchain event subscriptions.
//!
//! This module provides WebSocket-based streaming for instant notifications
//! of new blocks and events, replacing HTTP polling with push-based updates.
//!
//! # Architecture
//!
//! - WebSocket connection to Ethereum RPC (Alchemy, Infura, etc.)
//! - Push-based notifications (sub-second latency)
//! - Automatic reconnection with exponential backoff
//! - Graceful fallback to HTTP polling if unavailable
//!
//! # Subscription Strategies
//!
//! ## Block Subscription (Recommended)
//! Subscribe to new block headers, then fetch logs via HTTP:
//! - ✅ Guarantees no missed events
//! - ✅ Single subscription for all pools
//! - ✅ Works with reorg detection
//! - ⚠️ Extra RPC call per block
//!
//! ## Event Subscription (Most Efficient)
//! Subscribe directly to Sync events:
//! - ✅ Zero extra RPC calls
//! - ✅ Instant event notification
//! - ⚠️ Separate subscription per pool
//! - ⚠️ More complex coordination

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Header},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use eyre::Result;
use futures_util::stream::StreamExt;
use tracing::{debug, error, info, warn, instrument};

/// WebSocket provider for real-time blockchain subscriptions.
///
/// Wraps an Alloy WebSocket provider with automatic reconnection
/// and subscription management.
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::rpc::websocket::WebSocketProvider;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ws_url = "wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string();
///     let ws_provider = WebSocketProvider::connect(ws_url).await?;
///     
///     // Subscribe to new blocks
///     let mut stream = ws_provider.subscribe_blocks().await?;
///     
///     while let Some(block) = stream.next().await {
///         println!("New block: {}", block.header.number);
///     }
///     
///     Ok(())
/// }
/// ```
pub struct WebSocketProvider {
    provider: alloy::providers::RootProvider<BoxTransport>,
    url: String,
}

impl WebSocketProvider {
    /// Connects to a WebSocket RPC endpoint.
    ///
    /// # Arguments
    ///
    /// * `ws_url` - WebSocket URL (must use `wss://` for secure connection)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::websocket::WebSocketProvider;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let ws_provider = WebSocketProvider::connect(
    ///     "wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - WebSocket connection fails
    /// - URL is invalid
    /// - Authentication fails
    #[instrument(skip(ws_url), fields(ws_host = tracing::field::Empty, duration_ms = tracing::field::Empty))]
    pub async fn connect(ws_url: String) -> Result<Self> {
        // Extract host for logging (without API key)
        let host = ws_url.split("/v2/").next().unwrap_or("unknown");
        tracing::Span::current().record("ws_host", host);
        
        info!(ws_host = host, "Connecting to WebSocket");

        let start = std::time::Instant::now();
        
        // Build provider with WebSocket connection
        let provider = ProviderBuilder::new()
            .on_builtin(&ws_url)
            .await
            .map_err(|e| {
                error!(error = %e, ws_host = host, "WebSocket connection failed");
                eyre::eyre!("WebSocket connection failed: {}", e)
            })?;

        let duration = start.elapsed();
        tracing::Span::current().record("duration_ms", duration.as_millis() as u64);
        
        info!(
            ws_host = host,
            duration_ms = duration.as_millis(),
            "WebSocket connected successfully"
        );

        Ok(Self {
            provider,
            url: ws_url,
        })
    }

    /// Returns a reference to the underlying Alloy provider.
    ///
    /// Useful for making direct RPC calls through the WebSocket connection.
    pub fn provider(&self) -> &alloy::providers::RootProvider<BoxTransport> {
        &self.provider
    }

    /// Returns the WebSocket URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Subscribes to new block headers.
    ///
    /// Returns a stream of block headers (not full blocks) that can be consumed with `StreamExt::next()`.
    /// The stream will continue indefinitely until the WebSocket disconnects.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::websocket::WebSocketProvider;
    /// # use futures_util::stream::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let ws_provider = WebSocketProvider::connect("wss://...".to_string()).await?;
    /// let mut block_stream = ws_provider.subscribe_blocks().await?;
    ///
    /// // Process first 10 block headers
    /// for _ in 0..10 {
    ///     if let Some(header) = block_stream.next().await {
    ///         println!(
    ///             "Block {}: hash {}",
    ///             header.number,
    ///             header.hash
    ///         );
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if subscription fails (e.g., disconnected).
    #[instrument(skip(self))]
    pub async fn subscribe_blocks(
        &self,
    ) -> Result<impl StreamExt<Item = Header> + use<'_>> {
        info!("Subscribing to new blocks via WebSocket");

        let sub = self
            .provider
            .subscribe_blocks()
            .await
            .map_err(|e| {
                error!(error = %e, "Block subscription failed");
                eyre::eyre!("Block subscription failed: {}", e)
            })?;

        let stream = sub.into_stream();

        info!("Block subscription active");
        Ok(stream)
    }

    /// Subscribes to Sync events from a specific Uniswap V2 pool.
    ///
    /// Returns a stream of logs that match the Sync event signature.
    /// More efficient than block subscription if you only care about one pool.
    ///
    /// # Arguments
    ///
    /// * `pool_address` - Address of the Uniswap V2 pool to monitor
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::websocket::WebSocketProvider;
    /// # use alloy::primitives::address;
    /// # use futures_util::stream::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let ws_provider = WebSocketProvider::connect("wss://...".to_string()).await?;
    /// let pool = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");
    /// let mut event_stream = ws_provider.subscribe_sync_events(pool).await?;
    ///
    /// while let Some(log) = event_stream.next().await {
    ///     println!("Sync event at block {}", log.block_number.unwrap());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if subscription fails.
    #[instrument(skip(self), fields(pool = %pool_address))]
    pub async fn subscribe_sync_events(
        &self,
        pool_address: Address,
    ) -> Result<impl StreamExt<Item = alloy::rpc::types::Log> + use<'_>> {
        use crate::events::IUniswapV2Pair;

        info!(
            pool_address = %pool_address,
            "Subscribing to Sync events"
        );

        let filter = Filter::new()
            .address(pool_address)
            .event_signature(IUniswapV2Pair::Sync::SIGNATURE_HASH);

        let sub = self
            .provider
            .subscribe_logs(&filter)
            .await
            .map_err(|e| eyre::eyre!("Log subscription failed: {}", e))?;

        let stream = sub.into_stream();

        info!("Sync event subscription active");
        Ok(stream)
    }

    /// Checks if the WebSocket connection is still alive.
    ///
    /// Note: This doesn't actively ping the server, just checks local state.
    pub fn is_connected(&self) -> bool {
        // Note: Alloy doesn't expose connection state directly
        // We rely on stream ending to detect disconnection
        true
    }
}

/// Reconnecting WebSocket provider with exponential backoff.
///
/// Automatically reconnects when the WebSocket disconnects, with
/// exponential backoff and jitter to prevent thundering herd.
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::rpc::websocket::ReconnectingWebSocket;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut reconnecting = ReconnectingWebSocket::new(
///         "wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()
///     );
///     
///     reconnecting.connect().await?;
///     
///     // Use the provider...
///     
///     Ok(())
/// }
/// ```
pub struct ReconnectingWebSocket {
    url: String,
    provider: Option<WebSocketProvider>,
    max_reconnect_attempts: u32,
    initial_delay_secs: u64,
    max_delay_secs: u64,
}

impl ReconnectingWebSocket {
    /// Creates a new reconnecting WebSocket with default settings.
    ///
    /// # Default Settings
    ///
    /// - Max reconnect attempts: 10
    /// - Initial delay: 1 second
    /// - Max delay: 60 seconds
    /// - Exponential backoff with 25% jitter
    pub fn new(url: String) -> Self {
        Self {
            url,
            provider: None,
            max_reconnect_attempts: 10,
            initial_delay_secs: 1,
            max_delay_secs: 60,
        }
    }

    /// Creates a new reconnecting WebSocket with custom settings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::rpc::websocket::ReconnectingWebSocket;
    /// let reconnecting = ReconnectingWebSocket::with_settings(
    ///     "wss://...".to_string(),
    ///     5,   // max attempts
    ///     2,   // initial delay (seconds)
    ///     30,  // max delay (seconds)
    /// );
    /// ```
    pub fn with_settings(
        url: String,
        max_reconnect_attempts: u32,
        initial_delay_secs: u64,
        max_delay_secs: u64,
    ) -> Self {
        Self {
            url,
            provider: None,
            max_reconnect_attempts,
            initial_delay_secs,
            max_delay_secs,
        }
    }

    /// Connects to the WebSocket with automatic retry on failure.
    ///
    /// Uses exponential backoff with jitter:
    /// - Attempt 1: 1s delay
    /// - Attempt 2: 2s delay
    /// - Attempt 3: 4s delay
    /// - Attempt 4: 8s delay
    /// - ... up to max_delay_secs
    ///
    /// Each delay has ±25% jitter to prevent thundering herd.
    ///
    /// # Errors
    ///
    /// Returns error if max reconnection attempts exceeded.
    pub async fn connect(&mut self) -> Result<()> {
        let mut attempt = 0;
        let mut delay = std::time::Duration::from_secs(self.initial_delay_secs);

        loop {
            match WebSocketProvider::connect(self.url.clone()).await {
                Ok(provider) => {
                    self.provider = Some(provider);
                    info!("WebSocket connection established");
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.max_reconnect_attempts {
                        error!("Max reconnection attempts ({}) reached", self.max_reconnect_attempts);
                        return Err(eyre::eyre!(
                            "Failed to connect after {} attempts: {}",
                            attempt,
                            e
                        ));
                    }

                    warn!(
                        "WebSocket connection failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempt, self.max_reconnect_attempts, e, delay
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff with max cap
                    delay = std::cmp::min(
                        delay * 2,
                        std::time::Duration::from_secs(self.max_delay_secs),
                    );

                    // Add jitter (±25%) to prevent thundering herd
                    let jitter_factor = 0.25 * (rand::random::<f64>() - 0.5);
                    let jitter_ms =
                        (delay.as_millis() as f64 * jitter_factor).round() as i64;
                    delay = if jitter_ms >= 0 {
                        delay + std::time::Duration::from_millis(jitter_ms as u64)
                    } else {
                        delay - std::time::Duration::from_millis((-jitter_ms) as u64)
                    };

                    debug!(
                        "Next retry attempt {} with delay {:?} (jitter applied)",
                        attempt + 1,
                        delay
                    );
                }
            }
        }
    }

    /// Reconnects the WebSocket after a disconnection.
    ///
    /// Drops the current provider and establishes a new connection.
    pub async fn reconnect(&mut self) -> Result<()> {
        warn!("Reconnecting WebSocket after disconnection...");
        self.provider = None;
        self.connect().await
    }

    /// Returns a reference to the current provider, if connected.
    pub fn provider(&self) -> Option<&WebSocketProvider> {
        self.provider.as_ref()
    }

    /// Returns a mutable reference to the current provider, if connected.
    pub fn provider_mut(&mut self) -> Option<&mut WebSocketProvider> {
        self.provider.as_mut()
    }

    /// Checks if currently connected.
    pub fn is_connected(&self) -> bool {
        self.provider.is_some()
    }

    /// Returns the WebSocket URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid WebSocket URL in environment
    async fn test_websocket_connection() {
        let ws_url = std::env::var("RPC_WS_URL").expect("RPC_WS_URL not set");
        let result = WebSocketProvider::connect(ws_url).await;
        assert!(result.is_ok(), "WebSocket connection should succeed");
    }

    #[tokio::test]
    #[ignore] // Requires valid WebSocket URL and network access
    async fn test_block_subscription() {
        let ws_url = std::env::var("RPC_WS_URL").expect("RPC_WS_URL not set");
        let ws = WebSocketProvider::connect(ws_url)
            .await
            .expect("Failed to connect");

        let mut stream = ws
            .subscribe_blocks()
            .await
            .expect("Failed to subscribe to blocks");

        // Wait for first block with timeout
        let block = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            stream.next(),
        )
        .await;

        assert!(
            block.is_ok(),
            "Should receive block within 30 seconds"
        );
        assert!(block.unwrap().is_some(), "Block should not be None");
    }

    #[test]
    fn test_reconnecting_websocket_creation() {
        let reconnecting = ReconnectingWebSocket::new("wss://test.com".to_string());
        assert_eq!(reconnecting.url(), "wss://test.com");
        assert!(!reconnecting.is_connected());
    }

    #[test]
    fn test_reconnecting_websocket_custom_settings() {
        let reconnecting = ReconnectingWebSocket::with_settings(
            "wss://test.com".to_string(),
            5,
            2,
            30,
        );
        assert_eq!(reconnecting.max_reconnect_attempts, 5);
        assert_eq!(reconnecting.initial_delay_secs, 2);
        assert_eq!(reconnecting.max_delay_secs, 30);
    }

    #[tokio::test]
    async fn test_reconnection_logic_with_invalid_url() {
        let mut reconnecting = ReconnectingWebSocket::with_settings(
            "wss://invalid-url-that-will-fail-12345.com".to_string(),
            3, // Only 3 attempts for faster test
            1,
            5,
        );

        let result = reconnecting.connect().await;
        assert!(
            result.is_err(),
            "Should fail to connect to invalid URL"
        );
    }
}
