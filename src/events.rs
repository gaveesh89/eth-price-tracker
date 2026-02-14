//! Event handling for Uniswap V2 Sync events with compile-time type safety.
//!
//! This module uses Alloy's `sol!` macro to generate type-safe event structures
//! directly from Solidity signatures. This approach provides:
//!
//! ## Why Use the `sol!` Macro?
//!
//! **Compile-Time Safety:**
//! - Event signatures are validated at compile time
//! - Type mismatches are caught before runtime
//! - No manual ABI parsing or version drift
//!
//! **Automatic Decoding:**
//! - Alloy handles all topic and data decoding
//! - No manual bit manipulation or keccak hashing
//! - Correct handling of indexed vs non-indexed parameters
//!
//! **Zero-Cost Abstractions:**
//! - Generated code is as efficient as manual parsing
//! - No runtime overhead for type safety
//! - Direct mapping to Ethereum's event encoding
//!
//! **Maintainability:**
//! - Single source of truth (Solidity signature)
//! - Changes to event structure require recompilation
//! - Self-documenting code
//!
//! ## Manual Decoding Problems (What We Avoid)
//!
//! ❌ **Manual approach requires:**
//! - Hard-coded event signatures (prone to typos)
//! - Manual topic extraction by index
//! - Manual ABI decoding of data fields
//! - Runtime validation of types
//! - Keeping ABI JSON files in sync
//!
//! ✅ **`sol!` macro provides:**
//! - Compile-time validated signatures
//! - Automatic topic extraction
//! - Automatic ABI encoding/decoding
//! - Type safety guaranteed by compiler
//! - No external ABI files needed
//!
//! ## Example
//!
//! ```no_run
//! use eth_uniswap_alloy::events::{IUniswapV2Pair, create_sync_filter, Sync};
//! use eth_uniswap_alloy::rpc::create_provider;
//! use alloy::providers::Provider;
//! use alloy::sol_types::SolEvent;
//! use alloy::primitives::LogData;
//!
//! # async fn example() {
//! # let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/API_KEY").await.unwrap();
//! let filter = create_sync_filter(10_000_000, 10_001_000);
//!
//! # let logs = provider.get_logs(&filter).await.unwrap();
//! # for log in logs {
//!     // Automatic decoding using the generated Sync event type
//!     let log_data = LogData::new_unchecked(
//!         log.topics().to_vec(),
//!         log.data().data.clone()
//!     );
//!     if let Ok(decoded) = Sync::decode_log_data(&log_data, true) {
//!         println!("Reserve0: {}, Reserve1: {}", decoded.reserve0, decoded.reserve1);
//!     }
//! }
//! # }
//! ```

use alloy::primitives::{address, Address};
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;

// Generate Uniswap V2 Pair contract interface using the sol! macro.
// The macro creates type-safe Rust bindings with automatic ABI encoding/decoding.
sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        /// Emitted when reserves are synchronized.
        ///
        /// This event is emitted after every swap, mint, or burn operation
        /// to keep the reserves in sync with the actual token balances.
        ///
        /// # Fields
        /// - `reserve0`: Updated reserve for token0
        /// - `reserve1`: Updated reserve for token1
        event Sync(uint112 reserve0, uint112 reserve1);
    }
}

// Re-export the generated types for easier access
pub use IUniswapV2Pair::Sync;

/// Uniswap V2 WETH/USDT Pair contract address on Ethereum mainnet.
///
/// This is the canonical WETH/USDT pair on Uniswap V2.
/// - Token0: WETH (Wrapped Ether)
/// - Token1: USDT (Tether USD)
pub const UNISWAP_V2_WETH_USDT_PAIR: Address = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");

/// WETH (Wrapped Ether) token address on Ethereum mainnet.
///
/// This is the official WETH contract address.
pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

/// USDT (Tether USD) token address on Ethereum mainnet.
///
/// This is the official USDT contract address.
pub const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");

/// Create a typed filter for Sync events from the WETH/USDT pair.
///
/// This function creates an Alloy `Filter` that will match Sync events
/// emitted by the Uniswap V2 WETH/USDT pair within the specified block range.
///
/// ## Type Safety
///
/// The filter uses the compile-time generated event signature from the `sol!` macro,
/// ensuring that the event signature is always correct and matches the Solidity definition.
///
/// ## Arguments
///
/// * `from_block` - Starting block number (inclusive)
/// * `to_block` - Ending block number (inclusive)
///
/// ## Returns
///
/// A typed `Filter` that can be passed directly to `provider.get_logs()`.
///
/// ## Example
///
/// ```no_run
/// use eth_uniswap_alloy::events::create_sync_filter;
/// use eth_uniswap_alloy::rpc::create_provider;
/// use alloy::providers::Provider;
///
/// # async fn example() {
/// # let provider = create_provider("https://eth-mainnet.g.alchemy.com/v2/API_KEY").await.unwrap();
/// let filter = create_sync_filter(19_000_000, 19_001_000);
/// # let logs = provider.get_logs(&filter).await.unwrap();
/// println!("Found {} Sync events", logs.len());
/// # }
/// ```
#[must_use]
pub fn create_sync_filter(from_block: u64, to_block: u64) -> Filter {
    Filter::new()
        .address(UNISWAP_V2_WETH_USDT_PAIR)
        .event_signature(Sync::SIGNATURE_HASH)
        .from_block(from_block)
        .to_block(to_block)
}

/// Create a typed filter for Sync events from a custom pair address.
///
/// This function allows filtering Sync events from any Uniswap V2 pair,
/// not just the WETH/USDT pair. Useful for extending to multiple pools.
///
/// ## Arguments
///
/// * `pair_address` - The address of the Uniswap V2 pair contract
/// * `from_block` - Starting block number (inclusive)
/// * `to_block` - Ending block number (inclusive)
///
/// ## Returns
///
/// A typed `Filter` configured for the specified pair address.
///
/// ## Example
///
/// ```no_run
/// use eth_uniswap_alloy::events::create_sync_filter_for_pair;
/// use alloy::primitives::address;
///
/// # fn example() {
/// let custom_pair = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");
/// let filter = create_sync_filter_for_pair(custom_pair, 19_000_000, 19_001_000);
/// # }
/// ```
#[must_use]
pub fn create_sync_filter_for_pair(
    pair_address: Address,
    from_block: u64,
    to_block: u64,
) -> Filter {
    Filter::new()
        .address(pair_address)
        .event_signature(Sync::SIGNATURE_HASH)
        .from_block(from_block)
        .to_block(to_block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::LogData;

    #[test]
    fn test_sync_event_signature() {
        // The sol! macro generates the correct event signature hash
        // For Sync(uint112,uint112), the signature should be a valid B256
        assert_eq!(Sync::SIGNATURE_HASH.len(), 32);

        // Verify the signature is deterministic (same every time)
        let sig1 = Sync::SIGNATURE_HASH;
        let sig2 = IUniswapV2Pair::Sync::SIGNATURE_HASH;
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_filter_creation() {
        let filter = create_sync_filter(1000, 2000);

        // Verify filter has the correct components
        // Note: Filter structure may not expose all internals for testing
        // We just verify it compiles and returns a Filter
        let _ = filter;
    }

    #[test]
    fn test_filter_creation_custom_pair() {
        let custom_address = address!("0000000000000000000000000000000000000001");
        let filter = create_sync_filter_for_pair(custom_address, 5000, 6000);

        // Verify the filter compiles and is created
        let _ = filter;
    }

    #[test]
    fn test_constants() {
        // Verify addresses are well-formed (not zero)
        assert_ne!(UNISWAP_V2_WETH_USDT_PAIR, Address::ZERO);
        assert_ne!(WETH_ADDRESS, Address::ZERO);
        assert_ne!(USDT_ADDRESS, Address::ZERO);

        // Verify addresses are different
        assert_ne!(WETH_ADDRESS, USDT_ADDRESS);
        assert_ne!(UNISWAP_V2_WETH_USDT_PAIR, WETH_ADDRESS);
        assert_ne!(UNISWAP_V2_WETH_USDT_PAIR, USDT_ADDRESS);
    }

    #[test]
    fn test_sync_event_decode_structure() {
        // This test verifies that the generated Sync struct has the expected fields
        // The sol! macro generates Uint<112, 2> for uint112 types
        use alloy::primitives::Uint;

        // Create a mock Sync event (this proves the struct is generated correctly)
        let _mock_sync = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::from(2_000_000),
        };
    }

    #[tokio::test]
    #[ignore = "Requires RPC connection to test actual log decoding"]
    async fn test_sync_event_decoding_integration() {
        // This test would fetch real logs and decode them
        // Left as an integration test template
        use crate::rpc::create_provider;
        use alloy::providers::Provider;

        let rpc_url = std::env::var("ALCHEMY_API_KEY").map_or_else(
            |_| "http://localhost:8545".to_string(),
            |key| format!("https://eth-mainnet.g.alchemy.com/v2/{key}"),
        );

        if let Ok(provider) = create_provider(&rpc_url).await {
            // Fetch a small range of blocks known to have Sync events
            let filter = create_sync_filter(19_000_000, 19_000_010);
            if let Ok(logs) = provider.get_logs(&filter).await {
                for log in logs {
                    // Use the generated decode_log method
                    let log_data =
                        LogData::new_unchecked(log.topics().to_vec(), log.data().data.clone());
                    if let Ok(decoded) = Sync::decode_log_data(&log_data, true) {
                        // Verify reserves are non-zero (sanity check)
                        // Note: reserves are Uint<112, 2> not U256
                        assert!(!decoded.reserve0.is_zero());
                        assert!(!decoded.reserve1.is_zero());
                    }
                }
            }
        }
    }
}
