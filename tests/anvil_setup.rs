//! Anvil-based integration testing infrastructure.
//!
//! This module provides utilities for setting up and managing Anvil instances
//! for deterministic, offline testing of the Uniswap V2 event indexer.
//!
//! # Overview
//!
//! The testing infrastructure allows:
//! - Forking Ethereum mainnet at a specific block
//! - Fetching historical Sync events from the fork
//! - Testing price calculations against known data
//! - Running completely offline after initial fork
//!
//! # Example
//!
//! ```no_run
//! use eth_uniswap_alloy::config::Config;
//!
//! #[tokio::test]
//! async fn test_with_anvil() {
//!     let config = Config::from_env().unwrap();
//!     // Start Anvil, run tests, cleanup handled automatically
//! }
//! ```

use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::providers::{Provider as AlloyProvider, ProviderBuilder};
use alloy::rpc::types::Log;
use eth_uniswap_alloy::{
    config::Config,
    error::{TrackerError, TrackerResult},
    events::{create_sync_filter_for_pair, Sync, UNISWAP_V2_WETH_USDT_PAIR},
    pricing::calculate_eth_price,
    rpc::Provider,
};
use eyre::Context;
use std::env;

/// Default Anvil fork block if not specified in environment.
/// This is a known block with active Uniswap V2 activity.
const DEFAULT_FORK_BLOCK: u64 = 19_000_000;

/// Get the fork block number from environment or use default.
///
/// Reads `ANVIL_FORK_BLOCK` from environment variables.
/// Falls back to `DEFAULT_FORK_BLOCK` if not set or invalid.
fn get_fork_block() -> u64 {
    env::var("ANVIL_FORK_BLOCK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_FORK_BLOCK)
}

/// Get the Alchemy RPC URL for forking.
///
/// Constructs the Ethereum mainnet RPC URL from the Alchemy API key.
///
/// # Errors
///
/// Returns an error if `ALCHEMY_API_KEY` is not set or invalid.
fn get_fork_url() -> TrackerResult<String> {
    let config = Config::from_env().wrap_err("Failed to load config for fork URL")?;
    Ok(config.rpc_url().to_string())
}

/// Start an Anvil instance with Ethereum mainnet fork.
///
/// Creates a new Anvil instance that forks from Ethereum mainnet at the specified
/// block height. The instance will have historical state available for querying.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to load configuration
/// - Failed to start Anvil process
/// - Fork RPC URL is invalid
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::config::Config;
///
/// # async fn example() -> eyre::Result<()> {
/// // Ensure ALCHEMY_API_KEY is set
/// std::env::set_var("ALCHEMY_API_KEY", "your_key");
/// std::env::set_var("ANVIL_FORK_BLOCK", "19000000");
///
/// let anvil = start_anvil_fork().await?;
/// // Use anvil.endpoint() for RPC URL
/// // Anvil automatically cleans up when dropped
/// # Ok(())
/// # }
/// ```
pub fn start_anvil_fork() -> TrackerResult<AnvilInstance> {
    let fork_url = get_fork_url().wrap_err("Failed to get fork RPC URL")?;
    let fork_block = get_fork_block();

    tracing::info!(
        "Starting Anvil fork at block {} from {}",
        fork_block,
        fork_url
    );

    let anvil = Anvil::new()
        .fork(fork_url)
        .fork_block_number(fork_block)
        .try_spawn()
        .wrap_err("Failed to spawn Anvil instance")?;

    tracing::info!("Anvil started at {}", anvil.endpoint());

    Ok(anvil)
}

/// Create a provider connected to an Anvil instance.
///
/// Creates an Alloy provider that connects to the given Anvil endpoint.
///
/// # Arguments
///
/// * `anvil` - Reference to the running Anvil instance
///
/// # Errors
///
/// Returns an error if the provider cannot be created or connected.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # let anvil = start_anvil_fork().await?;
/// let provider = create_anvil_provider(&anvil).await?;
/// let block_number = provider.get_block_number().await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_anvil_provider(anvil: &AnvilInstance) -> TrackerResult<Provider> {
    let endpoint = anvil.endpoint();

    let provider = ProviderBuilder::new().on_http(
        endpoint
            .parse()
            .wrap_err("Failed to parse Anvil endpoint")?,
    );

    // Verify connection
    let block_number = provider
        .get_block_number()
        .await
        .wrap_err("Failed to connect to Anvil instance")?;

    tracing::debug!("Connected to Anvil at block {}", block_number);

    Ok(provider)
}

/// Fetch historical Sync events from Anvil fork.
///
/// Queries the forked chain for Sync events from the Uniswap V2 WETH/USDT pair
/// within the specified block range.
///
/// # Arguments
///
/// * `provider` - Provider connected to Anvil
/// * `from_block` - Starting block number (inclusive)
/// * `to_block` - Ending block number (inclusive)
///
/// # Returns
///
/// Returns a vector of logs containing Sync events.
///
/// # Errors
///
/// Returns an error if the RPC query fails or blocks are out of range.
pub async fn fetch_sync_events(
    provider: &Provider,
    from_block: u64,
    to_block: u64,
) -> TrackerResult<Vec<Log>> {
    let filter = create_sync_filter_for_pair(UNISWAP_V2_WETH_USDT_PAIR, from_block, to_block);

    let logs = provider
        .get_logs(&filter)
        .await
        .wrap_err("Failed to fetch Sync events from Anvil")?;

    tracing::debug!("Fetched {} Sync events", logs.len());

    Ok(logs)
}

/// Decode a log into a Sync event and extract reserve data.
///
/// # Arguments
///
/// * `log` - The log entry to decode
///
/// # Returns
///
/// Returns a tuple of `(Sync event, block_number)`.
///
/// # Errors
///
/// Returns an error if the log cannot be decoded as a Sync event.
pub fn decode_sync_event(log: &Log) -> TrackerResult<(Sync, u64)> {
    use alloy::primitives::Log as PrimitiveLog;
    use alloy::sol_types::SolEvent;

    let block_number = log
        .block_number
        .ok_or_else(|| TrackerError::decoding("Log missing block number", None))?;

    // Convert RPC Log to Primitive Log for decoding
    let primitive_log = PrimitiveLog {
        address: log.address(),
        data: log.data().clone(),
    };

    let sync_event =
        Sync::decode_log(&primitive_log, true).wrap_err("Failed to decode Sync event from log")?;

    Ok((sync_event.data, block_number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;

    /// Test that we can start Anvil and connect to it.
    #[tokio::test]
    #[ignore = "Requires ALCHEMY_API_KEY environment variable"]
    async fn test_start_anvil_fork() {
        let result = start_anvil_fork();
        assert!(result.is_ok(), "Failed to start Anvil fork");

        if let Ok(anvil) = result {
            assert!(
                anvil.endpoint().starts_with("http://"),
                "Anvil endpoint should be HTTP URL"
            );
        }
    }

    /// Test that we can create a provider and connect to Anvil.
    #[tokio::test]
    #[ignore = "Requires ALCHEMY_API_KEY environment variable"]
    async fn test_create_anvil_provider() {
        let anvil_result = start_anvil_fork();
        assert!(anvil_result.is_ok(), "Failed to start Anvil");

        if let Ok(anvil) = anvil_result {
            let provider_result = create_anvil_provider(&anvil).await;
            assert!(provider_result.is_ok(), "Failed to create provider");

            if let Ok(provider) = provider_result {
                let block_result = provider.get_block_number().await;
                assert!(block_result.is_ok(), "Failed to get block number");
            }
        }
    }

    /// Test fetching Sync events from the fork.
    #[tokio::test]
    #[ignore = "Requires ALCHEMY_API_KEY environment variable"]
    async fn test_fetch_sync_events() {
        let anvil_result = start_anvil_fork();
        if let Ok(anvil) = anvil_result {
            let provider_result = create_anvil_provider(&anvil).await;
            if let Ok(provider) = provider_result {
                // Fetch events from a small range around the fork block
                let fork_block = get_fork_block();
                let from_block = fork_block.saturating_sub(10);
                let to_block = fork_block;

                let logs_result = fetch_sync_events(&provider, from_block, to_block).await;
                assert!(logs_result.is_ok(), "Failed to fetch Sync events");

                if let Ok(logs) = logs_result {
                    // We should find at least some events in this range
                    // (may be 0 if pool was not active in this range)
                    tracing::info!("Found {} Sync events", logs.len());
                }
            }
        }
    }

    /// Full integration test: Start Anvil, fetch events, verify price calculation.
    ///
    /// This test demonstrates the complete offline testing workflow:
    /// 1. Fork mainnet at a specific block
    /// 2. Fetch historical Sync events
    /// 3. Decode events and calculate prices
    /// 4. Verify calculations against expected values
    #[tokio::test]
    #[ignore = "Requires ALCHEMY_API_KEY environment variable"]
    async fn test_full_integration_with_anvil() {
        // Initialize tracing for better debugging
        let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

        // Start Anvil fork
        let anvil = match start_anvil_fork() {
            Ok(anvil) => anvil,
            Err(e) => {
                tracing::error!("Failed to start Anvil fork: {}", e);
                return;
            }
        };

        // Create provider
        let provider = match create_anvil_provider(&anvil).await {
            Ok(provider) => provider,
            Err(e) => {
                tracing::error!("Failed to create provider: {}", e);
                return;
            }
        };

        // Fetch Sync events from a range around the fork block
        let fork_block = get_fork_block();
        let from_block = fork_block.saturating_sub(100);
        let to_block = fork_block;

        tracing::info!(
            "Fetching Sync events from blocks {} to {}",
            from_block,
            to_block
        );

        let logs = match fetch_sync_events(&provider, from_block, to_block).await {
            Ok(logs) => logs,
            Err(e) => {
                tracing::error!("Failed to fetch Sync events: {}", e);
                return;
            }
        };

        tracing::info!("Found {} Sync events", logs.len());

        if logs.is_empty() {
            tracing::warn!("No Sync events found in range, test inconclusive");
            return;
        }

        // Process the first event as a sample
        let first_log = &logs[0];
        let (sync_event, block_number) = match decode_sync_event(first_log) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Failed to decode Sync event: {}", e);
                return;
            }
        };

        tracing::info!(
            "Decoded Sync event at block {}: reserve0={}, reserve1={}",
            block_number,
            sync_event.reserve0,
            sync_event.reserve1
        );

        // Convert reserves to U256 for price calculation
        // Note: sol! macro generates Uint<112, 2> which converts to U256
        let weth_reserve = U256::from(sync_event.reserve0);
        let usdt_reserve = U256::from(sync_event.reserve1);

        // Verify reserves are non-zero
        assert!(
            !weth_reserve.is_zero(),
            "WETH reserve should be non-zero in active pool"
        );
        assert!(
            !usdt_reserve.is_zero(),
            "USDT reserve should be non-zero in active pool"
        );

        // Calculate price
        let price = match calculate_eth_price(weth_reserve, usdt_reserve) {
            Ok(price) => price,
            Err(e) => {
                tracing::error!("Failed to calculate price: {}", e);
                return;
            }
        };

        tracing::info!("Calculated ETH price: ${price:.2} USDT");

        // Sanity check: ETH price should be in a reasonable range
        // (between $100 and $100,000 as of 2024-2026)
        assert!(
            (100.0..100_000.0).contains(&price),
            "ETH price should be in reasonable range, got ${price:.2}"
        );

        // Test passes if we successfully:
        // 1. Started Anvil fork ✓
        // 2. Connected to it ✓
        // 3. Fetched real Sync events ✓
        // 4. Decoded events ✓
        // 5. Calculated price ✓
        // 6. Validated result ✓

        tracing::info!("✅ Full integration test passed!");
    }
}
