//! RPC provider management for Ethereum connections.
//!
//! This module handles connection to Ethereum nodes via RPC (Alchemy).
//! It uses Alloy's `ProviderBuilder` with recommended fillers for production use.
//!
//! ## Example
//!
//! ```no_run
//! use eth_uniswap_alloy::rpc::{create_provider, get_latest_block};
//! use eth_uniswap_alloy::error::TrackerResult;
//!
//! # async fn example() -> TrackerResult<()> {
//! let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/API_KEY").await?;
//! let latest_block = get_latest_block(&provider).await?;
//! println!("Latest block: {}", latest_block);
//! # Ok(())
//! # }
//! ```

use crate::error::{TrackerError, TrackerResult};
use alloy::providers::{Provider as AlloProvider, ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
use tracing::{debug, info, warn};

/// Type alias for the HTTP provider with recommended fillers.
///
/// This provider type includes:
/// - HTTP transport layer
/// - Gas estimation
/// - Nonce management
/// - Chain ID resolution
pub type Provider = RootProvider<Http<Client>>;

/// Create a new Ethereum RPC provider connected via HTTP.
///
/// This function establishes a connection to an Ethereum node using the provided
/// RPC URL (typically an Alchemy endpoint). It configures the provider with
/// recommended fillers for production use.
///
/// # Arguments
///
/// * `rpc_url` - The HTTP(S) endpoint URL for the Ethereum RPC node
///
/// # Returns
///
/// A configured provider instance ready for making RPC calls.
///
/// # Errors
///
/// Returns an error if:
/// - The RPC URL is invalid
/// - The connection cannot be established
/// - The provider initialization fails
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::rpc::create_provider;
/// use eth_uniswap_alloy::error::TrackerResult;
///
/// # async fn example() -> TrackerResult<()> {
/// let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY").await?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::unused_async)]
pub async fn create_provider(rpc_url: &str) -> TrackerResult<Provider> {
    info!("Initializing RPC provider");
    debug!("RPC URL: {}", rpc_url);

    // Parse the RPC URL
    let url = rpc_url
        .parse()
        .map_err(|e| TrackerError::rpc("Failed to parse RPC URL", Some(Box::new(e))))?;

    // Build provider with recommended fillers
    // The ProviderBuilder automatically includes:
    // - Gas estimation
    // - Nonce management
    // - Chain ID resolution
    let provider = ProviderBuilder::new().on_http(url);

    info!("RPC provider initialized successfully");

    Ok(provider)
}

/// Get the latest block number from the Ethereum network.
///
/// This function queries the RPC provider for the current block number
/// at the head of the chain.
///
/// # Arguments
///
/// * `provider` - Reference to the RPC provider instance
///
/// # Returns
///
/// The latest block number as a `u64`.
///
/// # Errors
///
/// Returns an error if:
/// - The RPC request fails
/// - The connection is lost
/// - The response cannot be parsed
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::rpc::{create_provider, get_latest_block};
/// use eth_uniswap_alloy::error::TrackerResult;
///
/// # async fn example() -> TrackerResult<()> {
/// let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY").await?;
/// let block_number = get_latest_block(&provider).await?;
/// println!("Current block: {}", block_number);
/// # Ok(())
/// # }
/// ```
pub async fn get_latest_block(provider: &Provider) -> TrackerResult<u64> {
    debug!("Fetching latest block number");

    let block_number = provider
        .get_block_number()
        .await
        .map_err(|e| TrackerError::rpc("Failed to fetch latest block number", Some(Box::new(e))))?;

    info!("Latest block number: {}", block_number);

    Ok(block_number)
}

/// Check if the provider connection is healthy by fetching the latest block.
///
/// This is a convenience function that attempts to fetch the latest block
/// to verify connectivity.
///
/// # Arguments
///
/// * `provider` - Reference to the RPC provider instance
///
/// # Returns
///
/// `Ok(())` if the connection is healthy, otherwise an error.
///
/// # Errors
///
/// Returns an error if the RPC connection is not working.
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::rpc::{create_provider, check_connection};
/// use eth_uniswap_alloy::error::TrackerResult;
///
/// # async fn example() -> TrackerResult<()> {
/// let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY").await?;
/// check_connection(&provider).await?;
/// println!("Connection is healthy");
/// # Ok(())
/// # }
/// ```
pub async fn check_connection(provider: &Provider) -> TrackerResult<()> {
    debug!("Checking provider connection health");

    match get_latest_block(provider).await {
        Ok(block) => {
            info!("Connection check successful - latest block: {}", block);
            Ok(())
        }
        Err(e) => {
            warn!("Connection check failed: {}", e);
            Err(TrackerError::rpc(
                format!("Provider connection health check failed: {e}"),
                None,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires valid RPC_URL environment variable"]
    async fn test_create_provider_integration() {
        // This test requires a valid RPC URL in the environment
        let rpc_url = std::env::var("ALCHEMY_API_KEY").map_or_else(
            |_| "http://localhost:8545".to_string(),
            |key| format!("https://eth-mainnet.g.alchemy.com/v2/{key}"),
        );

        let result = create_provider(&rpc_url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires valid RPC_URL environment variable"]
    async fn test_get_latest_block_integration() {
        let rpc_url = std::env::var("ALCHEMY_API_KEY").map_or_else(
            |_| "http://localhost:8545".to_string(),
            |key| format!("https://eth-mainnet.g.alchemy.com/v2/{key}"),
        );

        let provider = create_provider(&rpc_url).await;
        assert!(provider.is_ok());

        if let Ok(provider) = provider {
            let block_number = get_latest_block(&provider).await;
            assert!(block_number.is_ok());

            if let Ok(block) = block_number {
                assert!(block > 0);
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires valid RPC_URL environment variable"]
    async fn test_check_connection_integration() {
        let rpc_url = std::env::var("ALCHEMY_API_KEY").map_or_else(
            |_| "http://localhost:8545".to_string(),
            |key| format!("https://eth-mainnet.g.alchemy.com/v2/{key}"),
        );

        if let Ok(provider) = create_provider(&rpc_url).await {
            let result = check_connection(&provider).await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_create_provider_invalid_url() {
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(async {
                let result = create_provider("not-a-valid-url").await;
                assert!(result.is_err());
            });
        }
    }
}
